use std::fmt;

use serde::{Deserialize, Serialize};

/// Information about a jj change (commit).
#[derive(Debug, Clone, Serialize)]
pub struct ChangeInfo {
    pub change_id: String,
    pub commit_id: String,
    pub description: String,
    pub author: AuthorInfo,
    pub timestamp: String,
    pub is_empty: bool,
    pub is_working_copy: bool,
    pub bookmarks: Vec<String>,
}

/// Author/committer identity.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

/// Working copy status.
#[derive(Debug, Clone, Serialize)]
pub struct StatusInfo {
    pub working_copy: ChangeInfo,
    pub parent: Option<ChangeInfo>,
    pub modified_files: Vec<FileChange>,
}

/// A file changed in the working copy.
#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub change_type: FileChangeType,
}

/// Type of file change.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
}

/// Information about a bookmark (branch).
#[derive(Debug, Clone, Serialize)]
pub struct BookmarkInfo {
    pub name: String,
    pub target_change_id: String,
    pub target_commit_id: String,
    pub is_tracking_remote: bool,
}

/// Bookmark response from the API (remote bookmarks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkResponse {
    pub name: String,
    pub target_change_id: String,
    pub target_commit_id: String,
    pub is_tracking_remote: bool,
}

/// Input for creating a bookmark via API.
#[derive(Debug, Serialize)]
pub struct CreateBookmarkInput {
    pub name: String,
    pub target_change_id: String,
}

/// Diff output for a change.
#[derive(Debug, Clone, Serialize)]
pub struct DiffOutput {
    pub change_id: String,
    pub file_diffs: Vec<FileDiff>,
}

/// Diff for a single file.
#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub change_type: FileChangeType,
    pub hunks: Vec<DiffHunk>,
}

/// A hunk in a diff.
#[derive(Debug, Clone, Serialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone, Serialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

/// Kind of diff line.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// Landing request author identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingRequestAuthor {
    pub id: i64,
    pub login: String,
}

/// Landing request payload returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingRequestResponse {
    pub number: i64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub author: LandingRequestAuthor,
    pub change_ids: Vec<String>,
    pub target_bookmark: String,
    pub conflict_status: String,
    pub stack_size: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Landing request review payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingRequestReview {
    pub id: i64,
    pub landing_request_id: i64,
    pub reviewer_id: i64,
    #[serde(rename = "type")]
    pub review_type: String,
    pub body: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Landing request stack member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingRequestChange {
    pub id: i64,
    pub landing_request_id: i64,
    pub change_id: String,
    pub position_in_stack: i64,
    pub created_at: String,
}

/// Conflict object for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingConflict {
    pub file_path: String,
    pub conflict_type: String,
}

/// Aggregated landing conflict response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingConflictsResponse {
    pub conflict_status: String,
    pub has_conflicts: bool,
    #[serde(default)]
    pub conflicts_by_change: std::collections::BTreeMap<String, Vec<LandingConflict>>,
}

/// Issue author/assignee summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueUserSummary {
    pub id: i64,
    pub login: String,
}

/// Issue payload returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueResponse {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub author: IssueUserSummary,
    pub assignees: Vec<IssueUserSummary>,
    pub milestone_id: Option<i64>,
    pub comment_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating an issue.
#[derive(Debug, Serialize)]
pub struct CreateIssueInput {
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assignees: Vec<String>,
}

/// Input for updating an issue.
#[derive(Debug, Serialize, Default)]
pub struct UpdateIssueInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
}

/// Commit status used by `plue land checks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStatusResponse {
    pub context: String,
    pub status: String,
    pub description: String,
    pub target_url: String,
}

/// SSH key response from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: i64,
    pub name: String,
    pub fingerprint: String,
    pub key_type: String,
    pub created_at: String,
}

/// Request to add an SSH key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSshKeyInput {
    pub title: String,
    pub key: String,
}

/// A repository item returned by search (lighter than full RepoSummaryResponse).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoSearchItem {
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub is_public: bool,
    pub topics: Vec<String>,
}

/// An issue item returned by search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSearchItem {
    pub id: i64,
    pub repository_id: i64,
    pub repository_owner: String,
    pub repository_name: String,
    pub number: i64,
    pub title: String,
    pub state: String,
}

/// Search result page for repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySearchResultPage {
    pub items: Vec<RepoSearchItem>,
    pub total_count: i64,
    pub page: i32,
    pub per_page: i32,
}

/// Search result page for issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSearchResultPage {
    pub items: Vec<IssueSearchItem>,
    pub total_count: i64,
    pub page: i32,
    pub per_page: i32,
}

/// Label attached to issues/landing requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelResponse {
    pub id: i64,
    pub repository_id: i64,
    pub name: String,
    pub color: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a label.
#[derive(Debug, Serialize)]
pub struct CreateLabelInput {
    pub name: String,
    pub color: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A release attached to a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseResponse {
    pub id: i64,
    pub repository_id: i64,
    pub publisher_id: i64,
    pub tag_name: String,
    pub title: String,
    pub body: String,
    pub is_draft: bool,
    pub is_prerelease: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a release.
#[derive(Debug, Serialize)]
pub struct CreateReleaseInput {
    pub tag_name: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub body: String,
    pub is_draft: bool,
    pub is_prerelease: bool,
}

/// Workflow definition (returned by list/get workflow endpoints).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinitionResponse {
    pub id: i64,
    pub repository_id: i64,
    pub name: String,
    pub path: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow run returned by list/get run endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunResponse {
    pub id: i64,
    pub repository_id: i64,
    pub workflow_definition_id: i64,
    pub status: String,
    pub trigger_event: String,
    pub trigger_ref: String,
    pub trigger_commit_sha: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for dispatching (triggering) a workflow.
#[derive(Debug, Serialize)]
pub struct DispatchWorkflowInput {
    #[serde(rename = "ref")]
    pub git_ref: String,
}

/// Input for rerunning a workflow run.
#[derive(Debug, Serialize)]
pub struct RerunWorkflowRunInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Response from rerunning a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunRerunResponse {
    pub workflow_definition_id: i64,
    pub workflow_run_id: i64,
    pub steps: Vec<WorkflowStepResult>,
}

/// A workflow step result within a rerun response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepResult {
    pub step_id: i64,
    pub task_id: i64,
}

/// Agent session returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionResponse {
    pub id: String,
    pub repository_id: i64,
    pub user_id: i64,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating an agent session.
#[derive(Debug, Serialize)]
pub struct CreateAgentSessionInput {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

/// A single part of an agent message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPartResponse {
    pub part_index: i64,
    #[serde(rename = "type")]
    pub part_type: String,
    pub content: serde_json::Value,
}

/// An agent message returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageResponse {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub sequence: i64,
    pub parts: Vec<AgentPartResponse>,
    pub created_at: String,
}

/// Input for posting an agent message (user turn).
#[derive(Debug, Serialize)]
pub struct PostAgentMessageInput {
    pub role: String,
    pub parts: Vec<AgentMessagePartInput>,
}

/// A single part for a posted message.
#[derive(Debug, Serialize)]
pub struct AgentMessagePartInput {
    #[serde(rename = "type")]
    pub part_type: String,
    pub content: serde_json::Value,
}

/// An SSE event from the agent stream.
#[derive(Debug, Clone)]
pub struct AgentSSEEvent {
    pub event_type: String,
    pub data: String,
}

/// Summary repo payload used by list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoSummaryResponse {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub default_bookmark: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Detailed repo payload used by repo view endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoDetailResponse {
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub is_public: bool,
    pub default_bookmark: String,
    pub topics: Vec<String>,
    pub is_archived: bool,
    pub is_fork: bool,
    pub num_stars: i64,
    pub num_watches: i64,
    pub num_issues: i64,
    pub clone_url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Secret response from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretResponse {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for setting a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSecretInput {
    pub name: String,
    pub value: String,
}

/// Variable response from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableResponse {
    pub name: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for setting a variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariableInput {
    pub name: String,
    pub value: String,
}

/// Code search item from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchItem {
    pub path: String,
    pub repository: String,
    pub text_matches: Vec<String>,
}

/// Code search result page from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResultPage {
    pub items: Vec<CodeSearchItem>,
    pub total_count: i64,
}

/// Closed beta whitelist entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaWhitelistEntry {
    pub id: i64,
    pub identity_type: String,
    pub identity_value: String,
    pub created_by: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Closed beta waitlist entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaWaitlistEntry {
    pub id: i64,
    pub email: String,
    pub note: String,
    pub status: String,
    pub source: String,
    pub approved_by: Option<i64>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Paginated waitlist response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaWaitlistListResponse {
    pub items: Vec<BetaWaitlistEntry>,
    pub total_count: i64,
    pub page: i32,
    pub per_page: i32,
}

/// Raw API Response for `plue api`.
#[derive(Debug)]
pub struct RawApiResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl fmt::Display for ChangeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let wc_marker = if self.is_working_copy { "@ " } else { "  " };
        write!(f, "{wc_marker}{}", self.change_id)?;
        if !self.bookmarks.is_empty() {
            write!(f, " {}", self.bookmarks.join(" "))?;
        }
        write!(f, " {}", self.commit_id)?;
        let desc = if self.description.is_empty() {
            "(no description)"
        } else {
            &self.description
        };
        write!(f, " {desc}")?;
        if self.is_empty {
            write!(f, " (empty)")?;
        }
        Ok(())
    }
}

impl fmt::Display for StatusInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Working copy : {}", self.working_copy)?;
        if let Some(parent) = &self.parent {
            writeln!(f, "Parent commit: {parent}")?;
        }
        if !self.modified_files.is_empty() {
            writeln!(f, "Modified files:")?;
            for fc in &self.modified_files {
                writeln!(f, "  {fc}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for FileChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.change_type {
            FileChangeType::Added => "A",
            FileChangeType::Modified => "M",
            FileChangeType::Deleted => "D",
        };
        write!(f, "{prefix} {}", self.path)
    }
}

impl fmt::Display for BookmarkInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {}",
            self.name, self.target_change_id, self.target_commit_id
        )?;
        if self.is_tracking_remote {
            write!(f, " (tracking)")?;
        }
        Ok(())
    }
}

impl fmt::Display for DiffOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for file_diff in &self.file_diffs {
            write!(f, "{file_diff}")?;
        }
        Ok(())
    }
}

impl fmt::Display for FileDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "diff --git a/{path} b/{path}", path = self.path)?;
        for hunk in &self.hunks {
            write!(f, "{hunk}")?;
        }
        Ok(())
    }
}

impl fmt::Display for DiffHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        for line in &self.lines {
            write!(f, "{line}")?;
        }
        Ok(())
    }
}

impl fmt::Display for DiffLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.kind {
            DiffLineKind::Context => " ",
            DiffLineKind::Added => "+",
            DiffLineKind::Removed => "-",
        };
        writeln!(f, "{prefix}{}", self.content)
    }
}

impl fmt::Display for SecretResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t{}", self.name, self.updated_at)
    }
}

impl fmt::Display for VariableResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t{}\t{}", self.name, self.value, self.updated_at)
    }
}

impl fmt::Display for CodeSearchItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t{}", self.repository, self.path)
    }
}

impl fmt::Display for SshKeyResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}",
            self.id, self.name, self.key_type, self.created_at
        )
    }
}

impl fmt::Display for RepoSummaryResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let visibility = if self.is_public { "public" } else { "private" };
        write!(
            f,
            "{}\t{}\t{}\t{}",
            self.name, self.description, visibility, self.updated_at
        )
    }
}

impl fmt::Display for RepoDetailResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let visibility = if self.is_public { "public" } else { "private" };
        let topics = if self.topics.is_empty() {
            "-".to_string()
        } else {
            self.topics.join(", ")
        };
        writeln!(f, "Repository: {}", self.full_name)?;
        writeln!(f, "Description: {}", self.description)?;
        writeln!(f, "Visibility: {visibility}")?;
        writeln!(f, "Default bookmark: {}", self.default_bookmark)?;
        writeln!(f, "Stars: {}", self.num_stars)?;
        writeln!(f, "Watches: {}", self.num_watches)?;
        writeln!(f, "Open issues: {}", self.num_issues)?;
        writeln!(f, "Clone URL: {}", self.clone_url)?;
        write!(f, "Topics: {topics}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_change_info() -> ChangeInfo {
        ChangeInfo {
            change_id: "abc123".to_string(),
            commit_id: "def456".to_string(),
            description: "Add feature X".to_string(),
            author: AuthorInfo {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
            },
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            is_empty: false,
            is_working_copy: true,
            bookmarks: vec!["main".to_string()],
        }
    }

    fn sample_status_info() -> StatusInfo {
        StatusInfo {
            working_copy: sample_change_info(),
            parent: Some(ChangeInfo {
                change_id: "xyz789".to_string(),
                commit_id: "uvw012".to_string(),
                description: "Initial commit".to_string(),
                author: AuthorInfo {
                    name: "Test User".to_string(),
                    email: "test@example.com".to_string(),
                },
                timestamp: "2024-12-31T00:00:00Z".to_string(),
                is_empty: false,
                is_working_copy: false,
                bookmarks: vec![],
            }),
            modified_files: vec![FileChange {
                path: "src/main.rs".to_string(),
                change_type: FileChangeType::Modified,
            }],
        }
    }

    #[test]
    fn test_change_info_serializes_to_json() {
        let info = sample_change_info();
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["change_id"], "abc123");
        assert_eq!(json["commit_id"], "def456");
        assert_eq!(json["description"], "Add feature X");
        assert_eq!(json["author"]["name"], "Test User");
        assert_eq!(json["is_working_copy"], true);
        assert_eq!(json["bookmarks"][0], "main");
    }

    #[test]
    fn test_status_info_serializes_to_json() {
        let info = sample_status_info();
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["working_copy"]["change_id"], "abc123");
        assert_eq!(json["parent"]["change_id"], "xyz789");
        assert_eq!(json["modified_files"][0]["path"], "src/main.rs");
        assert_eq!(json["modified_files"][0]["change_type"], "Modified");
    }

    #[test]
    fn test_bookmark_info_serializes_to_json() {
        let info = BookmarkInfo {
            name: "main".to_string(),
            target_change_id: "abc123".to_string(),
            target_commit_id: "def456".to_string(),
            is_tracking_remote: true,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "main");
        assert_eq!(json["target_change_id"], "abc123");
        assert_eq!(json["is_tracking_remote"], true);
    }

    #[test]
    fn test_change_info_display_format() {
        let info = sample_change_info();
        let output = info.to_string();
        assert!(output.contains("@ abc123"));
        assert!(output.contains("main"));
        assert!(output.contains("def456"));
        assert!(output.contains("Add feature X"));
    }

    #[test]
    fn test_change_info_display_empty() {
        let info = ChangeInfo {
            is_empty: true,
            description: String::new(),
            is_working_copy: false,
            bookmarks: vec![],
            ..sample_change_info()
        };
        let output = info.to_string();
        assert!(output.contains("(no description)"));
        assert!(output.contains("(empty)"));
        assert!(output.starts_with("  "));
    }

    #[test]
    fn test_status_info_display_shows_working_copy_and_parent() {
        let info = sample_status_info();
        let output = info.to_string();
        assert!(output.contains("Working copy : @ abc123"));
        assert!(output.contains("Parent commit:   xyz789"));
    }

    #[test]
    fn test_status_info_display_with_no_parent() {
        let info = StatusInfo {
            parent: None,
            modified_files: vec![],
            ..sample_status_info()
        };
        let output = info.to_string();
        assert!(output.contains("Working copy"));
        assert!(!output.contains("Parent commit"));
    }

    #[test]
    fn test_status_info_display_with_modified_files() {
        let info = sample_status_info();
        let output = info.to_string();
        assert!(output.contains("Modified files:"));
        assert!(output.contains("M src/main.rs"));
    }

    #[test]
    fn test_bookmark_info_display_format() {
        let info = BookmarkInfo {
            name: "main".to_string(),
            target_change_id: "abc123".to_string(),
            target_commit_id: "def456".to_string(),
            is_tracking_remote: true,
        };
        let output = info.to_string();
        assert!(output.contains("main: abc123 def456"));
        assert!(output.contains("(tracking)"));
    }

    #[test]
    fn test_bookmark_info_display_no_tracking() {
        let info = BookmarkInfo {
            name: "feature".to_string(),
            target_change_id: "abc".to_string(),
            target_commit_id: "def".to_string(),
            is_tracking_remote: false,
        };
        let output = info.to_string();
        assert!(!output.contains("(tracking)"));
    }

    #[test]
    fn test_diff_output_serializes_to_json() {
        let diff = DiffOutput {
            change_id: "abc123".to_string(),
            file_diffs: vec![FileDiff {
                path: "file.txt".to_string(),
                change_type: FileChangeType::Added,
                hunks: vec![DiffHunk {
                    header: "@@ -0,0 +1,1 @@".to_string(),
                    lines: vec![DiffLine {
                        kind: DiffLineKind::Added,
                        content: "hello".to_string(),
                    }],
                }],
            }],
        };
        let json = serde_json::to_value(&diff).unwrap();
        assert_eq!(json["change_id"], "abc123");
        assert_eq!(json["file_diffs"][0]["path"], "file.txt");
        assert_eq!(
            json["file_diffs"][0]["hunks"][0]["lines"][0]["kind"],
            "Added"
        );
    }

    #[test]
    fn test_file_change_display() {
        assert_eq!(
            FileChange {
                path: "a.rs".to_string(),
                change_type: FileChangeType::Added
            }
            .to_string(),
            "A a.rs"
        );
        assert_eq!(
            FileChange {
                path: "b.rs".to_string(),
                change_type: FileChangeType::Deleted
            }
            .to_string(),
            "D b.rs"
        );
    }

    #[test]
    fn test_landing_request_response_serializes_snake_case() {
        let landing = LandingRequestResponse {
            number: 42,
            title: "Add auth".to_string(),
            body: "body".to_string(),
            state: "open".to_string(),
            author: LandingRequestAuthor {
                id: 1,
                login: "alice".to_string(),
            },
            change_ids: vec!["kxyz".to_string()],
            target_bookmark: "main".to_string(),
            conflict_status: "clean".to_string(),
            stack_size: 1,
            created_at: "2026-02-19T00:00:00Z".to_string(),
            updated_at: "2026-02-19T00:00:00Z".to_string(),
        };

        let value = serde_json::to_value(&landing).expect("serialize landing");
        assert_eq!(value["target_bookmark"], "main");
        assert_eq!(value["change_ids"][0], "kxyz");
        assert_eq!(value["stack_size"], 1);
    }

    #[test]
    fn test_landing_conflicts_response_serializes_snake_case() {
        let mut by_change = std::collections::BTreeMap::new();
        by_change.insert(
            "kxyz".to_string(),
            vec![LandingConflict {
                file_path: "main.rs".to_string(),
                conflict_type: "content".to_string(),
            }],
        );
        let resp = LandingConflictsResponse {
            conflict_status: "conflicted".to_string(),
            has_conflicts: true,
            conflicts_by_change: by_change,
        };

        let value = serde_json::to_value(&resp).expect("serialize conflicts");
        assert_eq!(value["conflict_status"], "conflicted");
        assert_eq!(value["has_conflicts"], true);
        assert_eq!(
            value["conflicts_by_change"]["kxyz"][0]["file_path"],
            "main.rs"
        );
    }

    #[test]
    fn test_repo_summary_response_serializes() {
        let json = r#"{
            "id": 1,
            "name": "my-repo",
            "description": "A demo repository",
            "is_public": true,
            "default_bookmark": "main",
            "created_at": "2026-02-19T00:00:00Z",
            "updated_at": "2026-02-20T00:00:00Z"
        }"#;

        let summary: RepoSummaryResponse =
            serde_json::from_str(json).expect("summary should deserialize");
        assert_eq!(summary.id, 1);
        assert_eq!(summary.name, "my-repo");
        assert_eq!(summary.description, "A demo repository");
        assert!(summary.is_public);
        assert_eq!(summary.default_bookmark, "main");
        assert_eq!(summary.created_at, "2026-02-19T00:00:00Z");
        assert_eq!(summary.updated_at, "2026-02-20T00:00:00Z");

        let value = serde_json::to_value(&summary).expect("summary should serialize");
        assert_eq!(value["default_bookmark"], "main");
        assert_eq!(value["is_public"], true);
    }

    #[test]
    fn test_repo_detail_response_serializes() {
        let json = r#"{
            "id": 7,
            "owner": "alice",
            "name": "my-repo",
            "full_name": "alice/my-repo",
            "description": "A repository",
            "is_public": false,
            "default_bookmark": "trunk",
            "topics": ["jj", "rust"],
            "is_archived": false,
            "is_fork": true,
            "num_stars": 8,
            "num_watches": 5,
            "num_issues": 3,
            "clone_url": "git@plue.dev:alice/my-repo.git",
            "created_at": "2026-02-19T00:00:00Z",
            "updated_at": "2026-02-21T00:00:00Z"
        }"#;

        let detail: RepoDetailResponse =
            serde_json::from_str(json).expect("detail should deserialize");
        assert_eq!(detail.id, 7);
        assert_eq!(detail.owner, "alice");
        assert_eq!(detail.name, "my-repo");
        assert_eq!(detail.full_name, "alice/my-repo");
        assert_eq!(detail.description, "A repository");
        assert!(!detail.is_public);
        assert_eq!(detail.default_bookmark, "trunk");
        assert_eq!(detail.topics, vec!["jj".to_string(), "rust".to_string()]);
        assert!(!detail.is_archived);
        assert!(detail.is_fork);
        assert_eq!(detail.num_stars, 8);
        assert_eq!(detail.num_watches, 5);
        assert_eq!(detail.num_issues, 3);
        assert_eq!(detail.clone_url, "git@plue.dev:alice/my-repo.git");
        assert_eq!(detail.created_at, "2026-02-19T00:00:00Z");
        assert_eq!(detail.updated_at, "2026-02-21T00:00:00Z");

        let value = serde_json::to_value(&detail).expect("detail should serialize");
        assert_eq!(value["full_name"], "alice/my-repo");
        assert_eq!(value["num_stars"], 8);
    }

    #[test]
    fn test_repo_summary_display_format() {
        let summary = RepoSummaryResponse {
            id: 1,
            name: "my-repo".to_string(),
            description: "A demo repository".to_string(),
            is_public: true,
            default_bookmark: "main".to_string(),
            created_at: "2026-02-19T00:00:00Z".to_string(),
            updated_at: "2026-02-20T00:00:00Z".to_string(),
        };

        let output = summary.to_string();
        assert!(output.contains("my-repo"));
        assert!(output.contains("A demo repository"));
        assert!(output.contains("public"));
        assert!(output.contains("2026-02-20T00:00:00Z"));
    }

    #[test]
    fn test_issue_response_deserializes() {
        let json = r#"{
            "id": 10,
            "number": 5,
            "title": "Bug report",
            "body": "Something is broken",
            "state": "open",
            "author": {"id": 1, "login": "alice"},
            "assignees": [{"id": 2, "login": "bob"}],
            "milestone_id": null,
            "comment_count": 3,
            "created_at": "2026-02-19T00:00:00Z",
            "updated_at": "2026-02-20T00:00:00Z"
        }"#;

        let issue: IssueResponse = serde_json::from_str(json).expect("issue should deserialize");
        assert_eq!(issue.id, 10);
        assert_eq!(issue.number, 5);
        assert_eq!(issue.title, "Bug report");
        assert_eq!(issue.body, "Something is broken");
        assert_eq!(issue.state, "open");
        assert_eq!(issue.author.id, 1);
        assert_eq!(issue.author.login, "alice");
        assert_eq!(issue.assignees.len(), 1);
        assert_eq!(issue.assignees[0].login, "bob");
        assert!(issue.milestone_id.is_none());
        assert_eq!(issue.comment_count, 3);
        assert_eq!(issue.created_at, "2026-02-19T00:00:00Z");
        assert_eq!(issue.updated_at, "2026-02-20T00:00:00Z");
    }

    #[test]
    fn test_issue_response_deserializes_with_milestone() {
        let json = r#"{
            "id": 10,
            "number": 5,
            "title": "Feature",
            "body": "",
            "state": "open",
            "author": {"id": 1, "login": "alice"},
            "assignees": [],
            "milestone_id": 42,
            "comment_count": 0,
            "created_at": "2026-02-19T00:00:00Z",
            "updated_at": "2026-02-19T00:00:00Z"
        }"#;

        let issue: IssueResponse =
            serde_json::from_str(json).expect("issue with milestone should deserialize");
        assert_eq!(issue.milestone_id, Some(42));
    }

    #[test]
    fn test_create_issue_input_serializes() {
        let input = CreateIssueInput {
            title: "New bug".to_string(),
            body: "Details here".to_string(),
            assignees: vec!["alice".to_string(), "bob".to_string()],
        };
        let value = serde_json::to_value(&input).expect("serialize create issue");
        assert_eq!(value["title"], "New bug");
        assert_eq!(value["body"], "Details here");
        assert_eq!(value["assignees"][0], "alice");
        assert_eq!(value["assignees"][1], "bob");
    }

    #[test]
    fn test_create_issue_input_omits_empty_assignees() {
        let input = CreateIssueInput {
            title: "Solo issue".to_string(),
            body: "No assignees".to_string(),
            assignees: vec![],
        };
        let value = serde_json::to_value(&input).expect("serialize create issue");
        assert_eq!(value["title"], "Solo issue");
        assert!(value.get("assignees").is_none());
    }

    #[test]
    fn test_update_issue_input_serializes_omit_empty() {
        let input = UpdateIssueInput {
            state: Some("closed".to_string()),
            ..Default::default()
        };
        let value = serde_json::to_value(&input).expect("serialize update issue");
        assert_eq!(value["state"], "closed");
        assert!(value.get("title").is_none());
        assert!(value.get("body").is_none());
        assert!(value.get("assignees").is_none());
    }

    #[test]
    fn test_update_issue_input_serializes_all_fields() {
        let input = UpdateIssueInput {
            title: Some("Updated title".to_string()),
            body: Some("Updated body".to_string()),
            state: Some("open".to_string()),
            assignees: Some(vec!["alice".to_string()]),
        };
        let value = serde_json::to_value(&input).expect("serialize update issue all fields");
        assert_eq!(value["title"], "Updated title");
        assert_eq!(value["body"], "Updated body");
        assert_eq!(value["state"], "open");
        assert_eq!(value["assignees"][0], "alice");
    }

    #[test]
    fn test_repo_detail_display_format() {
        let detail = RepoDetailResponse {
            id: 7,
            owner: "alice".to_string(),
            name: "my-repo".to_string(),
            full_name: "alice/my-repo".to_string(),
            description: "A repository".to_string(),
            is_public: false,
            default_bookmark: "trunk".to_string(),
            topics: vec!["jj".to_string(), "rust".to_string()],
            is_archived: false,
            is_fork: true,
            num_stars: 8,
            num_watches: 5,
            num_issues: 3,
            clone_url: "git@plue.dev:alice/my-repo.git".to_string(),
            created_at: "2026-02-19T00:00:00Z".to_string(),
            updated_at: "2026-02-21T00:00:00Z".to_string(),
        };

        let output = detail.to_string();
        assert!(output.contains("alice/my-repo"));
        assert!(output.contains("A repository"));
        assert!(output.contains("private"));
        assert!(output.contains("8"));
        assert!(output.contains("3"));
        assert!(output.contains("git@plue.dev:alice/my-repo.git"));
        assert!(output.contains("jj, rust"));
    }

    #[test]
    fn test_secret_response_serializes() {
        let resp = SecretResponse {
            name: "MY_SECRET".to_string(),
            created_at: "2026-02-19T00:00:00Z".to_string(),
            updated_at: "2026-02-20T00:00:00Z".to_string(),
        };
        let value = serde_json::to_value(&resp).expect("serialize secret");
        assert_eq!(value["name"], "MY_SECRET");
    }

    #[test]
    fn test_set_secret_input_serializes() {
        let input = SetSecretInput {
            name: "MY_SECRET".to_string(),
            value: "super_secret".to_string(),
        };
        let value = serde_json::to_value(&input).expect("serialize set secret");
        assert_eq!(value["value"], "super_secret");
    }

    #[test]
    fn test_variable_response_serializes() {
        let resp = VariableResponse {
            name: "MY_VAR".to_string(),
            value: "value".to_string(),
            created_at: "2026-02-19T00:00:00Z".to_string(),
            updated_at: "2026-02-20T00:00:00Z".to_string(),
        };
        let value = serde_json::to_value(&resp).expect("serialize var");
        assert_eq!(value["name"], "MY_VAR");
        assert_eq!(value["value"], "value");
    }

    #[test]
    fn test_set_variable_input_serializes() {
        let input = SetVariableInput {
            name: "MY_VAR".to_string(),
            value: "new_value".to_string(),
        };
        let value = serde_json::to_value(&input).expect("serialize set var");
        assert_eq!(value["value"], "new_value");
    }

    #[test]
    fn test_code_search_result_page_serializes() {
        let page = CodeSearchResultPage {
            items: vec![CodeSearchItem {
                path: "main.rs".to_string(),
                repository: "owner/repo".to_string(),
                text_matches: vec!["fn main() {}".to_string()],
            }],
            total_count: 1,
        };
        let value = serde_json::to_value(&page).expect("serialize code search");
        assert_eq!(value["total_count"], 1);
        assert_eq!(value["items"][0]["path"], "main.rs");
        assert_eq!(value["items"][0]["text_matches"][0], "fn main() {}");
    }
}
