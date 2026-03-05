mod common;

use jj_lib::ref_name::WorkspaceName;
use jj_lib::repo::Repo;
use plue::jj_ops::{JjWorkspaceOps, WorkspaceOps};
use plue::types::{DiffLineKind, FileChangeType};

// === JjWorkspaceOps::open tests ===

#[test]
fn test_open_valid_workspace() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let ops = JjWorkspaceOps::open(tmp.path()).expect("should open valid workspace");
    // Should not error
    let _ = ops;
}

#[test]
fn test_open_nonexistent_errors() {
    let err = JjWorkspaceOps::open(std::path::Path::new("/nonexistent/path"));
    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(msg.contains("not a jj workspace"), "got: {msg}");
}

// === get_status tests ===

#[test]
fn test_status_empty_repo() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let status = ops.get_status().unwrap();
    assert!(status.working_copy.is_working_copy);
    assert!(status.working_copy.is_empty);
    // Parent of working copy in a fresh repo is the root commit, which we skip
    assert!(status.parent.is_none());
    assert!(status.modified_files.is_empty());
}

#[test]
fn test_status_with_modified_files() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo =
        common::create_commit_with_files(&repo, &[], "Add file", &[("hello.txt", "hello world\n")]);
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let status = ops.get_status().unwrap();
    assert!(status.working_copy.is_working_copy);
    assert_eq!(status.working_copy.description, "Add file");
}

#[test]
fn test_status_shows_parent() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "First commit", &[("a.txt", "aaa\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo =
        common::create_commit_with_files(&repo, &[&wc_id], "Second commit", &[("b.txt", "bbb\n")]);
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let status = ops.get_status().unwrap();
    assert_eq!(status.working_copy.description, "Second commit");
    assert!(status.parent.is_some());
    assert_eq!(status.parent.unwrap().description, "First commit");
}

// === list_changes tests ===

#[test]
fn test_list_changes_empty_repo() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let changes = ops.list_changes(10).unwrap();
    // Fresh repo has only the working copy change (which is empty)
    assert!(changes.len() <= 1);
}

#[test]
fn test_list_changes_returns_commits() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "First", &[("a.txt", "aaa\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_commit_with_files(&repo, &[&wc_id], "Second", &[("b.txt", "bbb\n")]);
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let changes = ops.list_changes(10).unwrap();
    assert!(changes.len() >= 2);
    let descriptions: Vec<&str> = changes.iter().map(|c| c.description.as_str()).collect();
    assert!(descriptions.contains(&"First"));
    assert!(descriptions.contains(&"Second"));
}

#[test]
fn test_list_changes_respects_limit() {
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
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let changes = ops.list_changes(2).unwrap();
    assert_eq!(changes.len(), 2);
}

#[test]
fn test_list_changes_marks_working_copy() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo = common::create_commit_with_files(&repo, &[], "WC", &[("a.txt", "a\n")]);
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let changes = ops.list_changes(10).unwrap();
    let wc_change = changes.iter().find(|c| c.description == "WC");
    assert!(wc_change.is_some());
    assert!(wc_change.unwrap().is_working_copy);
}

#[test]
fn test_list_changes_includes_bookmarks() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Tagged", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "main", &wc_id);
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let changes = ops.list_changes(10).unwrap();
    let tagged = changes.iter().find(|c| c.description == "Tagged");
    assert!(tagged.is_some());
    assert!(tagged.unwrap().bookmarks.contains(&"main".to_string()));
}

// === show_change tests ===

#[test]
fn test_show_change_by_full_id() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "My change", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap();
    let commit = repo.store().get_commit(wc_id).unwrap();
    let change_id_hex = commit.change_id().reverse_hex();

    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let info = ops.show_change(&change_id_hex).unwrap();
    assert_eq!(info.description, "My change");
    assert_eq!(info.change_id, change_id_hex);
}

#[test]
fn test_show_change_by_prefix() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Prefixed", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap();
    let commit = repo.store().get_commit(wc_id).unwrap();
    let change_id_hex = commit.change_id().reverse_hex();
    // Use first 4 chars as prefix
    let prefix = &change_id_hex[..4.min(change_id_hex.len())];

    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let info = ops.show_change(prefix).unwrap();
    assert_eq!(info.description, "Prefixed");
}

#[test]
fn test_show_change_not_found() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    // Use a string that's not valid reverse-hex or hex
    let result = ops.show_change("!!invalid!!");
    assert!(result.is_err());
}

// === get_diff tests ===

#[test]
fn test_diff_working_copy_no_changes() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let diff = ops.get_diff(None).unwrap();
    assert!(diff.file_diffs.is_empty());
}

#[test]
fn test_diff_working_copy_with_added_file() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let _repo =
        common::create_commit_with_files(&repo, &[], "Add file", &[("hello.txt", "hello world\n")]);
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let diff = ops.get_diff(None).unwrap();
    assert!(!diff.file_diffs.is_empty());
    let file_diff = &diff.file_diffs[0];
    assert_eq!(file_diff.path, "hello.txt");
    assert_eq!(file_diff.change_type, FileChangeType::Added);
}

#[test]
fn test_diff_specific_change() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(
        &repo,
        &[],
        "Specific change",
        &[("file.txt", "content\n")],
    );
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap();
    let commit = repo.store().get_commit(wc_id).unwrap();
    let change_id_hex = commit.change_id().reverse_hex();

    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let diff = ops.get_diff(Some(&change_id_hex)).unwrap();
    assert!(!diff.file_diffs.is_empty());
    assert_eq!(diff.change_id, change_id_hex);
}

// === list_bookmarks tests ===

#[test]
fn test_list_bookmarks_empty() {
    let (tmp, _ws, _repo) = common::init_test_repo();
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let bookmarks = ops.list_bookmarks().unwrap();
    assert!(bookmarks.is_empty());
}

#[test]
fn test_list_bookmarks_returns_all() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "A", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let repo = common::create_bookmark(&repo, "main", &wc_id);
    let _repo = common::create_bookmark(&repo, "dev", &wc_id);

    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let bookmarks = ops.list_bookmarks().unwrap();
    assert_eq!(bookmarks.len(), 2);
    let names: Vec<&str> = bookmarks.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"dev"));
}

#[test]
fn test_list_bookmarks_shows_target_ids() {
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "A", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    let _repo = common::create_bookmark(&repo, "main", &wc_id);

    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let bookmarks = ops.list_bookmarks().unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert!(!bookmarks[0].target_change_id.is_empty());
    assert!(!bookmarks[0].target_commit_id.is_empty());
}

// === diff correctness test ===

#[test]
fn test_diff_modified_file_correct_lines() {
    let (tmp, _ws, repo) = common::init_test_repo();
    // Create initial file with known content
    let repo = common::create_commit_with_files(
        &repo,
        &[],
        "Initial",
        &[("file.txt", "line1\nline2\nline3\n")],
    );
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();
    // Modify: change line2 → line2-modified, keep line1 and line3
    let _repo = common::create_commit_with_files(
        &repo,
        &[&wc_id],
        "Modify",
        &[("file.txt", "line1\nline2-modified\nline3\n")],
    );
    let ops = JjWorkspaceOps::open(tmp.path()).unwrap();
    let diff = ops.get_diff(None).unwrap();
    assert_eq!(diff.file_diffs.len(), 1);
    let fd = &diff.file_diffs[0];
    assert_eq!(fd.path, "file.txt");
    assert_eq!(fd.hunks.len(), 1);

    let hunk = &fd.hunks[0];
    let removed: Vec<&str> = hunk
        .lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Removed)
        .map(|l| l.content.as_str())
        .collect();
    let added: Vec<&str> = hunk
        .lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Added)
        .map(|l| l.content.as_str())
        .collect();

    assert_eq!(removed, vec!["line2"], "should only remove line2");
    assert_eq!(
        added,
        vec!["line2-modified"],
        "should only add line2-modified"
    );
}
