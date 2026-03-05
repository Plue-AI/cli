mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use jj_lib::ref_name::WorkspaceName;
use std::collections::HashSet;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

#[test]
fn test_bookmark_list_empty_repo() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let output = plue_cmd()
        .args(["bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No bookmarks"), "got: {stdout}");
}

#[test]
fn test_bookmark_list_after_create() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "A", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "main", &wc_id);

    let output = plue_cmd()
        .args(["bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main"), "got: {stdout}");
}

#[test]
fn test_bookmark_list_json() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "B", &[("b.txt", "b\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "feature", &wc_id);

    let output = plue_cmd()
        .args(["--json", "bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["name"], "feature");
}

#[test]
fn test_bookmark_list_toon() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "B", &[("b.txt", "b\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "feature", &wc_id);

    let output = plue_cmd()
        .args(["--toon", "bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name:feature"), "got: {stdout}");
    assert!(stdout.contains("target_change_id:"), "got: {stdout}");
    assert!(!stdout.contains('{'), "got: {stdout}");
}

// ─────────────────────────────────────────────────────────────────
// bookmark create (uses local jj-lib — offline operation)
// ─────────────────────────────────────────────────────────────────

#[test]
fn test_bookmark_create_succeeds() {
    let (tmp, _ws, repo) = common::init_test_repo();
    // Need at least one commit so there's a working-copy commit to attach to
    let _repo = common::create_commit_with_files(&repo, &[], "Initial", &[("init.txt", "hi\n")]);

    let output = plue_cmd()
        .args(["bookmark", "create", "my-feature"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bookmark create should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("my-feature"),
        "should mention created bookmark: {stdout}"
    );
}

#[test]
fn test_bookmark_create_then_list() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "C", &[("c.txt", "c\n")]);

    // Create the bookmark
    let create_out = plue_cmd()
        .args(["bookmark", "create", "release-1.0"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        create_out.status.success(),
        "create should succeed: {}",
        String::from_utf8_lossy(&create_out.stderr)
    );

    // List should include the new bookmark
    let list_out = plue_cmd()
        .args(["bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(list_out.status.success());
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        stdout.contains("release-1.0"),
        "list should contain new bookmark: {stdout}"
    );
}

#[test]
fn test_bookmark_create_json_output() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "D", &[("d.txt", "d\n")]);

    let output = plue_cmd()
        .args(["--json", "bookmark", "create", "json-bm"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bookmark create --json should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}"));
    assert_eq!(parsed["name"].as_str(), Some("json-bm"));
}

#[test]
fn test_bookmark_create_toon_output() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "E", &[("e.txt", "e\n")]);

    let output = plue_cmd()
        .args(["--toon", "bookmark", "create", "toon-bm"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bookmark create --toon should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name:toon-bm"), "got: {stdout}");
}

// ─────────────────────────────────────────────────────────────────
// bookmark delete (uses local jj-lib — offline operation)
// ─────────────────────────────────────────────────────────────────

#[test]
fn test_bookmark_delete_succeeds() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "F", &[("f.txt", "f\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "to-delete", &wc_id);

    // Verify bookmark exists first
    let list_out = plue_cmd()
        .args(["bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        stdout.contains("to-delete"),
        "bookmark should exist: {stdout}"
    );

    // Now delete it
    let del_out = plue_cmd()
        .args(["bookmark", "delete", "to-delete"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        del_out.status.success(),
        "bookmark delete should succeed: {}",
        String::from_utf8_lossy(&del_out.stderr)
    );
    let del_stdout = String::from_utf8_lossy(&del_out.stdout);
    assert!(
        del_stdout.contains("to-delete"),
        "should mention deleted bookmark: {del_stdout}"
    );
}

#[test]
fn test_bookmark_delete_then_list_removes_it() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "G", &[("g.txt", "g\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "gone", &wc_id);

    // Delete
    let del_out = plue_cmd()
        .args(["bookmark", "delete", "gone"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        del_out.status.success(),
        "delete should succeed: {}",
        String::from_utf8_lossy(&del_out.stderr)
    );

    // Verify removed from list
    let list_out = plue_cmd()
        .args(["bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(list_out.status.success());
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        !stdout.contains("gone") || stdout.contains("No bookmarks"),
        "deleted bookmark should not appear in list: {stdout}"
    );
}

#[test]
fn test_bookmark_delete_nonexistent_fails() {
    let (tmp, _ws, _repo) = common::init_test_repo();

    let output = plue_cmd()
        .args(["bookmark", "delete", "nonexistent-bm"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "deleting nonexistent bookmark should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty(), "should output an error message");
}

#[test]
fn test_bookmark_create_outside_repo_fails() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();

    // Outside any jj repo — should fail with a meaningful error
    let output = plue_cmd()
        .args(["bookmark", "create", "test-bm"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail outside a jj repo");
}

#[test]
fn test_bookmark_delete_outside_repo_fails() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();

    let output = plue_cmd()
        .args(["bookmark", "delete", "test-bm"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail outside a jj repo");
}

#[test]
fn test_bookmark_create_multiple() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "H", &[("h.txt", "h\n")]);

    // Create multiple bookmarks
    for name in &["alpha", "beta", "gamma"] {
        let out = plue_cmd()
            .args(["bookmark", "create", name])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "create {} should succeed: {}",
            name,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // All should appear in list
    let list_out = plue_cmd()
        .args(["bookmark", "list"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(list_out.status.success());
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    for name in &["alpha", "beta", "gamma"] {
        assert!(stdout.contains(name), "should contain {name}: {stdout}");
    }
}

#[test]
fn test_bookmark_delete_silent_with_json_flag() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "I", &[("i.txt", "i\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "silent-del", &wc_id);

    let output = plue_cmd()
        .args(["--json", "bookmark", "delete", "silent-del"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bookmark delete --json should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // With --json, delete produces no output (silent success per spec)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "delete --json should be silent: {stdout}"
    );
}

#[test]
fn test_bookmark_create_requires_name_arg() {
    let (tmp, _ws, _repo) = common::init_test_repo();

    let output = plue_cmd()
        .args(["bookmark", "create"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "bookmark create without name should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "should mention missing arg: {stderr}"
    );
}

#[test]
fn test_bookmark_delete_requires_name_arg() {
    let (tmp, _ws, _repo) = common::init_test_repo();

    let output = plue_cmd()
        .args(["bookmark", "delete"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "bookmark delete without name should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("name") || stderr.contains("required") || stderr.contains("Usage"),
        "should mention missing arg: {stderr}"
    );
}

#[allow(dead_code)]
fn bookmark_names_from_list(stdout: &str) -> HashSet<String> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.contains("No bookmarks"))
        .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
