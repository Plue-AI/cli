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
// plue api — raw HTTP API calls
// ─────────────────────────────────────────────────────────────────

#[test]
fn api_raw_get_request() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/test/route")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"ok"}"#)
        .create();

    let out = run_plue(&["api", "-X", "GET", "/test/route"], tmp.path(), &cfg_home);
    assert!(
        out.status.success(),
        "api GET should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"), "should contain status: {stdout}");
}

#[test]
fn api_default_method_is_get() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login":"alice","id":1}"#)
        .create();

    // No -X flag — should default to GET
    let out = run_plue(&["api", "/user"], tmp.path(), &cfg_home);
    assert!(
        out.status.success(),
        "api /user (default GET) should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alice"), "should show user: {stdout}");
}

#[test]
fn api_post_with_fields() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/repos")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":1,"name":"newrepo","full_name":"alice/newrepo"}"#)
        .create();

    let out = run_plue(
        &["api", "-X", "POST", "/repos", "-f", "name=newrepo"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "api POST with fields should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("newrepo"),
        "should show created repo: {stdout}"
    );
}

#[test]
fn api_delete_request() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("DELETE", "/api/repos/alice/test")
        .with_status(204)
        .create();

    let out = run_plue(
        &["api", "-X", "DELETE", "/repos/alice/test"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "api DELETE should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn api_json_output_format() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login":"alice","id":1}"#)
        .create();

    let out = run_plue(&["--json", "api", "/user"], tmp.path(), &cfg_home);
    assert!(out.status.success(), "api --json should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Should be parseable JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["login"].as_str(), Some("alice"));
}

#[test]
fn api_toon_output_format() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login":"alice","id":1}"#)
        .create();

    let out = run_plue(&["--toon", "api", "/user"], tmp.path(), &cfg_home);
    assert!(out.status.success(), "api --toon should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("alice"),
        "TOON output should contain value: {stdout}"
    );
}

#[test]
fn api_with_custom_header() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let auth_mock = server
        .mock("GET", "/api/test")
        .match_header("X-Custom", "myvalue")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"ok"}"#)
        .create();

    let out = run_plue(
        &["api", "/test", "-H", "X-Custom:myvalue"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "api with custom header should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    auth_mock.assert();
}

#[test]
fn api_invalid_method_fails() {
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, "https://plue.dev/api");

    let out = run_plue(&["api", "-X", "CONNECT", "/test"], tmp.path(), &cfg_home);
    assert!(!out.status.success(), "api with invalid method should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid method") || stderr.contains("CONNECT"),
        "should mention invalid method: {stderr}"
    );
}

#[test]
fn api_non_json_response_prints_raw() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/health")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("OK")
        .create();

    let out = run_plue(&["api", "/health"], tmp.path(), &cfg_home);
    assert!(
        out.status.success(),
        "api with text response should succeed"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("OK"), "should print raw body: {stdout}");
}

#[test]
fn api_requires_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();
    // No config written — no token

    let out = plue_cmd()
        .args(["api", "/user"])
        .current_dir(tmp.path())
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .output()
        .expect("run plue api");

    assert!(!out.status.success(), "api should require auth");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not authenticated") || stderr.contains("auth"),
        "should mention auth: {stderr}"
    );
}

#[test]
fn api_sends_authorization_header() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let auth_mock = server
        .mock("GET", "/api/user")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login":"alice"}"#)
        .create();

    let out = run_plue(&["api", "/user"], tmp.path(), &cfg_home);
    assert!(out.status.success());
    auth_mock.assert();
}

#[test]
fn api_put_request() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("PUT", "/api/repos/alice/demo/landings/1/land")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":1,"state":"landed"}"#)
        .create();

    let out = run_plue(
        &["api", "-X", "PUT", "/repos/alice/demo/landings/1/land"],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "api PUT should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn api_patch_request_with_fields() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("PATCH", "/api/repos/alice/demo/issues/1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":1,"title":"Updated title","state":"open"}"#)
        .create();

    let out = run_plue(
        &[
            "api",
            "-X",
            "PATCH",
            "/repos/alice/demo/issues/1",
            "-f",
            "title=Updated title",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "api PATCH should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn api_multiple_fields_create_json_body() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("POST", "/api/user/repos")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":1,"name":"myrepo","description":"My repo"}"#)
        .create();

    let out = run_plue(
        &[
            "api",
            "-X",
            "POST",
            "/user/repos",
            "-f",
            "name=myrepo",
            "-f",
            "description=My repo",
        ],
        tmp.path(),
        &cfg_home,
    );
    assert!(
        out.status.success(),
        "api POST with multiple fields should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
