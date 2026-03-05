use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::{ApiClient, ApiError};
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;
use plue::types::SetVariableInput;

#[derive(Args)]
pub struct VariableArgs {
    #[command(subcommand)]
    command: VariableCommand,
    /// Repository in OWNER/REPO format (overrides remote detection)
    #[arg(short = 'R', long = "repo", global = true)]
    repo: Option<String>,
}

#[derive(Subcommand)]
enum VariableCommand {
    /// Set a variable
    Set(SetArgs),
    /// Get a variable value
    Get(GetArgs),
    /// List variables
    List(ListArgs),
    /// Delete a variable
    Delete(DeleteArgs),
}

#[derive(Args)]
struct SetArgs {
    /// Variable name
    name: String,
    /// Variable value
    #[arg(short = 'b', long = "body")]
    body: String,
}

#[derive(Args)]
struct GetArgs {
    /// Variable name
    name: String,
}

#[derive(Args)]
struct ListArgs {}

#[derive(Args)]
struct DeleteArgs {
    /// Variable name to delete
    name: String,
}

pub fn run(args: VariableArgs, format: OutputFormat) -> Result<()> {
    let repo_override = args.repo.as_deref();
    match args.command {
        VariableCommand::Set(a) => run_set(&a, repo_override, format),
        VariableCommand::Get(a) => run_get(&a, repo_override, format),
        VariableCommand::List(a) => run_list(&a, repo_override, format),
        VariableCommand::Delete(a) => run_delete(&a, repo_override, format),
    }
}

fn run_set(args: &SetArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        anyhow::bail!("variable name cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let input = SetVariableInput {
        name: args.name.clone(),
        value: args.body.clone(),
    };
    let variable = client.set_variable(&repo_ref.owner, &repo_ref.repo, &input)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&variable)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&variable, fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Set variable '{}' in {}/{}",
                variable.name, repo_ref.owner, repo_ref.repo
            );
        }
    }
    Ok(())
}

fn run_get(args: &GetArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        anyhow::bail!("variable name cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let variable = client.get_variable(&repo_ref.owner, &repo_ref.repo, &args.name)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&variable)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&variable, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("{}", variable.value);
        }
    }
    Ok(())
}

fn run_list(_args: &ListArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let variables = match client.list_variables(&repo_ref.owner, &repo_ref.repo) {
        Ok(variables) => variables,
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
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&variables)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&variables, fields.as_deref());
        }
        OutputFormat::Table => {
            if variables.is_empty() {
                println!("No variables found.");
                return Ok(());
            }
            println!("{:<30} {:<40} UPDATED", "NAME", "VALUE");
            for v in &variables {
                println!("{:<30} {:<40} {}", v.name, v.value, v.updated_at);
            }
        }
    }
    Ok(())
}

fn run_delete(args: &DeleteArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        anyhow::bail!("variable name cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    client.delete_variable(&repo_ref.owner, &repo_ref.repo, &args.name)?;
    match format {
        OutputFormat::Json { .. } => {}
        _ => {
            println!(
                "Deleted variable '{}' from {}/{}",
                args.name, repo_ref.owner, repo_ref.repo
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_rejects_empty_name() {
        let args = SetArgs {
            name: "  ".to_string(),
            body: "value".to_string(),
        };
        let err = run_set(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank name");
        assert!(err.to_string().contains("variable name cannot be empty"));
    }

    #[test]
    fn get_rejects_empty_name() {
        let args = GetArgs {
            name: "".to_string(),
        };
        let err = run_get(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank name");
        assert!(err.to_string().contains("variable name cannot be empty"));
    }

    #[test]
    fn delete_rejects_empty_name() {
        let args = DeleteArgs {
            name: "  ".to_string(),
        };
        let err = run_delete(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank name");
        assert!(err.to_string().contains("variable name cannot be empty"));
    }
}
