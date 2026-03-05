//! Integration tests for `plue search` commands.
//! Uses mockito to simulate the Plue API server.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use mockito::{Matcher, Server};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

fn write_config(config_home: &Path, api_url: &str) {
    let candidates = [
        config_home.join("plue"),
        config_home.join(".config").join("plue"),
        config_home
            .join("Library")
            .join("Application Support")
            .join("plue"),
    ];
    let config = format!("api_url: {api_url}\ntoken: plue_testtoken\n");
    for config_dir in candidates {
        fs::create_dir_all(&config_dir).expect("create config dir");
        fs::write(config_dir.join("config.yml"), &config).expect("write config");
    }
}

fn run_plue(args: &[&str], cwd: &Path, config_home: &Path) -> std::process::Output {
    plue_cmd()
        .args(args)
        .current_dir(cwd)
        .env("XDG_CONFIG_HOME", config_home)
        .env("HOME", config_home)
        .env("PLUE_TOKEN", "plue_testtoken")
        .output()
        .expect("run plue")
}

fn setup_temp_workspace() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = tmp.path().join("cfg");
    fs::create_dir_all(&cfg_home).expect("create cfg dir");
    (tmp, cfg_home)
}

const REPO_SEARCH_RESULT_JSON: &str = r#"{
  "items": [
    {"id":1,"owner":"alice","name":"demo","full_name":"alice/demo","description":"A demo repo","is_public":true,"topics":[]},
    {"id":2,"owner":"bob","name":"project","full_name":"bob/project","description":"Another project","is_public":false,"topics":["rust","cli"]}
  ],
  "total_count": 2,
  "page": 1,
  "per_page": 30
}"#;

const REPO_SEARCH_EMPTY_JSON: &str = r#"{"items":[],"total_count":0,"page":1,"per_page":30}"#;

const ISSUE_SEARCH_RESULT_JSON: &str = r#"{
  "items": [
    {"id":10,"repository_id":1,"repository_owner":"alice","repository_name":"demo","number":5,"title":"Bug report","state":"open"},
    {"id":11,"repository_id":1,"repository_owner":"alice","repository_name":"demo","number":6,"title":"Feature request","state":"closed"}
  ],
  "total_count": 2,
  "page": 1,
  "per_page": 30
}"#;

const ISSUE_SEARCH_EMPTY_JSON: &str = r#"{"items":[],"total_count":0,"page":1,"per_page":30}"#;

// ─────────────────────────────────────────────────────────────────
// plue search repos
// ─────────────────────────────────────────────────────────────────

#[test]
fn search_repos_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let search_mock = server
        .mock("GET", "/api/search/repositories")
        .match_header("authorization", "token plue_testtoken")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(&["search", "repos", "demo"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "search repos should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alice/demo"), "should show repo: {stdout}");
    assert!(stdout.contains("bob/project"), "should show repo: {stdout}");
    search_mock.assert();
}

#[test]
fn search_repos_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/repositories")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["--json", "search", "repos", "demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "search repos --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_object(), "should be object");
    assert_eq!(parsed["total_count"].as_i64(), Some(2));
    assert!(parsed["items"].is_array());
    assert_eq!(parsed["items"].as_array().unwrap().len(), 2);
}

#[test]
fn search_repos_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/repositories")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["--toon", "search", "repos", "demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "search repos --toon should succeed");
}

#[test]
fn search_repos_empty_results() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/repositories")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_SEARCH_EMPTY_JSON)
        .create();

    let out = run_plue(
        &["search", "repos", "nonexistent-xyz"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "search repos should succeed for empty results"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No repositories") || stdout.contains("nonexistent-xyz"),
        "should indicate empty: {stdout}"
    );
}

#[test]
fn search_repos_rejects_empty_query_at_runtime() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No mock needed — validation happens in the command before API call.
    // An empty query "  " passes clap but is caught by the runtime check.
    // Use run_plue to ensure config env vars are set.
    let out = plue_cmd()
        .args(["search", "repos", "   "])
        .current_dir(tmp.path())
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TOKEN", "plue_testtoken")
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "should reject blank query");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("query") || stderr.contains("empty"),
        "should mention empty query: {stderr}"
    );
}

#[test]
fn search_repos_with_limit_flag() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/repositories")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["search", "repos", "demo", "--limit", "5"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "search repos with --limit should succeed"
    );
}

// ─────────────────────────────────────────────────────────────────
// plue search issues
// ─────────────────────────────────────────────────────────────────

#[test]
fn search_issues_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let search_mock = server
        .mock("GET", "/api/search/issues")
        .match_header("authorization", "token plue_testtoken")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(&["search", "issues", "bug"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "search issues should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Bug report"), "should show issue: {stdout}");
    search_mock.assert();
}

#[test]
fn search_issues_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/issues")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["--json", "search", "issues", "bug"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "search issues --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(parsed["total_count"].as_i64(), Some(2));
    assert_eq!(parsed["items"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["items"][0]["title"].as_str(), Some("Bug report"));
}

#[test]
fn search_issues_with_state_filter() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/issues")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["search", "issues", "bug", "--state", "open"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "search issues with --state should succeed"
    );
}

#[test]
fn search_issues_empty_results() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/issues")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_SEARCH_EMPTY_JSON)
        .create();

    let out = run_plue(&["search", "issues", "nonexistent"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "search issues should succeed for empty results"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No issues") || stdout.contains("nonexistent"),
        "should indicate empty: {stdout}"
    );
}

#[test]
fn search_issues_rejects_empty_query() {
    let (tmp, cfg_home) = setup_temp_workspace();
    // Empty string is a required positional arg — clap should error.
    let out = plue_cmd()
        .args(["search", "issues", ""])
        .current_dir(tmp.path())
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TOKEN", "plue_testtoken")
        .output()
        .expect("run plue");

    // Empty string might be caught by clap (required arg) or by runtime validation
    assert!(!out.status.success(), "should reject empty query");
}

// ─────────────────────────────────────────────────────────────────
// plue search code (implemented — makes real API calls)
// ─────────────────────────────────────────────────────────────────

const CODE_SEARCH_RESULT_JSON: &str = r#"{
  "items": [
    {"id":1,"repository_id":1,"repository":"alice/demo","path":"src/main.rs","text_matches":["fn main() {"]},
    {"id":2,"repository_id":1,"repository":"alice/demo","path":"src/lib.rs","text_matches":["fn main_helper() {"]}
  ],
  "total_count": 2,
  "page": 1,
  "per_page": 30
}"#;

const CODE_SEARCH_EMPTY_JSON: &str = r#"{"items":[],"total_count":0,"page":1,"per_page":30}"#;

#[test]
fn search_code_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let search_mock = server
        .mock("GET", "/api/search/code")
        .match_header("authorization", "token plue_testtoken")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CODE_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(&["search", "code", "fn main"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "search code should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("src/main.rs"),
        "should show file path: {stdout}"
    );
    assert!(
        stdout.contains("alice/demo"),
        "should show repository: {stdout}"
    );
    search_mock.assert();
}

#[test]
fn search_code_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/code")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CODE_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["--json", "search", "code", "fn main"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "search code --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(parsed["total_count"].as_i64(), Some(2));
    assert!(parsed["items"].is_array());
    assert_eq!(parsed["items"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["items"][0]["path"].as_str(), Some("src/main.rs"));
}

#[test]
fn search_code_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/code")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CODE_SEARCH_RESULT_JSON)
        .create();

    let out = run_plue(
        &["--toon", "search", "code", "fn main"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "search code --toon should succeed");
}

#[test]
fn search_code_empty_results() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/search/code")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CODE_SEARCH_EMPTY_JSON)
        .create();

    let out = run_plue(
        &["search", "code", "nonexistent_fn_xyz"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "search code should succeed for empty results"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No code") || stdout.contains("nonexistent_fn_xyz"),
        "should indicate empty: {stdout}"
    );
}

#[test]
fn search_code_rejects_empty_query() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["search", "code", "   "])
        .current_dir(tmp.path())
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TOKEN", "plue_testtoken")
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "should reject blank query");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("query") || stderr.contains("empty"),
        "should mention empty query: {stderr}"
    );
}

#[test]
fn search_code_requires_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token configured — should fail with auth error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["search", "code", "fn main"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue search code");

    assert!(!out.status.success(), "should fail without auth");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "search code is implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("not authenticated") || stderr.contains("auth"),
        "should mention auth error: {stderr}"
    );
}
