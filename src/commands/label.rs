use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;
use plue::types::CreateLabelInput;

#[derive(Args)]
pub struct LabelArgs {
    #[command(subcommand)]
    command: LabelCommand,
    /// Repository in OWNER/REPO format (overrides remote detection)
    #[arg(short = 'R', long = "repo", global = true)]
    repo: Option<String>,
}

#[derive(Subcommand)]
enum LabelCommand {
    /// List labels for the repository
    List(ListArgs),
    /// Create a new label
    Create(CreateArgs),
    /// Delete a label by name
    Delete(DeleteArgs),
}

#[derive(Args)]
struct ListArgs {}

#[derive(Args)]
struct CreateArgs {
    /// Label name (positional form)
    #[arg(value_name = "NAME", required_unless_present = "name_flag")]
    name: Option<String>,
    /// Label name (flag form; compatibility with legacy scripts)
    #[arg(long = "name", value_name = "NAME", conflicts_with = "name")]
    name_flag: Option<String>,
    /// Hex color code (without #), e.g. ff0000
    #[arg(short = 'c', long = "color", default_value = "0075ca")]
    color: String,
    /// Optional description
    #[arg(short = 'd', long = "description", default_value = "")]
    description: String,
}

#[derive(Args)]
struct DeleteArgs {
    /// Label name to delete
    name: String,
    /// Skip confirmation prompt
    #[arg(long = "yes", short = 'y')]
    yes: bool,
}

pub fn run(args: LabelArgs, format: OutputFormat) -> Result<()> {
    let repo_override = args.repo.as_deref();
    match args.command {
        LabelCommand::List(a) => run_list(&a, repo_override, format),
        LabelCommand::Create(a) => run_create(&a, repo_override, format),
        LabelCommand::Delete(a) => run_delete(&a, repo_override, format),
    }
}

fn run_list(_args: &ListArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let labels = client.list_labels(&repo_ref.owner, &repo_ref.repo)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&labels)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&labels, fields.as_deref());
        }
        OutputFormat::Table => {
            if labels.is_empty() {
                println!("No labels found.");
                return Ok(());
            }
            println!("{:<30} {:<8} DESCRIPTION", "NAME", "COLOR");
            for label in &labels {
                println!(
                    "{:<30} #{:<7} {}",
                    label.name, label.color, label.description
                );
            }
        }
    }
    Ok(())
}

fn run_create(args: &CreateArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let label_name = args
        .name
        .as_deref()
        .or(args.name_flag.as_deref())
        .unwrap_or_default()
        .trim();

    if label_name.is_empty() {
        anyhow::bail!("label name cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let req = CreateLabelInput {
        name: label_name.to_string(),
        color: args.color.clone(),
        description: args.description.clone(),
    };

    let created = client.create_label(&repo_ref.owner, &repo_ref.repo, &req)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&created)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&created, fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Created label '{}' (#{}) in {}/{}",
                created.name, created.color, repo_ref.owner, repo_ref.repo
            );
            if !created.description.is_empty() {
                println!("  Description: {}", created.description);
            }
        }
    }
    Ok(())
}

fn run_delete(args: &DeleteArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    if args.name.trim().is_empty() {
        anyhow::bail!("label name cannot be empty");
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    // Resolve name -> id by listing labels first
    let labels = client.list_labels(&repo_ref.owner, &repo_ref.repo)?;
    let label = labels.iter().find(|l| l.name == args.name).ok_or_else(|| {
        anyhow::anyhow!(
            "label '{}' not found in {}/{}",
            args.name,
            repo_ref.owner,
            repo_ref.repo
        )
    })?;

    client.delete_label(&repo_ref.owner, &repo_ref.repo, label.id)?;
    match format {
        OutputFormat::Json { .. } | OutputFormat::Toon { .. } => {
            // Silent for machine-readable formats
        }
        OutputFormat::Table => {
            println!(
                "Deleted label '{}' from {}/{}",
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
    fn create_rejects_empty_name() {
        let args = CreateArgs {
            name: Some("  ".to_string()),
            name_flag: None,
            color: "ff0000".to_string(),
            description: String::new(),
        };
        let err = run_create(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank name");
        assert!(err.to_string().contains("label name cannot be empty"));
    }

    #[test]
    fn delete_rejects_empty_name() {
        let args = DeleteArgs {
            name: "  ".to_string(),
            yes: true,
        };
        let err = run_delete(&args, Some("owner/repo"), OutputFormat::Table)
            .expect_err("should fail on blank name");
        assert!(err.to_string().contains("label name cannot be empty"));
    }
}
