mod bundle;

use age::secrecy::ExposeSecret;
use age::secrecy::SecretString;
use anyhow::{Context, Result, bail};
use keyring_core::{Entry, Error as KeyringError};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::OnceLock;

const KEYRING_SERVICE: &str = "mntn";
const KEYRING_USERNAME: &str = "encryption";

fn keyring_entry() -> anyhow::Result<Entry> {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        keyring::use_native_store(false).ok();
    });
    Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).map_err(anyhow::Error::from)
}

pub(crate) use bundle::{
    create_temp_path, load_tar_member_map, set_private_file_permissions, write_entries_tar,
};

pub(crate) fn prompt_password(confirm: bool) -> Result<SecretString> {
    let password =
        rpassword::prompt_password("Enter encryption password: ").context("Read password")?;

    if password.is_empty() {
        bail!("Password cannot be empty");
    }

    if confirm {
        let confirmation = rpassword::prompt_password("Confirm encryption password: ")
            .context("Read password confirmation")?;
        if password != confirmation {
            bail!("Passwords do not match");
        }
    }

    Ok(SecretString::new(password.into()))
}

fn read_stored_password() -> Option<SecretString> {
    let entry = keyring_entry().ok()?;
    let password = entry.get_password().ok()?;
    (!password.is_empty()).then_some(SecretString::new(password.into()))
}

pub(crate) fn persist_encryption_password() -> Result<()> {
    let password = prompt_password(true).context("Read encryption password for system keychain")?;
    let entry = keyring_entry().context("Open system keychain")?;
    entry
        .set_password(password.expose_secret())
        .context("Save encryption password to system keychain")?;
    Ok(())
}

pub(crate) fn clear_stored_encryption_password() -> Result<()> {
    let entry = keyring_entry().context("Open system keychain")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => {
            Err(anyhow::Error::from(e)).context("Remove encryption password from system keychain")
        }
    }
}

pub(crate) fn resolve_encryption_password(
    ask_password: bool,
    confirm_on_prompt: bool,
) -> Result<SecretString> {
    let stored = read_stored_password();
    let had_stored = stored.is_some();
    if !ask_password && let Some(password) = stored {
        return Ok(password);
    }
    let password = prompt_password(confirm_on_prompt)?;
    if !had_stored {
        eprintln!(
            "Tip: run `mntn secret set` to save this password in your system keychain and skip prompts later."
        );
    }
    Ok(password)
}

pub(crate) fn encrypt_file(source: &Path, dest: &Path, password: &SecretString) -> Result<()> {
    let content = fs::read(source)
        .with_context(|| format!("Read source file for encryption: {}", source.display()))?;

    let encryptor = age::Encryptor::with_user_passphrase(password.clone());

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Create parent directory: {}", parent.display()))?;
    }

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .context("Initialize encryptor")?;
    writer
        .write_all(&content)
        .context("Write encrypted content")?;
    writer.finish().context("Finalize encryption output")?;

    fs::write(dest, encrypted)
        .with_context(|| format!("Write encrypted file: {}", dest.display()))?;
    Ok(())
}

pub(crate) fn decrypt_file(source: &Path, dest: &Path, password: &SecretString) -> Result<()> {
    let encrypted = fs::read(source)
        .with_context(|| format!("Read encrypted file for decryption: {}", source.display()))?;

    let decryptor = age::Decryptor::new(&encrypted[..]).context("Create decryptor")?;

    let identity = age::scrypt::Identity::new(password.clone());

    let mut decrypted = vec![];
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .context("Decrypt payload")?;
    reader
        .read_to_end(&mut decrypted)
        .context("Read decrypted payload")?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Create parent directory: {}", parent.display()))?;
    }

    fs::write(dest, decrypted)
        .with_context(|| format!("Write decrypted file: {}", dest.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        fs::set_permissions(dest, permissions)
            .with_context(|| format!("Set permissions on: {}", dest.display()))?;
    }

    Ok(())
}

pub(crate) fn get_encrypted_path(source_path: &str) -> String {
    format!("{}.age", source_path)
}
