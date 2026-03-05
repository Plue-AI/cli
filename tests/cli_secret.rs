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
// plue secret — repository secret management
// ─────────────────────────────────────────────────────────────────

#[test]
fn secret_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/secrets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"name":"MY_SECRET","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["secret", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "secret list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("MY_SECRET"), "should list secret: {stdout}");
}

#[test]
fn secret_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/secrets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"name":"MY_SECRET","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"},{"name":"ANOTHER","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["--json", "secret", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "secret list --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed.is_array(), "should be array");
    assert_eq!(parsed.as_array().unwrap().len(), 2);
    assert_eq!(parsed[0]["name"].as_str(), Some("MY_SECRET"));
}

#[test]
fn secret_list_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/secrets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"name":"TOON_SECRET","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["--toon", "secret", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "secret list --toon should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("TOON_SECRET"),
        "should contain secret: {stdout}"
    );
}

#[test]
fn secret_list_empty() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/owner/repo/secrets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let out = run_plue(
        &["secret", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "secret list with empty result should succeed"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No secrets") || stdout.trim().is_empty(),
        "should indicate no secrets: {stdout}"
    );
}

#[test]
fn secret_set_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/secrets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"MY_SECRET","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "secret",
            "set",
            "MY_SECRET",
            "--body=super_secret",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "secret set should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("MY_SECRET"),
        "should mention secret name: {stdout}"
    );
}

#[test]
fn secret_set_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos/owner/repo/secrets")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"JSON_SECRET","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "--json",
            "secret",
            "set",
            "JSON_SECRET",
            "--body=value",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(out.status.success(), "secret set --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["name"].as_str(), Some("JSON_SECRET"));
}

#[test]
fn secret_delete_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("DELETE", "/api/repos/owner/repo/secrets/MY_SECRET")
        .with_status(200)
        .create();

    let out = run_plue(
        &["secret", "delete", "MY_SECRET", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "secret delete should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Deleted secret 'MY_SECRET'"),
        "should confirm deletion: {stdout}"
    );
}

#[test]
fn secret_delete_json_silent() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("DELETE", "/api/repos/owner/repo/secrets/SILENT_SECRET")
        .with_status(200)
        .create();

    let out = run_plue(
        &[
            "--json",
            "secret",
            "delete",
            "SILENT_SECRET",
            "-R",
            "owner/repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "secret delete --json should succeed: {}",
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
fn secret_set_rejects_empty_name() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(
        &["secret", "set", "  ", "--body=value", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "should reject empty secret name");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("empty") || stderr.contains("name"),
        "should mention empty name: {stderr}"
    );
}

#[test]
fn secret_delete_rejects_empty_name() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(
        &["secret", "delete", "  ", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(!out.status.success(), "should reject empty secret name");
}

#[test]
fn secret_requires_repo_context() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token and no -R flag — should fail with a context or auth error
    let out = plue_cmd()
        .args(["secret", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue secret list");

    assert!(!out.status.success(), "should fail without auth or repo");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "secret list is implemented: {stderr}"
    );
}

#[test]
fn secret_sends_auth_header() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let auth_mock = server
        .mock("GET", "/api/repos/owner/repo/secrets")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let _ = run_plue(
        &["secret", "list", "-R", "owner/repo"],
        tmp.path(),
        &cfg_home,
    );
    auth_mock.assert();
}
