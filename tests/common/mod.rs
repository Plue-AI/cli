use std::sync::Arc;

use jj_lib::backend::{CopyId, TreeValue};
use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::merged_tree::MergedTree;
use jj_lib::op_store::RefTarget;
use jj_lib::ref_name::WorkspaceName;
use jj_lib::repo::{ReadonlyRepo, Repo};
use jj_lib::repo_path::RepoPathBuf;
use jj_lib::settings::UserSettings;
use jj_lib::tree_builder::TreeBuilder;
use jj_lib::workspace::Workspace;
use pollster::FutureExt as _;
use tempfile::TempDir;
use toml_edit::DocumentMut;

/// Create minimal UserSettings for tests.
#[allow(dead_code)]
pub fn test_settings() -> UserSettings {
    let mut config = StackedConfig::with_defaults();
    let toml: DocumentMut = r#"
[user]
name = "Test User"
email = "test@example.com"
"#
    .parse()
    .expect("valid TOML");
    config.add_layer(ConfigLayer::with_data(ConfigSource::User, toml));
    UserSettings::from_config(config).expect("valid config")
}

/// Initialize a new jj workspace with internal git backend in a temp directory.
#[allow(dead_code)]
pub fn init_test_repo() -> (TempDir, Workspace, Arc<ReadonlyRepo>) {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let settings = test_settings();
    let (ws, repo) =
        Workspace::init_internal_git(&settings, tmp.path()).expect("failed to init workspace");
    (tmp, ws, repo)
}

/// Create a commit with the given files and description.
/// `files` is a list of (path, content) pairs.
#[allow(dead_code)]
pub fn create_commit_with_files(
    repo: &Arc<ReadonlyRepo>,
    parents: &[&jj_lib::backend::CommitId],
    description: &str,
    files: &[(&str, &str)],
) -> Arc<ReadonlyRepo> {
    let store = repo.store();
    let mut tx = repo.start_transaction();

    // Build tree from parent's tree + new files
    let parent_tree_id = if parents.is_empty() {
        store.empty_tree_id().clone()
    } else {
        let parent_commit = store.get_commit(parents[0]).expect("parent commit exists");
        let tree_ids = parent_commit.tree().tree_ids().clone();
        tree_ids.into_resolved().expect("resolved tree").clone()
    };

    let mut tree_builder = TreeBuilder::new(store.clone(), parent_tree_id);
    for (path, content) in files {
        let path = RepoPathBuf::from_internal_string(*path).expect("valid path");
        let file_id = store
            .write_file(&path, &mut content.as_bytes())
            .block_on()
            .expect("write file");
        tree_builder.set(
            path,
            TreeValue::File {
                id: file_id,
                executable: false,
                copy_id: CopyId::placeholder(),
            },
        );
    }
    let tree_id = tree_builder.write_tree().expect("write tree");
    let tree = MergedTree::resolved(store.clone(), tree_id);

    let parent_ids: Vec<_> = if parents.is_empty() {
        vec![store.root_commit_id().clone()]
    } else {
        parents.iter().map(|id| (*id).clone()).collect()
    };

    let commit = tx
        .repo_mut()
        .new_commit(parent_ids, tree)
        .set_description(description)
        .write()
        .expect("write commit");

    // Update working copy to point to the new commit
    tx.repo_mut()
        .set_wc_commit(WorkspaceName::DEFAULT.to_owned(), commit.id().clone())
        .expect("set wc commit");

    tx.commit("test commit").expect("commit tx")
}

/// Create a bookmark (branch) pointing to the given commit.
#[allow(dead_code)]
pub fn create_bookmark(
    repo: &Arc<ReadonlyRepo>,
    name: &str,
    commit_id: &jj_lib::backend::CommitId,
) -> Arc<ReadonlyRepo> {
    let mut tx = repo.start_transaction();
    let ref_name = jj_lib::ref_name::RefName::new(name);
    let target = RefTarget::normal(commit_id.clone());
    tx.repo_mut().set_local_bookmark_target(ref_name, target);
    tx.commit("create bookmark").expect("commit tx")
}
