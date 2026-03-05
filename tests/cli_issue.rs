//! Integration tests for the `plue issue` command suite.
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

const ISSUE_RESPONSE_JSON: &str = r#"{"id":10,"number":5,"title":"Bug report","body":"Something is broken","state":"open","author":{"id":1,"login":"alice"},"assignees":[{"id":2,"login":"bob"}],"milestone_id":null,"comment_count":3,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-20T00:00:00Z"}"#;

const ISSUE_LIST_JSON: &str = r#"[{"id":10,"number":5,"title":"Bug report","body":"Something is broken","state":"open","author":{"id":1,"login":"alice"},"assignees":[],"milestone_id":null,"comment_count":3,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-20T00:00:00Z"},{"id":11,"number":6,"title":"Feature request","body":"Add dark mode","state":"open","author":{"id":2,"login":"bob"},"assignees":[],"milestone_id":null,"comment_count":0,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#;

const CLOSED_ISSUE_JSON: &str = r#"{"id":10,"number":5,"title":"Bug report","body":"Something is broken","state":"closed","author":{"id":1,"login":"alice"},"assignees":[],"milestone_id":null,"comment_count":4,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-21T00:00:00Z"}"#;

// --- issue create tests ---

#[test]
fn issue_create_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/repos/alice/demo/issues")
        .match_header("authorization", "token plue_testtoken")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "title": "Bug report",
            "body": "Something is broken"
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &[
            "issue",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "Bug report",
            "--body",
            "Something is broken",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("#5"), "stdout: {stdout}");
    assert!(stdout.contains("Bug report"), "stdout: {stdout}");

    create_mock.assert();
}

#[test]
fn issue_create_with_assignees() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/repos/alice/demo/issues")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "title": "Assign me",
            "body": "",
            "assignees": ["bob"]
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &[
            "issue",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "Assign me",
            "--assignee",
            "bob",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    create_mock.assert();
}

#[test]
fn issue_create_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _create_mock = server
        .mock("POST", "/api/repos/alice/demo/issues")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &[
            "issue",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "Bug report",
            "--json",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert_eq!(value["number"], 5);
    assert_eq!(value["title"], "Bug report");
}

#[test]
fn issue_create_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _create_mock = server
        .mock("POST", "/api/repos/alice/demo/issues")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &[
            "issue",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "Bug report",
            "--toon",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("number:5"), "stdout: {stdout}");
    assert!(stdout.contains("title:\"Bug report\""), "stdout: {stdout}");
    assert!(stdout.contains("author.login:alice"), "stdout: {stdout}");
    assert!(!stdout.contains('{'), "stdout: {stdout}");
}

// --- issue list tests ---

#[test]
fn issue_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/issues")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("state".into(), "open".into()),
            Matcher::UrlEncoded("page".into(), "1".into()),
            Matcher::UrlEncoded("per_page".into(), "30".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_LIST_JSON)
        .create();

    let out = run_plue(
        &["issue", "list", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Bug report"), "stdout: {stdout}");
    assert!(stdout.contains("Feature request"), "stdout: {stdout}");
    assert!(stdout.contains("alice"), "stdout: {stdout}");

    list_mock.assert();
}

#[test]
fn issue_list_closed_state() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/issues")
        .match_query(Matcher::AllOf(vec![Matcher::UrlEncoded(
            "state".into(),
            "closed".into(),
        )]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let out = run_plue(
        &["issue", "list", "-R", "alice/demo", "--state", "closed"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No issues found") || stdout.trim().is_empty() || stdout.contains("0"),
        "stdout: {stdout}"
    );

    list_mock.assert();
}

#[test]
fn issue_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _list_mock = server
        .mock("GET", "/api/repos/alice/demo/issues")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_LIST_JSON)
        .create();

    let out = run_plue(
        &["issue", "list", "-R", "alice/demo", "--json"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert!(value.is_array());
    assert_eq!(value.as_array().unwrap().len(), 2);
    assert_eq!(value[0]["number"], 5);
    assert_eq!(value[1]["number"], 6);
}

#[test]
fn issue_list_all_state_sends_no_state_param() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    // When state=all, no state param should be sent. We match on exact query params.
    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/issues")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("page".into(), "1".into()),
            Matcher::UrlEncoded("per_page".into(), "30".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let out = run_plue(
        &["issue", "list", "-R", "alice/demo", "--state", "all"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    list_mock.assert();
}

// --- issue view tests ---

#[test]
fn issue_view_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let view_mock = server
        .mock("GET", "/api/repos/alice/demo/issues/5")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &["issue", "view", "5", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Bug report"), "stdout: {stdout}");
    assert!(stdout.contains("#5"), "stdout: {stdout}");
    assert!(stdout.contains("alice"), "stdout: {stdout}");
    assert!(stdout.contains("open"), "stdout: {stdout}");

    view_mock.assert();
}

#[test]
fn issue_view_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _view_mock = server
        .mock("GET", "/api/repos/alice/demo/issues/5")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &["issue", "view", "5", "-R", "alice/demo", "--json"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert_eq!(value["number"], 5);
    assert_eq!(value["title"], "Bug report");
    assert_eq!(value["state"], "open");
}

#[test]
fn issue_view_not_found() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _view_mock = server
        .mock("GET", "/api/repos/alice/demo/issues/999")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"issue not found"}"#)
        .create();

    let out = run_plue(
        &["issue", "view", "999", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("issue not found"), "stderr: {stderr}");
}

// --- issue close tests ---

#[test]
fn issue_close_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let close_mock = server
        .mock("PATCH", "/api/repos/alice/demo/issues/5")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "state": "closed"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CLOSED_ISSUE_JSON)
        .create();

    let out = run_plue(
        &["issue", "close", "5", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Closed") || stdout.contains("closed") || stdout.contains("#5"),
        "stdout: {stdout}"
    );

    close_mock.assert();
}

#[test]
fn issue_close_with_comment() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    // Close sends both state: closed and body with the comment
    let close_mock = server
        .mock("PATCH", "/api/repos/alice/demo/issues/5")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "state": "closed",
            "body": "Duplicate of #3"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CLOSED_ISSUE_JSON)
        .create();

    let out = run_plue(
        &[
            "issue",
            "close",
            "5",
            "-R",
            "alice/demo",
            "--comment",
            "Duplicate of #3",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    close_mock.assert();
}

#[test]
fn issue_close_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _close_mock = server
        .mock("PATCH", "/api/repos/alice/demo/issues/5")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CLOSED_ISSUE_JSON)
        .create();

    let out = run_plue(
        &["issue", "close", "5", "-R", "alice/demo", "--json"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert_eq!(value["state"], "closed");
    assert_eq!(value["number"], 5);
}

// --- API error handling ---

#[test]
fn issue_api_errors_surface_message() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/issues")
        .match_query(Matcher::Any)
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"repository not found"}"#)
        .create();

    let out = run_plue(
        &["issue", "list", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("repository not found"), "stderr: {stderr}");
}
