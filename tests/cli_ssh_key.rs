//! Integration tests for `plue ssh-key` commands.
//! Uses mockito to simulate the Plue API server.

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

const SSH_KEY_RESPONSE_JSON: &str = r#"{"id":5,"name":"laptop","fingerprint":"SHA256:abc123def456","key_type":"ssh-ed25519","created_at":"2026-02-01T00:00:00Z"}"#;

const SSH_KEY_LIST_JSON: &str = r#"[
  {"id":5,"name":"laptop","fingerprint":"SHA256:abc123","key_type":"ssh-ed25519","created_at":"2026-02-01T00:00:00Z"},
  {"id":6,"name":"desktop","fingerprint":"SHA256:def456","key_type":"ssh-rsa","created_at":"2026-02-02T00:00:00Z"}
]"#;

const SSH_KEY_EMPTY_LIST_JSON: &str = r#"[]"#;

// ─────────────────────────────────────────────────────────────────
// plue ssh-key list
// ─────────────────────────────────────────────────────────────────

#[test]
fn ssh_key_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/user/keys")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SSH_KEY_LIST_JSON)
        .create();

    let out = run_plue(&["ssh-key", "list"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "ssh-key list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("laptop"), "should show key name: {stdout}");
    assert!(stdout.contains("desktop"), "should show key name: {stdout}");
    list_mock.assert();
}

#[test]
fn ssh_key_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user/keys")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SSH_KEY_LIST_JSON)
        .create();

    let out = run_plue(&["--json", "ssh-key", "list"], tmp.path(), &cfg_home);

    assert!(out.status.success(), "ssh-key list --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array(), "should be array");
    assert_eq!(parsed.as_array().unwrap().len(), 2, "should have 2 keys");
    assert_eq!(parsed[0]["name"].as_str(), Some("laptop"));
    assert_eq!(parsed[1]["name"].as_str(), Some("desktop"));
}

#[test]
fn ssh_key_list_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user/keys")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SSH_KEY_LIST_JSON)
        .create();

    let out = run_plue(&["--toon", "ssh-key", "list"], tmp.path(), &cfg_home);

    assert!(out.status.success(), "ssh-key list --toon should succeed");
    // TOON format should produce some output
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty(), "toon output should not be empty");
}

#[test]
fn ssh_key_list_empty() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user/keys")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SSH_KEY_EMPTY_LIST_JSON)
        .create();

    let out = run_plue(&["ssh-key", "list"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "ssh-key list should succeed for empty list"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No SSH keys") || stdout.is_empty(),
        "should indicate no keys: {stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────
// plue ssh-key add
// ─────────────────────────────────────────────────────────────────

#[test]
fn ssh_key_add_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let add_mock = server
        .mock("POST", "/api/user/keys")
        .match_header("authorization", "token plue_testtoken")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(SSH_KEY_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &[
            "ssh-key",
            "add",
            "--title",
            "laptop",
            "--key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test-key",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "ssh-key add should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("laptop") || stdout.contains("Added"),
        "should confirm key added: {stdout}"
    );
    add_mock.assert();
}

#[test]
fn ssh_key_add_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/user/keys")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(SSH_KEY_RESPONSE_JSON)
        .create();

    let out = run_plue(
        &[
            "--json",
            "ssh-key",
            "add",
            "--title",
            "laptop",
            "--key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test-key",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(out.status.success(), "ssh-key add --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(parsed["id"].as_i64(), Some(5));
    assert_eq!(parsed["name"].as_str(), Some("laptop"));
    assert_eq!(parsed["key_type"].as_str(), Some("ssh-ed25519"));
}

#[test]
fn ssh_key_add_rejects_empty_title() {
    let (tmp, cfg_home) = setup_temp_workspace();
    // No mock server needed — validation happens before API call

    let out = run_plue(
        &[
            "ssh-key",
            "add",
            "--title",
            "   ",
            "--key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test-key",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "should reject empty title");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("title") || stderr.contains("empty"),
        "should mention title error: {stderr}"
    );
}

#[test]
fn ssh_key_add_rejects_empty_key() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = run_plue(
        &["ssh-key", "add", "--title", "laptop", "--key", "   "],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "should reject empty key");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("key") || stderr.contains("empty"),
        "should mention key error: {stderr}"
    );
}

#[test]
fn ssh_key_add_handles_422() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/user/keys")
        .with_status(422)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"validation failed","errors":[{"field":"key","message":"key already in use"}]}"#)
        .create();

    let out = run_plue(
        &[
            "ssh-key",
            "add",
            "--title",
            "dupe",
            "--key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test-key",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "should fail on 422");
}

// ─────────────────────────────────────────────────────────────────
// plue ssh-key delete
// ─────────────────────────────────────────────────────────────────

#[test]
fn ssh_key_delete_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let delete_mock = server
        .mock("DELETE", "/api/user/keys/5")
        .match_header("authorization", "token plue_testtoken")
        .with_status(204)
        .create();

    let out = run_plue(&["ssh-key", "delete", "5"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "ssh-key delete should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Deleted") || stdout.contains("5"),
        "should confirm deletion: {stdout}"
    );
    delete_mock.assert();
}

#[test]
fn ssh_key_delete_not_found() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("DELETE", "/api/user/keys/9999")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"key not found"}"#)
        .create();

    let out = run_plue(&["ssh-key", "delete", "9999"], tmp.path(), &cfg_home);

    assert!(!out.status.success(), "should fail on 404");
}

#[test]
fn ssh_key_delete_rejects_zero_id() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = run_plue(&["ssh-key", "delete", "0"], tmp.path(), &cfg_home);

    assert!(!out.status.success(), "should reject id=0");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid SSH key id"),
        "should mention invalid id: {stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────
// plue ssh-key auth error handling
// ─────────────────────────────────────────────────────────────────

#[test]
fn ssh_key_list_without_auth_fails() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Write a config with no token to ensure auth failure before making any API call.
    // We do NOT write any token so ApiClient::from_config fails with "not authenticated".
    let candidates = [
        cfg_home.join("plue"),
        cfg_home.join(".config").join("plue"),
        cfg_home
            .join("Library")
            .join("Application Support")
            .join("plue"),
    ];
    for config_dir in &candidates {
        fs::create_dir_all(config_dir).expect("create config dir");
        // api_url only, no token
        fs::write(
            config_dir.join("config.yml"),
            "api_url: https://plue.dev/api\n",
        )
        .expect("write config");
    }

    // No PLUE_TOKEN env var set — ApiClient will fail to build
    let out = plue_cmd()
        .args(["ssh-key", "list"])
        .current_dir(tmp.path())
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        // Explicitly do NOT set PLUE_TOKEN
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "should fail without auth");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not authenticated") || stderr.contains("auth"),
        "should mention auth error: {stderr}"
    );
}
