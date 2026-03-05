use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{Context, Result};

// ---------------------------------------------------------------------------
// TokenSource – where a resolved token came from
// ---------------------------------------------------------------------------

/// Describes where a resolved token was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    EnvVar,
    Keyring,
    ConfigFile,
}

impl fmt::Display for TokenSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenSource::EnvVar => write!(f, "PLUE_TOKEN env"),
            TokenSource::Keyring => write!(f, "keyring"),
            TokenSource::ConfigFile => write!(f, "config file"),
        }
    }
}

/// A token together with its source.
#[derive(Debug, Clone)]
pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}

// ---------------------------------------------------------------------------
// TokenStorage trait – abstraction over keyring backends
// ---------------------------------------------------------------------------

/// Trait abstracting secure token storage (keyring, mock, etc.).
pub trait TokenStorage: Send + Sync {
    fn store(&self, host: &str, token: &str) -> Result<()>;
    fn get(&self, host: &str) -> Result<Option<String>>;
    fn delete(&self, host: &str) -> Result<()>;
    fn has(&self, host: &str) -> bool;
}

const TEST_STORE_FILE_ENV: &str = "PLUE_TEST_CREDENTIAL_STORE_FILE";

// ---------------------------------------------------------------------------
// KeyringStore – real OS keychain backend
// ---------------------------------------------------------------------------

/// Stores tokens in the OS keychain via the `keyring` crate.
pub struct KeyringStore;

impl KeyringStore {
    fn entry(host: &str) -> Result<keyring::Entry> {
        keyring::Entry::new("plue-cli", host).context("failed to create keyring entry")
    }
}

impl TokenStorage for KeyringStore {
    fn store(&self, host: &str, token: &str) -> Result<()> {
        Self::entry(host)?
            .set_password(token)
            .context("failed to store token in keyring")
    }

    fn get(&self, host: &str) -> Result<Option<String>> {
        match Self::entry(host) {
            Ok(entry) => match entry.get_password() {
                Ok(pw) => Ok(Some(pw)),
                Err(keyring::Error::NoEntry) => Ok(None),
                // Gracefully degrade when keyring is unavailable (CI, headless, etc.)
                Err(keyring::Error::PlatformFailure(_)) => Ok(None),
                Err(e) => Err(anyhow::anyhow!("keyring error: {e}")),
            },
            Err(_) => Ok(None), // entry creation failed, skip keyring
        }
    }

    fn delete(&self, host: &str) -> Result<()> {
        match Self::entry(host) {
            Ok(entry) => match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                // Gracefully degrade when keyring is unavailable
                Err(keyring::Error::PlatformFailure(_)) => Ok(()),
                Err(e) => Err(anyhow::anyhow!("failed to delete from keyring: {e}")),
            },
            Err(_) => Ok(()), // entry creation failed, nothing to delete
        }
    }

    fn has(&self, host: &str) -> bool {
        Self::entry(host)
            .ok()
            .and_then(|e| e.get_password().ok())
            .is_some()
    }
}

// ---------------------------------------------------------------------------
// MockStore – in-memory backend for tests
// ---------------------------------------------------------------------------

/// In-memory token store for unit testing.
pub struct MockStore {
    tokens: Mutex<HashMap<String, String>>,
}

impl MockStore {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStorage for MockStore {
    fn store(&self, host: &str, token: &str) -> Result<()> {
        self.tokens
            .lock()
            .unwrap()
            .insert(host.to_string(), token.to_string());
        Ok(())
    }

    fn get(&self, host: &str) -> Result<Option<String>> {
        Ok(self.tokens.lock().unwrap().get(host).cloned())
    }

    fn delete(&self, host: &str) -> Result<()> {
        self.tokens.lock().unwrap().remove(host);
        Ok(())
    }

    fn has(&self, host: &str) -> bool {
        self.tokens.lock().unwrap().contains_key(host)
    }
}

// ---------------------------------------------------------------------------
// FileStore – deterministic backend for integration tests
// ---------------------------------------------------------------------------

/// File-backed token store used only when `PLUE_TEST_CREDENTIAL_STORE_FILE` is set.
/// This keeps CLI integration tests deterministic across environments without
/// requiring an OS keychain.
struct FileStore {
    path: PathBuf,
}

impl FileStore {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn load_tokens(&self) -> Result<HashMap<String, String>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let raw = std::fs::read_to_string(&self.path).with_context(|| {
            format!(
                "failed to read test credential store file {}",
                self.path.display()
            )
        })?;
        if raw.trim().is_empty() {
            return Ok(HashMap::new());
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse test credential store file {}",
                self.path.display()
            )
        })
    }

    fn save_tokens(&self, tokens: &HashMap<String, String>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create test credential store directory {}",
                    parent.display()
                )
            })?;
        }

        let raw = serde_json::to_string_pretty(tokens)
            .context("failed to serialize test credential store contents")?;
        std::fs::write(&self.path, raw).with_context(|| {
            format!(
                "failed to write test credential store file {}",
                self.path.display()
            )
        })
    }
}

impl TokenStorage for FileStore {
    fn store(&self, host: &str, token: &str) -> Result<()> {
        let mut tokens = self.load_tokens()?;
        tokens.insert(host.to_string(), token.to_string());
        self.save_tokens(&tokens)
    }

    fn get(&self, host: &str) -> Result<Option<String>> {
        Ok(self.load_tokens()?.get(host).cloned())
    }

    fn delete(&self, host: &str) -> Result<()> {
        let mut tokens = self.load_tokens()?;
        tokens.remove(host);
        self.save_tokens(&tokens)
    }

    fn has(&self, host: &str) -> bool {
        self.get(host).ok().flatten().is_some()
    }
}

// ---------------------------------------------------------------------------
// CredentialStore – unified façade
// ---------------------------------------------------------------------------

/// Facade over a `TokenStorage` backend. Use `new()` for production (OS
/// keyring) or `with_backend()` for tests (MockStore).
pub struct CredentialStore {
    backend: Box<dyn TokenStorage>,
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore {
    /// Production constructor – uses the OS keychain.
    pub fn new() -> Self {
        if let Some(path) = std::env::var_os(TEST_STORE_FILE_ENV) {
            let path = PathBuf::from(path);
            if !path.as_os_str().is_empty() {
                return Self {
                    backend: Box::new(FileStore::new(path)),
                };
            }
        }

        Self {
            backend: Box::new(KeyringStore),
        }
    }

    /// Test constructor – inject any backend.
    pub fn with_backend(backend: Box<dyn TokenStorage>) -> Self {
        Self { backend }
    }

    pub fn store_token(&self, host: &str, token: &str) -> Result<()> {
        self.backend.store(host, token)
    }

    pub fn get_token(&self, host: &str) -> Result<Option<String>> {
        self.backend.get(host)
    }

    pub fn delete_token(&self, host: &str) -> Result<()> {
        self.backend.delete(host)
    }

    pub fn has_token(&self, host: &str) -> bool {
        self.backend.has(host)
    }
}

// ---------------------------------------------------------------------------
// Helper: resolve token via priority chain
// ---------------------------------------------------------------------------

/// Resolve a token using the priority chain: env var → keyring → config file.
pub fn resolve_token(
    host: &str,
    store: &CredentialStore,
    config_token: &Option<String>,
) -> Result<Option<ResolvedToken>> {
    // 1. PLUE_TOKEN env var always wins
    if let Ok(env_token) = std::env::var("PLUE_TOKEN") {
        if !env_token.is_empty() {
            return Ok(Some(ResolvedToken {
                token: env_token,
                source: TokenSource::EnvVar,
            }));
        }
    }

    // 2. Keyring
    if let Some(keyring_token) = store.get_token(host)? {
        return Ok(Some(ResolvedToken {
            token: keyring_token,
            source: TokenSource::Keyring,
        }));
    }

    // 3. Config file token
    if let Some(ref file_token) = config_token {
        if !file_token.is_empty() {
            return Ok(Some(ResolvedToken {
                token: file_token.clone(),
                source: TokenSource::ConfigFile,
            }));
        }
    }

    Ok(None)
}

/// Extract a hostname from an API base URL.
///
/// ```text
/// "https://plue.dev/api" -> "plue.dev"
/// "http://localhost:4000/api" -> "localhost"
/// ```
pub fn host_from_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("plue.dev")
        .split(':')
        .next()
        .unwrap_or("plue.dev")
        .to_string()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex as StdMutex;

    // Serialize env-var mutations so parallel tests don't race.
    static ENV_LOCK: StdMutex<()> = StdMutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn mock_store() -> CredentialStore {
        CredentialStore::with_backend(Box::new(MockStore::new()))
    }

    // -----------------------------------------------------------------------
    // TokenSource Display
    // -----------------------------------------------------------------------

    #[test]
    fn token_source_display() {
        assert_eq!(TokenSource::EnvVar.to_string(), "PLUE_TOKEN env");
        assert_eq!(TokenSource::Keyring.to_string(), "keyring");
        assert_eq!(TokenSource::ConfigFile.to_string(), "config file");
    }

    // -----------------------------------------------------------------------
    // MockStore basic operations
    // -----------------------------------------------------------------------

    #[test]
    fn test_store_and_retrieve_token() {
        let store = mock_store();
        store.store_token("host", "plue_abc").unwrap();
        assert_eq!(store.get_token("host").unwrap(), Some("plue_abc".into()));
    }

    #[test]
    fn test_get_token_returns_none_for_missing() {
        let store = mock_store();
        assert_eq!(store.get_token("missing-host").unwrap(), None);
    }

    #[test]
    fn test_delete_token() {
        let store = mock_store();
        store.store_token("host", "plue_abc").unwrap();
        assert!(store.has_token("host"));
        store.delete_token("host").unwrap();
        assert!(!store.has_token("host"));
        assert_eq!(store.get_token("host").unwrap(), None);
    }

    #[test]
    fn test_has_token() {
        let store = mock_store();
        assert!(!store.has_token("host"));
        store.store_token("host", "plue_abc").unwrap();
        assert!(store.has_token("host"));
    }

    #[test]
    fn mock_store_delete_nonexistent_is_ok() {
        let store = mock_store();
        assert!(store.delete_token("nope").is_ok());
    }

    #[test]
    fn mock_store_overwrite() {
        let store = mock_store();
        store.store_token("h", "a").unwrap();
        store.store_token("h", "b").unwrap();
        assert_eq!(store.get_token("h").unwrap(), Some("b".into()));
    }

    #[test]
    fn mock_store_multiple_hosts() {
        let store = mock_store();
        store.store_token("alpha", "t1").unwrap();
        store.store_token("beta", "t2").unwrap();
        assert_eq!(store.get_token("alpha").unwrap(), Some("t1".into()));
        assert_eq!(store.get_token("beta").unwrap(), Some("t2".into()));
    }

    // -----------------------------------------------------------------------
    // host_from_url
    // -----------------------------------------------------------------------

    #[test]
    fn host_from_url_https() {
        assert_eq!(host_from_url("https://plue.dev/api"), "plue.dev");
    }

    #[test]
    fn host_from_url_http_with_port() {
        assert_eq!(host_from_url("http://localhost:4000/api"), "localhost");
    }

    #[test]
    fn host_from_url_bare() {
        assert_eq!(host_from_url("example.com/api"), "example.com");
    }

    #[test]
    fn host_from_url_no_path() {
        assert_eq!(host_from_url("https://api.plue.dev"), "api.plue.dev");
    }

    // -----------------------------------------------------------------------
    // resolve_token priority chain
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_token_env_var_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _env = EnvVarGuard::set("PLUE_TOKEN", "plue_from_env");

        let store = mock_store();
        store.store_token("h", "plue_keyring").unwrap();
        let config_token = Some("plue_config".into());

        let resolved = resolve_token("h", &store, &config_token).unwrap().unwrap();
        assert_eq!(resolved.token, "plue_from_env");
        assert_eq!(resolved.source, TokenSource::EnvVar);
    }

    #[test]
    fn resolve_token_keyring_second() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let store = mock_store();
        store.store_token("h", "plue_keyring").unwrap();
        let config_token = Some("plue_config".into());

        let resolved = resolve_token("h", &store, &config_token).unwrap().unwrap();
        assert_eq!(resolved.token, "plue_keyring");
        assert_eq!(resolved.source, TokenSource::Keyring);
    }

    #[test]
    fn resolve_token_config_file_last() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let store = mock_store();
        let config_token = Some("plue_config".into());

        let resolved = resolve_token("h", &store, &config_token).unwrap().unwrap();
        assert_eq!(resolved.token, "plue_config");
        assert_eq!(resolved.source, TokenSource::ConfigFile);
    }

    #[test]
    fn resolve_token_none_when_empty() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let store = mock_store();
        let resolved = resolve_token("h", &store, &None).unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_token_skips_empty_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _env = EnvVarGuard::set("PLUE_TOKEN", "");

        let store = mock_store();
        store.store_token("h", "plue_keyring").unwrap();

        let resolved = resolve_token("h", &store, &None).unwrap().unwrap();
        assert_eq!(resolved.source, TokenSource::Keyring);
    }

    #[test]
    fn resolve_token_skips_empty_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _env = EnvVarGuard::remove("PLUE_TOKEN");

        let store = mock_store();
        let resolved = resolve_token("h", &store, &Some("".into())).unwrap();
        assert!(resolved.is_none());
    }

    // -----------------------------------------------------------------------
    // FileStore – file-backed credential store for integration tests
    // -----------------------------------------------------------------------

    fn file_store(dir: &std::path::Path) -> CredentialStore {
        let path = dir.join("cred-store.json");
        CredentialStore::with_backend(Box::new(FileStore::new(path)))
    }

    #[test]
    fn file_store_crud() {
        let tmp = tempfile::tempdir().unwrap();
        let store = file_store(tmp.path());

        assert!(!store.has_token("host"));
        assert_eq!(store.get_token("host").unwrap(), None);

        store.store_token("host", "plue_abc").unwrap();
        assert!(store.has_token("host"));
        assert_eq!(store.get_token("host").unwrap(), Some("plue_abc".into()));

        store.delete_token("host").unwrap();
        assert!(!store.has_token("host"));
        assert_eq!(store.get_token("host").unwrap(), None);
    }

    #[test]
    fn file_store_persists_across_instances() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("persist.json");

        // Store in one instance
        let store1 = CredentialStore::with_backend(Box::new(FileStore::new(path.clone())));
        store1.store_token("host", "plue_persist").unwrap();

        // Read from a new instance pointing to the same file
        let store2 = CredentialStore::with_backend(Box::new(FileStore::new(path)));
        assert_eq!(
            store2.get_token("host").unwrap(),
            Some("plue_persist".into())
        );
    }

    #[test]
    fn file_store_delete_nonexistent_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let store = file_store(tmp.path());
        assert!(store.delete_token("nope").is_ok());
    }

    #[test]
    fn file_store_multiple_hosts() {
        let tmp = tempfile::tempdir().unwrap();
        let store = file_store(tmp.path());

        store.store_token("alpha", "t1").unwrap();
        store.store_token("beta", "t2").unwrap();
        assert_eq!(store.get_token("alpha").unwrap(), Some("t1".into()));
        assert_eq!(store.get_token("beta").unwrap(), Some("t2".into()));
    }

    #[test]
    fn file_store_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let store = file_store(tmp.path());

        store.store_token("h", "a").unwrap();
        store.store_token("h", "b").unwrap();
        assert_eq!(store.get_token("h").unwrap(), Some("b".into()));
    }
}
