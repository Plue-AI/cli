use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use mockito::Server;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

fn write_config(config_home: &Path, api_url: &str) {
    let config_dir = config_home.join("plue");
    fs::create_dir_all(&config_dir).expect("create config dir");
    let config = format!("api_url: {api_url}\ntoken: plue_testtoken\n");
    fs::write(config_dir.join("config.yml"), &config).expect("write config");
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

const LABEL_JSON: &str = r#"{"id":1,"repository_id":1,"name":"bug","color":"ff0000","description":"Something broken","created_at":"","updated_at":""}"#;
const LABEL_LIST_JSON: &str = r#"[{"id":1,"repository_id":1,"name":"bug","color":"ff0000","description":"Something broken","created_at":"","updated_at":""},{"id":2,"repository_id":1,"name":"enhancement","color":"0075ca","description":"New feature","created_at":"","updated_at":""}]"#;

// ─────────────────────────────────────────────────────────────────
// plue label — label management
// ─────────────────────────────────────────────────────────────────

#[test]
fn label_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    let out = run_plue(
        &["label", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("bug"), "should list bug label: {stdout}");
    assert!(
        stdout.contains("enhancement"),
        "should list enhancement label: {stdout}"
    );
}

#[test]
fn label_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    let out = run_plue(
        &["--json", "label", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "label list --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed.is_array(), "should be array");
    assert_eq!(parsed.as_array().unwrap().len(), 2);
    assert_eq!(parsed[0]["name"].as_str(), Some("bug"));
}

#[test]
fn label_list_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    let out = run_plue(
        &["--toon", "label", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "label list --toon should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("bug"), "should contain label: {stdout}");
}

#[test]
fn label_list_empty() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let out = run_plue(
        &["label", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label list with empty result should succeed"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No labels") || stdout.trim().is_empty(),
        "should indicate no labels: {stdout}"
    );
}

#[test]
fn label_list_table_shows_header() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    let out = run_plue(
        &["label", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("NAME"),
        "table should have NAME header: {stdout}"
    );
    assert!(
        stdout.contains("COLOR"),
        "table should have COLOR header: {stdout}"
    );
}

#[test]
fn label_create_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_JSON)
        .create();

    let out = run_plue(
        &["label", "create", "bug", "-c", "ff0000", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label create should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("bug"),
        "should mention label name: {stdout}"
    );
}

#[test]
fn label_create_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_JSON)
        .create();

    let out = run_plue(
        &[
            "--json",
            "label",
            "create",
            "bug",
            "-c",
            "ff0000",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "label create --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("bug"));
    assert_eq!(parsed["color"].as_str(), Some("ff0000"));
}

#[test]
fn label_create_with_description() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_JSON)
        .create();

    let out = run_plue(
        &[
            "label",
            "create",
            "bug",
            "-c",
            "ff0000",
            "-d",
            "Something broken",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label create with description should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn label_create_default_color() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":1,"repository_id":1,"name":"feature","color":"0075ca","description":"","created_at":"","updated_at":""}"#)
        .create();

    // No -c flag — should use default color 0075ca
    let out = run_plue(
        &["label", "create", "feature", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label create with default color should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn label_delete_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    // First mock: list to find the label ID
    let _mock_list = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    // Second mock: delete by ID
    let _mock_delete = server
        .mock("DELETE", "/api/repos/owner/repo/labels/1")
        .with_status(200)
        .create();

    let out = run_plue(
        &["label", "delete", "bug", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label delete should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Deleted label 'bug'"),
        "should confirm deletion: {stdout}"
    );
}

#[test]
fn label_delete_json_silent() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock_list = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    let _mock_delete = server
        .mock("DELETE", "/api/repos/owner/repo/labels/1")
        .with_status(200)
        .create();

    let out = run_plue(
        &["--json", "label", "delete", "bug", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "label delete --json should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // With --json, delete is silent
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "delete --json should be silent: {stdout}"
    );
}

#[test]
fn label_delete_notfound_fails() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock_list = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_LIST_JSON)
        .create();

    // "nonexistent" label is not in the list — should fail
    let out = run_plue(
        &["label", "delete", "nonexistent-label", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        !out.status.success(),
        "deleting nonexistent label should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("nonexistent"),
        "should mention not found: {stderr}"
    );
}

#[test]
fn label_create_rejects_empty_name() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(
        &["label", "create", "  ", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "should reject empty label name");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("empty") || stderr.contains("name"),
        "should mention empty name: {stderr}"
    );
}

#[test]
fn label_requires_repo_context() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token and no -R flag — should fail with context or auth error
    let out = plue_cmd()
        .args(["label", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue label list");

    assert!(!out.status.success(), "should fail without auth or repo");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "label list is implemented: {stderr}"
    );
}

#[test]
fn label_sends_auth_header() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let auth_mock = server
        .mock("GET", "/api/repos/owner/repo/labels")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let _ = run_plue(
        &["label", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    auth_mock.assert();
}
