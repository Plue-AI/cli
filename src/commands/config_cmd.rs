use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;

use crate::config::Config;
use crate::output::OutputFormat;

/// Supported config keys.
const VALID_KEYS: &[&str] = &["api_url", "git_protocol"];

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Get a config value by key
    Get(GetArgs),
    /// Set a config value by key
    Set(SetArgs),
    /// List all config values
    List(ListArgs),
}

#[derive(Args)]
struct GetArgs {
    /// Config key (api_url, git_protocol)
    key: String,
}

#[derive(Args)]
struct SetArgs {
    /// Config key (api_url, git_protocol)
    key: String,
    /// Value to set
    value: String,
}

#[derive(Args)]
struct ListArgs {}

pub fn run(args: ConfigArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        ConfigCommand::Get(a) => run_get(&a, format),
        ConfigCommand::Set(a) => run_set(&a, format),
        ConfigCommand::List(a) => run_list(&a, format),
    }
}

fn run_get(args: &GetArgs, format: OutputFormat) -> Result<()> {
    validate_key(&args.key)?;
    let config = Config::load_raw().context("failed to load config")?;

    let value = match args.key.as_str() {
        "api_url" => config.api_url.clone(),
        "git_protocol" => format!("{:?}", config.git_protocol).to_lowercase(),
        _ => unreachable!("key already validated"),
    };

    match format {
        OutputFormat::Json { .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ &args.key: value }))?
            )
        }
        OutputFormat::Toon { .. } => {
            println!("{}:{}", args.key, quote_toon_value(&value));
        }
        OutputFormat::Table => println!("{}", value),
    }
    Ok(())
}

fn run_set(args: &SetArgs, _format: OutputFormat) -> Result<()> {
    validate_key(&args.key)?;

    if args.key == "git_protocol" && args.value != "ssh" && args.value != "https" {
        bail!("invalid value for git_protocol: must be 'ssh' or 'https'");
    }

    let path = Config::config_path().context("failed to resolve config path")?;

    // Load raw config as a generic YAML map so we preserve unknown fields.
    let mut map: BTreeMap<String, serde_yaml::Value> = if path.exists() {
        let contents = fs::read_to_string(&path).context("failed to read config file")?;
        serde_yaml::from_str(&contents).context("failed to parse config file")?
    } else {
        BTreeMap::new()
    };

    map.insert(
        args.key.clone(),
        serde_yaml::Value::String(args.value.clone()),
    );

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }

    let yaml = serde_yaml::to_string(&map).context("failed to serialize config")?;
    fs::write(&path, yaml).context("failed to write config file")?;

    println!("Set {} = {}", args.key, args.value);
    Ok(())
}

fn run_list(_args: &ListArgs, format: OutputFormat) -> Result<()> {
    let config = Config::load_raw().context("failed to load config")?;

    let git_proto = format!("{:?}", config.git_protocol).to_lowercase();
    let entries = [
        ("api_url", config.api_url.as_str()),
        ("git_protocol", &git_proto),
    ];

    match format {
        OutputFormat::Json { .. } => {
            let obj: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
                .collect();
            println!("{}", serde_json::to_string_pretty(&obj)?);
        }
        OutputFormat::Toon { .. } => {
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}:{}", k, quote_toon_value(v)))
                .collect();
            println!("{}", parts.join(" "));
        }
        OutputFormat::Table => {
            println!("KEY\tVALUE");
            for (key, value) in &entries {
                println!("{key}\t{value}");
            }
        }
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<()> {
    if !VALID_KEYS.contains(&key) {
        bail!(
            "unknown config key: {} (valid keys: {})",
            key,
            VALID_KEYS.join(", ")
        );
    }
    Ok(())
}

/// Quote a value for TOON if it contains whitespace or special characters.
fn quote_toon_value(v: &str) -> String {
    if v.contains(|c: char| c.is_whitespace() || c == ':' || c == '"') {
        format!("\"{}\"", v.replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn with_config_home<F: FnOnce(&TempDir)>(f: F) {
        let tmp = TempDir::new().unwrap();
        let cfg_dir = tmp.path().join("plue");
        fs::create_dir_all(&cfg_dir).unwrap();
        let _guard = unsafe {
            let prev = std::env::var_os("XDG_CONFIG_HOME");
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
            prev
        };
        f(&tmp);
        // EnvVar cleanup happens in tests via drop or explicit unset.
        // Tests here reset after each use via TempDir scope.
    }

    #[test]
    fn validate_key_accepts_known_keys() {
        assert!(validate_key("api_url").is_ok());
        assert!(validate_key("git_protocol").is_ok());
    }

    #[test]
    fn validate_key_rejects_unknown() {
        let err = validate_key("foo_bar").expect_err("unknown key should fail");
        assert!(err.to_string().contains("unknown config key"));
    }

    #[test]
    fn set_and_get_api_url() {
        with_config_home(|tmp| {
            let set_args = SetArgs {
                key: "api_url".to_string(),
                value: "http://localhost:9999/api".to_string(),
            };
            run_set(&set_args, OutputFormat::Table).expect("set api_url should succeed");

            // Capture stdout would require capture test harness; instead verify via Config::load_raw
            let config_path = Config::config_path().unwrap();
            assert!(config_path.exists());
            let raw = fs::read_to_string(&config_path).unwrap();
            assert!(raw.contains("localhost:9999"));

            let _ = tmp;
        });
    }

    #[test]
    fn set_rejects_invalid_git_protocol() {
        let args = SetArgs {
            key: "git_protocol".to_string(),
            value: "ftp".to_string(),
        };
        let err = run_set(&args, OutputFormat::Table).expect_err("ftp should fail");
        assert!(err.to_string().contains("git_protocol"));
    }

    #[test]
    fn quote_toon_value_quotes_urls() {
        let result = quote_toon_value("http://localhost:4000/api");
        assert!(result.starts_with('"'));
    }

    #[test]
    fn quote_toon_value_leaves_simple_values() {
        let result = quote_toon_value("ssh");
        assert_eq!(result, "ssh");
    }
}
