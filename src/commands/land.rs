use std::collections::HashSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::{ApiClient, ApiError, CreateLandingRequestInput, CreateLandingReviewInput};
use plue::jj_ops::{JjWorkspaceOps, WorkspaceOps};
use plue::output::print_toon;
use plue::repo_context::{resolve_repo_ref, RepoRef};
use plue::types::CommitStatusResponse;

#[derive(Args)]
pub struct LandArgs {
    #[command(subcommand)]
    command: LandCommand,
}

#[derive(Subcommand)]
enum LandCommand {
    /// Create a landing request
    Create(CreateArgs),
    /// List landing requests
    List(ListArgs),
    /// View a landing request
    View(ViewArgs),
    /// Review a landing request
    Review(ReviewArgs),
    /// Merge a landing request
    Merge(MergeArgs),
    /// View checks for a landing request
    Checks(ChecksArgs),
}

#[derive(Args)]
struct CreateArgs {
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
    /// Landing request title
    #[arg(short = 't', long = "title")]
    title: Option<String>,
    /// Landing request body
    #[arg(short = 'b', long = "body")]
    body: Option<String>,
    /// Target bookmark
    #[arg(long = "target", alias = "target-bookmark", default_value = "main")]
    target: String,
    /// Explicit change ID(s)
    #[arg(long = "change-id")]
    change_ids: Vec<String>,
    /// Include detected stack change IDs
    #[arg(long)]
    stack: bool,
}

#[derive(Args)]
struct ListArgs {
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
    /// State filter (open|closed|merged|draft|all)
    #[arg(short = 's', long = "state", default_value = "open")]
    state: String,
    /// Max items to fetch
    #[arg(short = 'L', long = "limit", default_value_t = 30)]
    limit: i32,
    /// Comma-separated JSON field projection when used with --json
    json_fields: Option<String>,
}

#[derive(Args)]
struct ViewArgs {
    /// Landing request number
    number: i64,
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
}

#[derive(Args)]
struct ReviewArgs {
    /// Landing request number
    number: i64,
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
    /// Approve the landing request
    #[arg(short = 'a', long = "approve")]
    approve: bool,
    /// Request changes
    #[arg(short = 'r', long = "request-changes")]
    request_changes: bool,
    /// Comment-only review
    #[arg(short = 'c', long = "comment")]
    comment: bool,
    /// Review body
    #[arg(short = 'b', long = "body")]
    body: Option<String>,
}

#[derive(Args)]
struct MergeArgs {
    /// Landing request number
    number: i64,
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
}

#[derive(Args)]
struct ChecksArgs {
    /// Landing request number
    number: i64,
    /// Repository in owner/repo format
    #[arg(short = 'R', long = "repo")]
    repo: Option<String>,
}

pub fn run(args: LandArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        LandCommand::Create(args) => run_create(&args, format),
        LandCommand::List(args) => run_list(&args, format),
        LandCommand::View(args) => run_view(&args, format),
        LandCommand::Review(args) => run_review(&args, format),
        LandCommand::Merge(args) => run_merge(&args, format),
        LandCommand::Checks(args) => run_checks(&args, format),
    }
}

fn run_create(args: &CreateArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;

    let title = args
        .title
        .clone()
        .or_else(|| detect_current_change_title(&cwd).ok())
        .unwrap_or_else(|| "Landing request".to_string());
    let change_ids = detect_change_ids(&cwd, &args.change_ids, args.stack)?;
    let req = CreateLandingRequestInput {
        title,
        body: args.body.clone().unwrap_or_default(),
        target_bookmark: args.target.clone(),
        change_ids,
    };

    let created = client.create_landing_request(&repo.owner, &repo.repo, &req)?;
    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&created)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&created, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Created landing request #{}", created.number);
            println!(
                "  URL: /{}/{}/landings/{}",
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

    let items =
        client.list_landing_requests(&repo.owner, &repo.repo, state.as_deref(), 1, args.limit)?;

    match format {
        OutputFormat::Json { ref fields } => {
            let as_values = items
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            // Prefer positional json_fields arg, then --json=fields value
            let effective_fields = args
                .json_fields
                .as_deref()
                .or(fields.as_deref())
                .filter(|f| !f.is_empty());
            let projected = project_list_json_fields(as_values, effective_fields)?;
            println!("{}", serde_json::to_string_pretty(&projected)?);
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&items, fields.as_deref());
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No landing requests found.");
                return Ok(());
            }
            println!("NUMBER\tTITLE\tAUTHOR\tSTATE\tSTACK_SIZE\tCHANGE_IDS");
            for item in items {
                let change_ids = if item.change_ids.is_empty() {
                    "-".to_string()
                } else {
                    item.change_ids.join(",")
                };
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    item.number,
                    item.title,
                    item.author.login,
                    item.state,
                    item.stack_size,
                    change_ids
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

    let landing = client.get_landing_request(&repo.owner, &repo.repo, args.number)?;
    let changes = client.list_landing_changes(&repo.owner, &repo.repo, args.number)?;
    let reviews = client.list_landing_reviews(&repo.owner, &repo.repo, args.number)?;
    let conflicts = client.get_landing_conflicts(&repo.owner, &repo.repo, args.number)?;

    match format {
        OutputFormat::Json { .. } => {
            let payload = serde_json::json!({
                "landing": landing,
                "changes": changes,
                "reviews": reviews,
                "conflicts": conflicts,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        OutputFormat::Toon { ref fields } => {
            let payload = serde_json::json!({
                "landing": landing,
                "changes": changes,
                "reviews": reviews,
                "conflicts": conflicts,
            });
            print_toon(&payload, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("#{} {}", landing.number, landing.title);
            println!("State: {}", landing.state);
            println!("Author: {}", landing.author.login);
            println!("Target: {}", landing.target_bookmark);
            println!("Conflict status: {}", landing.conflict_status);
            println!("Changes:");
            for change in &changes {
                println!(
                    "  - {} (position {})",
                    change.change_id, change.position_in_stack
                );
            }
            println!("Reviews:");
            for review in &reviews {
                println!(
                    "  - {}: {}",
                    review.review_type,
                    if review.body.is_empty() {
                        "(no body)"
                    } else {
                        &review.body
                    }
                );
            }
            if conflicts.has_conflicts {
                println!("Conflicts:");
                for (change_id, entries) in &conflicts.conflicts_by_change {
                    for entry in entries {
                        println!(
                            "  - {}: {} ({})",
                            change_id, entry.file_path, entry.conflict_type
                        );
                    }
                }
            } else {
                println!("Conflicts: none");
            }
        }
    }
    Ok(())
}

fn run_review(args: &ReviewArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;
    let mode = validate_review_mode(args.approve, args.request_changes, args.comment)?;

    let created = client.create_landing_review(
        &repo.owner,
        &repo.repo,
        args.number,
        &CreateLandingReviewInput {
            review_type: mode.to_string(),
            body: args.body.clone().unwrap_or_default(),
        },
    )?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&created)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&created, fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Submitted {} review on landing #{}",
                created.review_type, args.number
            );
        }
    }
    Ok(())
}

fn run_merge(args: &MergeArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;
    let merged = match client.land_landing_request(&repo.owner, &repo.repo, args.number) {
        Ok(merged) => merged,
        Err(err) => {
            if let Some(api_err) = err.downcast_ref::<ApiError>() {
                match api_err.status {
                    404 => bail!("landing request #{} was not found", args.number),
                    409 => {
                        let details = api_err.message.trim();
                        if details.is_empty() {
                            bail!(
                                "landing request #{} cannot be merged right now",
                                args.number
                            );
                        }
                        bail!(
                            "landing request #{} cannot be merged right now: {}",
                            args.number,
                            details
                        );
                    }
                    _ => {}
                }
            }
            return Err(err);
        }
    };

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&merged)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&merged, fields.as_deref());
        }
        OutputFormat::Table => {
            if merged.state.eq_ignore_ascii_case("queued") {
                println!("Landing request #{} queued for merge", merged.number);
            } else {
                println!("Landing request #{} merged", merged.number);
            }
        }
    }
    Ok(())
}

fn run_checks(args: &ChecksArgs, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let repo = resolve_repo_ref(&cwd, args.repo.as_deref())?;
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;
    let landing = client.get_landing_request(&repo.owner, &repo.repo, args.number)?;

    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut flat_statuses: Vec<CommitStatusResponse> = Vec::new();
    let mut placeholder = false;

    for change_id in &landing.change_ids {
        match client.list_commit_statuses(&repo.owner, &repo.repo, change_id) {
            Ok(statuses) => {
                for status in statuses {
                    rows.push(serde_json::json!({
                        "change_id": change_id,
                        "context": status.context,
                        "status": status.status,
                        "description": status.description,
                        "target_url": status.target_url,
                    }));
                    flat_statuses.push(status);
                }
            }
            Err(err) => {
                if let Some(api_err) = err.downcast_ref::<ApiError>() {
                    if api_err.status == 404 {
                        placeholder = true;
                        continue;
                    }
                }
                return Err(err);
            }
        }
    }

    match format {
        OutputFormat::Json { .. } => {
            if placeholder && rows.is_empty() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!([{"message":"checks endpoint not available"}])
                    )?
                );
                return Ok(());
            }
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
        OutputFormat::Toon { ref fields } => {
            if placeholder && rows.is_empty() {
                print_toon(
                    &serde_json::json!([{"message":"checks endpoint not available"}]),
                    fields.as_deref(),
                );
                return Ok(());
            }
            print_toon(&rows, fields.as_deref());
        }
        OutputFormat::Table => {
            if placeholder && flat_statuses.is_empty() {
                println!("Checks endpoint not available.");
                return Ok(());
            }
            if flat_statuses.is_empty() {
                println!("No checks reported.");
                return Ok(());
            }
            println!("CONTEXT\tSTATUS\tDESCRIPTION");
            for status in flat_statuses {
                println!(
                    "{}\t{}\t{}",
                    status.context, status.status, status.description
                );
            }
        }
    }

    Ok(())
}

fn detect_change_ids(cwd: &Path, explicit: &[String], stack: bool) -> Result<Vec<String>> {
    if !explicit.is_empty() {
        return Ok(explicit.to_vec());
    }

    let ops = JjWorkspaceOps::open(cwd)?;
    let status = ops.get_status()?;
    let mut ids = vec![status.working_copy.change_id];
    if stack {
        let mut seen: HashSet<String> = ids.iter().cloned().collect();
        for change in ops.list_working_copy_lineage(100)? {
            if !change.is_empty && seen.insert(change.change_id.clone()) {
                ids.push(change.change_id);
            }
        }
    }
    Ok(ids)
}

fn detect_current_change_title(cwd: &Path) -> Result<String> {
    let ops = open_workspace(cwd)?;
    let status = ops.get_status()?;
    let title = status.working_copy.description.trim();
    if title.is_empty() {
        bail!("working copy change has empty description")
    }
    Ok(title.to_string())
}

fn open_workspace(cwd: &Path) -> Result<Box<dyn WorkspaceOps>> {
    let ops = JjWorkspaceOps::open(cwd)?;
    Ok(Box::new(ops))
}

fn validate_review_mode(
    approve: bool,
    request_changes: bool,
    comment: bool,
) -> Result<&'static str> {
    let modes = approve as i32 + request_changes as i32 + comment as i32;
    if modes != 1 {
        bail!("exactly one of --approve, --request-changes, or --comment is required");
    }
    if approve {
        Ok("approve")
    } else if request_changes {
        Ok("request_changes")
    } else {
        Ok("comment")
    }
}

fn validate_list_state(state: &str) -> Result<Option<String>> {
    let normalized = state.trim().to_lowercase();
    match normalized.as_str() {
        "open" | "closed" | "merged" | "draft" => Ok(Some(normalized)),
        "all" => Ok(None),
        _ => bail!("invalid state: {state} (expected open|closed|merged|draft|all)"),
    }
}

fn project_list_json_fields(
    rows: Vec<serde_json::Value>,
    json_fields: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let Some(fields) = json_fields else {
        return Ok(rows);
    };

    let wanted: Vec<String> = fields
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if wanted.is_empty() {
        return Ok(rows);
    }

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(obj) = row.as_object() else {
            out.push(row);
            continue;
        };
        let mut projected = serde_json::Map::new();
        for key in &wanted {
            if let Some(value) = obj.get(key) {
                projected.insert(key.clone(), value.clone());
            }
        }
        out.push(serde_json::Value::Object(projected));
    }

    Ok(out)
}

#[allow(dead_code)]
fn _repo_to_string(repo: &RepoRef) -> String {
    format!("{}/{}", repo.owner, repo.repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_flags_require_exactly_one_mode() {
        let err = validate_review_mode(false, false, false).expect_err("should fail with no mode");
        assert!(err.to_string().contains("exactly one"));

        let err = validate_review_mode(true, true, false).expect_err("should fail with many modes");
        assert!(err.to_string().contains("exactly one"));
    }

    #[test]
    fn list_state_validation_rejects_unknown_states() {
        let err = validate_list_state("invalid").expect_err("invalid state should fail");
        assert!(err.to_string().contains("state"));
    }

    #[test]
    fn list_json_projection_keeps_requested_keys() {
        let rows = vec![serde_json::json!({
            "number": 1,
            "title": "Demo",
            "state": "open",
            "body": "ignore",
        })];
        let projected =
            project_list_json_fields(rows, Some("number,title,state")).expect("projection");
        assert_eq!(projected[0]["number"], 1);
        assert_eq!(projected[0]["title"], "Demo");
        assert_eq!(projected[0]["state"], "open");
        assert!(projected[0].get("body").is_none());
    }
}
