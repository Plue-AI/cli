mod commands;

// Re-export lib modules for use by commands
pub use plue::config;
pub use plue::credential_store;
pub use plue::jj_ops;
pub use plue::output;
pub use plue::types;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use commands::Commands;
use output::OutputArgs;

/// CLI for Plue — a jj-native code collaboration platform
#[derive(Parser)]
#[command(name = "plue", version, about)]
pub struct Cli {
    #[command(flatten)]
    output: OutputArgs,

    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    // Use try_parse so we can intercept --help and apply --toon/--json formatting.
    match Cli::try_parse() {
        Ok(cli) => {
            let format = cli.output.format();
            let result = commands::run(cli.command, format);
            if let Err(ref e) = result {
                // Best-effort error reporting — never interfere with normal error display.
                if let Ok(cfg) = plue::config::Config::load() {
                    plue::telemetry::report_error(
                        &cfg,
                        e,
                        &std::env::args().collect::<Vec<_>>().join(" "),
                    );
                }
            }
            result
        }
        Err(e)
            if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion =>
        {
            let args: Vec<String> = std::env::args().collect();
            let has_toon = args
                .iter()
                .any(|a| a == "--toon" || a.starts_with("--toon="));
            let has_json = args
                .iter()
                .any(|a| a == "--json" || a.starts_with("--json="));

            if (has_toon || has_json) && e.kind() == clap::error::ErrorKind::DisplayHelp {
                let cmd = Cli::command();
                let target = find_target_command(&cmd, &args);
                let help_data = build_help_data(target);
                if has_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&help_data).expect("failed to serialize help")
                    );
                } else {
                    plue::output::print_toon(&help_data, None);
                }
                Ok(())
            } else {
                e.exit()
            }
        }
        Err(e) => e.exit(),
    }
}

/// Walk the subcommand tree to find the command matching the user's args.
fn find_target_command<'a>(cmd: &'a clap::Command, args: &[String]) -> &'a clap::Command {
    let mut current = cmd;
    // Skip program name (args[0]) and any flags.
    for arg in args.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        if let Some(sub) = current.find_subcommand(arg) {
            current = sub;
        }
    }
    current
}

/// Build a JSON value representing a command's help information.
fn build_help_data(cmd: &clap::Command) -> serde_json::Value {
    let subcommands: Vec<serde_json::Value> = cmd
        .get_subcommands()
        .filter(|sc| sc.get_name() != "help")
        .map(|sc| {
            serde_json::json!({
                "name": sc.get_name(),
                "about": sc.get_about().map(|s| s.to_string()).unwrap_or_default(),
            })
        })
        .collect();

    let options: Vec<serde_json::Value> = cmd
        .get_arguments()
        .filter(|a| !a.is_positional())
        .map(|a| {
            let long = a.get_long().map(|l| format!("--{l}"));
            let short = a.get_short().map(|s| format!("-{s}"));
            let flag = long.or(short).unwrap_or_default();
            let mut opt = serde_json::json!({ "flag": flag });
            if let Some(help) = a.get_help() {
                opt["about"] = serde_json::json!(help.to_string());
            }
            opt
        })
        .collect();

    let positionals: Vec<serde_json::Value> = cmd
        .get_arguments()
        .filter(|a| a.is_positional())
        .map(|a| {
            let mut pos = serde_json::json!({ "name": a.get_id().as_str() });
            if let Some(help) = a.get_help() {
                pos["about"] = serde_json::json!(help.to_string());
            }
            pos
        })
        .collect();

    let mut data = serde_json::json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(|s| s.to_string()).unwrap_or_default(),
    });

    if !subcommands.is_empty() {
        data["subcommands"] = serde_json::json!(subcommands);
    }
    if !positionals.is_empty() {
        data["arguments"] = serde_json::json!(positionals);
    }
    if !options.is_empty() {
        data["options"] = serde_json::json!(options);
    }

    data
}
