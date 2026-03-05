//! Integration tests for `plue workflow` and `plue run` commands.
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

const WORKFLOW_LIST_JSON: &str = r#"[
  {"id":1,"repository_id":10,"name":"CI","path":".plue/workflows/ci.ts","is_active":true,"created_at":"2026-02-01T00:00:00Z","updated_at":"2026-02-01T00:00:00Z"},
  {"id":2,"repository_id":10,"name":"Deploy","path":".plue/workflows/deploy.ts","is_active":false,"created_at":"2026-02-01T00:00:00Z","updated_at":"2026-02-01T00:00:00Z"}
]"#;

const WORKFLOW_EMPTY_LIST_JSON: &str = r#"[]"#;

const WORKFLOW_RUN_JSON: &str = r#"{"id":42,"repository_id":10,"workflow_definition_id":1,"status":"completed","trigger_event":"dispatch","trigger_ref":"main","trigger_commit_sha":"abc123def456","started_at":"2026-02-01T10:00:00Z","completed_at":"2026-02-01T10:05:00Z","created_at":"2026-02-01T10:00:00Z","updated_at":"2026-02-01T10:05:00Z"}"#;

const WORKFLOW_RUN_LIST_JSON: &str = r#"[
  {"id":42,"repository_id":10,"workflow_definition_id":1,"status":"completed","trigger_event":"dispatch","trigger_ref":"main","trigger_commit_sha":"abc123","started_at":"2026-02-01T10:00:00Z","completed_at":"2026-02-01T10:05:00Z","created_at":"2026-02-01T10:00:00Z","updated_at":"2026-02-01T10:05:00Z"},
  {"id":43,"repository_id":10,"workflow_definition_id":1,"status":"running","trigger_event":"dispatch","trigger_ref":"main","trigger_commit_sha":"def456","started_at":"2026-02-01T11:00:00Z","completed_at":null,"created_at":"2026-02-01T11:00:00Z","updated_at":"2026-02-01T11:00:00Z"}
]"#;

// ─────────────────────────────────────────────────────────────────
// plue workflow list
// ─────────────────────────────────────────────────────────────────

#[test]
fn workflow_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/workflows")
        .match_header("authorization", "token plue_testtoken")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_LIST_JSON)
        .create();

    let out = run_plue(
        &["workflow", "list", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "workflow list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CI"), "should show workflow name: {stdout}");
    assert!(
        stdout.contains("Deploy"),
        "should show workflow name: {stdout}"
    );
    list_mock.assert();
}

#[test]
fn workflow_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/workflows")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_LIST_JSON)
        .create();

    let out = run_plue(
        &["--json", "workflow", "list", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "workflow list --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Should parse as JSON array
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array(), "should be array, got: {parsed}");
    assert_eq!(
        parsed.as_array().unwrap().len(),
        2,
        "should have 2 workflows"
    );
}

#[test]
fn workflow_list_empty_repo() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/workflows")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_EMPTY_LIST_JSON)
        .create();

    let out = run_plue(
        &["workflow", "list", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "workflow list should succeed for empty repo"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No workflows") || stdout.is_empty(),
        "should indicate no workflows: {stdout}"
    );
}

#[test]
fn workflow_run_dispatches_to_api() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let dispatch_mock = server
        .mock("POST", "/api/repos/alice/demo/workflows/1/dispatches")
        .match_header("authorization", "token plue_testtoken")
        .with_status(204)
        .create();

    let out = run_plue(
        &["workflow", "run", "1", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "workflow run should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Triggered") || stdout.contains("workflow"),
        "should confirm trigger: {stdout}"
    );
    dispatch_mock.assert();
}

#[test]
fn workflow_run_with_custom_ref() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let dispatch_mock = server
        .mock("POST", "/api/repos/alice/demo/workflows/2/dispatches")
        .match_header("authorization", "token plue_testtoken")
        .with_status(204)
        .create();

    let out = run_plue(
        &[
            "workflow",
            "run",
            "2",
            "--ref",
            "develop",
            "-R",
            "alice/demo",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "workflow run with custom ref should succeed"
    );
    dispatch_mock.assert();
}

#[test]
fn workflow_list_handles_401() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/workflows")
        .match_query(Matcher::Any)
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"unauthorized"}"#)
        .create();

    let out = run_plue(
        &["workflow", "list", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "workflow list should fail on 401");
}

#[test]
fn workflow_list_handles_404() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/nonexistent/workflows")
        .match_query(Matcher::Any)
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"repository not found"}"#)
        .create();

    let out = run_plue(
        &["workflow", "list", "-R", "alice/nonexistent"],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "workflow list should fail on 404");
}

// ─────────────────────────────────────────────────────────────────
// plue run (workflow run management)
// ─────────────────────────────────────────────────────────────────

#[test]
fn run_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/workflows/1/runs")
        .match_header("authorization", "token plue_testtoken")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_RUN_LIST_JSON)
        .create();

    let out = run_plue(
        &["run", "list", "1", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "run list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("42") || stdout.contains("completed"),
        "should show run info: {stdout}"
    );
    list_mock.assert();
}

#[test]
fn run_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/workflows/1/runs")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_RUN_LIST_JSON)
        .create();

    let out = run_plue(
        &["--json", "run", "list", "1", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "run list --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array(), "should be array");
    assert_eq!(parsed.as_array().unwrap().len(), 2, "should have 2 runs");
}

#[test]
fn run_view_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let view_mock = server
        .mock("GET", "/api/repos/alice/demo/actions/runs/42")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_RUN_JSON)
        .create();

    let out = run_plue(
        &["run", "view", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "run view should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("42") || stdout.contains("completed"),
        "should show run info: {stdout}"
    );
    view_mock.assert();
}

#[test]
fn run_view_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/actions/runs/42")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(WORKFLOW_RUN_JSON)
        .create();

    let out = run_plue(
        &["--json", "run", "view", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "run view --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_object(), "should be object");
    assert_eq!(parsed["id"].as_i64(), Some(42));
    assert_eq!(parsed["status"].as_str(), Some("completed"));
}

#[test]
fn run_view_not_found() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/demo/actions/runs/9999")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"run not found"}"#)
        .create();

    let out = run_plue(
        &["run", "view", "9999", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "run view should fail on 404");
}

#[test]
fn run_rerun_requires_run_id() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // run rerun requires run_id arg - tests it at clap level
    let out = plue_cmd()
        .args(["run", "rerun"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue run rerun");

    // Should fail (missing required arg)
    assert!(!out.status.success());
}

#[test]
fn run_rerun_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    // Mock the rerun endpoint
    let rerun_mock = server
        .mock("POST", "/api/repos/alice/demo/actions/runs/123/rerun")
        .match_header("authorization", "token plue_testtoken")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"workflow_definition_id":7,"workflow_run_id":456,"steps":[{"step_id":1,"task_id":10}]}"#)
        .create();

    let out = run_plue(
        &["run", "rerun", "123", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    rerun_mock.assert();
    assert!(
        out.status.success(),
        "run rerun should succeed: stderr={}",
        stderr
    );
    assert!(
        stdout.contains("456"),
        "output should contain new run ID: {}",
        stdout
    );
}

#[test]
fn run_rerun_not_found() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let rerun_mock = server
        .mock("POST", "/api/repos/alice/demo/actions/runs/999/rerun")
        .match_header("authorization", "token plue_testtoken")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"workflow run not found"}"#)
        .create();

    let out = run_plue(
        &["run", "rerun", "999", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    rerun_mock.assert();
    assert!(!out.status.success(), "run rerun should fail on 404");
}
