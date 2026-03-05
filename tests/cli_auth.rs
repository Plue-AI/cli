mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

fn setup_temp_workspace() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = tmp.path().join("cfg");
    fs::create_dir_all(&cfg_home).expect("create cfg dir");
    (tmp, cfg_home)
}

/// Get all possible config paths that plue might use
fn get_possible_config_paths(cfg_home: &Path) -> Vec<PathBuf> {
    vec![
        cfg_home.join("plue").join("config.yml"),
        cfg_home.join(".config").join("plue").join("config.yml"),
        cfg_home
            .join("Library")
            .join("Application Support")
            .join("plue")
            .join("config.yml"),
    ]
}

/// Find the existing config file or return the default path
fn find_config_file(cfg_home: &Path) -> Option<PathBuf> {
    get_possible_config_paths(cfg_home)
        .into_iter()
        .find(|p| p.exists())
}

/// Setup config directories for all platforms
fn setup_config_dirs(cfg_home: &Path) {
    for path in get_possible_config_paths(cfg_home) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
    }
}

fn unique_test_host(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    format!("{prefix}-{nanos}.test")
}

// ---------------------------------------------------------------------------
// auth status
// ---------------------------------------------------------------------------

#[test]
fn auth_status_shows_not_logged_in_when_no_config() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .arg("auth")
        .arg("status")
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Not logged in"), "stdout: {stdout}");
}

#[test]
fn auth_status_shows_logged_in_when_token_set() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Setup all possible config directories and write config
    setup_config_dirs(&cfg_home);
    for path in get_possible_config_paths(&cfg_home) {
        let _ = fs::write(
            &path,
            "api_url: http://localhost:4000/api\ntoken: plue_testtoken123\n",
        );
    }

    // Clear PLUE_TOKEN env to ensure we read from config file
    let output = plue_cmd()
        .arg("auth")
        .arg("status")
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Logged in"), "stdout: {stdout}");
    assert!(
        stdout.contains("config file") || stdout.contains("PLUE_TOKEN env"),
        "stdout: {stdout}"
    );
}

#[test]
fn auth_status_json_format() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["--json", "auth", "status"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed["logged_in"].is_boolean());
    assert!(parsed["api_url"].is_string());
    assert!(parsed["token_set"].is_boolean());
}

#[test]
fn auth_status_toon_format() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["--toon", "auth", "status"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("logged_in:false"), "stdout: {stdout}");
    assert!(stdout.contains("token_set:false"), "stdout: {stdout}");
    assert!(!stdout.contains('{'), "stdout: {stdout}");
}

// ---------------------------------------------------------------------------
// auth login
// ---------------------------------------------------------------------------

#[test]
fn auth_login_with_insecure_storage_writes_config() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "login", "--with-token", "--insecure-storage"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_valid_token_123")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Logged in"), "stdout: {stdout}");

    // Verify config was written
    let config_file = find_config_file(&cfg_home);
    assert!(
        config_file.is_some(),
        "Config file should exist in one of the expected locations"
    );

    let config_content = fs::read_to_string(config_file.unwrap()).expect("read config");
    assert!(config_content.contains("plue_valid_token_123"));

    // Verify stderr has insecure warning
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("plain text") || stderr.contains("warning"),
        "stderr should warn about insecure storage: {stderr}"
    );
}

#[test]
fn auth_login_with_token_rejects_invalid_prefix() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "login", "--with-token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .write_stdin("invalid_token_without_prefix")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("plue_"), "stderr: {stderr}");
}

#[test]
fn auth_login_with_token_rejects_empty() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "login", "--with-token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .write_stdin("")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no token") || stderr.contains("empty"),
        "stderr: {stderr}"
    );
}

#[test]
fn auth_login_with_token_defaults_to_keyring_storage() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let host = unique_test_host("login-keyring");
    let keyring_file = tmp.path().join("keyring-store.json");

    let output = plue_cmd()
        .args(["auth", "login", "--with-token", "--hostname", &host])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_secure_keyring_token_123")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Secure login should not persist token into config file by default.
    if let Some(config_file) = find_config_file(&cfg_home) {
        let config_content = fs::read_to_string(config_file).expect("read config");
        assert!(
            !config_content.contains("plue_secure_keyring_token_123"),
            "token should not be written to config: {config_content}"
        );
    }
}

#[test]
fn auth_login_without_token_flag_errors() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "login"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("interactive") || stderr.contains("not yet implemented"),
        "stderr: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// auth logout
// ---------------------------------------------------------------------------

#[test]
fn auth_logout_succeeds() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "logout"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Logged out"), "stdout: {stdout}");
}

#[test]
fn auth_logout_clears_config_file_token() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // First write a config file with a token
    setup_config_dirs(&cfg_home);
    for path in get_possible_config_paths(&cfg_home) {
        let _ = fs::write(
            &path,
            "api_url: https://plue.dev/api\ntoken: plue_should_be_cleared\n",
        );
    }

    let output = plue_cmd()
        .args(["auth", "logout"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify config file token was cleared
    if let Some(config_file) = find_config_file(&cfg_home) {
        let content = fs::read_to_string(&config_file).expect("read config");
        assert!(
            !content.contains("plue_should_be_cleared"),
            "Token should have been cleared from config: {content}"
        );
    }
}

#[test]
fn auth_logout_preserves_api_url_in_config() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Write a config file with token and custom api_url
    setup_config_dirs(&cfg_home);
    for path in get_possible_config_paths(&cfg_home) {
        let _ = fs::write(
            &path,
            "api_url: https://custom.plue.dev/api\ntoken: plue_to_remove\n",
        );
    }

    let output = plue_cmd()
        .args(["auth", "logout"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Token should be gone but api_url should be preserved
    if let Some(config_file) = find_config_file(&cfg_home) {
        let content = fs::read_to_string(&config_file).expect("read config");
        assert!(
            !content.contains("plue_to_remove"),
            "Token should be cleared: {content}"
        );
        assert!(
            content.contains("custom.plue.dev"),
            "api_url should be preserved: {content}"
        );
    }
}

#[test]
fn auth_logout_with_hostname_shows_correct_host() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "logout", "--hostname", "staging.plue.dev"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("staging.plue.dev"),
        "Should show the hostname in logout message: {stdout}"
    );
}

#[test]
fn auth_logout_is_idempotent() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Run logout twice — both should succeed
    for _ in 0..2 {
        let output = plue_cmd()
            .args(["auth", "logout"])
            .env("XDG_CONFIG_HOME", &cfg_home)
            .env("HOME", &cfg_home)
            .env_remove("PLUE_TOKEN")
            .current_dir(tmp.path())
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Logged out"), "stdout: {stdout}");
    }
}

#[test]
fn auth_logout_then_status_shows_not_logged_in() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // First login with insecure storage
    let login_output = plue_cmd()
        .args(["auth", "login", "--with-token", "--insecure-storage"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_roundtrip_token")
        .output()
        .unwrap();
    assert!(
        login_output.status.success(),
        "login failed: {}",
        String::from_utf8_lossy(&login_output.stderr)
    );

    // Verify logged in
    let status_output = plue_cmd()
        .args(["auth", "status"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(
        stdout.contains("Logged in"),
        "Should be logged in: {stdout}"
    );

    // Now logout
    let logout_output = plue_cmd()
        .args(["auth", "logout"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        logout_output.status.success(),
        "logout failed: {}",
        String::from_utf8_lossy(&logout_output.stderr)
    );

    // Verify not logged in anymore
    let status_output2 = plue_cmd()
        .args(["auth", "status"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout2 = String::from_utf8_lossy(&status_output2.stdout);
    assert!(
        stdout2.contains("Not logged in"),
        "Should show not logged in after logout: {stdout2}"
    );
}

#[test]
fn auth_logout_then_token_shows_no_token() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // First login
    let login_output = plue_cmd()
        .args(["auth", "login", "--with-token", "--insecure-storage"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_will_be_cleared")
        .output()
        .unwrap();
    assert!(login_output.status.success());

    // Logout
    let logout_output = plue_cmd()
        .args(["auth", "logout"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(logout_output.status.success());

    // Token should now error
    let token_output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !token_output.status.success(),
        "Should fail after logout, stdout: {}",
        String::from_utf8_lossy(&token_output.stdout)
    );
    let stderr = String::from_utf8_lossy(&token_output.stderr);
    assert!(stderr.contains("no token found"), "stderr: {stderr}");
}

// ---------------------------------------------------------------------------
// auth token
// ---------------------------------------------------------------------------

#[test]
fn auth_token_shows_no_token_error() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no token found"),
        "stderr should say no token: {stderr}"
    );
}

#[test]
fn auth_token_no_token_error_suggests_login() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Error message should suggest how to fix
    assert!(
        stderr.contains("plue auth login") || stderr.contains("PLUE_TOKEN"),
        "Error should suggest how to authenticate: {stderr}"
    );
}

#[test]
fn auth_token_shows_token_from_config() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Write config with token
    setup_config_dirs(&cfg_home);
    for path in get_possible_config_paths(&cfg_home) {
        let _ = fs::write(
            &path,
            "api_url: https://plue.dev/api\ntoken: plue_my_secret_token\n",
        );
    }

    let output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim() == "plue_my_secret_token",
        "stdout should be just the token: {stdout}"
    );

    // Source info goes to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("config file"),
        "stderr should show source: {stderr}"
    );
}

#[test]
fn auth_token_shows_token_from_env() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TOKEN", "plue_env_token_123")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim() == "plue_env_token_123",
        "stdout should be just the token: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("PLUE_TOKEN env"),
        "stderr should show env source: {stderr}"
    );
}

#[test]
fn auth_token_reads_secure_token_for_hostname_from_keyring() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let host = unique_test_host("token-keyring");
    let keyring_file = tmp.path().join("keyring-store.json");

    let login = plue_cmd()
        .args(["auth", "login", "--with-token", "--hostname", &host])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_keyring_lookup_token")
        .output()
        .unwrap();
    assert!(
        login.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&login.stderr)
    );

    let token = plue_cmd()
        .args(["auth", "token", "--hostname", &host])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        token.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&token.stderr)
    );
    let stdout = String::from_utf8_lossy(&token.stdout);
    assert_eq!(stdout.trim(), "plue_keyring_lookup_token");
    let stderr = String::from_utf8_lossy(&token.stderr);
    assert!(
        stderr.contains("keyring"),
        "stderr should show keyring source: {stderr}"
    );
}

#[test]
fn auth_logout_deletes_keyring_token_for_hostname() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let host = unique_test_host("logout-keyring");
    let keyring_file = tmp.path().join("keyring-store.json");

    let login = plue_cmd()
        .args(["auth", "login", "--with-token", "--hostname", &host])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_logout_token_123")
        .output()
        .unwrap();
    assert!(
        login.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&login.stderr)
    );

    let logout = plue_cmd()
        .args(["auth", "logout", "--hostname", &host])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        logout.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&logout.stderr)
    );

    let token = plue_cmd()
        .args(["auth", "token", "--hostname", &host])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!token.status.success());
    let stderr = String::from_utf8_lossy(&token.stderr);
    assert!(stderr.contains("no token found"), "stderr: {stderr}");
}

#[test]
fn auth_status_reports_keyring_source() {
    let (tmp, cfg_home) = setup_temp_workspace();
    let keyring_file = tmp.path().join("keyring-store.json");

    let login = plue_cmd()
        .args(["auth", "login", "--with-token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .write_stdin("plue_status_keyring_token")
        .output()
        .unwrap();
    assert!(
        login.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&login.stderr)
    );

    let status = plue_cmd()
        .args(["auth", "status"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TEST_CREDENTIAL_STORE_FILE", &keyring_file)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("via keyring"),
        "status should report keyring source: {stdout}"
    );
}

#[test]
fn auth_token_stdout_is_pipeable() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Write config with token
    setup_config_dirs(&cfg_home);
    for path in get_possible_config_paths(&cfg_home) {
        let _ = fs::write(
            &path,
            "api_url: https://plue.dev/api\ntoken: plue_pipe_test\n",
        );
    }

    let output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // stdout should contain ONLY the token and a newline — no extra metadata
    // This ensures it's safe to pipe: `plue auth token | xargs curl -H "Authorization: token "`
    assert_eq!(
        stdout.trim(),
        "plue_pipe_test",
        "stdout should contain only the token for piping: {stdout}"
    );
    // Ensure no source info leaks to stdout
    assert!(
        !stdout.contains("config file") && !stdout.contains("Token source"),
        "Source metadata should NOT be on stdout: {stdout}"
    );
}

#[test]
fn auth_token_env_overrides_config_file() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Write config with one token
    setup_config_dirs(&cfg_home);
    for path in get_possible_config_paths(&cfg_home) {
        let _ = fs::write(
            &path,
            "api_url: https://plue.dev/api\ntoken: plue_config_token\n",
        );
    }

    // Set env to different token
    let output = plue_cmd()
        .args(["auth", "token"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TOKEN", "plue_env_wins")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "plue_env_wins",
        "Env var should override config file token: {stdout}"
    );
}

#[test]
fn auth_token_with_hostname_shows_host_in_stderr() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "token", "--hostname", "custom.plue.dev"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env("PLUE_TOKEN", "plue_with_host")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("custom.plue.dev"),
        "stderr should mention the hostname: {stderr}"
    );
}

#[test]
fn auth_token_with_hostname_no_token_shows_host_in_error() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let output = plue_cmd()
        .args(["auth", "token", "--hostname", "missing.plue.dev"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .env_remove("PLUE_TOKEN")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing.plue.dev"),
        "Error should mention the hostname: {stderr}"
    );
}
