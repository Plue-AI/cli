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

// ─────────────────────────────────────────────────────────────────
// plue variable — repository variable management
// ─────────────────────────────────────────────────────────────────

#[test]
fn variable_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/variables")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"name":"MY_VAR","value":"test","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["variable", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "variable list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("MY_VAR"), "should list variable: {stdout}");
}

#[test]
fn variable_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/variables")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"name":"VAR_A","value":"val_a","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"},{"name":"VAR_B","value":"val_b","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["--json", "variable", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "variable list --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed.is_array(), "should be array");
    assert_eq!(parsed.as_array().unwrap().len(), 2);
    assert_eq!(parsed[0]["name"].as_str(), Some("VAR_A"));
}

#[test]
fn variable_list_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/variables")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"name":"TOON_VAR","value":"toon_val","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["--toon", "variable", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "variable list --toon should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("TOON_VAR"),
        "should contain variable: {stdout}"
    );
}

#[test]
fn variable_list_empty() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/variables")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let out = run_plue(
        &["variable", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "variable list with empty result should succeed"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No variables") || stdout.trim().is_empty(),
        "should indicate no variables: {stdout}"
    );
}

#[test]
fn variable_get_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/variables/MY_VAR")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"MY_VAR","value":"test","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &["variable", "get", "MY_VAR", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "variable get should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("test"),
        "should show variable value: {stdout}"
    );
}

#[test]
fn variable_get_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/variables/JSON_VAR")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"JSON_VAR","value":"json_val","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &["--json", "variable", "get", "JSON_VAR", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "variable get --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("JSON_VAR"));
    assert_eq!(parsed["value"].as_str(), Some("json_val"));
}

#[test]
fn variable_set_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/variables")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"MY_VAR","value":"test","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "variable",
            "set",
            "MY_VAR",
            "--body=test",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "variable set should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("MY_VAR"),
        "should mention variable name: {stdout}"
    );
}

#[test]
fn variable_set_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/variables")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"SET_VAR","value":"set_val","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "--json",
            "variable",
            "set",
            "SET_VAR",
            "--body=set_val",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "variable set --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("SET_VAR"));
}

#[test]
fn variable_delete_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("DELETE", "/api/repos/owner/repo/variables/MY_VAR")
        .with_status(200)
        .create();

    let out = run_plue(
        &["variable", "delete", "MY_VAR", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "variable delete should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Deleted variable 'MY_VAR'"),
        "should confirm deletion: {stdout}"
    );
}

#[test]
fn variable_delete_json_silent() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("DELETE", "/api/repos/owner/repo/variables/SILENT_VAR")
        .with_status(200)
        .create();

    let out = run_plue(
        &[
            "--json",
            "variable",
            "delete",
            "SILENT_VAR",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "variable delete --json should succeed: {}",
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
fn variable_set_rejects_empty_name() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(
        &["variable", "set", "  ", "--body=value", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "should reject empty variable name");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("empty") || stderr.contains("name"),
        "should mention empty name: {stderr}"
    );
}

#[test]
fn variable_delete_rejects_empty_name() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(
        &["variable", "delete", "  ", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "should reject empty variable name");
}

#[test]
fn variable_get_rejects_empty_name() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(
        &["variable", "get", "  ", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "should reject empty variable name");
}

#[test]
fn variable_requires_repo_context() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token and no -R flag — should fail with a context or auth error
    let out = plue_cmd()
        .args(["variable", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue variable list");

    assert!(!out.status.success(), "should fail without auth or repo");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "variable list is implemented: {stderr}"
    );
}

#[test]
fn variable_sends_auth_header() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let auth_mock = server
        .mock("GET", "/api/repos/owner/repo/variables")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let _ = run_plue(
        &["variable", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    auth_mock.assert();
}
