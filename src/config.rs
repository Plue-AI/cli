use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::credential_store::{host_from_url, resolve_token, CredentialStore, ResolvedToken};

/// Git transport protocol to use for clone/fetch URLs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GitProtocol {
    #[default]
    Ssh,
    Https,
}

/// Plue CLI configuration loaded from `~/.config/plue/config.yml` and env vars.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// API base URL (default: https://plue.dev/api)
    #[serde(default = "default_api_url")]
    pub api_url: String,

    /// Authentication token (overridden by PLUE_TOKEN env var)
    #[serde(default)]
    pub token: Option<String>,

    /// Preferred git protocol for clone URLs (ssh or https).
    #[serde(default)]
    pub git_protocol: GitProtocol,
}

fn default_api_url() -> String {
    "https://plue.dev/api".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            token: None,
            git_protocol: GitProtocol::default(),
        }
    }
}

impl Config {
    /// Load config from file and overlay environment variables.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        let mut config = if path.exists() {
            let contents = std::fs::read_to_string(&path).context("failed to read config file")?;
            serde_yaml::from_str(&contents).context("failed to parse config file")?
        } else {
            Self::default()
        };

        // PLUE_TOKEN env var always wins
        if let Ok(token) = std::env::var("PLUE_TOKEN") {
            config.token = Some(token);
        }

        Ok(config)
    }

    /// Returns the path to the config file.
    pub fn config_path() -> Result<PathBuf> {
        if let Some(xdg_config_home) = std::env::var_os("XDG_CONFIG_HOME") {
            let path = PathBuf::from(xdg_config_home);
            if !path.as_os_str().is_empty() {
                return Ok(path.join("plue").join("config.yml"));
            }
        }

        let config_dir = dirs::config_dir().context("cannot determine config directory")?;
        Ok(config_dir.join("plue").join("config.yml"))
    }

    /// Load config from file *without* overlaying PLUE_TOKEN env var.
    /// Useful for seeing what's actually stored on disk.
    pub fn load_raw() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let contents = std::fs::read_to_string(&path).context("failed to read config file")?;
            serde_yaml::from_str(&contents).context("failed to parse config file")
        } else {
            Ok(Self::default())
        }
    }

    /// Resolve the authentication token using priority chain:
    /// PLUE_TOKEN env → keyring → config file.
    pub fn token_for_host(&self, store: &CredentialStore) -> Result<Option<ResolvedToken>> {
        let host = self.host();
        resolve_token(&host, store, &self.token)
    }

    /// Check whether the keyring has a token for this config's host.
    pub fn is_token_in_keyring(&self, store: &CredentialStore) -> bool {
        store.has_token(&self.host())
    }

    /// Extract the hostname from `api_url`.
    pub fn host(&self) -> String {
        host_from_url(&self.api_url)
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, GitProtocol};
    use crate::credential_store::{CredentialStore, MockStore, TokenSource};
    use std::ffi::OsString;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set_path(key: &'static str, value: &std::path::Path) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }

    fn mock_store() -> CredentialStore {
        CredentialStore::with_backend(Box::new(MockStore::new()))
    }

    #[test]
    fn config_path_respects_xdg_config_home() {
        let _guard = lock_env();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let fake_xdg = std::env::temp_dir().join(format!("plue-xdg-{unique}"));
        let _env = EnvVarGuard::set_path("XDG_CONFIG_HOME", &fake_xdg);

        let path = Config::config_path().expect("config path should resolve");

        assert_eq!(path, fake_xdg.join("plue").join("config.yml"));
    }

    // -----------------------------------------------------------------------
    // git_protocol
    // -----------------------------------------------------------------------

    #[test]
    fn git_protocol_defaults_to_ssh() {
        let config = Config::default();
        assert_eq!(config.git_protocol, GitProtocol::Ssh);
    }

    #[test]
    fn git_protocol_parses_https_from_file() {
        let _guard = lock_env();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fake_xdg = std::env::temp_dir().join(format!("plue-git-proto-{unique}"));
        std::fs::create_dir_all(fake_xdg.join("plue")).unwrap();
        std::fs::write(
            fake_xdg.join("plue").join("config.yml"),
            "api_url: https://plue.dev/api\ngit_protocol: https\n",
        )
        .unwrap();

        let _xdg = EnvVarGuard::set_path("XDG_CONFIG_HOME", &fake_xdg);
        let config = Config::load_raw().unwrap();
        assert_eq!(config.git_protocol, GitProtocol::Https);
    }

    #[test]
    fn git_protocol_invalid_value_fails_to_parse() {
        let _guard = lock_env();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fake_xdg = std::env::temp_dir().join(format!("plue-git-proto-bad-{unique}"));
        std::fs::create_dir_all(fake_xdg.join("plue")).unwrap();
        std::fs::write(
            fake_xdg.join("plue").join("config.yml"),
            "api_url: https://plue.dev/api\ngit_protocol: ftp\n",
        )
        .unwrap();

        let _xdg = EnvVarGuard::set_path("XDG_CONFIG_HOME", &fake_xdg);
        let err = Config::load_raw().expect_err("invalid git_protocol should fail");
        assert!(
            err.to_string().contains("git_protocol")
                || err.to_string().contains("unknown variant")
                || err.to_string().contains("failed to parse config file"),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // host()
    // -----------------------------------------------------------------------

    #[test]
    fn host_extracts_hostname() {
        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: None,
            git_protocol: GitProtocol::Ssh,
        };
        assert_eq!(config.host(), "plue.dev");
    }

    #[test]
    fn host_extracts_localhost() {
        let config = Config {
            api_url: "http://localhost:4000/api".into(),
            token: None,
            git_protocol: GitProtocol::Ssh,
        };
        assert_eq!(config.host(), "localhost");
    }

    // -----------------------------------------------------------------------
    // token_for_host()
    // -----------------------------------------------------------------------

    #[test]
    fn token_for_host_returns_none_when_nothing_set() {
        let _guard = lock_env();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: None,
            git_protocol: GitProtocol::Ssh,
        };
        let store = mock_store();
        assert!(config.token_for_host(&store).unwrap().is_none());
    }

    #[test]
    fn token_for_host_returns_config_file_token() {
        let _guard = lock_env();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: Some("plue_from_config".into()),
            git_protocol: GitProtocol::Ssh,
        };
        let store = mock_store();
        let resolved = config.token_for_host(&store).unwrap().unwrap();
        assert_eq!(resolved.token, "plue_from_config");
        assert_eq!(resolved.source, TokenSource::ConfigFile);
    }

    #[test]
    fn token_for_host_prefers_keyring_over_config() {
        let _guard = lock_env();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: Some("plue_config".into()),
            git_protocol: GitProtocol::Ssh,
        };
        let store = mock_store();
        store.store_token("plue.dev", "plue_keyring").unwrap();
        let resolved = config.token_for_host(&store).unwrap().unwrap();
        assert_eq!(resolved.token, "plue_keyring");
        assert_eq!(resolved.source, TokenSource::Keyring);
    }

    #[test]
    fn token_for_host_prefers_env_over_all() {
        let _guard = lock_env();
        let _env = EnvVarGuard::set("PLUE_TOKEN", "plue_env");

        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: Some("plue_config".into()),
            git_protocol: GitProtocol::Ssh,
        };
        let store = mock_store();
        store.store_token("plue.dev", "plue_keyring").unwrap();
        let resolved = config.token_for_host(&store).unwrap().unwrap();
        assert_eq!(resolved.token, "plue_env");
        assert_eq!(resolved.source, TokenSource::EnvVar);
    }

    // -----------------------------------------------------------------------
    // is_token_in_keyring()
    // -----------------------------------------------------------------------

    #[test]
    fn is_token_in_keyring_false_when_empty() {
        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: None,
            git_protocol: GitProtocol::Ssh,
        };
        let store = mock_store();
        assert!(!config.is_token_in_keyring(&store));
    }

    #[test]
    fn is_token_in_keyring_true_when_stored() {
        let config = Config {
            api_url: "https://plue.dev/api".into(),
            token: None,
            git_protocol: GitProtocol::Ssh,
        };
        let store = mock_store();
        store.store_token("plue.dev", "plue_x").unwrap();
        assert!(config.is_token_in_keyring(&store));
    }

    // -----------------------------------------------------------------------
    // load_raw() – does not overlay env
    // -----------------------------------------------------------------------

    #[test]
    fn load_raw_does_not_apply_env_overlay() {
        let _guard = lock_env();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fake_xdg = std::env::temp_dir().join(format!("plue-raw-{unique}"));
        std::fs::create_dir_all(fake_xdg.join("plue")).unwrap();
        std::fs::write(
            fake_xdg.join("plue").join("config.yml"),
            "api_url: https://plue.dev/api\n",
        )
        .unwrap();

        let _xdg = EnvVarGuard::set_path("XDG_CONFIG_HOME", &fake_xdg);
        let _tok = EnvVarGuard::set("PLUE_TOKEN", "plue_should_not_appear");

        let config = Config::load_raw().unwrap();
        assert!(config.token.is_none());
    }
}
