use crate::encryption::prompt_password;
use crate::logger::log_warning;
use crate::profile::ActiveProfile;
use crate::utils::paths::{get_security_config_path, get_secrets_dir};
use age::secrecy::{ExposeSecret, SecretString};
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
    password: String,
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
            Err(_) => Self::default(),
        }
    }

    pub fn load_or_create() -> Self {
        let path = get_security_config_path();
        if path.exists() {
            return Self::load_or_default();
        }

        let config = Self::default();
        let _ = config.save(&path);
        config
    }

    pub fn get_effective_password_cache(
        &self,
        profile: &ActiveProfile,
    ) -> (PasswordCacheConfig, bool) {
        if let Some(profile_name) = &profile.name {
            if let Some(profile_config) = self.profiles.get(profile_name) {
                if let Some(password_cache) = &profile_config.password_cache {
                    return (password_cache.clone(), true);
                }
            }
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

    if cache_config.enabled {
        if let Some(password) = load_cached_password(profile, cache_config.ttl, per_profile) {
            return Ok(password);
        }
    }

    let password = prompt_password(confirm)?;
    if cache_config.enabled {
        if let Err(e) = store_cached_password(profile, &password, per_profile) {
            log_warning(&format!(
                "Failed to update password cache (continuing without cache): {}",
                e
            ));
        }
    }

    Ok(password)
}

pub fn invalidate_password_cache(profile: &ActiveProfile) {
    let config = SecurityConfig::load_or_default();
    let (_cache_config, per_profile) = config.get_effective_password_cache(profile);
    let cache_path = get_password_cache_path(profile, per_profile);
    if cache_path.exists() {
        let _ = fs::remove_file(cache_path);
    }
}

fn get_password_cache_path(profile: &ActiveProfile, per_profile: bool) -> PathBuf {
    let secrets_dir = get_secrets_dir();
    if per_profile {
        let name = profile
            .name
            .as_deref()
            .unwrap_or("common")
            .replace('/', "_");
        secrets_dir.join(format!("{}.{}.age", PASSWORD_CACHE_FILE_PREFIX, name))
    } else {
        secrets_dir.join(format!("{}.global.age", PASSWORD_CACHE_FILE_PREFIX))
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

    let encrypted = match fs::read(&cache_path) {
        Ok(data) => data,
        Err(e) => {
            log_warning(&format!("Failed to read password cache: {}", e));
            return None;
        }
    };

    let decryptor = match Decryptor::new(&encrypted[..]) {
        Ok(decryptor) => decryptor,
        Err(e) => {
            log_warning(&format!("Failed to parse password cache: {}", e));
            return None;
        }
    };

    let mut decrypted = Vec::new();
    match decryptor {
        Decryptor::Recipients(d) => {
            let mut reader = match d.decrypt(std::iter::once(&identity as &dyn age::Identity)) {
                Ok(reader) => reader,
                Err(e) => {
                    log_warning(&format!("Failed to decrypt password cache: {}", e));
                    return None;
                }
            };
            if reader.read_to_end(&mut decrypted).is_err() {
                log_warning("Failed to read decrypted password cache");
                return None;
            }
        }
        _ => {
            log_warning("Unsupported password cache format");
            return None;
        }
    }

    let record: PasswordCacheRecord = match serde_json::from_slice(&decrypted) {
        Ok(record) => record,
        Err(e) => {
            log_warning(&format!("Failed to parse password cache record: {}", e));
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

    Some(SecretString::new(record.password.into()))
}

fn store_cached_password(
    profile: &ActiveProfile,
    password: &SecretString,
    per_profile: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = get_password_cache_path(profile, per_profile);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        if let Some(parent) = cache_path.parent() {
            let _ = fs::set_permissions(parent, perms);
        }
    }

    let identity = get_or_create_cache_identity()?;
    let recipient = identity.to_public();
    let encryptor = Encryptor::with_recipients(vec![Box::new(recipient)])?;

    let record = PasswordCacheRecord {
        created_at: Utc::now().timestamp(),
        password: password.expose_secret().to_string(),
    };
    let serialized = serde_json::to_vec(&record)?;

    let mut encrypted = Vec::new();
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(&serialized)?;
    writer.finish()?;

    fs::write(&cache_path, encrypted)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&cache_path, perms);
    }

    Ok(())
}

fn get_or_create_cache_identity() -> Result<age::x25519::Identity, Box<dyn std::error::Error>> {
    let secrets_dir = get_secrets_dir();
    fs::create_dir_all(&secrets_dir)?;

    let key_path = secrets_dir.join(PASSWORD_CACHE_KEY_FILE);
    if key_path.exists() {
        let key = fs::read_to_string(&key_path)?;
        let identity: age::x25519::Identity = key.parse()?;
        return Ok(identity);
    }

    let identity = age::x25519::Identity::generate();
    fs::write(&key_path, identity.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&key_path, perms);
    }

    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_cache_ttl_seconds() {
        assert_eq!(PasswordCacheTtl::FifteenMinutes.as_seconds(), Some(900));
        assert_eq!(PasswordCacheTtl::OneHour.as_seconds(), Some(3600));
        assert_eq!(PasswordCacheTtl::EightHours.as_seconds(), Some(28800));
        assert_eq!(PasswordCacheTtl::OneDay.as_seconds(), Some(86400));
        assert_eq!(PasswordCacheTtl::SevenDays.as_seconds(), Some(604800));
        assert_eq!(PasswordCacheTtl::Never.as_seconds(), None);
    }
}
