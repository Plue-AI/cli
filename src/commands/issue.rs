use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::{filter_fields, OutputFormat};
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;
use plue::types::{CreateIssueInput, UpdateIssueInput};

#[derive(Args)]
pub struct IssueArgs {
    #[command(subcommand)]
    command: IssueCommand,
}

#[derive(Subcommand)]
enum IssueCommand {
    /// Create a new issue
    Create(CreateArgs),
    /// List issues
    List(ListArgs),
    /// View an issue
    View(ViewArgs),
    /// Close an issue
    Close(CloseArgs),
}

#[derive(Args)]
struct CreateArgs {
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
    /// Issue title
    #[arg(short = 't', long = "title")]
    title: Option<String>,
    /// Issue body
    #[arg(short = 'b', long = "body")]
    body: Option<String>,
    /// Assignee username (can be repeated)
    #[arg(short = 'a', long = "assignee")]
    assignee: Vec<String>,
}

#[derive(Args)]
struct ListArgs {
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
    /// State filter (open|closed|all)
    #[arg(short = 's', long = "state", default_value = "open")]
    state: String,
    /// Max items to fetch
    #[arg(short = 'L', long = "limit", default_value_t = 30)]
    limit: i32,
}

#[derive(Args)]
struct ViewArgs {
    /// Issue number
    number: i64,
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
}

#[derive(Args)]
struct CloseArgs {
    /// Issue number
    number: i64,
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
    /// Comment to add when closing
    #[arg(short = 'c', long = "comment")]
    comment: Option<String>,
}

pub fn run(args: IssueArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        IssueCommand::Create(args) => run_create(&args, format),
        IssueCommand::List(args) => run_list(&args, format),
        IssueCommand::View(args) => run_view(&args, format),
        IssueCommand::Close(args) => run_close(&args, format),
    }
}

fn run_create(args: &CreateArgs, format: OutputFormat) -> Result<()> {
    let title = args
        .title
        .clone()
        .unwrap_or_else(|| "Untitled issue".to_string());

    if title.trim().is_empty() {
        bail!("issue title cannot be empty");
    }

    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;

    let req = CreateIssueInput {
        title,
        body: args.body.clone().unwrap_or_default(),
        assignees: args.assignee.clone(),
    };

    let created = client.create_issue(&repo.owner, &repo.repo, &req)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&created)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&created, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Created issue #{} {}", created.number, created.title);
            println!(
                "  URL: /{}/{}/issues/{}",
                repo.owner, repo.repo, created.number
            );
        }
    }
    Ok(())
}

fn run_list(args: &ListArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;
    let state = validate_list_state(&args.state)?;

    let items = client.list_issues(&repo.owner, &repo.repo, state.as_deref(), 1, args.limit)?;

    match format {
        OutputFormat::Json { ref fields } => {
            print_json_with_projection(&items, fields.as_deref())?;
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&items, fields.as_deref());
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No issues found.");
                return Ok(());
            }
            println!("NUMBER\tTITLE\tAUTHOR\tSTATE\tCOMMENTS");
            for item in items {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    item.number, item.title, item.author.login, item.state, item.comment_count,
                );
            }
        }
    }

    Ok(())
}

fn run_view(args: &ViewArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;

    let issue = client.get_issue(&repo.owner, &repo.repo, args.number)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&issue)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&issue, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("#{} {}", issue.number, issue.title);
            println!("State: {}", issue.state);
            println!("Author: {}", issue.author.login);
            if !issue.assignees.is_empty() {
                let logins: Vec<&str> = issue.assignees.iter().map(|a| a.login.as_str()).collect();
                println!("Assignees: {}", logins.join(", "));
            }
            println!("Comments: {}", issue.comment_count);
            if !issue.body.is_empty() {
                println!();
                println!("{}", issue.body);
            }
        }
    }
    Ok(())
}

fn run_close(args: &CloseArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;

    let req = UpdateIssueInput {
        state: Some("closed".to_string()),
        body: args.comment.clone(),
        ..Default::default()
    };

    let updated = client.update_issue(&repo.owner, &repo.repo, args.number, &req)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&updated)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&updated, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Closed issue #{} {}", updated.number, updated.title);
        }
    }
    Ok(())
}

fn validate_list_state(state: &str) -> Result<Option<String>> {
    let normalized = state.trim().to_lowercase();
    match normalized.as_str() {
        "open" | "closed" => Ok(Some(normalized)),
        "all" => Ok(None),
        _ => bail!("invalid state: {state} (expected open|closed|all)"),
    }
}

fn print_json_with_projection<T: serde::Serialize>(value: &T, fields: Option<&str>) -> Result<()> {
    let json_value = serde_json::to_value(value)?;
    let projected = fields
        .filter(|requested| !requested.is_empty())
        .map(|requested| filter_fields(&json_value, requested))
        .unwrap_or(json_value);
    println!("{}", serde_json::to_string_pretty(&projected)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_state_validation_accepts_valid_states() {
        assert_eq!(
            validate_list_state("open").unwrap(),
            Some("open".to_string())
        );
        assert_eq!(
            validate_list_state("closed").unwrap(),
            Some("closed".to_string())
        );
        assert_eq!(validate_list_state("all").unwrap(), None);
    }

    #[test]
    fn list_state_validation_rejects_unknown_states() {
        let err = validate_list_state("invalid").expect_err("invalid state should fail");
        assert!(err.to_string().contains("state"));
    }

    #[test]
    fn list_state_validation_normalizes_case() {
        assert_eq!(
            validate_list_state("OPEN").unwrap(),
            Some("open".to_string())
        );
        assert_eq!(
            validate_list_state("Closed").unwrap(),
            Some("closed".to_string())
        );
    }
}
