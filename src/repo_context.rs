use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
}

pub fn resolve_repo_ref(cwd: &Path, repo_override: Option<&str>) -> Result<RepoRef> {
    if let Some(value) = repo_override {
        return parse_repo_override(value);
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .context("failed to run git to inspect remotes")?;

    if !output.status.success() {
        bail!("could not detect repository from origin remote; pass -R owner/repo");
    }

    let remote_url = String::from_utf8(output.stdout)
        .context("origin remote URL is not valid UTF-8")?
        .trim()
        .to_string();
    if remote_url.is_empty() {
        bail!("origin remote is empty; pass -R owner/repo");
    }

    parse_remote_url(&remote_url)
        .with_context(|| format!("unsupported origin remote URL format: {remote_url}"))
}

pub fn parse_repo_override(value: &str) -> Result<RepoRef> {
    let parts: Vec<&str> = value.split('/').collect();
    match parts.as_slice() {
        [owner, repo] => normalize_repo_ref(owner, repo),
        [_host, owner, repo] => normalize_repo_ref(owner, repo),
        _ => bail!("invalid repository format: expected owner/repo or host/owner/repo"),
    }
}

pub fn parse_remote_url(remote_url: &str) -> Option<RepoRef> {
    let value = remote_url.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(path) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    {
        let (_, repo_path) = path.split_once('/')?;
        return parse_owner_repo_path(repo_path);
    }
    if let Some(path) = value.strip_prefix("ssh://") {
        let after_host = if let Some((_authority, rest)) = path.split_once('/') {
            rest
        } else {
            return None;
        };
        return parse_owner_repo_path(after_host);
    }
    if let Some((_host, repo_path)) = value.split_once(':') {
        if value.contains('@') {
            return parse_owner_repo_path(repo_path);
        }
    }

    None
}

fn parse_owner_repo_path(path: &str) -> Option<RepoRef> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    if segments.len() < 2 {
        return None;
    }
    let owner = segments[segments.len() - 2];
    let repo = segments[segments.len() - 1];
    normalize_repo_ref(owner, repo).ok()
}

fn normalize_repo_ref(owner: &str, repo: &str) -> Result<RepoRef> {
    let owner = owner.trim();
    let repo = repo.trim().trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        bail!("invalid repository format: expected owner/repo");
    }
    Ok(RepoRef {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_override_owner_repo() {
        let parsed = parse_repo_override("alice/demo").expect("parse owner/repo");
        assert_eq!(
            parsed,
            RepoRef {
                owner: "alice".to_string(),
                repo: "demo".to_string(),
            }
        );
    }

    #[test]
    fn parse_override_host_owner_repo() {
        let parsed = parse_repo_override("plue.dev/alice/demo").expect("parse host/owner/repo");
        assert_eq!(parsed.owner, "alice");
        assert_eq!(parsed.repo, "demo");
    }

    #[test]
    fn parse_override_invalid_rejected() {
        let err = parse_repo_override("alice").expect_err("invalid format should fail");
        assert!(
            err.to_string().contains("owner/repo"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_remote_scp_style() {
        let parsed = parse_remote_url("git@plue.dev:alice/demo.git").expect("parse remote");
        assert_eq!(parsed.owner, "alice");
        assert_eq!(parsed.repo, "demo");
    }

    #[test]
    fn parse_remote_https_style() {
        let parsed = parse_remote_url("https://plue.dev/alice/demo.git").expect("parse remote");
        assert_eq!(parsed.owner, "alice");
        assert_eq!(parsed.repo, "demo");
    }

    #[test]
    fn parse_remote_ssh_url_style() {
        let parsed = parse_remote_url("ssh://git@plue.dev/alice/demo.git").expect("parse remote");
        assert_eq!(parsed.owner, "alice");
        assert_eq!(parsed.repo, "demo");
    }

    #[test]
    fn resolve_repo_ref_missing_origin_errors() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let err = resolve_repo_ref(tmp.path(), None).expect_err("missing origin should fail");
        assert!(
            err.to_string().contains("origin") || err.to_string().contains("repository"),
            "unexpected error: {err}"
        );
    }
}
