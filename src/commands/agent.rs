use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::config::Config;
use crate::output::OutputFormat;
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;
use plue::types::{AgentMessagePartInput, CreateAgentSessionInput, PostAgentMessageInput};

#[derive(Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    command: AgentCommand,
    /// Repository in OWNER/REPO format (overrides remote detection)
    #[arg(short = 'R', long = "repo", global = true)]
    repo: Option<String>,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// List agent sessions for the repository
    List(ListArgs),
    /// View an agent session
    View(ViewArgs),
    /// Start a new agent session and send a message (non-interactive)
    Run(RunArgs),
    /// Post a message to an existing agent session
    Chat(ChatArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Page number
    #[arg(long, default_value = "1")]
    page: i32,
    /// Results per page
    #[arg(long, default_value = "30")]
    per_page: i32,
}

#[derive(Args)]
struct ViewArgs {
    /// Session ID
    session_id: String,
    /// Show messages in the session
    #[arg(long)]
    messages: bool,
}

#[derive(Args)]
struct RunArgs {
    /// Prompt/message text to send as the initial user message
    prompt: String,
    /// Session title
    #[arg(long, default_value = "")]
    title: String,
}

#[derive(Args)]
struct ChatArgs {
    /// Session ID to send a message to
    session_id: String,
    /// Message text
    message: String,
}

pub fn run(args: AgentArgs, format: OutputFormat) -> Result<()> {
    let repo_override = args.repo.as_deref();
    match args.command {
        AgentCommand::List(a) => run_list(&a, repo_override, format),
        AgentCommand::View(a) => run_view(&a, repo_override, format),
        AgentCommand::Run(a) => run_noninteractive(&a, repo_override, format),
        AgentCommand::Chat(a) => run_chat(&a, repo_override, format),
    }
}

fn run_list(args: &ListArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let sessions =
        client.list_agent_sessions(&repo_ref.owner, &repo_ref.repo, args.page, args.per_page)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&sessions)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&sessions, fields.as_deref());
        }
        OutputFormat::Table => {
            if sessions.is_empty() {
                println!("No agent sessions found.");
                return Ok(());
            }
            println!("{:<38} {:<12} {:<40}", "SESSION ID", "STATUS", "TITLE");
            for s in &sessions {
                println!("{:<38} {:<12} {:<40}", s.id, s.status, s.title);
            }
        }
    }
    Ok(())
}

fn run_view(args: &ViewArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let session = client.get_agent_session(&repo_ref.owner, &repo_ref.repo, &args.session_id)?;

    match &format {
        OutputFormat::Json { .. } => {
            if args.messages {
                let messages = client.list_agent_messages(
                    &repo_ref.owner,
                    &repo_ref.repo,
                    &args.session_id,
                    1,
                    100,
                )?;
                let combined = serde_json::json!({ "session": &session, "messages": &messages });
                println!("{}", serde_json::to_string_pretty(&combined)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&session)?);
            }
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&session, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Session {}", session.id);
            println!("  Title:   {}", session.title);
            println!("  Status:  {}", session.status);
            println!("  Created: {}", session.created_at);

            if args.messages {
                let messages = client.list_agent_messages(
                    &repo_ref.owner,
                    &repo_ref.repo,
                    &args.session_id,
                    1,
                    100,
                )?;
                println!("\nMessages ({}):", messages.len());
                for msg in &messages {
                    println!("  [{}] #{} {}", msg.role, msg.sequence, msg.created_at);
                    for part in &msg.parts {
                        println!("    [{}] {}", part.part_type, part.content);
                    }
                }
            }
        }
    }
    Ok(())
}

fn run_noninteractive(
    args: &RunArgs,
    repo_override: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    // Create a new session.
    let session_input = CreateAgentSessionInput {
        title: args.title.clone(),
    };
    let session = client.create_agent_session(&repo_ref.owner, &repo_ref.repo, &session_input)?;

    // Post the user message.
    let msg_input = PostAgentMessageInput {
        role: "user".to_string(),
        parts: vec![AgentMessagePartInput {
            part_type: "text".to_string(),
            content: serde_json::Value::String(args.prompt.clone()),
        }],
    };
    let _msg =
        client.post_agent_message(&repo_ref.owner, &repo_ref.repo, &session.id, &msg_input)?;

    match format {
        OutputFormat::Json { .. } => println!("{}", serde_json::to_string_pretty(&session)?),
        OutputFormat::Toon { ref fields } => {
            print_toon(&session, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Created session {} and sent message.", session.id);
            println!("  Use `plue agent view {}` to check progress.", session.id);
        }
    }
    Ok(())
}

fn run_chat(args: &ChatArgs, repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let repo_ref = resolve_repo_ref(&cwd, repo_override)?;

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let msg_input = PostAgentMessageInput {
        role: "user".to_string(),
        parts: vec![AgentMessagePartInput {
            part_type: "text".to_string(),
            content: serde_json::Value::String(args.message.clone()),
        }],
    };
    let msg = client.post_agent_message(
        &repo_ref.owner,
        &repo_ref.repo,
        &args.session_id,
        &msg_input,
    )?;

    match &format {
        OutputFormat::Json { .. } => {
            println!("{}", serde_json::to_string_pretty(&msg)?);
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&msg, fields.as_deref());
        }
        OutputFormat::Table => {
            println!(
                "Message #{} sent to session {}. Streaming response...",
                msg.sequence, msg.session_id
            );
        }
    }

    // Stream SSE events from the agent session until done
    client.stream_agent_session(&repo_ref.owner, &repo_ref.repo, &args.session_id, |event| {
        match event.event_type.as_str() {
            "message" => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
                    if let Some(parts) = parsed.get("parts").and_then(|p| p.as_array()) {
                        for part in parts {
                            if let Some(content) = part.get("content") {
                                if let Some(val) = content.get("value").and_then(|v| v.as_str()) {
                                    print!("{}", val);
                                }
                            }
                        }
                    }
                }
            }
            "tool_call" => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
                    if let Some(parts) = parsed.get("parts").and_then(|p| p.as_array()) {
                        for part in parts {
                            if let Some(name) = part
                                .get("content")
                                .and_then(|c| c.get("name"))
                                .and_then(|n| n.as_str())
                            {
                                eprintln!("[tool_call] {}", name);
                            }
                        }
                    }
                }
            }
            "tool_result" => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
                    if let Some(parts) = parsed.get("parts").and_then(|p| p.as_array()) {
                        for part in parts {
                            if let Some(name) = part
                                .get("content")
                                .and_then(|c| c.get("name"))
                                .and_then(|n| n.as_str())
                            {
                                eprintln!("[tool_result] {}", name);
                            }
                        }
                    }
                }
            }
            "done" => {
                println!();
                return false;
            }
            _ => {}
        }
        true
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use plue::types::AgentSSEEvent;

    #[test]
    fn run_args_stores_prompt() {
        let a = RunArgs {
            prompt: "Hello agent".to_string(),
            title: String::new(),
        };
        assert_eq!(a.prompt, "Hello agent");
    }

    #[test]
    fn chat_args_stores_message() {
        let a = ChatArgs {
            session_id: "abc-123".to_string(),
            message: "What is 2+2?".to_string(),
        };
        assert_eq!(a.message, "What is 2+2?");
    }

    #[test]
    fn agent_sse_event_fields() {
        let event = AgentSSEEvent {
            event_type: "message".to_string(),
            data: r#"{"parts":[{"type":"text","content":{"value":"hello"}}]}"#.to_string(),
        };
        assert_eq!(event.event_type, "message");
        assert!(event.data.contains("hello"));
    }

    #[test]
    fn agent_sse_event_done_type() {
        let event = AgentSSEEvent {
            event_type: "done".to_string(),
            data: r#"{"parts":[{"type":"done","content":{"status":"completed"}}]}"#.to_string(),
        };
        assert_eq!(event.event_type, "done");
    }

    #[test]
    fn agent_sse_event_tool_call_type() {
        let event = AgentSSEEvent {
            event_type: "tool_call".to_string(),
            data: r#"{"parts":[{"type":"tool_call","content":{"name":"read_file"}}]}"#.to_string(),
        };
        assert_eq!(event.event_type, "tool_call");
    }
}
