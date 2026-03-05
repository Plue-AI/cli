use std::process::Command as ProcessCommand;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::config::{Config, GitProtocol};
use crate::output::{filter_fields, OutputFormat};
use plue::api_client::ApiClient;
use plue::output::print_toon;
use plue::repo_context::resolve_repo_ref;

#[derive(Args)]
pub struct RepoArgs {
    #[command(subcommand)]
    command: RepoCommand,
}

#[derive(Subcommand)]
enum RepoCommand {
    /// Create a new repository
    Create {
        /// Repository name
        name: String,
        /// Repository description
        #[arg(short, long)]
        description: Option<String>,
        /// Make the repository public
        #[arg(long, conflicts_with = "private")]
        public: bool,
        /// Make the repository private
        #[arg(long)]
        private: bool,
    },
    /// Clone a repository
    Clone {
        /// Repository to clone (owner/repo or full git URL)
        repository: String,
        /// Optional target directory
        directory: Option<String>,
        /// Additional clone flags passed through to jj/git
        #[arg(last = true)]
        gitflags: Vec<String>,
    },
    /// List repositories for the authenticated user
    List {
        /// Max items to return
        #[arg(short = 'L', long = "limit", default_value_t = 30)]
        limit: i32,
    },
    /// View repository details
    View {
        /// Repository in OWNER/REPO format (defaults to current repo)
        #[arg(short = 'R', long = "repo")]
        repo: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoShorthand {
    owner: String,
    repo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedCloneUrl {
    clone_url: String,
    shorthand: Option<RepoShorthand>,
}

pub fn run(args: RepoArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        RepoCommand::Create {
            name,
            description,
            public,
            private,
        } => run_create(
            &name,
            description.as_deref(),
            if public { false } else { private },
            format,
        ),
        RepoCommand::Clone {
            repository,
            directory,
            gitflags,
        } => run_clone(&repository, directory.as_deref(), &gitflags),
        RepoCommand::List { limit } => run_list(limit, format),
        RepoCommand::View { repo } => run_view(repo.as_deref(), format),
    }
}

fn run_create(
    name: &str,
    description: Option<&str>,
    private: bool,
    format: OutputFormat,
) -> Result<()> {
    let config = Config::load()?;
    let client = ApiClient::from_config(&config)?;
    let repo = client.create_repo(name, description, private)?;

    match format {
        OutputFormat::Json { ref fields } => {
            print_json_with_projection(&repo, fields.as_deref())?;
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&repo, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("Created repository {}", repo.full_name);
            println!("  Clone URL: {}", repo.clone_url);
        }
    }

    Ok(())
}

fn run_clone(repository: &str, directory: Option<&str>, gitflags: &[String]) -> Result<()> {
    let config = Config::load()?;
    let metadata_clone_url = if let Some(shorthand) = parse_repo_shorthand(repository) {
        Some(lookup_clone_metadata(&config, &shorthand)?)
    } else {
        None
    };
    let resolved = resolve_clone_url(repository, &config, metadata_clone_url.as_deref())?;

    match run_jj_clone(&resolved.clone_url, directory, gitflags) {
        Ok(()) => Ok(()),
        Err(jj_err) => match run_git_clone(&resolved.clone_url, directory, gitflags) {
            Ok(()) => Ok(()),
            Err(git_err) => bail!("jj clone failed: {jj_err}; git clone failed: {git_err}"),
        },
    }
}

fn run_list(limit: i32, format: OutputFormat) -> Result<()> {
    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let repos = client.list_repos(None, 1, limit)?;

    match format {
        OutputFormat::Json { ref fields } => {
            print_json_with_projection(&repos, fields.as_deref())?;
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&repos, fields.as_deref());
        }
        OutputFormat::Table => {
            if repos.is_empty() {
                println!("No repositories found.");
                return Ok(());
            }
            println!("{:<40} {:<8} DESCRIPTION", "NAME", "VISIBILITY");
            for r in &repos {
                let visibility = if r.is_public { "public" } else { "private" };
                println!("{:<40} {:<8} {}", r.name, visibility, r.description);
            }
        }
    }
    Ok(())
}

fn run_view(repo_override: Option<&str>, format: OutputFormat) -> Result<()> {
    let repo_ref = if let Some(r) = repo_override {
        let parts: Vec<&str> = r.splitn(2, '/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            anyhow::bail!("expected owner/repo format, got: {r}");
        }
        plue::repo_context::RepoRef {
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
        }
    } else {
        let cwd = std::env::current_dir().context("cannot determine current directory")?;
        resolve_repo_ref(&cwd, None)?
    };

    let config = Config::load().context("failed to load config")?;
    let client = ApiClient::from_config(&config)?;

    let repo = client.get_repo(&repo_ref.owner, &repo_ref.repo)?;

    match format {
        OutputFormat::Json { ref fields } => {
            print_json_with_projection(&repo, fields.as_deref())?;
        }
        OutputFormat::Toon { ref fields } => {
            print_toon(&repo, fields.as_deref());
        }
        OutputFormat::Table => {
            println!("{repo}");
        }
    }
    Ok(())
}

fn print_json_with_projection<T: serde::Serialize>(value: &T, fields: Option<&str>) -> Result<()> {
    let json_value = serde_json::to_value(value)?;
    let projected = fields
        .filter(|requested| !requested.is_empty())
        .map(|requested| filter_fields(&json_value, requested))
        .unwrap_or(json_value);
    println!("{}", serde_json::to_string_pretty(&projected)?);
    Ok(())
}

fn lookup_clone_metadata(config: &Config, shorthand: &RepoShorthand) -> Result<String> {
    let client = ApiClient::from_config(config)?;
    let repo = client.get_repo(&shorthand.owner, &shorthand.repo)?;
    Ok(repo.clone_url)
}

fn run_jj_clone(clone_url: &str, directory: Option<&str>, gitflags: &[String]) -> Result<()> {
    let args = build_jj_clone_args(clone_url, directory, gitflags);
    run_clone_command("jj", &args)
}

fn run_git_clone(clone_url: &str, directory: Option<&str>, gitflags: &[String]) -> Result<()> {
    let args = build_git_clone_args(clone_url, directory, gitflags);
    run_clone_command("git", &args)
}

fn build_jj_clone_args(
    clone_url: &str,
    directory: Option<&str>,
    gitflags: &[String],
) -> Vec<String> {
    let mut args = vec![
        "git".to_string(),
        "clone".to_string(),
        clone_url.to_string(),
    ];
    if let Some(directory) = directory {
        args.push(directory.to_string());
    }
    args.extend(gitflags.iter().cloned());
    args
}

fn build_git_clone_args(
    clone_url: &str,
    directory: Option<&str>,
    gitflags: &[String],
) -> Vec<String> {
    let mut args = vec!["clone".to_string(), clone_url.to_string()];
    if let Some(directory) = directory {
        args.push(directory.to_string());
    }
    args.extend(gitflags.iter().cloned());
    args
}

fn run_clone_command(program: &str, args: &[String]) -> Result<()> {
    let status = ProcessCommand::new(program).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        bail!("{program} clone exited with non-zero status: {status}");
    }
}

fn is_explicit_repository_url(repository: &str) -> bool {
    let value = repository.trim();
    if value.starts_with("https://")
        || value.starts_with("http://")
        || value.starts_with("ssh://")
        || value.starts_with("git://")
    {
        return true;
    }

    if let Some((left, right)) = value.split_once(':') {
        return left.contains('@') && right.contains('/');
    }

    false
}

fn parse_repo_shorthand(repository: &str) -> Option<RepoShorthand> {
    let value = repository.trim();
    if value.is_empty() || is_explicit_repository_url(value) {
        return None;
    }

    let parts: Vec<&str> = value.split('/').collect();
    let (owner, repo) = match parts.as_slice() {
        [owner, repo] => (*owner, *repo),
        [host, owner, repo] if host.contains('.') => (*owner, *repo),
        _ => return None,
    };
    let owner = owner.trim();
    let repo = repo.trim().trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some(RepoShorthand {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn convert_ssh_clone_url_to_https(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if let Some(path) = trimmed.strip_prefix("ssh://") {
        let (authority, repo_path) = path.split_once('/')?;
        let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        return Some(format!(
            "https://{}/{}",
            host,
            repo_path.trim_start_matches('/')
        ));
    }

    let (authority, repo_path) = trimmed.split_once(':')?;
    if !authority.contains('@') {
        return None;
    }
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    Some(format!(
        "https://{}/{}",
        host,
        repo_path.trim_start_matches('/')
    ))
}

fn resolve_clone_url(
    repository: &str,
    config: &Config,
    metadata_clone_url: Option<&str>,
) -> Result<ResolvedCloneUrl> {
    if is_explicit_repository_url(repository) {
        let clone_url = match config.git_protocol {
            GitProtocol::Ssh => repository.to_string(),
            GitProtocol::Https => {
                convert_ssh_clone_url_to_https(repository).unwrap_or_else(|| repository.to_string())
            }
        };
        return Ok(ResolvedCloneUrl {
            clone_url,
            shorthand: None,
        });
    }

    let shorthand = parse_repo_shorthand(repository)
        .ok_or_else(|| anyhow::anyhow!("invalid repository format: expected owner/repo or URL"))?;

    let clone_url = if let Some(metadata) = metadata_clone_url {
        match config.git_protocol {
            GitProtocol::Ssh => metadata.to_string(),
            GitProtocol::Https => {
                convert_ssh_clone_url_to_https(metadata).unwrap_or_else(|| metadata.to_string())
            }
        }
    } else {
        let host = config.host();
        match config.git_protocol {
            GitProtocol::Ssh => format!("git@{host}:{}.git", shorthand_path(&shorthand)),
            GitProtocol::Https => format!("https://{host}/{}.git", shorthand_path(&shorthand)),
        }
    };

    Ok(ResolvedCloneUrl {
        clone_url,
        shorthand: Some(shorthand),
    })
}

fn shorthand_path(shorthand: &RepoShorthand) -> String {
    format!("{}/{}", shorthand.owner, shorthand.repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GitProtocol;

    #[test]
    fn explicit_url_detection() {
        assert!(is_explicit_repository_url(
            "https://plue.dev/alice/demo.git"
        ));
        assert!(is_explicit_repository_url("http://plue.dev/alice/demo.git"));
        assert!(is_explicit_repository_url(
            "ssh://git@plue.dev/alice/demo.git"
        ));
        assert!(is_explicit_repository_url("git@plue.dev:alice/demo.git"));
        assert!(!is_explicit_repository_url("alice/demo"));
    }

    #[test]
    fn shorthand_parsing() {
        let parsed = parse_repo_shorthand("alice/demo").expect("valid shorthand");
        assert_eq!(parsed.owner, "alice");
        assert_eq!(parsed.repo, "demo");
        assert!(parse_repo_shorthand("alice").is_none());
        assert!(parse_repo_shorthand("https://plue.dev/alice/demo.git").is_none());
    }

    #[test]
    fn ssh_to_https_conversion() {
        let converted = convert_ssh_clone_url_to_https("git@plue.dev:alice/demo.git")
            .expect("ssh url should convert");
        assert_eq!(converted, "https://plue.dev/alice/demo.git");
        assert!(convert_ssh_clone_url_to_https("https://plue.dev/alice/demo.git").is_none());
    }

    #[test]
    fn url_resolution_for_shorthand_ssh() {
        let config = Config {
            api_url: "https://plue.dev/api".to_string(),
            token: None,
            git_protocol: GitProtocol::Ssh,
        };
        let resolved = resolve_clone_url("alice/demo", &config, None).expect("resolve");
        assert_eq!(resolved.clone_url, "git@plue.dev:alice/demo.git");
        assert_eq!(
            resolved.shorthand,
            Some(RepoShorthand {
                owner: "alice".to_string(),
                repo: "demo".to_string(),
            })
        );
    }

    #[test]
    fn url_resolution_for_ssh_url_with_https_protocol_converts() {
        let config = Config {
            api_url: "https://plue.dev/api".to_string(),
            token: None,
            git_protocol: GitProtocol::Https,
        };
        let resolved =
            resolve_clone_url("git@plue.dev:alice/demo.git", &config, None).expect("resolve");
        assert_eq!(resolved.clone_url, "https://plue.dev/alice/demo.git");
        assert!(resolved.shorthand.is_none());
    }

    #[test]
    fn build_jj_clone_args_orders_url_directory_then_flags() {
        let args = build_jj_clone_args(
            "https://plue.dev/alice/demo.git",
            Some("my-dir"),
            &["--depth".into(), "1".into()],
        );
        assert_eq!(
            args,
            vec![
                "git".to_string(),
                "clone".to_string(),
                "https://plue.dev/alice/demo.git".to_string(),
                "my-dir".to_string(),
                "--depth".to_string(),
                "1".to_string()
            ]
        );
    }

    #[test]
    fn build_git_clone_args_omits_directory_when_missing() {
        let args = build_git_clone_args(
            "https://plue.dev/alice/demo.git",
            None,
            &["--depth".into(), "1".into()],
        );
        assert_eq!(
            args,
            vec![
                "clone".to_string(),
                "https://plue.dev/alice/demo.git".to_string(),
                "--depth".to_string(),
                "1".to_string()
            ]
        );
    }
}
