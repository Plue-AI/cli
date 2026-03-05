//! Tests for the `plue completion` command, which is fully implemented
//! (generates shell completions via clap_complete — not a stub).

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

#[test]
fn completion_bash_generates_output() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["completion", "bash"])
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
    assert!(
        !stdout.is_empty(),
        "bash completion output should not be empty"
    );
    // bash completions typically start with a function definition
    assert!(
        stdout.contains("plue") || stdout.contains("complete"),
        "bash completion should reference 'plue' or 'complete': {stdout}"
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
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.is_empty(),
        "zsh completion output should not be empty"
    );
    // zsh completions use the #compdef format
    assert!(
        stdout.contains("plue") || stdout.contains("compdef") || stdout.contains("_arguments"),
        "zsh completion should reference plue or use zsh conventions: {stdout}"
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
        .expect("run plue");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.is_empty(),
        "fish completion output should not be empty"
    );
    // fish completions use 'complete -c <cmd>' format
    assert!(
        stdout.contains("plue"),
        "fish completion should reference 'plue': {stdout}"
    );
}

#[test]
fn completion_bash_includes_all_top_level_commands() {
    let (tmp, cfg_home) = setup_temp_workspace();

    let out = plue_cmd()
        .args(["completion", "bash"])
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

    // All top-level commands should appear in completion output
    let expected_commands = [
        "auth",
        "repo",
        "issue",
        "land",
        "change",
        "bookmark",
        "status",
        "search",
        "workflow",
        "run",
        "agent",
        "ssh-key",
        "secret",
        "variable",
        "label",
        "release",
        "config",
        "api",
        "completion",
    ];
    for cmd in &expected_commands {
        assert!(
            stdout.contains(cmd),
            "bash completion should include '{cmd}' command: {stdout}"
        );
    }
}
