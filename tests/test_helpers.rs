mod common;

use jj_lib::ref_name::WorkspaceName;
use jj_lib::repo::Repo;

#[test]
fn test_init_test_repo_creates_workspace() {
    let (tmp, ws, repo) = common::init_test_repo();
    assert!(tmp.path().join(".jj").is_dir());
    // On macOS /var is a symlink to /private/var, so canonicalize both
    assert_eq!(
        ws.workspace_root().canonicalize().unwrap(),
        tmp.path().canonicalize().unwrap()
    );
    let root_id = repo.store().root_commit_id();
    let root_commit = repo.store().get_commit(root_id).unwrap();
    assert!(root_commit.parent_ids().is_empty());
}

#[test]
fn test_create_commit_with_files_adds_commit() {
    let (_tmp, _ws, repo) = common::init_test_repo();

    let repo = common::create_commit_with_files(
        &repo,
        &[],
        "Initial commit",
        &[("hello.txt", "hello world\n")],
    );

    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .expect("wc commit");
    let wc_commit = repo.store().get_commit(wc_id).unwrap();
    assert_eq!(wc_commit.description().trim(), "Initial commit");
}

#[test]
fn test_create_bookmark_points_to_commit() {
    let (_tmp, _ws, repo) = common::init_test_repo();

    let repo =
        common::create_commit_with_files(&repo, &[], "First commit", &[("file.txt", "content\n")]);

    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .unwrap()
        .clone();

    let repo = common::create_bookmark(&repo, "main", &wc_id);

    let bookmark_target = repo
        .view()
        .get_local_bookmark(jj_lib::ref_name::RefName::new("main"));
    assert!(bookmark_target.is_present());
    assert!(bookmark_target.added_ids().any(|id| id == &wc_id));
}
