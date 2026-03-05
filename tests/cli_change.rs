mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use jj_lib::ref_name::WorkspaceName;
use jj_lib::repo::Repo;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

#[test]
fn test_change_list_shows_initial() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let output = plue_cmd()
        .args(["change", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_change_list_after_commits() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Alpha", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_commit_with_files(&repo, &[&wc_id], "Beta", &[("b.txt", "b\n")]);

    let output = plue_cmd()
        .args(["change", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Alpha"), "got: {stdout}");
    assert!(stdout.contains("Beta"), "got: {stdout}");
}

#[test]
fn test_change_list_limit() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "A", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let repo = common::create_commit_with_files(&repo, &[&wc_id], "B", &[("b.txt", "b\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_commit_with_files(&repo, &[&wc_id], "C", &[("c.txt", "c\n")]);

    let output = plue_cmd()
        .args(["change", "list", "--limit", "1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "expected 1 line, got: {stdout}");
}

#[test]
fn test_change_list_json() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "Test", &[("a.txt", "a\n")]);

    let output = plue_cmd()
        .args(["--json", "change", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed.is_array());
}

#[test]
fn test_change_list_toon() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "TOON change", &[("a.txt", "a\n")]);

    let output = plue_cmd()
        .args(["--toon", "change", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("change_id:"), "got: {stdout}");
    assert!(
        stdout.contains("description:\"TOON change\""),
        "got: {stdout}"
    );
    assert!(!stdout.contains('{'), "got: {stdout}");
}

#[test]
fn test_show_existing() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "My change", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap();
    let commit = repo.store().get_commit(wc_id).unwrap();
    let change_id = commit.change_id().reverse_hex();

    let output = plue_cmd()
        .args(["change", "show", &change_id])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("My change"), "got: {stdout}");
}

#[test]
fn test_show_prefix_match() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Prefix test", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap();
    let commit = repo.store().get_commit(wc_id).unwrap();
    let change_id = commit.change_id().reverse_hex();
    let prefix = &change_id[..4.min(change_id.len())];

    let output = plue_cmd()
        .args(["change", "show", prefix])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Prefix test"), "got: {stdout}");
}

#[test]
fn test_show_invalid_id() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let output = plue_cmd()
        .args(["change", "show", "!!invalid!!"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_diff_working_copy() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo =
        common::create_commit_with_files(&repo, &[], "Add file", &[("hello.txt", "hello\n")]);
    let output = plue_cmd()
        .args(["change", "diff"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_diff_specific_change() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Diffable", &[("x.txt", "x\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap();
    let commit = repo.store().get_commit(wc_id).unwrap();
    let change_id = commit.change_id().reverse_hex();

    let output = plue_cmd()
        .args(["change", "diff", &change_id])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_diff_json() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "D", &[("d.txt", "d\n")]);

    let output = plue_cmd()
        .args(["--json", "change", "diff"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed["change_id"].is_string());
}
