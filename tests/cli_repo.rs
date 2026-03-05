mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use mockito::Server;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

fn write_config(config_home: &Path, api_url: &str) {
    // Write to multiple possible config locations for cross-platform compatibility
    let possible_dirs = [
        config_home.join("plue"),
        config_home.join(".config").join("plue"),
        config_home
            .join("Library")
            .join("Application Support")
            .join("plue"),
    ];
    let config = format!("api_url: {}\ntoken: plue_testtoken\n", api_url);
    for dir in &possible_dirs {
        let _ = fs::create_dir_all(dir);
        let _ = fs::write(dir.join("config.yml"), &config);
    }
}

fn setup_temp_workspace() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = tmp.path().join("cfg");
    fs::create_dir_all(&cfg_home).expect("create cfg dir");
    (tmp, cfg_home)
}

#[cfg(unix)]
fn write_executable_script(path: &Path, script: &str) {
    fs::write(path, script).expect("write script");
    let mut perms = fs::metadata(path).expect("script metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("set executable bit");
}

#[cfg(unix)]
fn setup_fake_clone_binaries(tmp: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let bin_dir = tmp.path().join("fake-bin");
    fs::create_dir_all(&bin_dir).expect("create fake bin dir");
    let log_file = tmp.path().join("clone-calls.log");

    write_executable_script(
        &bin_dir.join("jj"),
        r#"#!/bin/sh
echo "jj:$*" >> "$PLUE_CMD_LOG"
exit "${PLUE_FAKE_JJ_EXIT:-0}"
"#,
    );
    write_executable_script(
        &bin_dir.join("git"),
        r#"#!/bin/sh
echo "git:$*" >> "$PLUE_CMD_LOG"
exit "${PLUE_FAKE_GIT_EXIT:-0}"
"#,
    );

    (bin_dir, log_file)
}

#[test]
fn repo_create_posts_request_and_prints_success() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/user/repos")
        .match_header("authorization", "token plue_testtoken")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":1,"owner":"alice","name":"my-new-repo","full_name":"alice/my-new-repo","description":"A test repository","is_public":true,"default_branch":"main","clone_url":"git@plue.dev:alice/my-new-repo.git","created_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args([
            "repo",
            "create",
            "my-new-repo",
            "--description",
            "A test repository",
        ])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Created repository"), "stdout: {stdout}");
    assert!(stdout.contains("alice/my-new-repo"), "stdout: {stdout}");
    assert!(stdout.contains("Clone URL:"), "stdout: {stdout}");

    create_mock.assert();
}

#[test]
fn repo_create_private_posts_private_flag() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/user/repos")
        .match_header("authorization", "token plue_testtoken")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":2,"owner":"alice","name":"private-repo","full_name":"alice/private-repo","description":"","is_public":false,"default_branch":"main","clone_url":"git@plue.dev:alice/private-repo.git","created_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args(["repo", "create", "private-repo", "--private"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    create_mock.assert();
}

#[test]
fn repo_create_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _create_mock = server
        .mock("POST", "/api/user/repos")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":3,"owner":"alice","name":"json-repo","full_name":"alice/json-repo","description":"","is_public":true,"default_branch":"main","clone_url":"git@plue.dev:alice/json-repo.git","created_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args(["--json", "repo", "create", "json-repo"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert_eq!(parsed["name"], "json-repo");
    assert_eq!(parsed["full_name"], "alice/json-repo");
}

#[test]
fn repo_create_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _create_mock = server
        .mock("POST", "/api/user/repos")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":4,"owner":"alice","name":"toon-repo","full_name":"alice/toon-repo","description":"Token output","is_public":true,"default_branch":"main","clone_url":"git@plue.dev:alice/toon-repo.git","created_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args([
            "--toon",
            "repo",
            "create",
            "toon-repo",
            "--description",
            "Token output",
        ])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("name:toon-repo"), "stdout: {stdout}");
    assert!(
        stdout.contains("full_name:alice/toon-repo"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("description:\"Token output\""),
        "stdout: {stdout}"
    );
    assert!(!stdout.contains('{'), "stdout: {stdout}");
}

#[test]
fn repo_create_api_error_surfaces_message() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _create_mock = server
        .mock("POST", "/api/user/repos")
        .with_status(422)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"repository name already exists"}"#)
        .create();

    let out = plue_cmd()
        .args(["repo", "create", "existing-repo"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("already exists") || stderr.contains("message"),
        "stderr: {stderr}"
    );
}

#[test]
fn repo_clone_requires_repository_argument() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["repo", "clone"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "expected clap usage error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("required arguments were not provided"),
        "stderr: {stderr}"
    );
}

#[test]
fn repo_clone_accepts_repository_directory_and_gitflags_shape() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let empty_path = tmp.path().join("empty-bin");
    fs::create_dir_all(&empty_path).expect("create fake path");

    let out = plue_cmd()
        .args([
            "repo",
            "clone",
            "alice/my-repo",
            "my-dir",
            "--",
            "--depth",
            "1",
        ])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PATH", &empty_path)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "expected runtime failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("unexpected argument"), "stderr: {stderr}");
}

#[test]
fn repo_clone_shorthand_looks_up_repo_metadata() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));
    let empty_path = tmp.path().join("empty-bin");
    fs::create_dir_all(&empty_path).expect("create fake path");

    let lookup = server
        .mock("GET", "/api/repos/alice/demo")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":7,"owner":"alice","name":"demo","full_name":"alice/demo","description":"","is_public":true,"default_bookmark":"main","topics":[],"is_archived":false,"is_fork":false,"num_stars":0,"num_watches":0,"num_issues":0,"clone_url":"git@plue.dev:alice/demo.git","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args(["repo", "clone", "alice/demo"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PATH", &empty_path)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "expected clone command failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "stderr should not be clap parse error: {stderr}"
    );
    lookup.assert();
}

#[test]
fn repo_clone_shorthand_surfaces_api_errors() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _lookup = server
        .mock("GET", "/api/repos/alice/missing")
        .match_header("authorization", "token plue_testtoken")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"repository not found"}"#)
        .create();

    let out = plue_cmd()
        .args(["repo", "clone", "alice/missing"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "expected clone to fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("repository not found"), "stderr: {stderr}");
}

#[cfg(unix)]
#[test]
fn repo_clone_uses_jj_first_and_preserves_arg_order() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let (bin_dir, log_file) = setup_fake_clone_binaries(&tmp);

    let out = plue_cmd()
        .args([
            "repo",
            "clone",
            "https://plue.dev/alice/demo.git",
            "my-dir",
            "--",
            "--depth",
            "1",
        ])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PATH", &bin_dir)
        .env("PLUE_CMD_LOG", &log_file)
        .env("PLUE_FAKE_JJ_EXIT", "0")
        .env("PLUE_FAKE_GIT_EXIT", "0")
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let log = fs::read_to_string(&log_file).expect("read command log");
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 1, "log: {log}");
    assert_eq!(
        lines[0],
        "jj:git clone https://plue.dev/alice/demo.git my-dir --depth 1"
    );
}

#[cfg(unix)]
#[test]
fn repo_clone_falls_back_to_git_when_jj_fails() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let (bin_dir, log_file) = setup_fake_clone_binaries(&tmp);

    let out = plue_cmd()
        .args([
            "repo",
            "clone",
            "https://plue.dev/alice/demo.git",
            "my-dir",
            "--",
            "--depth",
            "1",
        ])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PATH", &bin_dir)
        .env("PLUE_CMD_LOG", &log_file)
        .env("PLUE_FAKE_JJ_EXIT", "1")
        .env("PLUE_FAKE_GIT_EXIT", "0")
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let log = fs::read_to_string(&log_file).expect("read command log");
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2, "log: {log}");
    assert_eq!(
        lines[0],
        "jj:git clone https://plue.dev/alice/demo.git my-dir --depth 1"
    );
    assert_eq!(
        lines[1],
        "git:clone https://plue.dev/alice/demo.git my-dir --depth 1"
    );
}

#[cfg(unix)]
#[test]
fn repo_clone_fails_when_jj_and_git_fail() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let (bin_dir, log_file) = setup_fake_clone_binaries(&tmp);

    let out = plue_cmd()
        .args(["repo", "clone", "https://plue.dev/alice/demo.git"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PATH", &bin_dir)
        .env("PLUE_CMD_LOG", &log_file)
        .env("PLUE_FAKE_JJ_EXIT", "1")
        .env("PLUE_FAKE_GIT_EXIT", "2")
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success(), "expected clone to fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("jj") && stderr.contains("git"),
        "stderr: {stderr}"
    );
}

#[test]
fn repo_list_requires_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token configured — should fail with auth error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["repo", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "repo list is now implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("not authenticated") || stderr.contains("auth"),
        "stderr should mention authentication: {stderr}"
    );
}

#[test]
fn repo_list_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user/repos")
        .match_header("authorization", "token plue_testtoken")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"id":1,"name":"my-repo","description":"A repo","is_public":true,"default_bookmark":"main","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-20T00:00:00Z"}]"#,
        )
        .create();

    let out = plue_cmd()
        .args(["repo", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue repo list");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("my-repo"), "stdout: {stdout}");
}

#[test]
fn repo_list_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/user/repos")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"id":1,"name":"my-repo","description":"A repo","is_public":true,"default_bookmark":"main","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-20T00:00:00Z"}]"#,
        )
        .create();

    let out = plue_cmd()
        .args(["--json", "repo", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue repo list --json");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["name"], "my-repo");
}

#[test]
fn repo_view_requires_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token configured — should fail with auth or repo detection error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["repo", "view"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "repo view is now implemented — should not say not yet implemented: {stderr}"
    );
}

#[test]
fn repo_view_with_mock_server() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/my-repo")
        .match_header("authorization", "token plue_testtoken")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":7,"owner":"alice","name":"my-repo","full_name":"alice/my-repo","description":"A repository","is_public":false,"default_bookmark":"trunk","topics":["jj","rust"],"is_archived":false,"is_fork":false,"num_stars":8,"num_watches":5,"num_issues":3,"clone_url":"git@plue.dev:alice/my-repo.git","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-21T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args(["repo", "view", "--repo=alice/my-repo"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue repo view");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alice/my-repo"), "stdout: {stdout}");
}

#[test]
fn repo_view_json_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _mock = server
        .mock("GET", "/api/repos/alice/my-repo")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":7,"owner":"alice","name":"my-repo","full_name":"alice/my-repo","description":"A repository","is_public":false,"default_bookmark":"trunk","topics":[],"is_archived":false,"is_fork":false,"num_stars":0,"num_watches":0,"num_issues":0,"clone_url":"git@plue.dev:alice/my-repo.git","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-21T00:00:00Z"}"#,
        )
        .create();

    let out = plue_cmd()
        .args(["--json", "repo", "view", "--repo=alice/my-repo"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue repo view --json");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert_eq!(parsed["owner"], "alice");
    assert_eq!(parsed["name"], "my-repo");
}
