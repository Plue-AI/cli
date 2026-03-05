use anyhow::Result;
use clap::{Args, Subcommand};

use crate::output::OutputFormat;
use plue::jj_ops::WorkspaceOps;
use plue::output::{print_toon, print_value};

#[derive(Args)]
pub struct ChangeArgs {
    #[command(subcommand)]
    command: ChangeCommand,
}

#[derive(Subcommand)]
enum ChangeCommand {
    /// List recent changes
    List {
        /// Maximum number of changes to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Show change details
    Show {
        /// Change ID (or prefix)
        change_id: String,
    },
    /// Show diff for a change
    Diff {
        /// Change ID (defaults to working copy)
        change_id: Option<String>,
    },
}

pub fn run(args: ChangeArgs, format: OutputFormat) -> Result<()> {
    let ops = open_workspace()?;
    match args.command {
        ChangeCommand::List { limit } => run_list(&*ops, format, limit),
        ChangeCommand::Show { change_id } => run_show(&*ops, format, &change_id),
        ChangeCommand::Diff { change_id } => run_diff(&*ops, format, change_id.as_deref()),
    }
}

pub fn run_list(ops: &dyn WorkspaceOps, format: OutputFormat, limit: usize) -> Result<()> {
    let changes = ops.list_changes(limit)?;
    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&changes).expect("serialize")
            );
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&changes, fields.as_deref());
        }
        OutputFormat::Table => {
            if changes.is_empty() {
                println!("No changes found.");
            } else {
                for change in &changes {
                    println!("{change}");
                }
            }
        }
    }
    Ok(())
}

pub fn run_show(ops: &dyn WorkspaceOps, format: OutputFormat, change_id: &str) -> Result<()> {
    let info = ops.show_change(change_id)?;
    print_value(&info, format);
    Ok(())
}

pub fn run_diff(
    ops: &dyn WorkspaceOps,
    format: OutputFormat,
    change_id: Option<&str>,
) -> Result<()> {
    let diff = ops.get_diff(change_id)?;
    print_value(&diff, format);
    Ok(())
}

fn open_workspace() -> Result<Box<dyn WorkspaceOps>> {
    let cwd = std::env::current_dir()?;
    let ops = plue::jj_ops::JjWorkspaceOps::open(&cwd)?;
    Ok(Box::new(ops))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{bail, Context};
    use plue::types::*;

    struct FakeOps {
        changes: Vec<ChangeInfo>,
        diff: Option<DiffOutput>,
        show_error: bool,
    }

    impl FakeOps {
        fn new() -> Self {
            Self {
                changes: vec![],
                diff: None,
                show_error: false,
            }
        }
    }

    impl WorkspaceOps for FakeOps {
        fn get_status(&self) -> Result<StatusInfo> {
            bail!("not implemented")
        }
        fn list_changes(&self, limit: usize) -> Result<Vec<ChangeInfo>> {
            Ok(self.changes.iter().take(limit).cloned().collect())
        }
        fn show_change(&self, id: &str) -> Result<ChangeInfo> {
            if self.show_error {
                bail!("not found");
            }
            self.changes
                .iter()
                .find(|c| c.change_id.starts_with(id))
                .cloned()
                .context(format!("change not found: {id}"))
        }
        fn get_diff(&self, _: Option<&str>) -> Result<DiffOutput> {
            self.diff.clone().context("no diff")
        }
        fn list_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
            Ok(vec![])
        }
        fn create_bookmark(&self, name: &str) -> Result<BookmarkInfo> {
            Ok(BookmarkInfo {
                name: name.to_string(),
                target_change_id: "fake000".to_string(),
                target_commit_id: "fake000".to_string(),
                is_tracking_remote: false,
            })
        }
        fn delete_bookmark(&self, _name: &str) -> Result<()> {
            Ok(())
        }
    }

    fn sample_changes() -> Vec<ChangeInfo> {
        vec![
            ChangeInfo {
                change_id: "abc123".into(),
                commit_id: "def456".into(),
                description: "First change".into(),
                author: AuthorInfo {
                    name: "Test".into(),
                    email: "t@t.com".into(),
                },
                timestamp: "2025-01-01T00:00:00Z".into(),
                is_empty: false,
                is_working_copy: true,
                bookmarks: vec!["main".into()],
            },
            ChangeInfo {
                change_id: "xyz789".into(),
                commit_id: "uvw012".into(),
                description: "Second change".into(),
                author: AuthorInfo {
                    name: "Test".into(),
                    email: "t@t.com".into(),
                },
                timestamp: "2025-01-02T00:00:00Z".into(),
                is_empty: false,
                is_working_copy: false,
                bookmarks: vec![],
            },
        ]
    }

    #[test]
    fn test_change_list_table_output() {
        let mut fake = FakeOps::new();
        fake.changes = sample_changes();
        let result = run_list(&fake, OutputFormat::Table, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_list_json_output() {
        let mut fake = FakeOps::new();
        fake.changes = sample_changes();
        let result = run_list(&fake, OutputFormat::Json { fields: None }, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_list_empty() {
        let fake = FakeOps::new();
        let result = run_list(&fake, OutputFormat::Table, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_table_output() {
        let mut fake = FakeOps::new();
        fake.changes = sample_changes();
        let result = run_show(&fake, OutputFormat::Table, "abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_json_output() {
        let mut fake = FakeOps::new();
        fake.changes = sample_changes();
        let result = run_show(&fake, OutputFormat::Json { fields: None }, "abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_not_found() {
        let fake = FakeOps::new();
        let result = run_show(&fake, OutputFormat::Table, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_diff_table_output() {
        let mut fake = FakeOps::new();
        fake.diff = Some(DiffOutput {
            change_id: "abc123".into(),
            file_diffs: vec![FileDiff {
                path: "file.txt".into(),
                change_type: FileChangeType::Added,
                hunks: vec![DiffHunk {
                    header: "@@ -0,0 +1,1 @@".into(),
                    lines: vec![DiffLine {
                        kind: DiffLineKind::Added,
                        content: "hello".into(),
                    }],
                }],
            }],
        });
        let result = run_diff(&fake, OutputFormat::Table, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_diff_json_output() {
        let mut fake = FakeOps::new();
        fake.diff = Some(DiffOutput {
            change_id: "abc".into(),
            file_diffs: vec![],
        });
        let result = run_diff(&fake, OutputFormat::Json { fields: None }, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_diff_no_changes() {
        let mut fake = FakeOps::new();
        fake.diff = Some(DiffOutput {
            change_id: "abc".into(),
            file_diffs: vec![],
        });
        let result = run_diff(&fake, OutputFormat::Table, None);
        assert!(result.is_ok());
    }
}
