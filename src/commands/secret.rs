use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::{ApiClient, ApiError};
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;
use plue::types::SetSecretInput;

#[derive(Args)]
pub struct SecretArgs {
    #[command(subcommand)]
    command: SecretCommand,
    /// Repository in OWNER/REPO format (overrides remote detection)
    #[arg(short = 'R', long = "repo", global = true)]
    repo: Option<String>,
}

#[derive(Subcommand)]
enum SecretCommand {
    /// Set a secret
    Set(SetArgs),
    /// List secrets
    List(ListArgs),
    /// Delete a secret
    Delete(DeleteArgs),
}

#[derive(Args)]
struct SetArgs {
    /// Secret name
    name: String,
    /// Secret value
    #[arg(short = 'b', long = "body")]
    body: String,
}

#[derive(Args)]
struct ListArgs {}

#[derive(Args)]
struct DeleteArgs {
    /// Secret name to delete
    name: String,
}

pub fn run(args: SecretArgs, format: OutputFormat) -> Result<()> {
    let repo_override = args.repo.as_deref();
    match args.command {
        SecretCommand::Set(a) => run_set(&a, repo_override, format),
        SecretCommand::List(a) => run_list(&a, repo_override, format),
        SecretCommand::Delete(a) => run_delete(&a, repo_override, format),
    }
}

fn run_set(args: &SetArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        anyhow::bail!("secret name cannot be empty");
    }
    if args.body.is_empty() {
        anyhow::bail!("secret value cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let input = SetSecretInput {
        name: args.name.clone(),
        value: args.body.clone(),
    };
    let secret = client.set_secret(&repo_ref.owner, &repo_ref.repo, &input)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&secret)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&secret, fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Set secret '{}' in {}/{}",
                secret.name, repo_ref.owner, repo_ref.repo
            );
        }
    }
    Ok(())
}

fn run_list(_args: &ListArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let secrets = match client.list_secrets(&repo_ref.owner, &repo_ref.repo) {
        Ok(secrets) => secrets,
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
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&secrets)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&secrets, fields.as_deref());
        }
        OutputFormat::Table => {
            if secrets.is_empty() {
                println!("No secrets found.");
                return Ok(());
            }
            println!("{:<30} UPDATED", "NAME");
            for s in &secrets {
                println!("{:<30} {}", s.name, s.updated_at);
            }
        }
    }
    Ok(())
}

fn run_delete(args: &DeleteArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        anyhow::bail!("secret name cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    client.delete_secret(&repo_ref.owner, &repo_ref.repo, &args.name)?;
    match format {
        OutputFormat::Json { .. } => {}
        _ => {
            println!(
                "Deleted secret '{}' from {}/{}",
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
        assert!(err.to_string().contains("secret name cannot be empty"));
    }

    #[test]
    fn set_rejects_empty_value() {
        let args = SetArgs {
            name: "MY_SECRET".to_string(),
            body: String::new(),
        };
        let err = run_set(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank value");
        assert!(err.to_string().contains("secret value cannot be empty"));
    }

    #[test]
    fn delete_rejects_empty_name() {
        let args = DeleteArgs {
            name: "  ".to_string(),
        };
        let err = run_delete(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank name");
        assert!(err.to_string().contains("secret name cannot be empty"));
    }
}
