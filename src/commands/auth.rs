use std::io::{self, Read};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::config::Config;
use crate::credential_store::{resolve_token, CredentialStore};
use crate::output::OutputFormat;
use plue::output::print_toon;

#[derive(Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    command: AuthCommand,
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Log in to Plue
    Login {
        /// Read token from stdin (e.g., echo "plue_xxx" | plue auth login --with-token)
        #[arg(long)]
        with_token: bool,

        /// Store token in plain-text config file instead of OS keychain
        #[arg(long)]
        insecure_storage: bool,

        /// Hostname to authenticate with (default: from config api_url)
        #[arg(long)]
        hostname: Option<String>,
    },
    /// Log out of Plue
    Logout {
        /// Hostname to log out from (default: from config api_url)
        #[arg(long)]
        hostname: Option<String>,
    },
    /// Show authentication status
    Status,
    /// Print the authentication token to stdout
    Token {
        /// Hostname to get token for (default: from config api_url)
        #[arg(long)]
        hostname: Option<String>,
    },
}

pub fn run(args: AuthArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        AuthCommand::Login {
            with_token,
            insecure_storage,
            hostname,
        } => {
            if with_token {
                run_login_with_token(insecure_storage, hostname)
            } else {
                run_login_interactive(insecure_storage, hostname)
            }
        }
        AuthCommand::Logout { hostname } => run_logout(hostname),
        AuthCommand::Status => run_status(format),
        AuthCommand::Token { hostname } => run_token(hostname),
    }
}

fn run_login_interactive(insecure_storage: bool, hostname: Option<String>) -> Result<()> {
    let config = Config::load_raw().unwrap_or_default();
    let host = hostname.clone().unwrap_or_else(|| config.host());
    let api_url = config.api_url.clone();

    // Use tokio runtime since we need async TCP.
    let rt = tokio::runtime::Runtime::new().context("failed to create async runtime")?;

    rt.block_on(async {
        // 1. Bind on a random port.
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind local callback server")?;
        let port = listener
            .local_addr()
            .context("failed to get local address")?
            .port();

        // 2. Open browser to the API's CLI OAuth endpoint.
        let auth_url = format!("{api_url}/auth/github/cli?callback_port={port}");
        eprintln!("Opening browser to authenticate with GitHub...");
        eprintln!("If the browser doesn't open, visit:\n  {auth_url}\n");

        if let Err(e) = open_browser(&auth_url) {
            eprintln!("warning: could not open browser: {e}");
            eprintln!("Please open this URL manually:\n  {auth_url}");
        }

        // 3. Wait for the callback.
        eprintln!("Waiting for authentication...");
        let (token, username) = wait_for_callback(listener).await?;

        // 4. Store the token.
        if insecure_storage {
            let config_path = Config::config_path()?;
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent).context("failed to create config directory")?;
            }
            let new_config = Config {
                token: Some(token.clone()),
                ..config
            };
            let yaml = serde_yaml::to_string(&new_config).context("failed to serialize config")?;
            std::fs::write(&config_path, yaml).context("failed to write config file")?;
            eprintln!(
                "warning: token stored in plain text at {}",
                config_path.display()
            );
        } else {
            let store = CredentialStore::new();
            store
                .store_token(&host, &token)
                .context("failed to store token in keychain")?;
        }

        let display_user = if username.is_empty() {
            host.clone()
        } else {
            format!("{username} on {host}")
        };

        eprintln!("✓ Logged in as {display_user}");
        eprintln!("  Token stored in keychain");

        Ok(())
    })
}

/// Open a URL in the default browser.
fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("failed to open browser")?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("failed to open browser")?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .context("failed to open browser")?;
    }
    Ok(())
}

/// Wait for the OAuth callback on the local TCP listener.
/// Parses `?token=plue_xxx&username=xxx` from the request.
async fn wait_for_callback(listener: TcpListener) -> Result<(String, String)> {
    let (mut stream, _) = listener
        .accept()
        .await
        .context("failed to accept callback connection")?;

    let mut buf = vec![0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .context("failed to read callback request")?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the GET request line: "GET /callback?token=plue_xxx&username=yyy HTTP/1.1"
    let (token, username) = parse_callback_params(&request)?;

    // Serve a success HTML page.
    let html = success_html(&username);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html,
    );
    stream
        .write_all(response.as_bytes())
        .await
        .context("failed to write callback response")?;
    stream.flush().await.ok();

    Ok((token, username))
}

/// Parse token and username from the callback request.
fn parse_callback_params(request: &str) -> Result<(String, String)> {
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    let query = path.split('?').nth(1).unwrap_or("");
    let mut token = String::new();
    let mut username = String::new();

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "token" => token = value.to_string(),
                "username" => username = value.to_string(),
                _ => {}
            }
        }
    }

    if token.is_empty() || !token.starts_with("plue_") {
        bail!("authentication failed — no valid token received from server");
    }

    Ok((token, username))
}

/// Generates a polished success HTML page shown in the browser after login.
fn success_html(username: &str) -> String {
    let display = if username.is_empty() {
        "You're logged in!".to_string()
    } else {
        format!("Welcome, {}!", username)
    };

    SUCCESS_HTML_TEMPLATE.replace("{{DISPLAY}}", &display)
}

const SUCCESS_HTML_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Plue CLI — Authenticated</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: linear-gradient(135deg, #0f0c29 0%, #1a1a2e 50%, #16213e 100%);
    color: #e0e0e0;
  }
  .card {
    text-align: center;
    padding: 3rem 4rem;
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 16px;
    backdrop-filter: blur(10px);
    box-shadow: 0 8px 32px rgba(0,0,0,0.3);
  }
  .check {
    width: 64px; height: 64px;
    margin: 0 auto 1.5rem;
    border-radius: 50%;
    background: linear-gradient(135deg, #00b09b, #96c93d);
    display: flex; align-items: center; justify-content: center;
    animation: pop 0.4s ease-out;
  }
  .check svg { width: 32px; height: 32px; }
  h1 { font-size: 1.5rem; font-weight: 600; margin-bottom: 0.5rem; color: #fff; }
  p { color: #9ca3af; font-size: 0.95rem; }
  .hint { margin-top: 1.5rem; font-size: 0.85rem; color: #6b7280; }
  @keyframes pop {
    0% { transform: scale(0); opacity: 0; }
    80% { transform: scale(1.1); }
    100% { transform: scale(1); opacity: 1; }
  }
</style>
</head>
<body>
<div class="card">
  <div class="check">
    <svg fill="none" stroke="#fff" stroke-width="3" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"/>
    </svg>
  </div>
  <h1>{{DISPLAY}}</h1>
  <p>Authentication successful. You can close this tab.</p>
  <p class="hint">Return to your terminal to continue using Plue.</p>
</div>
</body>
</html>"##;

fn run_login_with_token(insecure_storage: bool, hostname: Option<String>) -> Result<()> {
    let mut token = String::new();
    io::stdin()
        .read_to_string(&mut token)
        .context("failed to read token from stdin")?;

    let token = token.trim().to_string();
    if token.is_empty() {
        bail!("no token provided");
    }
    if !token.starts_with("plue_") {
        bail!("token must start with 'plue_'");
    }

    let config = Config::load_raw().unwrap_or_default();
    let host = hostname.unwrap_or_else(|| config.host());

    if insecure_storage {
        // Write to config file
        let config_path = Config::config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("failed to create config directory")?;
        }

        let new_config = Config {
            token: Some(token),
            ..config
        };

        let yaml = serde_yaml::to_string(&new_config).context("failed to serialize config")?;
        std::fs::write(&config_path, yaml).context("failed to write config file")?;

        eprintln!(
            "warning: token stored in plain text at {}",
            config_path.display()
        );
        println!("Logged in to {host}");
    } else {
        // Store in keyring (default, secure)
        let store = CredentialStore::new();
        store
            .store_token(&host, &token)
            .context("failed to store token in keyring")?;
        println!("Logged in to {host}");
    }

    Ok(())
}

fn run_logout(hostname: Option<String>) -> Result<()> {
    let config = Config::load_raw().unwrap_or_default();
    let host = hostname.unwrap_or_else(|| config.host());

    // Delete from keyring
    let store = CredentialStore::new();
    store.delete_token(&host)?;

    // Clear config file token if present
    if config.token.is_some() {
        let config_path = Config::config_path()?;
        if config_path.exists() {
            let cleared = Config {
                token: None,
                ..config
            };
            let yaml = serde_yaml::to_string(&cleared).context("failed to serialize config")?;
            std::fs::write(&config_path, yaml).context("failed to write config file")?;
        }
    }

    println!("Logged out of {host}");
    Ok(())
}

fn run_status(format: OutputFormat) -> Result<()> {
    let config = Config::load_raw()?;
    let store = CredentialStore::new();
    let resolved = config.token_for_host(&store)?;

    match format {
        OutputFormat::Json { .. } => {
            let status = serde_json::json!({
                "logged_in": resolved.is_some(),
                "api_url": config.api_url,
                "token_set": resolved.is_some(),
                "token_source": resolved.as_ref().map(|r| r.source.to_string()),
            });
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        OutputFormat::Toon { ref fields } => {
            let status = serde_json::json!({
                "logged_in": resolved.is_some(),
                "api_url": config.api_url,
                "token_set": resolved.is_some(),
                "token_source": resolved.as_ref().map(|r| r.source.to_string()),
            });
            print_toon(&status, fields.as_deref());
        }
        OutputFormat::Table => {
            if let Some(ref r) = resolved {
                println!("Logged in to {}", config.api_url);
                println!("  Token: set (via {})", r.source);
            } else {
                println!("Not logged in");
                println!("  Run: plue auth login --with-token");
                println!("  Or:  export PLUE_TOKEN=plue_...");
            }
        }
    }

    Ok(())
}

fn run_token(hostname: Option<String>) -> Result<()> {
    let config = Config::load_raw()?;
    let host = hostname.unwrap_or_else(|| config.host());
    let store = CredentialStore::new();
    let resolved = resolve_token(&host, &store, &config.token)?;

    match resolved {
        Some(r) => {
            // Token to stdout, source info to stderr
            println!("{}", r.token);
            eprintln!("Token source: {} (host: {host})", r.source);
            Ok(())
        }
        None => {
            bail!(
                "no token found for {host} — run `plue auth login --with-token` or set PLUE_TOKEN"
            )
        }
    }
}
