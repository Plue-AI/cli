use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;
use plue::output::print_toon;

#[derive(Args)]
pub struct SearchArgs {
    #[command(subcommand)]
    command: SearchCommand,
}

#[derive(Subcommand)]
enum SearchCommand {
    /// Search repositories
    Repos(ReposArgs),
    /// Search issues
    Issues(IssuesArgs),
    /// Search code
    Code(CodeArgs),
}

#[derive(Args)]
struct ReposArgs {
    /// Search query
    query: String,
    /// Max items to return
    #[arg(short = 'L', long = "limit", default_value_t = 30)]
    limit: i32,
}

#[derive(Args)]
struct IssuesArgs {
    /// Search query
    query: String,
    /// Filter by state (open|closed|all)
    #[arg(short = 's', long = "state")]
    state: Option<String>,
    /// Max items to return
    #[arg(short = 'L', long = "limit", default_value_t = 30)]
    limit: i32,
}

#[derive(Args)]
struct CodeArgs {
    /// Search query
    query: String,
    /// Max items to return
    #[arg(short = 'L', long = "limit", default_value_t = 30)]
    limit: i32,
}

pub fn run(args: SearchArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        SearchCommand::Repos(a) => run_repos(&a, format),
        SearchCommand::Issues(a) => run_issues(&a, format),
        SearchCommand::Code(a) => run_code(&a, format),
    }
}

fn run_repos(args: &ReposArgs, format: OutputFormat) -> Result<()> {
    if args.query.trim().is_empty() {
        anyhow::bail!("search query cannot be empty");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let result = client.search_repositories(&args.query, 1, args.limit)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&result, fields.as_deref());
        }
        OutputFormat::Table => {
            if result.items.is_empty() {
                println!("No repositories found for query: {}", args.query);
                return Ok(());
            }
            println!(
                "Showing {} of {} repositories",
                result.items.len(),
                result.total_count
            );
            println!("{:<40} {:<8} DESCRIPTION", "FULL_NAME", "VISIBILITY");
            for repo in &result.items {
                let visibility = if repo.is_public { "public" } else { "private" };
                println!(
                    "{:<40} {:<8} {}",
                    repo.full_name, visibility, repo.description
                );
            }
        }
    }
    Ok(())
}

fn run_issues(args: &IssuesArgs, format: OutputFormat) -> Result<()> {
    if args.query.trim().is_empty() {
        anyhow::bail!("search query cannot be empty");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let result = client.search_issues(&args.query, args.state.as_deref(), 1, args.limit)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&result, fields.as_deref());
        }
        OutputFormat::Table => {
            if result.items.is_empty() {
                println!("No issues found for query: {}", args.query);
                return Ok(());
            }
            println!(
                "Showing {} of {} issues",
                result.items.len(),
                result.total_count
            );
            println!("{:<30} {:<6} {:<40} STATE", "REPO", "NUMBER", "TITLE");
            for issue in &result.items {
                println!(
                    "{}/{}\t{}\t{}\t{}",
                    issue.repository_owner,
                    issue.repository_name,
                    issue.number,
                    issue.title,
                    issue.state
                );
            }
        }
    }
    Ok(())
}

fn run_code(args: &CodeArgs, format: OutputFormat) -> Result<()> {
    if args.query.trim().is_empty() {
        anyhow::bail!("search query cannot be empty");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let result = client.search_code(&args.query, 1, args.limit)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&result, fields.as_deref());
        }
        OutputFormat::Table => {
            if result.items.is_empty() {
                println!("No code results found for query: {}", args.query);
                return Ok(());
            }
            println!(
                "Showing {} of {} results",
                result.items.len(),
                result.total_count
            );
            println!("{:<40} PATH", "REPOSITORY");
            for item in &result.items {
                println!("{}\t{}", item.repository, item.path);
                for m in &item.text_matches {
                    println!("  {m}");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repos_rejects_empty_query() {
        let args = ReposArgs {
            query: "  ".to_string(),
            limit: 30,
        };
        let err = run_repos(&args, OutputFormat::Table).expect_err("should fail on empty query");
        assert!(err.to_string().contains("query cannot be empty"));
    }

    #[test]
    fn issues_rejects_empty_query() {
        let args = IssuesArgs {
            query: "".to_string(),
            state: None,
            limit: 30,
        };
        let err = run_issues(&args, OutputFormat::Table).expect_err("should fail on empty query");
        assert!(err.to_string().contains("query cannot be empty"));
    }

    #[test]
    fn code_rejects_empty_query() {
        let args = CodeArgs {
            query: "  ".to_string(),
            limit: 30,
        };
        let err = run_code(&args, OutputFormat::Table).expect_err("should fail on empty query");
        assert!(err.to_string().contains("query cannot be empty"));
    }
}
