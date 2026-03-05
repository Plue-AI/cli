use anyhow::{Context, Result};
use clap::Args;

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;

/// Make raw API calls to the Plue server.
#[derive(Args)]
pub struct ApiArgs {
    /// HTTP method (GET, POST, PUT, PATCH, DELETE)
    #[arg(short = 'X', long = "method", default_value = "GET")]
    method: String,

    /// API endpoint path (e.g. /repos/owner/name)
    endpoint: String,

    /// Add a typed request body field (key=value). Repeat for multiple fields.
    #[arg(short = 'f', long = "field")]
    fields: Vec<String>,

    /// Add a request header (key:value). Repeat for multiple headers.
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,
}

pub fn run(args: ApiArgs, format: OutputFormat) -> Result<()> {
    let method = args.method.to_uppercase();
    let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE"];
    if !valid_methods.contains(&method.as_str()) {
        anyhow::bail!(
            "invalid HTTP method '{}'; expected one of: {}",
            args.method,
            valid_methods.join(", ")
        );
    }

    if !args.endpoint.starts_with('/') {
        anyhow::bail!("endpoint must begin with '/'");
    }

    // Parse -F key=value fields into JSON body
    let body = if args.fields.is_empty() {
        None
    } else {
        let mut map = serde_json::Map::new();
        for field in &args.fields {
            let (key, value) = field
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("field must be in key=value format: {field}"))?;
            map.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
        Some(serde_json::Value::Object(map))
    };

    // Parse -H key:value headers
    let extra_headers: Vec<(String, String)> = args
        .headers
        .iter()
        .map(|h| {
            h.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .ok_or_else(|| anyhow::anyhow!("header must be in key:value format: {h}"))
        })
        .collect::<Result<Vec<_>>>()?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let resp = client.raw_request(&method, &args.endpoint, &extra_headers, body)?;

    // Try to format as JSON if the response body is JSON
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&resp.body) {
        match format {
            OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&json_val)?),
            OutputFormat::Toon { ref fields } => {
                plue::output::print_toon(&json_val, fields.as_deref());
            }
            OutputFormat::Table => println!("{}", serde_json::to_string_pretty(&json_val)?),
        }
    } else {
        // Raw body -- print as-is
        print!("{}", resp.body);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_method() {
        let args = ApiArgs {
            method: "FOOBAR".to_string(),
            endpoint: "/repos/test".to_string(),
            fields: vec![],
            headers: vec![],
        };
        let err = run(args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("invalid HTTP method"));
    }

    #[test]
    fn rejects_endpoint_not_starting_with_slash() {
        let args = ApiArgs {
            method: "GET".to_string(),
            endpoint: "repos/test".to_string(),
            fields: vec![],
            headers: vec![],
        };
        let err = run(args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("endpoint must begin with '/'"));
    }

    #[test]
    fn rejects_malformed_field() {
        let args = ApiArgs {
            method: "POST".to_string(),
            endpoint: "/repos".to_string(),
            fields: vec!["bad-field-without-equals".to_string()],
            headers: vec![],
        };
        let err = run(args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("key=value"));
    }

    #[test]
    fn rejects_malformed_header() {
        let args = ApiArgs {
            method: "GET".to_string(),
            endpoint: "/repos".to_string(),
            fields: vec![],
            headers: vec!["bad-header-without-colon".to_string()],
        };
        let err = run(args, OutputFormat::Table).expect_err("should fail");
        assert!(err.to_string().contains("key:value"));
    }
}
