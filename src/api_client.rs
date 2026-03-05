use std::fmt;

use anyhow::{Context, Result};
use reqwest::blocking::{RequestBuilder, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::config::Config;
use crate::credential_store::CredentialStore;
use crate::types::{
    AgentMessageResponse, AgentSSEEvent, AgentSessionResponse, BetaWaitlistEntry,
    BetaWaitlistListResponse, BetaWhitelistEntry, BookmarkResponse, CodeSearchResultPage,
    CommitStatusResponse, CreateAgentSessionInput, CreateBookmarkInput, CreateIssueInput,
    CreateLabelInput, CreateReleaseInput, CreateSshKeyInput, DispatchWorkflowInput, IssueResponse,
    IssueSearchResultPage, LabelResponse, LandingConflictsResponse, LandingRequestChange,
    LandingRequestResponse, LandingRequestReview, PostAgentMessageInput, RawApiResponse,
    ReleaseResponse, RepoDetailResponse, RepoSummaryResponse, RepositorySearchResultPage,
    RerunWorkflowRunInput, SecretResponse, SetSecretInput, SetVariableInput, SshKeyResponse,
    UpdateIssueInput, VariableResponse, WorkflowDefinitionResponse, WorkflowRunRerunResponse,
    WorkflowRunResponse,
};

/// HTTP client for the Plue API.
pub struct ApiClient {
    base_url: String,
    token: String,
    client: reqwest::blocking::Client,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API {}: {}", self.status, self.message)
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct CreateRepoRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    private: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRepoResponse {
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub is_public: bool,
    #[serde(alias = "default_branch")]
    pub default_bookmark: String,
    pub clone_url: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowListEnvelope {
    workflows: Vec<WorkflowDefinitionResponse>,
}

#[derive(Debug, Serialize)]
pub struct CreateLandingRequestInput {
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub body: String,
    pub target_bookmark: String,
    pub change_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateLandingReviewInput {
    #[serde(rename = "type")]
    pub review_type: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct AddBetaWhitelistEntryRequest {
    pub identity_type: String,
    pub identity_value: String,
}

#[derive(Debug, Serialize)]
pub struct ApproveBetaWaitlistRequest {
    pub email: String,
}

#[derive(Default, Deserialize)]
struct ApiErrorResponse {
    message: Option<String>,
    error: Option<String>,
}

impl ApiClient {
    /// Create an ApiClient from the loaded config, using the default
    /// credential store (OS keychain).
    pub fn from_config(config: &Config) -> Result<Self> {
        let store = CredentialStore::new();
        Self::from_config_with_store(config, &store)
    }

    /// Create an ApiClient with an explicit credential store (useful for tests).
    pub fn from_config_with_store(config: &Config, store: &CredentialStore) -> Result<Self> {
        let resolved = config
            .token_for_host(store)?
            .context("not authenticated — run `plue auth login --with-token` or set PLUE_TOKEN")?;

        Ok(Self {
            base_url: config.api_url.clone(),
            token: resolved.token,
            client: reqwest::blocking::Client::new(),
        })
    }

    fn with_auth(&self, req: RequestBuilder) -> RequestBuilder {
        req.header("Authorization", format!("token {}", self.token))
    }

    fn decode_response<T: DeserializeOwned>(resp: Response) -> Result<T> {
        if !resp.status().is_success() {
            return Err(Self::decode_error(resp).into());
        }
        resp.json::<T>().context("failed to parse API response")
    }

    fn decode_error(resp: Response) -> ApiError {
        let status = resp.status().as_u16();
        let fallback = format!("request failed with status {status}");
        let body = resp.text().unwrap_or_default();
        if body.trim().is_empty() {
            return ApiError {
                status,
                message: fallback,
            };
        }

        if let Ok(parsed) = serde_json::from_str::<ApiErrorResponse>(&body) {
            if let Some(message) = parsed.message.or(parsed.error) {
                return ApiError { status, message };
            }
        }

        ApiError {
            status,
            message: body.trim().to_string(),
        }
    }

    /// POST /api/user/repos
    pub fn create_repo(
        &self,
        name: &str,
        description: Option<&str>,
        private: bool,
    ) -> Result<CreateRepoResponse> {
        let url = format!("{}/user/repos", self.base_url);
        let body = CreateRepoRequest {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            private,
        };

        let resp = self
            .with_auth(self.client.post(&url))
            .json(&body)
            .send()
            .context("failed to connect to Plue API")?;

        Self::decode_response(resp)
    }

    pub fn list_repos(
        &self,
        owner: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<RepoSummaryResponse>> {
        let url = match owner {
            Some(org) => format!("{}/orgs/{org}/repos", self.base_url),
            None => format!("{}/user/repos", self.base_url),
        };
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", page), ("per_page", per_page)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn get_repo(&self, owner: &str, repo: &str) -> Result<RepoDetailResponse> {
        let url = format!("{}/repos/{owner}/{repo}", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn list_landing_requests(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<LandingRequestResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/landings", self.base_url);
        let mut query = vec![
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
        if let Some(state) = state {
            query.push(("state".to_string(), state.to_string()));
        }

        let resp = self
            .with_auth(self.client.get(&url))
            .query(&query)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn create_landing_request(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateLandingRequestInput,
    ) -> Result<LandingRequestResponse> {
        let url = format!("{}/repos/{owner}/{repo}/landings", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn get_landing_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<LandingRequestResponse> {
        let url = format!("{}/repos/{owner}/{repo}/landings/{number}", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn list_landing_reviews(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<Vec<LandingRequestReview>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/landings/{number}/reviews",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", 1), ("per_page", 100)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn create_landing_review(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: &CreateLandingReviewInput,
    ) -> Result<LandingRequestReview> {
        let url = format!(
            "{}/repos/{owner}/{repo}/landings/{number}/reviews",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn list_landing_changes(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<Vec<LandingRequestChange>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/landings/{number}/changes",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", 1), ("per_page", 100)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn get_landing_conflicts(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<LandingConflictsResponse> {
        let url = format!(
            "{}/repos/{owner}/{repo}/landings/{number}/conflicts",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn land_landing_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<LandingRequestResponse> {
        let url = format!(
            "{}/repos/{owner}/{repo}/landings/{number}/land",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.put(&url))
            // Some edge proxies reject body-less write verbs.
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn list_commit_statuses(
        &self,
        owner: &str,
        repo: &str,
        reference: &str,
    ) -> Result<Vec<CommitStatusResponse>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/commits/{reference}/statuses",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Issue endpoints ---

    /// GET /api/repos/:owner/:repo/issues
    pub fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<IssueResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/issues", self.base_url);
        let mut query = vec![
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
        if let Some(state) = state {
            query.push(("state".to_string(), state.to_string()));
        }

        let resp = self
            .with_auth(self.client.get(&url))
            .query(&query)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/repos/:owner/:repo/issues
    pub fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateIssueInput,
    ) -> Result<IssueResponse> {
        let url = format!("{}/repos/{owner}/{repo}/issues", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// GET /api/repos/:owner/:repo/issues/:number
    pub fn get_issue(&self, owner: &str, repo: &str, number: i64) -> Result<IssueResponse> {
        let url = format!("{}/repos/{owner}/{repo}/issues/{number}", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// PATCH /api/repos/:owner/:repo/issues/:number
    pub fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: &UpdateIssueInput,
    ) -> Result<IssueResponse> {
        let url = format!("{}/repos/{owner}/{repo}/issues/{number}", self.base_url);
        let resp = self
            .with_auth(self.client.patch(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- SSH key endpoints ---

    /// GET /api/user/keys
    pub fn list_ssh_keys(&self) -> Result<Vec<SshKeyResponse>> {
        let url = format!("{}/user/keys", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/user/keys
    pub fn add_ssh_key(&self, req: &CreateSshKeyInput) -> Result<SshKeyResponse> {
        let url = format!("{}/user/keys", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// DELETE /api/user/keys/:id
    pub fn delete_ssh_key(&self, key_id: i64) -> Result<()> {
        let url = format!("{}/user/keys/{key_id}", self.base_url);
        let resp = self
            .with_auth(self.client.delete(&url))
            .send()
            .context("failed to connect to Plue API")?;
        if resp.status().as_u16() == 204 {
            return Ok(());
        }
        Err(Self::decode_error(resp).into())
    }

    // --- Label endpoints ---

    /// GET /api/repos/{owner}/{repo}/labels
    pub fn list_labels(&self, owner: &str, repo: &str) -> Result<Vec<LabelResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/labels", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/repos/{owner}/{repo}/labels
    pub fn create_label(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateLabelInput,
    ) -> Result<LabelResponse> {
        let url = format!("{}/repos/{owner}/{repo}/labels", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Release endpoints ---

    /// GET /api/repos/{owner}/{repo}/releases
    pub fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<ReleaseResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/releases", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/repos/{owner}/{repo}/releases
    pub fn create_release(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateReleaseInput,
    ) -> Result<ReleaseResponse> {
        let url = format!("{}/repos/{owner}/{repo}/releases", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Search endpoints ---

    /// GET /api/search/repositories?q=...
    pub fn search_repositories(
        &self,
        query: &str,
        page: i32,
        per_page: i32,
    ) -> Result<RepositorySearchResultPage> {
        let url = format!("{}/search/repositories", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[
                ("q", query),
                ("page", &page.to_string()),
                ("per_page", &per_page.to_string()),
            ])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// GET /api/search/issues?q=...
    pub fn search_issues(
        &self,
        query: &str,
        state: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> Result<IssueSearchResultPage> {
        let url = format!("{}/search/issues", self.base_url);
        let mut query_params: Vec<(&str, String)> = vec![
            ("q", query.to_string()),
            ("page", page.to_string()),
            ("per_page", per_page.to_string()),
        ];
        if let Some(s) = state {
            query_params.push(("state", s.to_string()));
        }
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&query_params)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Workflow endpoints ---

    /// GET /api/repos/:owner/:repo/workflows
    pub fn list_workflows(
        &self,
        owner: &str,
        repo: &str,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<WorkflowDefinitionResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/workflows", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", page), ("per_page", per_page)])
            .send()
            .context("failed to connect to Plue API")?;
        if !resp.status().is_success() {
            return Err(Self::decode_error(resp).into());
        }

        let body = resp
            .text()
            .context("failed to read response body for workflow list")?;

        if body.trim().is_empty() {
            return Ok(Vec::new());
        }

        if let Ok(workflows) = serde_json::from_str::<Vec<WorkflowDefinitionResponse>>(&body) {
            return Ok(workflows);
        }

        if let Ok(envelope) = serde_json::from_str::<WorkflowListEnvelope>(&body) {
            return Ok(envelope.workflows);
        }

        anyhow::bail!("failed to parse workflow list response");
    }

    /// POST /api/repos/:owner/:repo/workflows/:id/dispatches
    pub fn dispatch_workflow(
        &self,
        owner: &str,
        repo: &str,
        workflow_id: i64,
        git_ref: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/workflows/{workflow_id}/dispatches",
            self.base_url
        );
        let body = DispatchWorkflowInput {
            git_ref: git_ref.to_string(),
        };
        let resp = self
            .with_auth(self.client.post(&url))
            .json(&body)
            .send()
            .context("failed to connect to Plue API")?;
        if !resp.status().is_success() {
            return Err(Self::decode_error(resp).into());
        }
        Ok(())
    }

    // --- Workflow run endpoints ---

    /// GET /api/repos/:owner/:repo/actions/runs/:id
    pub fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: i64,
    ) -> Result<WorkflowRunResponse> {
        let url = format!(
            "{}/repos/{owner}/{repo}/actions/runs/{run_id}",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// GET /api/repos/:owner/:repo/workflows/:id/runs
    pub fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        workflow_id: i64,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<WorkflowRunResponse>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/workflows/{workflow_id}/runs",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", page), ("per_page", per_page)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/repos/:owner/:repo/actions/runs/:id/rerun
    pub fn rerun_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: i64,
        provider: Option<&str>,
    ) -> Result<WorkflowRunRerunResponse> {
        let url = format!(
            "{}/repos/{owner}/{repo}/actions/runs/{run_id}/rerun",
            self.base_url
        );
        let body = RerunWorkflowRunInput {
            provider: provider.map(|s| s.to_string()),
        };
        let resp = self
            .with_auth(self.client.post(&url))
            .json(&body)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Agent session endpoints ---

    /// POST /api/repos/:owner/:repo/agent/sessions
    pub fn create_agent_session(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateAgentSessionInput,
    ) -> Result<AgentSessionResponse> {
        let url = format!("{}/repos/{owner}/{repo}/agent/sessions", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// GET /api/repos/:owner/:repo/agent/sessions
    pub fn list_agent_sessions(
        &self,
        owner: &str,
        repo: &str,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<AgentSessionResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/agent/sessions", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", page), ("per_page", per_page)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// GET /api/repos/:owner/:repo/agent/sessions/:session_id
    pub fn get_agent_session(
        &self,
        owner: &str,
        repo: &str,
        session_id: &str,
    ) -> Result<AgentSessionResponse> {
        let url = format!(
            "{}/repos/{owner}/{repo}/agent/sessions/{session_id}",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/repos/:owner/:repo/agent/sessions/:session_id/messages
    pub fn post_agent_message(
        &self,
        owner: &str,
        repo: &str,
        session_id: &str,
        req: &PostAgentMessageInput,
    ) -> Result<AgentMessageResponse> {
        let url = format!(
            "{}/repos/{owner}/{repo}/agent/sessions/{session_id}/messages",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// GET /api/repos/:owner/:repo/agent/sessions/:session_id/stream
    /// Opens an SSE connection and calls `on_event` for each event until "done" is received
    /// or the stream closes. Returns Ok(()) on normal completion.
    pub fn stream_agent_session<F>(
        &self,
        owner: &str,
        repo: &str,
        session_id: &str,
        mut on_event: F,
    ) -> Result<()>
    where
        F: FnMut(AgentSSEEvent) -> bool,
    {
        let url = format!(
            "{}/repos/{owner}/{repo}/agent/sessions/{session_id}/stream",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .header("Accept", "text/event-stream")
            .send()
            .context("failed to connect to Plue SSE stream")?;

        if !resp.status().is_success() {
            anyhow::bail!("SSE stream returned status {}", resp.status());
        }

        use std::io::BufRead;
        let reader = std::io::BufReader::new(resp);
        let mut current_event_type = String::from("message");
        let mut current_data = String::new();

        for line in reader.lines() {
            let line = line.context("failed to read SSE line")?;
            if let Some(stripped) = line.strip_prefix("event: ") {
                current_event_type = stripped.to_string();
            } else if let Some(stripped) = line.strip_prefix("data: ") {
                current_data = stripped.to_string();
            } else if line.is_empty() && !current_data.is_empty() {
                let event = AgentSSEEvent {
                    event_type: current_event_type.clone(),
                    data: current_data.clone(),
                };
                let is_done = current_event_type == "done";
                let keep_going = on_event(event);
                current_event_type = String::from("message");
                current_data.clear();
                if is_done || !keep_going {
                    break;
                }
            }
        }
        Ok(())
    }

    /// GET /api/repos/:owner/:repo/agent/sessions/:session_id/messages
    pub fn list_agent_messages(
        &self,
        owner: &str,
        repo: &str,
        session_id: &str,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<AgentMessageResponse>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/agent/sessions/{session_id}/messages",
            self.base_url
        );
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", page), ("per_page", per_page)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Secrets ---
    pub fn list_secrets(&self, owner: &str, repo: &str) -> Result<Vec<SecretResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/secrets", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn set_secret(
        &self,
        owner: &str,
        repo: &str,
        req: &SetSecretInput,
    ) -> Result<SecretResponse> {
        let url = format!("{}/repos/{owner}/{repo}/secrets", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn delete_secret(&self, owner: &str, repo: &str, name: &str) -> Result<()> {
        let url = format!("{}/repos/{owner}/{repo}/secrets/{name}", self.base_url);
        let resp = self
            .with_auth(self.client.delete(&url))
            .send()
            .context("failed to connect to Plue API")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Self::decode_error(resp).into())
        }
    }

    // --- Variables ---
    pub fn list_variables(&self, owner: &str, repo: &str) -> Result<Vec<VariableResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/variables", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn get_variable(&self, owner: &str, repo: &str, name: &str) -> Result<VariableResponse> {
        let url = format!("{}/repos/{owner}/{repo}/variables/{name}", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn set_variable(
        &self,
        owner: &str,
        repo: &str,
        req: &SetVariableInput,
    ) -> Result<VariableResponse> {
        let url = format!("{}/repos/{owner}/{repo}/variables", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn delete_variable(&self, owner: &str, repo: &str, name: &str) -> Result<()> {
        let url = format!("{}/repos/{owner}/{repo}/variables/{name}", self.base_url);
        let resp = self
            .with_auth(self.client.delete(&url))
            .send()
            .context("failed to connect to Plue API")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Self::decode_error(resp).into())
        }
    }

    // --- Search Code ---
    pub fn search_code(
        &self,
        query: &str,
        page: i32,
        per_page: i32,
    ) -> Result<CodeSearchResultPage> {
        let url = format!("{}/search/code", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[
                ("q", query),
                ("page", &page.to_string()),
                ("per_page", &per_page.to_string()),
            ])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Bookmarks (Remote API) ---

    /// GET /api/repos/:owner/:repo/bookmarks
    pub fn list_bookmarks(
        &self,
        owner: &str,
        repo: &str,
        page: i32,
        per_page: i32,
    ) -> Result<Vec<BookmarkResponse>> {
        let url = format!("{}/repos/{owner}/{repo}/bookmarks", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&[("page", page), ("per_page", per_page)])
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// POST /api/repos/:owner/:repo/bookmarks
    pub fn create_bookmark(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateBookmarkInput,
    ) -> Result<BookmarkResponse> {
        let url = format!("{}/repos/{owner}/{repo}/bookmarks", self.base_url);
        let resp = self
            .with_auth(self.client.post(&url))
            .json(req)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    /// DELETE /api/repos/:owner/:repo/bookmarks/:name
    pub fn delete_bookmark(&self, owner: &str, repo: &str, name: &str) -> Result<()> {
        let url = format!("{}/repos/{owner}/{repo}/bookmarks/{name}", self.base_url);
        let resp = self
            .with_auth(self.client.delete(&url))
            .send()
            .context("failed to connect to Plue API")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Self::decode_error(resp).into())
        }
    }

    // --- Label Delete ---
    pub fn delete_label(&self, owner: &str, repo: &str, label_id: i64) -> Result<()> {
        let url = format!("{}/repos/{owner}/{repo}/labels/{label_id}", self.base_url);
        let resp = self
            .with_auth(self.client.delete(&url))
            .send()
            .context("failed to connect to Plue API")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Self::decode_error(resp).into())
        }
    }

    // --- Closed Beta Access ---
    pub fn list_beta_whitelist(&self) -> Result<Vec<BetaWhitelistEntry>> {
        let url = format!("{}/admin/beta/whitelist", self.base_url);
        let resp = self
            .with_auth(self.client.get(&url))
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn add_beta_whitelist_entry(
        &self,
        identity_type: &str,
        identity_value: &str,
    ) -> Result<BetaWhitelistEntry> {
        let url = format!("{}/admin/beta/whitelist", self.base_url);
        let payload = AddBetaWhitelistEntryRequest {
            identity_type: identity_type.to_string(),
            identity_value: identity_value.to_string(),
        };
        let resp = self
            .with_auth(self.client.post(&url))
            .json(&payload)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn remove_beta_whitelist_entry(
        &self,
        identity_type: &str,
        identity_value: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/admin/beta/whitelist/{}/{}",
            self.base_url, identity_type, identity_value
        );
        let resp = self
            .with_auth(self.client.delete(&url))
            .send()
            .context("failed to connect to Plue API")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Self::decode_error(resp).into())
        }
    }

    pub fn list_beta_waitlist(
        &self,
        status: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> Result<BetaWaitlistListResponse> {
        let url = format!("{}/admin/beta/waitlist", self.base_url);
        let mut query: Vec<(String, String)> = vec![
            ("page".to_string(), page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
        if let Some(status) = status {
            query.push(("status".to_string(), status.to_string()));
        }
        let resp = self
            .with_auth(self.client.get(&url))
            .query(&query)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    pub fn approve_beta_waitlist_entry(&self, email: &str) -> Result<BetaWaitlistEntry> {
        let url = format!("{}/admin/beta/waitlist/approve", self.base_url);
        let payload = ApproveBetaWaitlistRequest {
            email: email.to_string(),
        };
        let resp = self
            .with_auth(self.client.post(&url))
            .json(&payload)
            .send()
            .context("failed to connect to Plue API")?;
        Self::decode_response(resp)
    }

    // --- Raw Request ---
    pub fn raw_request(
        &self,
        method: &str,
        endpoint: &str,
        headers: &[(String, String)],
        body: Option<serde_json::Value>,
    ) -> Result<RawApiResponse> {
        let normalized_endpoint = if endpoint.starts_with('/') {
            endpoint.to_string()
        } else {
            format!("/{endpoint}")
        };

        // Health is commonly served at the site root, even when API routes are under /api.
        let url = if normalized_endpoint == "/health" && self.base_url.ends_with("/api") {
            format!(
                "{}{}",
                self.base_url.trim_end_matches("/api"),
                normalized_endpoint
            )
        } else {
            format!("{}{}", self.base_url, normalized_endpoint)
        };
        let method = method.to_uppercase();
        let mut req = match method.as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PATCH" => self.client.patch(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => anyhow::bail!("Unsupported HTTP method: {method}"),
        };
        req = self.with_auth(req);
        for (k, v) in headers {
            req = req.header(k, v);
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().context("failed to execute raw request")?;
        let status = resp.status().as_u16();
        let res_headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body_str = resp.text().unwrap_or_default();
        Ok(RawApiResponse {
            status,
            headers: res_headers,
            body: body_str,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ApiClient;
    use crate::config::Config;
    use crate::credential_store::{CredentialStore, MockStore};
    use std::ffi::OsString;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set_path(key: &'static str, value: &std::path::Path) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }

    #[test]
    fn from_config_with_store_uses_resolved_host_token() {
        let config = Config {
            api_url: "https://custom.plue.dev/api".into(),
            token: Some("plue_config_token".into()),
            ..Config::default()
        };
        let store = CredentialStore::with_backend(Box::new(MockStore::new()));
        store
            .store_token("custom.plue.dev", "plue_keyring_token")
            .unwrap();

        let client = ApiClient::from_config_with_store(&config, &store).unwrap();
        assert_eq!(client.token, "plue_keyring_token");
    }

    #[test]
    fn from_config_uses_default_store_and_test_override_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _clear_token = EnvVarGuard::remove("PLUE_TOKEN");
        let temp = TempDir::new().unwrap();
        let keyring_file = temp.path().join("credential-store.json");
        let _store_file = EnvVarGuard::set_path("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file);

        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: None,
            ..Config::default()
        };
        let store = CredentialStore::new();
        store.store_token("plue.dev", "plue_file_token").unwrap();

        let client = ApiClient::from_config(&config).unwrap();
        assert_eq!(client.token, "plue_file_token");
    }

    #[test]
    fn from_config_with_store_errors_when_no_token_available() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _clear_token = EnvVarGuard::remove("PLUE_TOKEN");

        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: None,
            ..Config::default()
        };
        let store = CredentialStore::with_backend(Box::new(MockStore::new()));

        let err = match ApiClient::from_config_with_store(&config, &store) {
            Ok(_) => panic!("expected unauthenticated error"),
            Err(err) => err,
        };
        assert!(err
            .to_string()
            .contains("not authenticated — run `plue auth login --with-token` or set PLUE_TOKEN"));
    }

    // --- Bookmark API tests ---
    use crate::types::{BookmarkResponse, CreateBookmarkInput};
    use serde_json::json;

    #[test]
    fn bookmark_response_deserializes() {
        let json = r#"{
            "name": "main",
            "target_change_id": "abc123",
            "target_commit_id": "def456",
            "is_tracking_remote": true
        }"#;

        let bookmark: BookmarkResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(bookmark.name, "main");
        assert_eq!(bookmark.target_change_id, "abc123");
        assert_eq!(bookmark.target_commit_id, "def456");
        assert!(bookmark.is_tracking_remote);
    }

    #[test]
    fn bookmark_response_deserializes_not_tracking() {
        let json = json!({
            "name": "feature",
            "target_change_id": "xyz789",
            "target_commit_id": "uvw012",
            "is_tracking_remote": false
        });

        let bookmark: BookmarkResponse = serde_json::from_value(json).expect("should deserialize");
        assert_eq!(bookmark.name, "feature");
        assert!(!bookmark.is_tracking_remote);
    }

    #[test]
    fn create_bookmark_input_serializes() {
        let input = CreateBookmarkInput {
            name: "my-feature".to_string(),
            target_change_id: "abc123".to_string(),
        };

        let value = serde_json::to_value(&input).expect("should serialize");
        assert_eq!(value["name"], "my-feature");
        assert_eq!(value["target_change_id"], "abc123");
    }
}
