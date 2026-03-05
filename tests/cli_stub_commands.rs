//! Tests for CLI commands — verifies all implemented commands behave correctly.
//! All major stub commands have been implemented. They now require auth/repo context
//! and should NOT return "not yet implemented" anymore.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

fn setup_temp_workspace() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = tmp.path().join("cfg");
    fs::create_dir_all(&cfg_home).expect("create cfg dir");
    (tmp, cfg_home)
}

// Issue commands are implemented — see cli/tests/cli_issue.rs for integration tests.

// Label command tests
// label is now IMPLEMENTED — it requires a repo context and makes real API calls.
#[test]
fn label_list_requires_repo_context() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No repo remote configured and no -R flag — should fail with repo detection error,
    // NOT "not yet implemented"
    let out = plue_cmd()
        .args(["label", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue label list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "label list is now implemented — should not say not yet implemented: {stderr}"
    );
}

#[test]
fn label_create_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No name arg — clap should reject with usage error, not "not yet implemented"
    let out = plue_cmd()
        .args(["label", "create"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue label create");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "label create is now implemented: {stderr}"
    );
    // clap should mention the missing argument
    assert!(
        stderr.contains("NAME") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// Search command tests
// search repos and search issues are now IMPLEMENTED — they require a query arg.
#[test]
fn search_repos_requires_query_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No query arg — clap should reject with usage error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["search", "repos"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue search repos");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Should mention the missing argument, NOT "not yet implemented"
    assert!(
        !stderr.contains("not yet implemented"),
        "stderr should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("QUERY") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

#[test]
fn search_issues_requires_query_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["search", "issues"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue search issues");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "stderr should not say not yet implemented: {stderr}"
    );
}

// search code is still a stub (needs query arg AND returns not yet implemented)

// Release command tests
// release is now IMPLEMENTED — it requires a repo context and makes real API calls.
#[test]
fn release_create_requires_tag_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No tag arg — clap should reject with usage error, not "not yet implemented"
    let out = plue_cmd()
        .args(["release", "create"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue release create");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "release create is now implemented: {stderr}"
    );
    // clap should mention the missing argument
    assert!(
        stderr.contains("TAG_NAME") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

#[test]
fn release_list_requires_repo_context() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No repo remote configured and no -R flag — should fail with repo detection error,
    // NOT "not yet implemented"
    let out = plue_cmd()
        .args(["release", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue release list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "release list is now implemented — should not say not yet implemented: {stderr}"
    );
}

// Secret command tests — now IMPLEMENTED, require auth + repo context
#[test]
fn secret_set_requires_name_and_value() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["secret", "set"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue secret set");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "secret set is now implemented: {stderr}"
    );
    // Should fail with missing args, not "not yet implemented"
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing args: {stderr}"
    );
}

#[test]
fn secret_list_fails_without_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["secret", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue secret list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "secret list is now implemented: {stderr}"
    );
}

#[test]
fn secret_delete_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["secret", "delete"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue secret delete");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "secret delete is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// Variable command tests — now IMPLEMENTED, require auth + repo context
#[test]
fn variable_set_requires_name_and_value() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["variable", "set"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue variable set");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "variable set is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing args: {stderr}"
    );
}

#[test]
fn variable_get_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["variable", "get"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue variable get");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "variable get is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

#[test]
fn variable_list_fails_without_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["variable", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue variable list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "variable list is now implemented: {stderr}"
    );
}

#[test]
fn variable_delete_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["variable", "delete"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue variable delete");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "variable delete is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// API command tests — now IMPLEMENTED, requires auth
#[test]
fn api_get_requires_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["api", "/repos/test"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue api");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "api command is now implemented: {stderr}"
    );
}

#[test]
fn api_missing_endpoint_shows_usage() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["api"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue api");

    // Without endpoint, shows usage
    assert!(!out.status.success());
}

// Bookmark create/delete — now IMPLEMENTED, require jj workspace
#[test]
fn bookmark_create_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["bookmark", "create"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue bookmark create");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "bookmark create is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name")
            || stderr.contains("required")
            || stderr.contains("Usage")
            || stderr.contains("jj workspace")
            || stderr.contains("not a jj"),
        "stderr should mention missing arg or workspace error: {stderr}"
    );
}

#[test]
fn bookmark_delete_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["bookmark", "delete"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue bookmark delete");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "bookmark delete is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name")
            || stderr.contains("required")
            || stderr.contains("Usage")
            || stderr.contains("jj workspace")
            || stderr.contains("not a jj"),
        "stderr should mention missing arg or workspace error: {stderr}"
    );
}

// Search code — now IMPLEMENTED
#[test]
fn search_code_requires_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["search", "code", "test-query"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue search code");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "search code is now implemented: {stderr}"
    );
}

// Label delete — now IMPLEMENTED
#[test]
fn label_delete_requires_name_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["label", "delete"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue label delete");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "label delete is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// Agent command tests
// agent is now IMPLEMENTED — it has list/view/run/chat subcommands that make real API calls.
#[test]
fn agent_chat_requires_session_id_and_message() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No session_id or message — clap should reject with usage error, not "not yet implemented"
    let out = plue_cmd()
        .args(["agent", "chat"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue agent chat");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "agent chat is now implemented — should not say not yet implemented: {stderr}"
    );
    // clap should mention the missing arguments
    assert!(
        stderr.contains("session_id") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing args: {stderr}"
    );
}

#[test]
fn agent_run_requires_prompt_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No prompt arg — clap should reject with usage error, not "not yet implemented"
    let out = plue_cmd()
        .args(["agent", "run"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue agent run");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "agent run is now implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("prompt") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// Run command tests
// run is now IMPLEMENTED — subcommands make real API calls.
#[test]
fn run_list_requires_workflow_id_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No workflow_id arg — clap should reject with usage error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["run", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue run list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "run list is now implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("workflow_id") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

#[test]
fn run_view_requires_run_id_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No run_id arg — clap should reject with usage error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["run", "view"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue run view");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "run view is now implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("run_id") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

#[test]
fn run_watch_requires_run_id_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No run_id arg — clap should reject with usage error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["run", "watch"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue run watch");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "run watch is now implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("run_id") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

#[test]
fn run_rerun_requires_run_id_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No run_id arg — clap should reject with usage error
    let out = plue_cmd()
        .args(["run", "rerun"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue run rerun");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("run_id") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// Workflow command tests
// workflow is now IMPLEMENTED — subcommands make real API calls.
#[test]
fn workflow_list_requires_repo_context() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No repo remote configured and no -R flag — should fail with repo detection error,
    // NOT "not yet implemented"
    let out = plue_cmd()
        .args(["workflow", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue workflow list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "workflow list is now implemented — should not say not yet implemented: {stderr}"
    );
}

#[test]
fn workflow_run_requires_workflow_id_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No workflow_id arg — clap should reject with usage error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["workflow", "run"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue workflow run");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "workflow run is now implemented — should not say not yet implemented: {stderr}"
    );
    assert!(
        stderr.contains("workflow_id") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing arg: {stderr}"
    );
}

// SSH Key command tests
// ssh-key is now IMPLEMENTED — it makes real API calls.
#[test]
fn ssh_key_add_requires_title_and_key() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No args — clap should reject with usage error, not "not yet implemented"
    let out = plue_cmd()
        .args(["ssh-key", "add"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue ssh-key add");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "ssh-key add is now implemented — should not say not yet implemented: {stderr}"
    );
    // Should mention missing required flags
    assert!(
        stderr.contains("title") || stderr.contains("required") || stderr.contains("Usage"),
        "stderr should mention missing flags: {stderr}"
    );
}

#[test]
fn ssh_key_list_exits_nonzero_without_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No token configured — should fail with auth error, NOT "not yet implemented"
    let out = plue_cmd()
        .args(["ssh-key", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue ssh-key list");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "ssh-key list is now implemented: {stderr}"
    );
    assert!(
        stderr.contains("not authenticated") || stderr.contains("auth"),
        "stderr should mention authentication: {stderr}"
    );
}

#[test]
fn ssh_key_delete_requires_id_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No id — clap should reject with usage error
    let out = plue_cmd()
        .args(["ssh-key", "delete"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue ssh-key delete");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "ssh-key delete is now implemented: {stderr}"
    );
}

// Config command tests
// config is now IMPLEMENTED — reads/writes local config file.
#[test]
fn config_list_succeeds_with_defaults() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["config", "list"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config list");

    assert!(
        out.status.success(),
        "config list should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("api_url"), "stdout: {stdout}");
    assert!(stdout.contains("git_protocol"), "stdout: {stdout}");
}

#[test]
fn config_get_requires_key_arg() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No key — clap should reject
    let out = plue_cmd()
        .args(["config", "get"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config get");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "config get is now implemented: {stderr}"
    );
}

#[test]
fn config_set_requires_key_and_value() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // No key/value — clap should reject
    let out = plue_cmd()
        .args(["config", "set"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config set");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not yet implemented"),
        "config set is now implemented: {stderr}"
    );
}

#[test]
fn config_get_api_url_returns_value() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["config", "get", "api_url"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config get api_url");

    assert!(
        out.status.success(),
        "config get api_url should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Default API URL
    assert!(
        stdout.contains("plue.dev") || stdout.contains("localhost"),
        "stdout: {stdout}"
    );
}

#[test]
fn config_set_and_get_roundtrip() {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = tmp.path().join("cfg");
    fs::create_dir_all(cfg_home.join("plue")).expect("create plue config dir");

    // Set git_protocol to https
    let set_out = plue_cmd()
        .args(["config", "set", "git_protocol", "https"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config set");

    assert!(
        set_out.status.success(),
        "config set should succeed: {}",
        String::from_utf8_lossy(&set_out.stderr)
    );

    // Get it back
    let get_out = plue_cmd()
        .args(["config", "get", "git_protocol"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config get");

    assert!(get_out.status.success(), "config get should succeed");
    let stdout = String::from_utf8_lossy(&get_out.stdout);
    assert!(stdout.trim() == "https", "expected 'https', got: {stdout}");
}

#[test]
fn config_get_unknown_key_returns_error() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["config", "get", "nonexistent_key"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue config get nonexistent_key");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown config key"), "stderr: {stderr}");
}

// API command tests

// Bookmark create/delete (stubbed subcommands)

// ─────────────────────────────────────────────────────────────────
// Additional tests: help flags, JSON/TOON format flags with stubs,
// exit-code consistency, and completion command (actually implemented)
// ─────────────────────────────────────────────────────────────────

/// Verify that all still-stub commands produce a non-zero exit code AND "not yet implemented".
/// NOTE: search repos/issues, ssh-key *, config *, label *, release * are now IMPLEMENTED.
/// NOTE: workflow *, run *, agent * are now IMPLEMENTED — removed from this list.
///
/// Commands with --json/--toon flags should still exit non-zero without auth/repo context.
/// These are now implemented and fail with auth errors, not "not yet implemented".
#[test]
fn implemented_commands_with_json_flag_fail_without_auth() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let cases: &[&[&str]] = &[
        &["--json", "secret", "list"],
        &["--json", "variable", "list"],
        &["--toon", "secret", "list"],
        &["--toon", "variable", "list"],
    ];

    for args in cases {
        let out = plue_cmd()
            .args(*args)
            .env("XDG_CONFIG_HOME", &cfg_home)
            .env("HOME", &cfg_home)
            .current_dir(tmp.path())
            .output()
            .unwrap_or_else(|_| panic!("failed to run plue {:?}", args));

        assert!(
            !out.status.success(),
            "Expected non-zero exit for: {:?}",
            args
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("not yet implemented"),
            "Expected no 'not yet implemented' for {:?}, got: {stderr}",
            args
        );
    }
}

/// Each command's help flag should succeed (exit 0) and mention the command name.
#[test]
fn all_commands_have_working_help() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let help_invocations: &[(&[&str], &str)] = &[
        (&["label", "--help"], "label"),
        (&["search", "--help"], "search"),
        (&["release", "--help"], "release"),
        (&["secret", "--help"], "secret"),
        (&["variable", "--help"], "variable"),
        (&["agent", "--help"], "agent"),
        (&["run", "--help"], "run"),
        (&["workflow", "--help"], "workflow"),
        (&["ssh-key", "--help"], "ssh-key"),
        (&["config", "--help"], "config"),
        (&["api", "--help"], "api"),
    ];

    for (args, expected_name) in help_invocations {
        let out = plue_cmd()
            .args(*args)
            .env("XDG_CONFIG_HOME", &cfg_home)
            .env("HOME", &cfg_home)
            .current_dir(tmp.path())
            .output()
            .unwrap_or_else(|_| panic!("failed to run plue {:?}", args));

        // --help exits 0
        assert!(
            out.status.success(),
            "Expected success for help of {:?}, got: {} - stdout: {} stderr: {}",
            args,
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            combined.contains(expected_name),
            "Expected '{}' in help output for {:?}, got: {combined}",
            expected_name,
            args
        );
    }
}

/// Completion command is actually implemented — verify it generates output for bash/zsh/fish.
#[test]
fn completion_bash_generates_output() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["completion", "bash"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue completion bash");

    assert!(
        out.status.success(),
        "completion bash should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty(), "completion bash should produce output");
    // Bash completions typically contain function definitions
    assert!(
        stdout.contains("plue") || stdout.contains("_plue") || stdout.contains("complete"),
        "bash completions should reference the binary name"
    );
}

#[test]
fn completion_zsh_generates_output() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["completion", "zsh"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue completion zsh");

    assert!(
        out.status.success(),
        "completion zsh should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.stdout.is_empty(),
        "completion zsh should produce output"
    );
}

#[test]
fn completion_fish_generates_output() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["completion", "fish"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue completion fish");

    assert!(
        out.status.success(),
        "completion fish should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.stdout.is_empty(),
        "completion fish should produce output"
    );
}

/// Verify the error message includes the specific subcommand name in the error.
/// NOTE: search repos/issues, ssh-key *, workflow *, run *, agent * are now implemented.
///
/// The `api` command includes the method and endpoint in its error message.
///
/// Commands without any subcommand should print help (exit non-zero) not panic.
#[test]
fn commands_without_subcommand_show_help() {
    let (tmp, cfg_home) = setup_temp_workspace();

    // Commands that require a subcommand (not the `api` command which takes positional args)
    let cmds = [
        "label", "search", "release", "secret", "variable", "agent", "run", "workflow", "ssh-key",
        "config",
    ];

    for cmd in &cmds {
        let out = plue_cmd()
            .args([*cmd])
            .env("XDG_CONFIG_HOME", &cfg_home)
            .env("HOME", &cfg_home)
            .current_dir(tmp.path())
            .output()
            .unwrap_or_else(|_| panic!("failed to run plue {cmd}"));

        // Without a subcommand, clap exits non-zero and prints usage/help
        assert!(
            !out.status.success(),
            "Expected non-zero exit for '{cmd}' without subcommand"
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            combined.contains("Usage") || combined.contains("usage") || combined.contains("USAGE"),
            "Expected usage text for '{cmd}' without subcommand, got: {combined}"
        );
    }
}

/// The top-level plue --help should list all commands so users know they exist.
#[test]
fn top_level_help_lists_all_commands() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["--help"])
        .env("XDG_CONFIG_HOME", &cfg_home)
        .env("HOME", &cfg_home)
        .current_dir(tmp.path())
        .output()
        .expect("run plue --help");

    assert!(out.status.success(), "plue --help should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);

    let expected_commands = [
        "search",
        "workflow",
        "run",
        "agent",
        "secret",
        "variable",
        "ssh-key",
        "config",
        "label",
        "release",
        "api",
        "completion",
    ];

    for cmd in &expected_commands {
        assert!(
            stdout.contains(cmd),
            "Expected '{cmd}' in top-level help, got:\n{stdout}"
        );
    }
}
