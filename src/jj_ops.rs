use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use jj_lib::backend::CommitId;
use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::matchers::EverythingMatcher;
use jj_lib::merged_tree::{TreeDiffEntry, TreeDiffStream};
use jj_lib::object_id::{HexPrefix, ObjectId, PrefixResolution};
use jj_lib::op_store::RefTarget;
use jj_lib::ref_name::RefName;
use jj_lib::repo::{ReadonlyRepo, Repo, StoreFactories};
use jj_lib::settings::UserSettings;
use jj_lib::store::Store;
use jj_lib::str_util::StringMatcher;
use jj_lib::workspace::{default_working_copy_factories, Workspace};
use toml_edit::DocumentMut;

use crate::types::*;

// ── jj-core inlined ─────────────────────────────────────────────

struct UserConfig {
    name: String,
    email: String,
}

fn create_settings(config: &UserConfig) -> UserSettings {
    let mut stacked_config = StackedConfig::with_defaults();
    let toml_str = format!(
        r#"
[user]
name = "{}"
email = "{}"
"#,
        config.name, config.email
    );
    let toml: DocumentMut = toml_str.parse().expect("valid TOML");
    stacked_config.add_layer(ConfigLayer::with_data(ConfigSource::User, toml));
    UserSettings::from_config(stacked_config).expect("valid config")
}

fn load_repo_at_head(
    repo_path: &Path,
    settings: &UserSettings,
) -> Result<(Workspace, Arc<ReadonlyRepo>)> {
    let ws = Workspace::load(
        settings,
        repo_path,
        &StoreFactories::default(),
        &default_working_copy_factories(),
    )
    .with_context(|| format!("failed to load workspace at {}", repo_path.display()))?;

    let repo = ws
        .repo_loader()
        .load_at_head()
        .context("failed to load repo at head")?;

    Ok((ws, repo))
}

enum ChangeIdResolution {
    Found(CommitId),
    Ambiguous,
    NotFound,
}

fn resolve_change_id(repo: &Arc<ReadonlyRepo>, change_id: &str) -> ChangeIdResolution {
    let Some(prefix) = HexPrefix::try_from_reverse_hex(change_id.as_bytes()) else {
        return ChangeIdResolution::NotFound;
    };

    match repo.resolve_change_id_prefix(&prefix) {
        Ok(PrefixResolution::SingleMatch(targets)) => {
            if let Some((_, commit_id)) = targets.visible_with_offsets().next() {
                ChangeIdResolution::Found(commit_id.clone())
            } else {
                ChangeIdResolution::NotFound
            }
        }
        Ok(PrefixResolution::AmbiguousMatch) => ChangeIdResolution::Ambiguous,
        Ok(PrefixResolution::NoMatch) | Err(_) => ChangeIdResolution::NotFound,
    }
}

fn resolve_commit_id(repo: &Arc<ReadonlyRepo>, commit_id_str: &str) -> ChangeIdResolution {
    let Some(prefix) = HexPrefix::try_from_hex(commit_id_str.as_bytes()) else {
        return ChangeIdResolution::NotFound;
    };

    match repo.index().resolve_commit_id_prefix(&prefix) {
        Ok(PrefixResolution::SingleMatch(commit_id)) => ChangeIdResolution::Found(commit_id),
        Ok(PrefixResolution::AmbiguousMatch) => ChangeIdResolution::Ambiguous,
        Ok(PrefixResolution::NoMatch) | Err(_) => ChangeIdResolution::NotFound,
    }
}

fn collect_tree_diff(stream: TreeDiffStream) -> Vec<TreeDiffEntry> {
    use futures::StreamExt;
    futures::executor::block_on(async { stream.collect().await })
}

fn format_timestamp(ts: &jj_lib::backend::Timestamp) -> String {
    match ts.to_datetime() {
        Ok(dt) => dt.to_rfc3339(),
        Err(_) => {
            let sign = if ts.tz_offset >= 0 { '+' } else { '-' };
            let hours = ts.tz_offset.abs() / 60;
            let mins = ts.tz_offset.abs() % 60;
            format!("{}{}{:02}:{:02}", ts.timestamp.0, sign, hours, mins)
        }
    }
}

/// Trait abstracting jj workspace operations for testability.
pub trait WorkspaceOps {
    fn get_status(&self) -> Result<StatusInfo>;
    fn list_changes(&self, limit: usize) -> Result<Vec<ChangeInfo>>;
    fn show_change(&self, change_id: &str) -> Result<ChangeInfo>;
    fn get_diff(&self, change_id: Option<&str>) -> Result<DiffOutput>;
    fn list_bookmarks(&self) -> Result<Vec<BookmarkInfo>>;
    fn create_bookmark(&self, name: &str) -> Result<BookmarkInfo>;
    fn delete_bookmark(&self, name: &str) -> Result<()>;
}

/// Real implementation backed by jj-lib.
#[derive(Debug)]
pub struct JjWorkspaceOps {
    workspace_root: PathBuf,
}

impl JjWorkspaceOps {
    /// Open a jj workspace by searching upward from `path` for a `.jj` directory.
    pub fn open(path: &Path) -> Result<Self> {
        let mut current = path.to_path_buf();
        loop {
            if current.join(".jj").is_dir() {
                return Ok(Self {
                    workspace_root: current,
                });
            }
            if !current.pop() {
                bail!("not a jj workspace (or any parent): {}", path.display());
            }
        }
    }

    fn load_workspace(&self) -> Result<(Workspace, Arc<ReadonlyRepo>)> {
        let config = UserConfig {
            name: "Plue CLI".to_string(),
            email: "plue@localhost".to_string(),
        };
        let settings = create_settings(&config);
        load_repo_at_head(&self.workspace_root, &settings)
    }

    /// List commits along the working-copy first-parent lineage.
    pub fn list_working_copy_lineage(&self, limit: usize) -> Result<Vec<ChangeInfo>> {
        let (_ws, repo) = self.load_workspace()?;
        let wc_name = jj_lib::ref_name::WorkspaceName::DEFAULT;
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(wc_name)
            .context("no working copy commit")?
            .clone();

        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current = Some(wc_commit_id.clone());

        while let Some(commit_id) = current {
            if result.len() >= limit || !visited.insert(commit_id.clone()) {
                break;
            }

            let commit = repo.store().get_commit(&commit_id)?;

            // Skip root commit
            if commit.parent_ids().is_empty() && commit.description().is_empty() {
                break;
            }

            let is_wc = commit.id() == &wc_commit_id;
            result.push(self.commit_to_change_info(&repo, &commit, is_wc)?);

            current = commit.parent_ids().first().cloned();
        }

        Ok(result)
    }

    fn commit_to_change_info(
        &self,
        repo: &Arc<ReadonlyRepo>,
        commit: &jj_lib::commit::Commit,
        is_wc: bool,
    ) -> Result<ChangeInfo> {
        let view = repo.view();
        let change_id_hex = commit.change_id().reverse_hex();
        let commit_id_hex = commit.id().hex();

        let bookmarks: Vec<String> = view
            .local_bookmarks_for_commit(commit.id())
            .map(|(name, _)| name.as_str().to_string())
            .collect();

        let author = commit.author();
        let ts = author.timestamp;
        let timestamp = format_timestamp(&ts);
        let is_empty = commit.is_empty(repo.as_ref()).unwrap_or(false);

        Ok(ChangeInfo {
            change_id: change_id_hex,
            commit_id: commit_id_hex,
            description: commit.description().trim().to_string(),
            author: AuthorInfo {
                name: author.name.clone(),
                email: author.email.clone(),
            },
            timestamp,
            is_empty,
            is_working_copy: is_wc,
            bookmarks,
        })
    }

    fn resolve_change_id_str(
        &self,
        repo: &Arc<ReadonlyRepo>,
        change_id_str: &str,
    ) -> Result<CommitId> {
        // Try as reverse-hex change ID prefix (jj change IDs use reverse hex)
        match resolve_change_id(repo, change_id_str) {
            ChangeIdResolution::Found(commit_id) => return Ok(commit_id),
            ChangeIdResolution::Ambiguous => {
                bail!("ambiguous change ID prefix: {change_id_str}");
            }
            ChangeIdResolution::NotFound => {}
        }

        // Try as hex commit ID prefix
        match resolve_commit_id(repo, change_id_str) {
            ChangeIdResolution::Found(commit_id) => return Ok(commit_id),
            ChangeIdResolution::Ambiguous => {
                bail!("ambiguous commit ID prefix: {change_id_str}");
            }
            ChangeIdResolution::NotFound => {}
        }

        bail!("no matching change or commit ID: {change_id_str}");
    }

    fn collect_file_changes(
        &self,
        parent_tree: &jj_lib::merged_tree::MergedTree,
        tree: &jj_lib::merged_tree::MergedTree,
    ) -> Result<Vec<FileChange>> {
        let mut changes = Vec::new();
        let diff_stream = parent_tree.diff_stream(tree, &EverythingMatcher);
        let entries: Vec<TreeDiffEntry> = collect_stream(diff_stream);

        for entry in entries {
            let path = entry.path.as_internal_file_string().to_string();
            if let Ok(diff) = entry.values {
                let change_type = if diff.before.is_absent() {
                    FileChangeType::Added
                } else if diff.after.is_absent() {
                    FileChangeType::Deleted
                } else {
                    FileChangeType::Modified
                };
                changes.push(FileChange { path, change_type });
            }
        }
        Ok(changes)
    }

    fn build_diff_output(
        &self,
        store: &Arc<Store>,
        change_id_hex: &str,
        parent_tree: &jj_lib::merged_tree::MergedTree,
        tree: &jj_lib::merged_tree::MergedTree,
    ) -> Result<DiffOutput> {
        let mut file_diffs = Vec::new();
        let diff_stream = parent_tree.diff_stream(tree, &EverythingMatcher);
        let entries: Vec<TreeDiffEntry> = collect_stream(diff_stream);

        for entry in entries {
            let path = entry.path.as_internal_file_string().to_string();
            if let Ok(diff) = entry.values {
                let change_type = if diff.before.is_absent() {
                    FileChangeType::Added
                } else if diff.after.is_absent() {
                    FileChangeType::Deleted
                } else {
                    FileChangeType::Modified
                };

                let hunks = self.build_hunks(store, &entry.path, &diff)?;

                file_diffs.push(FileDiff {
                    path,
                    change_type,
                    hunks,
                });
            }
        }

        Ok(DiffOutput {
            change_id: change_id_hex.to_string(),
            file_diffs,
        })
    }

    fn build_hunks(
        &self,
        store: &Arc<Store>,
        path: &jj_lib::repo_path::RepoPath,
        diff: &jj_lib::merge::Diff<jj_lib::merge::Merge<Option<jj_lib::backend::TreeValue>>>,
    ) -> Result<Vec<DiffHunk>> {
        let before_content = self.materialize_content(store, path, &diff.before)?;
        let after_content = self.materialize_content(store, path, &diff.after)?;

        let before_lines: Vec<&str> = if before_content.is_empty() {
            vec![]
        } else {
            before_content.lines().collect()
        };
        let after_lines: Vec<&str> = if after_content.is_empty() {
            vec![]
        } else {
            after_content.lines().collect()
        };

        let diff_lines = lcs_diff(&before_lines, &after_lines);

        if diff_lines.is_empty() {
            return Ok(vec![]);
        }

        let header = format!(
            "@@ -{},{} +{},{} @@",
            1.min(before_lines.len()),
            before_lines.len(),
            1.min(after_lines.len()),
            after_lines.len()
        );

        Ok(vec![DiffHunk {
            header,
            lines: diff_lines,
        }])
    }

    fn materialize_content(
        &self,
        store: &Arc<Store>,
        path: &jj_lib::repo_path::RepoPath,
        value: &jj_lib::merge::Merge<Option<jj_lib::backend::TreeValue>>,
    ) -> Result<String> {
        if value.is_absent() {
            return Ok(String::new());
        }
        if let Some(Some(jj_lib::backend::TreeValue::File { id, .. })) = value.as_resolved() {
            use pollster::FutureExt as _;
            use tokio::io::AsyncReadExt;
            let mut reader = store
                .read_file(path, id)
                .block_on()
                .context("failed to read file")?;
            let mut content = Vec::new();
            reader
                .read_to_end(&mut content)
                .block_on()
                .context("failed to read file content")?;
            Ok(String::from_utf8_lossy(&content).to_string())
        } else {
            Ok(String::new())
        }
    }
}

/// Compute a line-by-line diff using LCS (Longest Common Subsequence).
/// Returns only Added and Removed lines (no context lines).
fn lcs_diff(before: &[&str], after: &[&str]) -> Vec<DiffLine> {
    let n = before.len();
    let m = after.len();

    // Build LCS table
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if before[i - 1] == after[j - 1] {
                table[i][j] = table[i - 1][j - 1] + 1;
            } else {
                table[i][j] = table[i - 1][j].max(table[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff
    let mut lines = Vec::new();
    let mut i = n;
    let mut j = m;
    let mut stack = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && before[i - 1] == after[j - 1] {
            // Common line — skip (no context output)
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || table[i][j - 1] >= table[i - 1][j]) {
            stack.push(DiffLine {
                kind: DiffLineKind::Added,
                content: after[j - 1].to_string(),
            });
            j -= 1;
        } else {
            stack.push(DiffLine {
                kind: DiffLineKind::Removed,
                content: before[i - 1].to_string(),
            });
            i -= 1;
        }
    }

    // Reverse since we built it backwards
    stack.reverse();
    lines.extend(stack);
    lines
}

fn collect_stream(stream: jj_lib::merged_tree::TreeDiffStream) -> Vec<TreeDiffEntry> {
    collect_tree_diff(stream)
}

impl WorkspaceOps for JjWorkspaceOps {
    fn get_status(&self) -> Result<StatusInfo> {
        let (_ws, repo) = self.load_workspace()?;
        let wc_name = jj_lib::ref_name::WorkspaceName::DEFAULT;
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(wc_name)
            .context("no working copy commit")?;
        let wc_commit = repo.store().get_commit(wc_commit_id)?;
        let wc_info = self.commit_to_change_info(&repo, &wc_commit, true)?;

        let parent = if !wc_commit.parent_ids().is_empty() {
            let parent_commit = repo.store().get_commit(&wc_commit.parent_ids()[0])?;
            // Skip the root commit (has no parents and empty description)
            if parent_commit.parent_ids().is_empty() && parent_commit.description().is_empty() {
                None
            } else {
                Some(self.commit_to_change_info(&repo, &parent_commit, false)?)
            }
        } else {
            None
        };

        let parent_tree = if !wc_commit.parent_ids().is_empty() {
            let parent_commit = repo.store().get_commit(&wc_commit.parent_ids()[0])?;
            parent_commit.tree()
        } else {
            jj_lib::merged_tree::MergedTree::resolved(
                repo.store().clone(),
                repo.store().empty_tree_id().clone(),
            )
        };
        let wc_tree = wc_commit.tree();
        let modified_files = self.collect_file_changes(&parent_tree, &wc_tree)?;

        Ok(StatusInfo {
            working_copy: wc_info,
            parent,
            modified_files,
        })
    }

    fn list_changes(&self, limit: usize) -> Result<Vec<ChangeInfo>> {
        let (_ws, repo) = self.load_workspace()?;
        let wc_name = jj_lib::ref_name::WorkspaceName::DEFAULT;
        let wc_commit_id = repo.view().get_wc_commit_id(wc_name).cloned();

        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut result = Vec::new();

        for head_id in repo.view().heads() {
            queue.push_back(head_id.clone());
        }

        while let Some(commit_id) = queue.pop_front() {
            if !visited.insert(commit_id.clone()) {
                continue;
            }
            let commit = match repo.store().get_commit(&commit_id) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Skip root commit
            if commit.parent_ids().is_empty() && commit.description().is_empty() {
                continue;
            }

            let is_wc = wc_commit_id.as_ref() == Some(commit.id());
            let info = self.commit_to_change_info(&repo, &commit, is_wc)?;
            result.push(info);

            for parent_id in commit.parent_ids() {
                queue.push_back(parent_id.clone());
            }
        }

        // Sort by timestamp descending (most recent first)
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        result.truncate(limit);
        Ok(result)
    }

    fn show_change(&self, change_id_str: &str) -> Result<ChangeInfo> {
        let (_ws, repo) = self.load_workspace()?;
        let wc_name = jj_lib::ref_name::WorkspaceName::DEFAULT;
        let wc_commit_id = repo.view().get_wc_commit_id(wc_name).cloned();
        let commit_id = self.resolve_change_id_str(&repo, change_id_str)?;
        let commit = repo.store().get_commit(&commit_id)?;
        let is_wc = wc_commit_id.as_ref() == Some(commit.id());
        self.commit_to_change_info(&repo, &commit, is_wc)
    }

    fn get_diff(&self, change_id_str: Option<&str>) -> Result<DiffOutput> {
        let (_ws, repo) = self.load_workspace()?;
        let wc_name = jj_lib::ref_name::WorkspaceName::DEFAULT;

        let (commit, change_id_hex) = if let Some(id_str) = change_id_str {
            let commit_id = self.resolve_change_id_str(&repo, id_str)?;
            let commit = repo.store().get_commit(&commit_id)?;
            let hex = commit.change_id().reverse_hex();
            (commit, hex)
        } else {
            let wc_commit_id = repo
                .view()
                .get_wc_commit_id(wc_name)
                .context("no working copy commit")?;
            let commit = repo.store().get_commit(wc_commit_id)?;
            let hex = commit.change_id().reverse_hex();
            (commit, hex)
        };

        let parent_tree = if !commit.parent_ids().is_empty() {
            let parent_commit = repo.store().get_commit(&commit.parent_ids()[0])?;
            parent_commit.tree()
        } else {
            jj_lib::merged_tree::MergedTree::resolved(
                repo.store().clone(),
                repo.store().empty_tree_id().clone(),
            )
        };
        let tree = commit.tree();

        self.build_diff_output(repo.store(), &change_id_hex, &parent_tree, &tree)
    }

    fn create_bookmark(&self, name: &str) -> Result<BookmarkInfo> {
        let (ws, repo) = self.load_workspace()?;
        let wc_name = jj_lib::ref_name::WorkspaceName::DEFAULT;
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(wc_name)
            .context("no working copy commit — cannot create bookmark without a commit")?
            .clone();

        let _ = ws; // hold workspace alive for transaction
        let ref_name = RefName::new(name);
        let target = RefTarget::normal(wc_commit_id);
        let mut tx = repo.start_transaction();
        tx.repo_mut().set_local_bookmark_target(ref_name, target);
        tx.commit("plue bookmark create")
            .context("failed to commit bookmark creation")?;

        let bookmarks = self.list_bookmarks()?;
        bookmarks
            .into_iter()
            .find(|b| b.name == name)
            .context(format!("bookmark {} not found after creation", name))
    }

    fn delete_bookmark(&self, name: &str) -> Result<()> {
        let (ws, repo) = self.load_workspace()?;
        let _ = ws; // hold workspace alive for transaction

        // Verify the bookmark exists before deleting
        let exists = repo
            .view()
            .get_local_bookmark(RefName::new(name))
            .is_present();
        if !exists {
            bail!("bookmark '{}' not found", name);
        }

        let ref_name = RefName::new(name);
        let mut tx = repo.start_transaction();
        tx.repo_mut()
            .set_local_bookmark_target(ref_name, RefTarget::absent());
        tx.commit("plue bookmark delete")
            .context("failed to commit bookmark deletion")?;
        Ok(())
    }

    fn list_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let (_ws, repo) = self.load_workspace()?;
        let mut bookmarks = Vec::new();

        for (name, target) in repo.view().local_bookmarks() {
            if let Some(commit_id) = target.added_ids().next() {
                let commit = repo.store().get_commit(commit_id)?;
                let change_id_hex = commit.change_id().reverse_hex();
                let commit_id_hex = commit_id.hex();

                // Check if any remote is tracking this bookmark
                let name_matcher = StringMatcher::Exact(name.as_str().to_string());
                let is_tracking = repo
                    .view()
                    .remote_bookmarks_matching(&name_matcher, &StringMatcher::All)
                    .next()
                    .is_some();

                bookmarks.push(BookmarkInfo {
                    name: name.as_str().to_string(),
                    target_change_id: change_id_hex,
                    target_commit_id: commit_id_hex,
                    is_tracking_remote: is_tracking,
                });
            }
        }

        bookmarks.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(bookmarks)
    }
}

/// Fake implementation for unit-testing command handlers.
#[cfg(test)]
#[derive(Default)]
pub struct FakeWorkspaceOps {
    pub status: Option<StatusInfo>,
    pub changes: Vec<ChangeInfo>,
    pub diff: Option<DiffOutput>,
    pub bookmarks: Vec<BookmarkInfo>,
    pub show_error: Option<String>,
}

#[cfg(test)]
impl FakeWorkspaceOps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
impl WorkspaceOps for FakeWorkspaceOps {
    fn get_status(&self) -> Result<StatusInfo> {
        self.status.clone().context("no status configured in fake")
    }

    fn list_changes(&self, limit: usize) -> Result<Vec<ChangeInfo>> {
        Ok(self.changes.iter().take(limit).cloned().collect())
    }

    fn show_change(&self, change_id: &str) -> Result<ChangeInfo> {
        if let Some(ref err) = self.show_error {
            bail!("{err}");
        }
        self.changes
            .iter()
            .find(|c| c.change_id.starts_with(change_id))
            .cloned()
            .context(format!("change not found: {change_id}"))
    }

    fn get_diff(&self, _change_id: Option<&str>) -> Result<DiffOutput> {
        self.diff.clone().context("no diff configured in fake")
    }

    fn list_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        Ok(self.bookmarks.clone())
    }

    fn create_bookmark(&self, name: &str) -> Result<BookmarkInfo> {
        Ok(BookmarkInfo {
            name: name.to_string(),
            target_change_id: "abc12345".to_string(),
            target_commit_id: "def67890".to_string(),
            is_tracking_remote: false,
        })
    }

    fn delete_bookmark(&self, _name: &str) -> Result<()> {
        Ok(())
    }
}
