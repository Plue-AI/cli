use std::time::Duration;

use anyhow::Error;
use serde_json::json;

use crate::config::Config;

/// Best-effort error reporting to the Plue telemetry endpoint.
/// This function never panics or returns errors — failures are silently ignored
/// so that error reporting never interferes with normal CLI error display.
pub fn report_error(config: &Config, error: &Error, command: &str) {
    let api_url = &config.api_url;

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let _ = client
        .post(format!("{api_url}/telemetry/errors"))
        .json(&json!({
            "client": "cli",
            "version": env!("CARGO_PKG_VERSION"),
            "error": {
                "message": format!("{error:#}").chars().take(512).collect::<String>(),
                "type": "anyhow",
            },
            "context": {
                "command": command,
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            }
        }))
        .send();
}
