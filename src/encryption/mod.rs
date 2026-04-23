mod bundle;

use age::secrecy::ExposeSecret;
use age::secrecy::SecretString;
use anyhow::{Context, Result, bail};
use keyring::Entry;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

const KEYRING_SERVICE: &str = "mntn";
const KEYRING_USERNAME: &str = "encryption";

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
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).ok()?;
    let password = entry.get_password().ok()?;
    (!password.is_empty()).then_some(SecretString::new(password.into()))
}

fn offer_store_password(password: &SecretString) {
    print!("Store this password in the system keychain for next time? [y/N] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return;
    }
    let line = line.trim().to_ascii_lowercase();
    if !matches!(line.as_str(), "y" | "yes") {
        return;
    }
    let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_USERNAME) else {
        eprintln!("Could not access system keychain");
        return;
    };
    if let Err(e) = entry.set_password(password.expose_secret()) {
        eprintln!("Could not save password to system keychain: {e}");
    }
}

pub(crate) fn resolve_encryption_password(confirm: bool) -> Result<SecretString> {
    if !confirm && let Some(password) = read_stored_password() {
        return Ok(password);
    }
    let password = prompt_password(confirm)?;
    offer_store_password(&password);
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
