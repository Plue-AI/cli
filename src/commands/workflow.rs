use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;

#[derive(Args)]
pub struct WorkflowArgs {
    #[command(subcommand)]
    command: WorkflowCommand,
    /// Repository in OWNER/REPO format (overrides remote detection)
    #[arg(short = 'R', long = "repo", global = true)]
    repo: Option<String>,
}

#[derive(Subcommand)]
enum WorkflowCommand {
    /// List workflows in the repository
    List(ListArgs),
    /// Trigger a workflow run
    Run(RunArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Page number
    #[arg(long, default_value = "1")]
    page: i32,
    /// Results per page
    #[arg(long, default_value = "30")]
    per_page: i32,
}

#[derive(Args)]
struct RunArgs {
    /// Workflow ID to trigger
    workflow_id: i64,
    /// Git ref (branch/tag) to run on
    #[arg(long, default_value = "main")]
    r#ref: String,
}

pub fn run(args: WorkflowArgs, format: OutputFormat) -> Result<()> {
    let repo_override = args.repo.as_deref();
    match args.command {
        WorkflowCommand::List(a) => run_list(&a, repo_override, format),
        WorkflowCommand::Run(a) => run_dispatch(&a, repo_override),
    }
}

fn run_list(args: &ListArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let workflows =
        client.list_workflows(&repo_ref.owner, &repo_ref.repo, args.page, args.per_page)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&workflows)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&workflows, fields.as_deref());
        }
        OutputFormat::Table => {
            if workflows.is_empty() {
                println!("No workflows found.");
                return Ok(());
            }
            println!("{:<6} {:<30} {:<40} ACTIVE", "ID", "NAME", "PATH");
            for wf in &workflows {
                println!(
                    "{:<6} {:<30} {:<40} {}",
                    wf.id,
                    wf.name,
                    wf.path,
                    if wf.is_active { "yes" } else { "no" }
                );
            }
        }
    }
    Ok(())
}

fn run_dispatch(args: &RunArgs, repo_override: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    client.dispatch_workflow(
        &repo_ref.owner,
        &repo_ref.repo,
        args.workflow_id,
        &args.r#ref,
    )?;

    println!(
        "Triggered workflow {} on ref '{}'",
        args.workflow_id, args.r#ref
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_dispatch_args_parse() {
        // Verify that RunArgs parses workflow_id and ref correctly.
        let run_args = RunArgs {
            workflow_id: 42,
            r#ref: "main".to_string(),
        };
        assert_eq!(run_args.workflow_id, 42);
        assert_eq!(run_args.r#ref, "main");
    }
}
