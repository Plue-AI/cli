use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use plue::api_client::ApiClient;
use plue::config::Config;
use plue::jj_ops::WorkspaceOps;
use plue::output::print_toon;
use plue::output::OutputFormat;
use plue::repo_context::{resolve_repo_ref, RepoRef};
use plue::types::{BookmarkResponse, CreateBookmarkInput};

#[derive(Args)]
pub struct BookmarkArgs {
    #[command(subcommand)]
    command: BookmarkCommand,
}

#[derive(Subcommand)]
enum BookmarkCommand {
    /// List bookmarks
    List(ListArgs),
    /// Create a bookmark at the current working copy
    Create(CreateArgs),
    /// Delete a bookmark
    Delete(DeleteArgs),
}

#[derive(Args)]
pub(crate) struct ListArgs {
    /// Use remote API instead of local jj operations
    #[arg(short, long)]
    remote: bool,
    /// Repository override (owner/repo format)
    #[arg(short = 'R', long)]
    repo: Option<String>,
}

#[derive(Args)]
pub(crate) struct CreateArgs {
    /// Bookmark name
    name: String,
    /// Target change ID (required for remote operations)
    #[arg(short, long)]
    change_id: Option<String>,
    /// Use remote API instead of local jj operations
    #[arg(short, long)]
    remote: bool,
    /// Repository override (owner/repo format)
    #[arg(short = 'R', long)]
    repo: Option<String>,
}

#[derive(Args)]
pub(crate) struct DeleteArgs {
    /// Bookmark name to delete
    name: String,
    /// Use remote API instead of local jj operations
    #[arg(short, long)]
    remote: bool,
    /// Repository override (owner/repo format)
    #[arg(short = 'R', long)]
    repo: Option<String>,
}

pub fn run(args: BookmarkArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        BookmarkCommand::List(a) => run_list(a, format),
        BookmarkCommand::Create(a) => run_create(a, format),
        BookmarkCommand::Delete(a) => run_delete(a, format),
    }
}

fn run_list(args: ListArgs, format: OutputFormat) -> Result<()> {
    if args.remote {
        let repo_ref = resolve_repo_context(args.repo)?;
        let client = create_api_client()?;
        let bookmarks = client.list_bookmarks(&repo_ref.owner, &repo_ref.repo, 1, 100)?;
        output_bookmarks(&bookmarks, format)
    } else {
        let ops = open_workspace()?;
        run_list_local(&*ops, format)
    }
}

fn run_create(args: CreateArgs, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        bail!("bookmark name cannot be empty");
    }

    if args.remote {
        let repo_ref = resolve_repo_context(args.repo)?;
        let client = create_api_client()?;

        let change_id = match args.change_id {
            Some(id) => id,
            None => bail!("--change-id is required for remote bookmark creation"),
        };

        let input = CreateBookmarkInput {
            name: args.name.clone(),
            target_change_id: change_id,
        };

        let bookmark = client.create_bookmark(&repo_ref.owner, &repo_ref.repo, &input)?;
        output_bookmark_created(&bookmark, format)
    } else {
        let ops = open_workspace()?;
        run_create_local(&*ops, &args, format)
    }
}

fn run_delete(args: DeleteArgs, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        bail!("bookmark name cannot be empty");
    }

    if args.remote {
        let repo_ref = resolve_repo_context(args.repo)?;
        let client = create_api_client()?;
        client.delete_bookmark(&repo_ref.owner, &repo_ref.repo, &args.name)?;
        match format {
            OutputFormat::Json { .. } | OutputFormat::Toon { .. } => {}
            OutputFormat::Table => {
                println!("Deleted remote bookmark '{}'", args.name);
            }
        }
        Ok(())
    } else {
        let ops = open_workspace()?;
        run_delete_local(&*ops, &args, format)
    }
}

// Local jj operations
pub fn run_list_local(ops: &dyn WorkspaceOps, format: OutputFormat) -> Result<()> {
    let bookmarks = ops.list_bookmarks()?;
    output_bookmark_infos(&bookmarks, format)
}

pub fn run_create_local(
    ops: &dyn WorkspaceOps,
    args: &CreateArgs,
    format: OutputFormat,
) -> Result<()> {
    if args.name.trim().is_empty() {
        bail!("bookmark name cannot be empty");
    }
    let info = ops.create_bookmark(&args.name)?;
    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&info).expect("serialize")
            );
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&[info], fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Created bookmark '{}' at {}",
                info.name, info.target_change_id
            );
        }
    }
    Ok(())
}

pub fn run_delete_local(
    ops: &dyn WorkspaceOps,
    args: &DeleteArgs,
    format: OutputFormat,
) -> Result<()> {
    if args.name.trim().is_empty() {
        bail!("bookmark name cannot be empty");
    }
    ops.delete_bookmark(&args.name)?;
    match format {
        OutputFormat::Json { .. } | OutputFormat::Toon { .. } => {
            // Silent for machine-readable formats
        }
        OutputFormat::Table => {
            println!("Deleted bookmark '{}'", args.name);
        }
    }
    Ok(())
}

// Helper functions
fn resolve_repo_context(repo_override: Option<String>) -> Result<RepoRef> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    resolve_repo_ref(&cwd, repo_override.as_deref())
}

fn create_api_client() -> Result<ApiClient> {
    let config = Config::load()?;
    ApiClient::from_config(&config)
}

fn open_workspace() -> Result<Box<dyn WorkspaceOps>> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let ops = plue::jj_ops::JjWorkspaceOps::open(&cwd)?;
    Ok(Box::new(ops))
}

// Output formatting helpers
fn output_bookmarks(bookmarks: &[BookmarkResponse], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(bookmarks).expect("serialize")
            );
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&bookmarks.to_vec(), fields.as_deref());
        }
        OutputFormat::Table => {
            if bookmarks.is_empty() {
                println!("No remote bookmarks found.");
            } else {
                for bm in bookmarks {
                    println!("{}", format_bookmark_response(bm));
                }
            }
        }
    }
    Ok(())
}

fn output_bookmark_infos(
    bookmarks: &[plue::types::BookmarkInfo],
    format: OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(bookmarks).expect("serialize")
            );
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&bookmarks.to_vec(), fields.as_deref());
        }
        OutputFormat::Table => {
            if bookmarks.is_empty() {
                println!("No bookmarks found.");
            } else {
                for bm in bookmarks {
                    println!("{bm}");
                }
            }
        }
    }
    Ok(())
}

fn output_bookmark_created(bookmark: &BookmarkResponse, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(bookmark).expect("serialize")
            );
        }
        OutputFormat::Toon { ref fields } => {
            #[allow(clippy::cloned_ref_to_slice_refs)]
            print_toon(&[bookmark.clone()], fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Created remote bookmark '{}' at {}",
                bookmark.name, bookmark.target_change_id
            );
        }
    }
    Ok(())
}

fn format_bookmark_response(bm: &BookmarkResponse) -> String {
    let tracking = if bm.is_tracking_remote {
        " (tracking)"
    } else {
        ""
    };
    format!(
        "{}: {} {}{}",
        bm.name, bm.target_change_id, bm.target_commit_id, tracking
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use plue::types::*;

    struct FakeOps {
        bookmarks: Vec<BookmarkInfo>,
        delete_fails: bool,
    }

    impl WorkspaceOps for FakeOps {
        fn get_status(&self) -> Result<StatusInfo> {
            bail!("not implemented")
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
            Ok(self.bookmarks.clone())
        }
        fn create_bookmark(&self, name: &str) -> Result<BookmarkInfo> {
            Ok(BookmarkInfo {
                name: name.to_string(),
                target_change_id: "abc123".to_string(),
                target_commit_id: "def456".to_string(),
                is_tracking_remote: false,
            })
        }
        fn delete_bookmark(&self, name: &str) -> Result<()> {
            if self.delete_fails {
                bail!("bookmark '{}' not found", name)
            }
            Ok(())
        }
    }

    fn sample_bookmarks() -> Vec<BookmarkInfo> {
        vec![
            BookmarkInfo {
                name: "main".into(),
                target_change_id: "abc123".into(),
                target_commit_id: "def456".into(),
                is_tracking_remote: false,
            },
            BookmarkInfo {
                name: "dev".into(),
                target_change_id: "xyz789".into(),
                target_commit_id: "uvw012".into(),
                is_tracking_remote: true,
            },
        ]
    }

    #[test]
    fn test_bookmark_list_table() {
        let fake = FakeOps {
            bookmarks: sample_bookmarks(),
            delete_fails: false,
        };
        let result = run_list_local(&fake, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bookmark_list_json() {
        let fake = FakeOps {
            bookmarks: sample_bookmarks(),
            delete_fails: false,
        };
        let result = run_list_local(&fake, OutputFormat::Json { fields: None });
        assert!(result.is_ok());
    }

    #[test]
    fn test_bookmark_list_empty() {
        let fake = FakeOps {
            bookmarks: vec![],
            delete_fails: false,
        };
        let result = run_list_local(&fake, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bookmark_create_empty_name_fails() {
        let fake = FakeOps {
            bookmarks: vec![],
            delete_fails: false,
        };
        let args = CreateArgs {
            name: "  ".to_string(),
            change_id: None,
            remote: false,
            repo: None,
        };
        let err = run_create_local(&fake, &args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("bookmark name cannot be empty"));
    }

    #[test]
    fn test_bookmark_create_succeeds() {
        let fake = FakeOps {
            bookmarks: vec![],
            delete_fails: false,
        };
        let args = CreateArgs {
            name: "my-feature".to_string(),
            change_id: None,
            remote: false,
            repo: None,
        };
        let result = run_create_local(&fake, &args, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bookmark_delete_empty_name_fails() {
        let fake = FakeOps {
            bookmarks: vec![],
            delete_fails: false,
        };
        let args = DeleteArgs {
            name: "".to_string(),
            remote: false,
            repo: None,
        };
        let err = run_delete_local(&fake, &args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("bookmark name cannot be empty"));
    }

    #[test]
    fn test_bookmark_delete_succeeds() {
        let fake = FakeOps {
            bookmarks: sample_bookmarks(),
            delete_fails: false,
        };
        let args = DeleteArgs {
            name: "main".to_string(),
            remote: false,
            repo: None,
        };
        let result = run_delete_local(&fake, &args, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bookmark_delete_not_found_fails() {
        let fake = FakeOps {
            bookmarks: vec![],
            delete_fails: true,
        };
        let args = DeleteArgs {
            name: "nonexistent".to_string(),
            remote: false,
            repo: None,
        };
        let err = run_delete_local(&fake, &args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("not found"));
    }

    // Remote bookmark tests
    #[test]
    fn test_format_bookmark_response() {
        let bm = BookmarkResponse {
            name: "main".to_string(),
            target_change_id: "abc123".to_string(),
            target_commit_id: "def456".to_string(),
            is_tracking_remote: true,
        };
        let output = format_bookmark_response(&bm);
        assert!(output.contains("main: abc123 def456"));
        assert!(output.contains("(tracking)"));
    }

    #[test]
    fn test_format_bookmark_response_no_tracking() {
        let bm = BookmarkResponse {
            name: "feature".to_string(),
            target_change_id: "abc".to_string(),
            target_commit_id: "def".to_string(),
            is_tracking_remote: false,
        };
        let output = format_bookmark_response(&bm);
        assert!(!output.contains("(tracking)"));
    }
}
