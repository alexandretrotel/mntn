use crate::encryption::prompt_password;
use crate::logger::log_warning;
use crate::profile::ActiveProfile;
use crate::utils::paths::{get_secrets_cache_dir, get_secrets_keys_dir, get_security_config_path};
use crate::utils::permissions::{lock_down_dir, lock_down_file};
use age::secrecy::{ExposeSecret, SecretBox, SecretString};
use age::{Decryptor, Encryptor};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const PASSWORD_CACHE_KEY_FILE: &str = "password-cache.key";
const PASSWORD_CACHE_FILE_PREFIX: &str = "password-cache";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub version: String,
    pub password_cache: PasswordCacheConfig,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileSecurityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSecurityConfig {
    #[serde(default)]
    pub password_cache: Option<PasswordCacheConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordCacheConfig {
    pub enabled: bool,
    pub ttl: PasswordCacheTtl,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PasswordCacheTtl {
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "8h")]
    EightHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "never")]
    Never,
}

impl PasswordCacheTtl {
    pub fn as_seconds(self) -> Option<i64> {
        match self {
            PasswordCacheTtl::FifteenMinutes => Some(15 * 60),
            PasswordCacheTtl::OneHour => Some(60 * 60),
            PasswordCacheTtl::EightHours => Some(8 * 60 * 60),
            PasswordCacheTtl::OneDay => Some(24 * 60 * 60),
            PasswordCacheTtl::SevenDays => Some(7 * 24 * 60 * 60),
            PasswordCacheTtl::Never => None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            password_cache: PasswordCacheConfig {
                enabled: true,
                ttl: PasswordCacheTtl::OneHour,
            },
            profiles: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PasswordCacheRecord {
    created_at: i64,
    #[serde(with = "secret_string_serde")]
    password: SecretString,
}

mod secret_string_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value.expose_secret())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(SecretString::new(value.into()))
    }
}

impl SecurityConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load_or_default() -> Self {
        let path = get_security_config_path();
        if !path.exists() {
            return Self::default();
        }

        match Self::load(&path) {
            Ok(config) => config,
            Err(e) => {
                log_warning(&format!(
                    "Failed to load security configuration from '{}': {}. Falling back to defaults.",
                    path.display(),
                    e
                ));
                Self::default()
            }
        }
    }

    pub fn load_or_create() -> Self {
        let path = get_security_config_path();
        if path.exists() {
            return Self::load_or_default();
        }

        let config = Self::default();
        if let Err(e) = config.save(&path) {
            log_warning(&format!("Failed to save security config: {}", e));
        }
        config
    }

    pub fn get_effective_password_cache(
        &self,
        profile: &ActiveProfile,
    ) -> (PasswordCacheConfig, bool) {
        if let Some(profile_name) = &profile.name
            && let Some(profile_config) = self.profiles.get(profile_name)
            && let Some(password_cache) = &profile_config.password_cache
        {
            return (password_cache.clone(), true);
        }

        (self.password_cache.clone(), false)
    }
}

pub fn get_or_prompt_password(
    profile: &ActiveProfile,
    confirm: bool,
) -> Result<SecretString, Box<dyn std::error::Error>> {
    let config = SecurityConfig::load_or_create();
    let (cache_config, per_profile) = config.get_effective_password_cache(profile);

    if cache_config.enabled
        && let Some(password) = load_cached_password(profile, cache_config.ttl, per_profile)
    {
        return Ok(password);
    }

    let password = prompt_password(confirm)?;
    if cache_config.enabled
        && let Err(e) = store_cached_password(profile, &password, per_profile)
    {
        log_warning(&format!(
            "Failed to update password cache (continuing without cache): {}",
            e
        ));
    }

    Ok(password)
}

pub fn invalidate_password_cache(profile: &ActiveProfile) {
    let cache_paths = [
        get_password_cache_path(profile, true),
        get_password_cache_path(profile, false),
    ];
    for cache_path in cache_paths {
        if cache_path.exists() {
            let _ = fs::remove_file(cache_path);
        }
    }
}

fn get_password_cache_path(profile: &ActiveProfile, per_profile: bool) -> PathBuf {
    let secrets_dir = get_secrets_cache_dir();
    if per_profile {
        let name = sanitize_profile_name(profile.name.as_deref().unwrap_or("common"));
        secrets_dir.join(format!("{}.{}.age", PASSWORD_CACHE_FILE_PREFIX, name))
    } else {
        secrets_dir.join(format!("{}.global.age", PASSWORD_CACHE_FILE_PREFIX))
    }
}

fn sanitize_profile_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        "common".to_string()
    } else {
        sanitized
    }
}

fn load_cached_password(
    profile: &ActiveProfile,
    ttl: PasswordCacheTtl,
    per_profile: bool,
) -> Option<SecretString> {
    let cache_path = get_password_cache_path(profile, per_profile);
    if !cache_path.exists() {
        return None;
    }

    let identity = match get_or_create_cache_identity() {
        Ok(identity) => identity,
        Err(e) => {
            log_warning(&format!("Failed to load password cache identity: {}", e));
            return None;
        }
    };

    let record = match read_cached_password_record(&cache_path, &identity) {
        Ok(record) => record,
        Err(err) => {
            log_warning(&format!("Failed to read password cache record: {}", err));
            let _ = fs::remove_file(&cache_path);
            return None;
        }
    };

    if let Some(ttl_seconds) = ttl.as_seconds() {
        let now = Utc::now().timestamp();
        if now.saturating_sub(record.created_at) > ttl_seconds {
            let _ = fs::remove_file(&cache_path);
            return None;
        }
    }

    Some(record.password)
}

fn store_cached_password(
    profile: &ActiveProfile,
    password: &SecretString,
    per_profile: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = get_password_cache_path(profile, per_profile);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
        if let Err(e) = lock_down_dir(parent) {
            log_warning(&format!("Failed to lock down cache directory: {}", e));
        }
    }

    let identity = get_or_create_cache_identity()?;
    let recipient = identity.to_public();
    let encryptor = Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))?;

    let record = PasswordCacheRecord {
        created_at: Utc::now().timestamp(),
        password: password.clone(),
    };
    let serialized = SecretBox::new(Box::new(serde_json::to_vec(&record)?));

    let mut tmp_path = cache_path.with_extension("tmp");
    if tmp_path.exists() {
        tmp_path = cache_path.with_extension(format!("tmp.{}", std::process::id()));
    }

    let write_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut tmp_file = fs::File::create(&tmp_path)?;
        let mut writer = encryptor.wrap_output(&mut tmp_file)?;
        writer.write_all(serialized.expose_secret())?;
        writer.finish()?;
        drop(serialized);
        tmp_file.sync_all()?;

        if let Err(e) = lock_down_file(&tmp_path) {
            log_warning(&format!("Failed to lock down cache file: {}", e));
        }

        #[cfg(windows)]
        {
            if cache_path.exists() {
                let _ = fs::remove_file(&cache_path);
            }
        }
        fs::rename(&tmp_path, &cache_path)?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    write_result
}

fn get_or_create_cache_identity() -> Result<age::x25519::Identity, Box<dyn std::error::Error>> {
    let keys_dir = get_secrets_keys_dir();
    fs::create_dir_all(&keys_dir)?;
    if let Err(e) = lock_down_dir(&keys_dir) {
        log_warning(&format!("Failed to lock down key directory: {}", e));
    }

    let key_path = keys_dir.join(PASSWORD_CACHE_KEY_FILE);
    if key_path.exists() {
        let key = fs::read_to_string(&key_path).map_err(|e| {
            log_warning(&format!("Failed to read cache key: {}", e));
            e
        })?;
        let identity: age::x25519::Identity = key.parse().inspect_err(|&e| {
            log_warning(&format!("Failed to parse cache key: {}", e));
        })?;
        return Ok(identity);
    }

    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&key_path)
    {
        Ok(mut file) => {
            let identity = age::x25519::Identity::generate();
            let key = identity.to_string();
            file.write_all(key.expose_secret().as_bytes())
                .map_err(|e| {
                    log_warning(&format!("Failed to write cache key: {}", e));
                    e
                })?;
            file.sync_all().map_err(|e| {
                log_warning(&format!("Failed to sync cache key: {}", e));
                e
            })?;
            if let Err(e) = lock_down_file(&key_path) {
                log_warning(&format!("Failed to lock down cache key file: {}", e));
            }
            Ok(identity)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let key = fs::read_to_string(&key_path).map_err(|e| {
                log_warning(&format!("Failed to read cache key: {}", e));
                e
            })?;
            let identity: age::x25519::Identity = key.parse().inspect_err(|&e| {
                log_warning(&format!("Failed to parse cache key: {}", e));
            })?;
            Ok(identity)
        }
        Err(e) => {
            log_warning(&format!("Failed to create cache key: {}", e));
            Err(e.into())
        }
    }
}

fn read_cached_password_record(
    cache_path: &Path,
    identity: &age::x25519::Identity,
) -> Result<PasswordCacheRecord, String> {
    let mut last_error = None;

    for attempt in 0..2 {
        let encrypted = fs::read(cache_path).map_err(|e| {
            last_error = Some(format!("Failed to read password cache: {}", e));
            last_error.clone().unwrap()
        })?;

        let decryptor = match Decryptor::new(&encrypted[..]) {
            Ok(decryptor) => decryptor,
            Err(e) => {
                last_error = Some(format!("Failed to parse password cache: {}", e));
                if attempt == 0 {
                    continue;
                }
                return Err(last_error.clone().unwrap());
            }
        };

        let mut decrypted = Vec::new();
        let mut reader = match decryptor.decrypt(std::iter::once(identity as &dyn age::Identity)) {
            Ok(reader) => reader,
            Err(e) => {
                last_error = Some(format!("Failed to decrypt password cache: {}", e));
                if attempt == 0 {
                    continue;
                }
                return Err(last_error.clone().unwrap());
            }
        };
        if reader.read_to_end(&mut decrypted).is_err() {
            last_error = Some("Failed to read decrypted password cache".to_string());
            if attempt == 0 {
                continue;
            }
            return Err(last_error.clone().unwrap());
        }

        let decrypted = SecretBox::new(Box::new(decrypted));
        let record: PasswordCacheRecord = match serde_json::from_slice(decrypted.expose_secret()) {
            Ok(record) => record,
            Err(e) => {
                last_error = Some(format!("Failed to parse password cache record: {}", e));
                if attempt == 0 {
                    continue;
                }
                return Err(last_error.clone().unwrap());
            }
        };
        drop(decrypted);
        return Ok(record);
    }

    Err(last_error.unwrap_or_else(|| "Failed to read password cache".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce()>(f: F) {
        let _lock = TEST_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let old_home = env::var_os("HOME");
        let old_userprofile = env::var_os("USERPROFILE");

        struct EnvGuard {
            old_home: Option<std::ffi::OsString>,
            old_userprofile: Option<std::ffi::OsString>,
        }

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match self.old_home.take() {
                        Some(value) => env::set_var("HOME", value),
                        None => env::remove_var("HOME"),
                    }
                    match self.old_userprofile.take() {
                        Some(value) => env::set_var("USERPROFILE", value),
                        None => env::remove_var("USERPROFILE"),
                    }
                }
            }
        }

        unsafe {
            env::set_var("HOME", temp_dir.path());
            env::set_var("USERPROFILE", temp_dir.path());
        }

        let _guard = EnvGuard {
            old_home,
            old_userprofile,
        };

        f();
    }

    fn write_cache_record(
        profile: &ActiveProfile,
        per_profile: bool,
        created_at: i64,
        password: &str,
    ) {
        let cache_path = get_password_cache_path(profile, per_profile);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let identity = get_or_create_cache_identity().unwrap();
        let recipient = identity.to_public();
        let encryptor =
            Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient)).unwrap();

        let record = PasswordCacheRecord {
            created_at,
            password: SecretString::new(password.to_string().into()),
        };
        let serialized = serde_json::to_vec(&record).unwrap();

        let mut encrypted = Vec::new();
        let mut writer = encryptor.wrap_output(&mut encrypted).unwrap();
        writer.write_all(&serialized).unwrap();
        writer.finish().unwrap();

        fs::write(&cache_path, encrypted).unwrap();
    }

    #[test]
    fn test_password_cache_ttl_seconds() {
        assert_eq!(PasswordCacheTtl::FifteenMinutes.as_seconds(), Some(900));
        assert_eq!(PasswordCacheTtl::OneHour.as_seconds(), Some(3600));
        assert_eq!(PasswordCacheTtl::EightHours.as_seconds(), Some(28800));
        assert_eq!(PasswordCacheTtl::OneDay.as_seconds(), Some(86400));
        assert_eq!(PasswordCacheTtl::SevenDays.as_seconds(), Some(604800));
        assert_eq!(PasswordCacheTtl::Never.as_seconds(), None);
    }

    #[test]
    fn test_password_cache_roundtrip() {
        with_temp_home(|| {
            let profile = ActiveProfile::with_profile("test-profile");
            let password = SecretString::new("test-pass".to_string().into());

            store_cached_password(&profile, &password, true).unwrap();

            let loaded =
                load_cached_password(&profile, PasswordCacheTtl::Never, true).expect("missing");
            assert_eq!(loaded.expose_secret(), "test-pass");
        });
    }

    #[test]
    fn test_password_cache_ttl_expiry_removes_cache() {
        with_temp_home(|| {
            let profile = ActiveProfile::with_profile("test-profile");
            let ttl_seconds = PasswordCacheTtl::OneHour.as_seconds().unwrap();
            let expired_at = Utc::now().timestamp() - ttl_seconds - 1;

            let password = format!("expired-pass-{}-{}", std::process::id(), expired_at);
            write_cache_record(&profile, true, expired_at, &password);

            let cache_path = get_password_cache_path(&profile, true);
            assert!(cache_path.exists());

            let loaded = load_cached_password(&profile, PasswordCacheTtl::OneHour, true);
            assert!(loaded.is_none());
            assert!(!cache_path.exists());
        });
    }

    #[test]
    fn test_invalidate_password_cache_removes_all_paths() {
        with_temp_home(|| {
            let profile = ActiveProfile::with_profile("test-profile");
            let password = SecretString::new("test-pass".to_string().into());

            store_cached_password(&profile, &password, true).unwrap();
            store_cached_password(&profile, &password, false).unwrap();

            let per_profile_path = get_password_cache_path(&profile, true);
            let global_path = get_password_cache_path(&profile, false);
            assert!(per_profile_path.exists());
            assert!(global_path.exists());

            invalidate_password_cache(&profile);

            assert!(!per_profile_path.exists());
            assert!(!global_path.exists());
        });
    }
}
