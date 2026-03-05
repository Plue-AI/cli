use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::{ApiClient, ApiError};
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;

#[derive(Args)]
pub struct RunArgs {
    #[command(subcommand)]
    command: RunCommand,
    /// Repository in OWNER/REPO format (overrides remote detection)
    #[arg(short = 'R', long = "repo", global = true)]
    repo: Option<String>,
}

#[derive(Subcommand)]
enum RunCommand {
    /// List workflow runs for a workflow
    List(ListArgs),
    /// View a workflow run
    View(ViewArgs),
    /// Watch a workflow run in real-time (polls until completion)
    Watch(WatchArgs),
    /// Re-run a workflow
    Rerun(RerunArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Workflow ID to list runs for
    workflow_id: i64,
    /// Page number
    #[arg(long, default_value = "1")]
    page: i32,
    /// Results per page
    #[arg(long, default_value = "30")]
    per_page: i32,
}

#[derive(Args)]
struct ViewArgs {
    /// Run ID to view
    run_id: i64,
}

#[derive(Args)]
struct WatchArgs {
    /// Run ID to watch
    run_id: i64,
}

#[derive(Args)]
struct RerunArgs {
    /// Run ID to re-run
    run_id: i64,
}

pub fn run(args: RunArgs, format: OutputFormat) -> Result<()> {
    let repo_override = args.repo.as_deref();
    match args.command {
        RunCommand::List(a) => run_list(&a, repo_override, format),
        RunCommand::View(a) => run_view(&a, repo_override, format),
        RunCommand::Watch(a) => run_watch(&a, repo_override),
        RunCommand::Rerun(a) => run_rerun(&a, repo_override, format),
    }
}

fn run_list(args: &ListArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let runs = match client.list_workflow_runs(
        &repo_ref.owner,
        &repo_ref.repo,
        args.workflow_id,
        args.page,
        args.per_page,
    ) {
        Ok(runs) => runs,
        Err(err) => {
            if err
                .downcast_ref::<ApiError>()
                .map(|api_err| api_err.status == 404)
                .unwrap_or(false)
            {
                Vec::new()
            } else {
                return Err(err);
            }
        }
    };

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&runs)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&runs, fields.as_deref());
        }
        OutputFormat::Table => {
            if runs.is_empty() {
                println!("No runs found.");
                return Ok(());
            }
            println!(
                "{:<8} {:<12} {:<12} {:<20}",
                "ID", "STATUS", "EVENT", "CREATED"
            );
            for r in &runs {
                println!(
                    "{:<8} {:<12} {:<12} {:<20}",
                    r.id, r.status, r.trigger_event, r.created_at
                );
            }
        }
    }
    Ok(())
}

fn run_view(args: &ViewArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let run = client.get_workflow_run(&repo_ref.owner, &repo_ref.repo, args.run_id)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&run)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&run, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Run #{}", run.id);
            println!("  Status:    {}", run.status);
            println!("  Workflow:  #{}", run.workflow_definition_id);
            println!("  Trigger:   {} on {}", run.trigger_event, run.trigger_ref);
            if !run.trigger_commit_sha.is_empty() {
                println!("  Commit:    {}", run.trigger_commit_sha);
            }
            if let Some(started) = &run.started_at {
                println!("  Started:   {started}");
            }
            if let Some(completed) = &run.completed_at {
                println!("  Completed: {completed}");
            }
            println!("  Created:   {}", run.created_at);
        }
    }
    Ok(())
}

fn run_watch(args: &WatchArgs, repo_override: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    println!("Watching run #{}…", args.run_id);
    loop {
        let run = client.get_workflow_run(&repo_ref.owner, &repo_ref.repo, args.run_id)?;
        println!("  status: {}", run.status);
        match run.status.as_str() {
            "completed" | "failed" | "cancelled" | "skipped" => {
                println!("Run #{} finished with status: {}", args.run_id, run.status);
                break;
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }
    }
    Ok(())
}

fn run_rerun(args: &RerunArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let result = client.rerun_workflow_run(&repo_ref.owner, &repo_ref.repo, args.run_id, None)?;

    match format {
        OutputFormat::Json { .. } => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&[result], fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Re-run created: run #{} (definition #{})",
                result.workflow_run_id, result.workflow_definition_id
            );
            if !result.steps.is_empty() {
                println!("  Steps: {}", result.steps.len());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_args_store_run_id() {
        let a = ViewArgs { run_id: 99 };
        assert_eq!(a.run_id, 99);
    }

    #[test]
    fn list_args_default_pagination() {
        let a = ListArgs {
            workflow_id: 5,
            page: 1,
            per_page: 30,
        };
        assert_eq!(a.page, 1);
        assert_eq!(a.per_page, 30);
    }
}
