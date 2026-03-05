use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use reqwest::blocking::Client;
use serde::Serialize;

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::types::{BetaWaitlistEntry, BetaWaitlistListResponse, BetaWhitelistEntry};

#[derive(Args)]
pub struct BetaArgs {
    #[command(subcommand)]
    command: BetaCommand,
}

#[derive(Subcommand)]
enum BetaCommand {
    /// Manage waitlist entries
    Waitlist(WaitlistArgs),
    /// Manage whitelist entries
    Whitelist(WhitelistArgs),
}

#[derive(Args)]
struct WaitlistArgs {
    #[command(subcommand)]
    command: WaitlistCommand,
}

#[derive(Subcommand)]
enum WaitlistCommand {
    /// Join the closed beta waitlist (no authentication required)
    Join(WaitlistJoinArgs),
    /// List waitlist entries (admin)
    List(WaitlistListArgs),
    /// Approve a waitlist entry by email (admin)
    Approve(WaitlistApproveArgs),
}

#[derive(Args)]
struct WaitlistJoinArgs {
    /// Email to submit to the waitlist
    #[arg(long)]
    email: String,
    /// Optional note for admins
    #[arg(long)]
    note: Option<String>,
    /// Source tag (defaults to "cli")
    #[arg(long, default_value = "cli")]
    source: String,
}

#[derive(Args)]
struct WaitlistListArgs {
    /// Filter by status (pending, approved, rejected)
    #[arg(long)]
    status: Option<String>,
    /// Page number (1-based)
    #[arg(long, default_value_t = 1)]
    page: i32,
    /// Results per page
    #[arg(long = "per-page", default_value_t = 50)]
    per_page: i32,
}

#[derive(Args)]
struct WaitlistApproveArgs {
    /// Waitlist email to approve
    #[arg(long)]
    email: String,
}

#[derive(Args)]
struct WhitelistArgs {
    #[command(subcommand)]
    command: WhitelistCommand,
}

#[derive(Subcommand)]
enum WhitelistCommand {
    /// Add or update a whitelist entry (admin)
    Add(WhitelistAddArgs),
    /// List whitelist entries (admin)
    List,
    /// Remove a whitelist entry (admin)
    Remove(WhitelistRemoveArgs),
}

#[derive(Args)]
struct WhitelistAddArgs {
    /// Identity type: email, wallet, username
    #[arg(long = "type")]
    identity_type: String,
    /// Identity value for the selected type
    #[arg(long = "value")]
    identity_value: String,
}

#[derive(Args)]
struct WhitelistRemoveArgs {
    /// Identity type: email, wallet, username
    #[arg(long = "type")]
    identity_type: String,
    /// Identity value for the selected type
    #[arg(long = "value")]
    identity_value: String,
}

#[derive(Serialize)]
struct WaitlistJoinRequest {
    email: String,
    note: String,
    source: String,
}

#[derive(serde::Deserialize)]
struct ErrorBody {
    message: Option<String>,
    error: Option<String>,
}

pub fn run(args: BetaArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        BetaCommand::Waitlist(waitlist_args) => run_waitlist(waitlist_args, format),
        BetaCommand::Whitelist(whitelist_args) => run_whitelist(whitelist_args, format),
    }
}

fn run_waitlist(args: WaitlistArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        WaitlistCommand::Join(join_args) => run_waitlist_join(join_args, format),
        WaitlistCommand::List(list_args) => run_waitlist_list(list_args, format),
        WaitlistCommand::Approve(approve_args) => run_waitlist_approve(approve_args, format),
    }
}

fn run_whitelist(args: WhitelistArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        WhitelistCommand::Add(add_args) => run_whitelist_add(add_args, format),
        WhitelistCommand::List => run_whitelist_list(format),
        WhitelistCommand::Remove(remove_args) => run_whitelist_remove(remove_args, format),
    }
}

fn run_waitlist_join(args: WaitlistJoinArgs, format: OutputFormat) -> Result<()> {
    if args.email.trim().is_empty() {
        bail!("--email is required");
    }

    let config = Config::load().context("failed to load config")?;
    let base_url = config.api_url.trim_end_matches('/');
    let url = format!("{base_url}/beta/waitlist");
    let payload = WaitlistJoinRequest {
        email: args.email.trim().to_string(),
        note: args.note.unwrap_or_default(),
        source: args.source.trim().to_string(),
    };

    let response = Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .context("failed to connect to Plue API")?;

    if !response.status().is_success() {
        return Err(anyhow!(render_http_error(response)));
    }

    let entry: BetaWaitlistEntry = response
        .json()
        .context("failed to parse waitlist join response")?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&entry)?),
        OutputFormat::Toon { ref fields } => print_toon(&entry, fields.as_deref()),
        OutputFormat::Table => {
            println!("Waitlist request submitted for {}", entry.email);
            println!("Status: {}", entry.status);
        }
    }

    Ok(())
}

fn run_waitlist_list(args: WaitlistListArgs, format: OutputFormat) -> Result<()> {
    if let Some(status) = args.status.as_deref() {
        validate_waitlist_status(status)?;
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;
    let page = client.list_beta_waitlist(args.status.as_deref(), args.page, args.per_page)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&page)?),
        OutputFormat::Toon { ref fields } => print_toon(&page, fields.as_deref()),
        OutputFormat::Table => print_waitlist_table(&page),
    }

    Ok(())
}

fn run_waitlist_approve(args: WaitlistApproveArgs, format: OutputFormat) -> Result<()> {
    if args.email.trim().is_empty() {
        bail!("--email is required");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;
    let entry = client.approve_beta_waitlist_entry(args.email.trim())?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&entry)?),
        OutputFormat::Toon { ref fields } => print_toon(&entry, fields.as_deref()),
        OutputFormat::Table => {
            println!("Approved waitlist entry for {}", entry.email);
        }
    }

    Ok(())
}

fn run_whitelist_add(args: WhitelistAddArgs, format: OutputFormat) -> Result<()> {
    let identity_type = normalize_identity_type(&args.identity_type)?;
    if args.identity_value.trim().is_empty() {
        bail!("--value is required");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;
    let entry = client.add_beta_whitelist_entry(&identity_type, args.identity_value.trim())?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&entry)?),
        OutputFormat::Toon { ref fields } => print_toon(&entry, fields.as_deref()),
        OutputFormat::Table => {
            println!(
                "Whitelisted {} {}",
                entry.identity_type, entry.identity_value
            );
        }
    }

    Ok(())
}

fn run_whitelist_list(format: OutputFormat) -> Result<()> {
    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;
    let entries = client.list_beta_whitelist()?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&entries)?),
        OutputFormat::Toon { ref fields } => print_toon(&entries, fields.as_deref()),
        OutputFormat::Table => print_whitelist_table(&entries),
    }

    Ok(())
}

fn run_whitelist_remove(args: WhitelistRemoveArgs, format: OutputFormat) -> Result<()> {
    let identity_type = normalize_identity_type(&args.identity_type)?;
    if args.identity_value.trim().is_empty() {
        bail!("--value is required");
    }

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;
    client.remove_beta_whitelist_entry(&identity_type, args.identity_value.trim())?;

    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::json!({
                    "removed": true,
                    "identity_type": identity_type,
                    "identity_value": args.identity_value.trim(),
                })
            );
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(
                &serde_json::json!({
                    "removed": true,
                    "identity_type": identity_type,
                    "identity_value": args.identity_value.trim(),
                }),
                fields.as_deref(),
            );
        }
        OutputFormat::Table => {
            println!(
                "Removed whitelist entry {} {}",
                identity_type,
                args.identity_value.trim()
            );
        }
    }

    Ok(())
}

fn print_whitelist_table(entries: &[BetaWhitelistEntry]) {
    if entries.is_empty() {
        println!("No whitelist entries.");
        return;
    }
    println!("TYPE\tVALUE\tCREATED");
    for entry in entries {
        println!(
            "{}\t{}\t{}",
            entry.identity_type, entry.identity_value, entry.created_at
        );
    }
}

fn print_waitlist_table(page: &BetaWaitlistListResponse) {
    if page.items.is_empty() {
        println!("No waitlist entries.");
        return;
    }
    println!("EMAIL\tSTATUS\tSOURCE\tCREATED");
    for entry in &page.items {
        println!(
            "{}\t{}\t{}\t{}",
            entry.email, entry.status, entry.source, entry.created_at
        );
    }
    println!("Total: {}", page.total_count);
}

fn normalize_identity_type(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_lowercase();
    match normalized.as_str() {
        "email" => Ok("email".to_string()),
        "wallet" => Ok("wallet".to_string()),
        "username" => Ok("username".to_string()),
        _ => bail!("--type must be one of: email, wallet, username"),
    }
}

fn validate_waitlist_status(raw: &str) -> Result<()> {
    match raw.trim().to_lowercase().as_str() {
        "pending" | "approved" | "rejected" => Ok(()),
        _ => bail!("--status must be one of: pending, approved, rejected"),
    }
}

fn render_http_error(response: reqwest::blocking::Response) -> String {
    let status = response.status().as_u16();
    let fallback = format!("API {} request failed", status);
    let body = match response.text() {
        Ok(text) => text,
        Err(_) => return fallback,
    };

    if let Ok(parsed) = serde_json::from_str::<ErrorBody>(&body) {
        if let Some(message) = parsed.message.or(parsed.error) {
            return format!("API {}: {}", status, message);
        }
    }

    if body.trim().is_empty() {
        fallback
    } else {
        format!("API {}: {}", status, body.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_identity_type_accepts_known_values() {
        assert_eq!(normalize_identity_type("email").expect("email"), "email");
        assert_eq!(normalize_identity_type("wallet").expect("wallet"), "wallet");
        assert_eq!(
            normalize_identity_type("username").expect("username"),
            "username"
        );
    }

    #[test]
    fn normalize_identity_type_rejects_unknown_values() {
        let err = normalize_identity_type("org").expect_err("invalid type should fail");
        assert!(err.to_string().contains("email, wallet, username"));
    }

    #[test]
    fn validate_waitlist_status_rejects_unknown_values() {
        let err = validate_waitlist_status("queued").expect_err("invalid status should fail");
        assert!(err.to_string().contains("pending, approved, rejected"));
    }
}
