use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::types::CreateSshKeyInput;

#[derive(Args)]
pub struct SshKeyArgs {
    #[command(subcommand)]
    command: SshKeyCommand,
}

#[derive(Subcommand)]
enum SshKeyCommand {
    /// Add an SSH public key to your account
    Add(AddArgs),
    /// List SSH keys on your account
    List(ListArgs),
    /// Delete an SSH key from your account
    Delete(DeleteArgs),
}

#[derive(Args)]
struct AddArgs {
    /// Human-readable title for the key
    #[arg(short = 't', long = "title")]
    title: String,
    /// SSH public key string (e.g. "ssh-ed25519 AAAA...")
    #[arg(short = 'k', long = "key")]
    key: String,
}

#[derive(Args)]
struct ListArgs {}

#[derive(Args)]
struct DeleteArgs {
    /// ID of the SSH key to delete
    id: i64,
}

pub fn run(args: SshKeyArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        SshKeyCommand::Add(a) => run_add(&a, format),
        SshKeyCommand::List(a) => run_list(&a, format),
        SshKeyCommand::Delete(a) => run_delete(&a, format),
    }
}

fn run_add(args: &AddArgs, format: OutputFormat) -> Result<()> {
    if args.title.trim().is_empty() {
        anyhow::bail!("title cannot be empty");
    }
    if args.key.trim().is_empty() {
        anyhow::bail!("key cannot be empty");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let req = CreateSshKeyInput {
        title: args.title.clone(),
        key: args.key.clone(),
    };

    let created = client.add_ssh_key(&req)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&created)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&created, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Added SSH key: {} (id: {})", created.name, created.id);
            println!("  Type: {}", created.key_type);
            println!("  Fingerprint: {}", created.fingerprint);
        }
    }
    Ok(())
}

fn run_list(_args: &ListArgs, format: OutputFormat) -> Result<()> {
    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let keys = client.list_ssh_keys()?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&keys)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&keys, fields.as_deref());
        }
        OutputFormat::Table => {
            if keys.is_empty() {
                println!("No SSH keys registered.");
                return Ok(());
            }
            println!("ID\tNAME\tTYPE\tCREATED");
            for key in &keys {
                println!(
                    "{}\t{}\t{}\t{}",
                    key.id, key.name, key.key_type, key.created_at
                );
            }
        }
    }
    Ok(())
}

fn run_delete(args: &DeleteArgs, _format: OutputFormat) -> Result<()> {
    if args.id <= 0 {
        anyhow::bail!("invalid SSH key id: {}", args.id);
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    client.delete_ssh_key(args.id)?;
    println!("Deleted SSH key {}", args.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_args_reject_empty_title() {
        let args = AddArgs {
            title: "   ".to_string(),
            key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI test".to_string(),
        };
        let err = run_add(&args, OutputFormat::Table).expect_err("should fail on blank title");
        assert!(err.to_string().contains("title cannot be empty"));
    }

    #[test]
    fn add_args_reject_empty_key() {
        let args = AddArgs {
            title: "laptop".to_string(),
            key: "  ".to_string(),
        };
        let err = run_add(&args, OutputFormat::Table).expect_err("should fail on blank key");
        assert!(err.to_string().contains("key cannot be empty"));
    }

    #[test]
    fn delete_rejects_non_positive_id() {
        let args = DeleteArgs { id: 0 };
        let err = run_delete(&args, OutputFormat::Table).expect_err("should fail on id=0");
        assert!(err.to_string().contains("invalid SSH key id"));
    }

    #[test]
    fn delete_rejects_negative_id() {
        let args = DeleteArgs { id: -1 };
        let err = run_delete(&args, OutputFormat::Table).expect_err("should fail on id=-1");
        assert!(err.to_string().contains("invalid SSH key id"));
    }
}
