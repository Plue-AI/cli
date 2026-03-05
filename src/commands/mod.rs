pub mod agent;
pub mod api;
pub mod auth;
pub mod beta;
pub mod bookmark;
pub mod change;
pub mod completion;
pub mod config_cmd;
pub mod issue;
pub mod label;
pub mod land;
pub mod release;
pub mod repo;
pub mod run_cmd;
pub mod search;
pub mod secret;
pub mod ssh_key;
pub mod status;
pub mod variable;
pub mod workflow;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputFormat;

#[derive(Subcommand)]
pub enum Commands {
    /// Manage authentication (login, logout, token)
    Auth(auth::AuthArgs),
    /// Manage closed beta whitelist and waitlist
    Beta(beta::BetaArgs),
    /// Manage repositories
    Repo(repo::RepoArgs),
    /// Manage issues
    Issue(issue::IssueArgs),
    /// Manage landing requests
    Land(land::LandArgs),
    /// View changes
    Change(change::ChangeArgs),
    /// Manage bookmarks (branches)
    Bookmark(bookmark::BookmarkArgs),
    /// View and manage workflow runs
    Run(run_cmd::RunArgs),
    /// Manage workflows
    Workflow(workflow::WorkflowArgs),
    /// Interact with AI agents
    Agent(agent::AgentArgs),
    /// Search repos, issues, and code
    Search(search::SearchArgs),
    /// Manage labels
    Label(label::LabelArgs),
    /// Manage releases
    Release(release::ReleaseArgs),
    /// Manage secrets
    Secret(secret::SecretArgs),
    /// Manage variables
    Variable(variable::VariableArgs),
    /// Manage SSH keys
    SshKey(ssh_key::SshKeyArgs),
    /// Get and set configuration
    Config(config_cmd::ConfigArgs),
    /// Show working copy status
    Status(status::StatusArgs),
    /// Generate shell completions
    Completion(completion::CompletionArgs),
    /// Make raw API calls
    Api(api::ApiArgs),
}

pub fn run(cmd: Commands, format: OutputFormat) -> Result<()> {
    match cmd {
        Commands::Auth(args) => auth::run(args, format),
        Commands::Beta(args) => beta::run(args, format),
        Commands::Repo(args) => repo::run(args, format),
        Commands::Issue(args) => issue::run(args, format),
        Commands::Land(args) => land::run(args, format),
        Commands::Change(args) => change::run(args, format),
        Commands::Bookmark(args) => bookmark::run(args, format),
        Commands::Run(args) => run_cmd::run(args, format),
        Commands::Workflow(args) => workflow::run(args, format),
        Commands::Agent(args) => agent::run(args, format),
        Commands::Search(args) => search::run(args, format),
        Commands::Label(args) => label::run(args, format),
        Commands::Release(args) => release::run(args, format),
        Commands::Secret(args) => secret::run(args, format),
        Commands::Variable(args) => variable::run(args, format),
        Commands::SshKey(args) => ssh_key::run(args, format),
        Commands::Config(args) => config_cmd::run(args, format),
        Commands::Status(args) => status::run(args, format),
        Commands::Completion(args) => completion::run(args),
        Commands::Api(args) => api::run(args, format),
    }
}
