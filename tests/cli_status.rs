mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

#[test]
fn test_status_in_empty_repo() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let output = plue_cmd()
        .arg("status")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Working copy"), "got: {stdout}");
}

#[test]
fn test_status_with_modified_file() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo =
        common::create_commit_with_files(&repo, &[], "Add hello", &[("hello.txt", "hello\n")]);
    let output = plue_cmd()
        .arg("status")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Working copy"), "got: {stdout}");
    assert!(stdout.contains("Add hello"), "got: {stdout}");
}

#[test]
fn test_status_json_flag() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let output = plue_cmd()
        .args(["--json", "status"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed["working_copy"].is_object());
}

#[test]
fn test_status_toon_flag() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let output = plue_cmd()
        .args(["--toon", "status"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("working_copy.change_id:"), "got: {stdout}");
    assert!(!stdout.contains('{'), "got: {stdout}");
}

#[test]
fn test_status_outside_repo_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = plue_cmd()
        .arg("status")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}
