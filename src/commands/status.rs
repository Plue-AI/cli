use anyhow::Result;
use clap::Args;

use crate::output::OutputFormat;
use plue::jj_ops::WorkspaceOps;
use plue::output::print_value;

#[derive(Args)]
pub struct StatusArgs;

pub fn run(_args: StatusArgs, format: OutputFormat) -> Result<()> {
    let ops = open_workspace()?;
    run_with_ops(&*ops, format)
}

pub fn run_with_ops(ops: &dyn WorkspaceOps, format: OutputFormat) -> Result<()> {
    let status = ops.get_status()?;
    print_value(&status, format);
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
        status: Option<StatusInfo>,
    }

    impl WorkspaceOps for FakeOps {
        fn get_status(&self) -> Result<StatusInfo> {
            self.status.clone().context("no status")
        }
        fn list_changes(&self, _: usize) -> Result<Vec<ChangeInfo>> {
            Ok(vec![])
        }
        fn show_change(&self, _: &str) -> Result<ChangeInfo> {
            bail!("not implemented")
        }
        fn get_diff(&self, _: Option<&str>) -> Result<DiffOutput> {
            bail!("not implemented")
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

    fn sample_status() -> StatusInfo {
        StatusInfo {
            working_copy: ChangeInfo {
                change_id: "abc123".into(),
                commit_id: "def456".into(),
                description: "WC change".into(),
                author: AuthorInfo {
                    name: "Test".into(),
                    email: "t@t.com".into(),
                },
                timestamp: "2025-01-01T00:00:00Z".into(),
                is_empty: false,
                is_working_copy: true,
                bookmarks: vec![],
            },
            parent: Some(ChangeInfo {
                change_id: "xyz789".into(),
                commit_id: "uvw012".into(),
                description: "Parent".into(),
                author: AuthorInfo {
                    name: "Test".into(),
                    email: "t@t.com".into(),
                },
                timestamp: "2024-12-31T00:00:00Z".into(),
                is_empty: false,
                is_working_copy: false,
                bookmarks: vec![],
            }),
            modified_files: vec![FileChange {
                path: "src/main.rs".into(),
                change_type: FileChangeType::Modified,
            }],
        }
    }

    #[test]
    fn test_status_table_output() {
        let fake = FakeOps {
            status: Some(sample_status()),
        };
        let result = run_with_ops(&fake, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_json_output() {
        let fake = FakeOps {
            status: Some(sample_status()),
        };
        let result = run_with_ops(&fake, OutputFormat::Json { fields: None });
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_no_parent() {
        let fake = FakeOps {
            status: Some(StatusInfo {
                parent: None,
                modified_files: vec![],
                ..sample_status()
            }),
        };
        let result = run_with_ops(&fake, OutputFormat::Table);
        assert!(result.is_ok());
    }
}
