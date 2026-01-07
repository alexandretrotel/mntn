use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Prompts the user for a password securely (input is hidden)
/// If `confirm` is true, asks for password confirmation
pub fn prompt_password(confirm: bool) -> Result<String, Box<dyn std::error::Error>> {
    let password = rpassword::prompt_password("Enter encryption password: ")?;

    if password.is_empty() {
        return Err("Password cannot be empty".into());
    }

    if confirm {
        let confirmation = rpassword::prompt_password("Confirm encryption password: ")?;
        if password != confirmation {
            return Err("Passwords do not match".into());
        }
    }

    Ok(password)
}

/// Encrypts a file using age with password-based encryption
pub fn encrypt_file(
    source: &Path,
    dest: &Path,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read(source)?;

    let encryptor =
        age::Encryptor::with_user_passphrase(age::secrecy::Secret::new(password.to_string()));

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(&content)?;
    writer.finish()?;

    fs::write(dest, encrypted)?;
    Ok(())
}

/// Decrypts a file using age with password-based encryption
pub fn decrypt_file(
    source: &Path,
    dest: &Path,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let encrypted = fs::read(source)?;

    let decryptor = match age::Decryptor::new(&encrypted[..])? {
        age::Decryptor::Passphrase(d) => d,
        _ => return Err("Unexpected decryptor type".into()),
    };

    let mut decrypted = vec![];
    let mut reader = decryptor.decrypt(&age::secrecy::Secret::new(password.to_string()), None)?;
    reader.read_to_end(&mut decrypted)?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(dest, decrypted)?;

    // Set restrictive permissions on sensitive files (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        fs::set_permissions(dest, permissions)?;
    }

    Ok(())
}

/// Encrypts a filename using SHA256 hash and base64 encoding
/// Returns a deterministic, filesystem-safe string
pub fn encrypt_filename(filename: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(filename.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Gets the encrypted file path based on source path and encryption settings
/// If encrypt_names is true, the filename portion is hashed
/// Always appends .age extension
pub fn get_encrypted_path(source_path: &str, encrypt_names: bool) -> String {
    if encrypt_names {
        // Hash the entire path to create a unique, unreadable filename
        format!("{}.age", encrypt_filename(source_path))
    } else {
        // Keep the original path structure but add .age extension
        format!("{}.age", source_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let encrypted = temp_dir.path().join("encrypted.age");
        let decrypted = temp_dir.path().join("decrypted.txt");

        let original_content = b"Hello, this is secret content!";
        fs::write(&source, original_content).unwrap();

        let password = "test-password-123";

        encrypt_file(&source, &encrypted, password).unwrap();
        assert!(encrypted.exists());

        // Encrypted content should be different from original
        let encrypted_content = fs::read(&encrypted).unwrap();
        assert_ne!(encrypted_content, original_content);

        decrypt_file(&encrypted, &decrypted, password).unwrap();
        assert!(decrypted.exists());

        let decrypted_content = fs::read(&decrypted).unwrap();
        assert_eq!(decrypted_content, original_content);
    }

    #[test]
    fn test_decrypt_wrong_password_fails() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let encrypted = temp_dir.path().join("encrypted.age");
        let decrypted = temp_dir.path().join("decrypted.txt");

        fs::write(&source, b"secret content").unwrap();

        encrypt_file(&source, &encrypted, "correct-password").unwrap();
        let result = decrypt_file(&encrypted, &decrypted, "wrong-password");

        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_filename() {
        let filename = "ssh/id_ed25519";
        let encrypted = encrypt_filename(filename);

        // Should be deterministic
        assert_eq!(encrypted, encrypt_filename(filename));

        // Should be different from original
        assert_ne!(encrypted, filename);

        // Should be base64 URL-safe (no special chars except - and _)
        assert!(
            encrypted
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn test_get_encrypted_path_without_name_encryption() {
        let path = get_encrypted_path("ssh/config", false);
        assert_eq!(path, "ssh/config.age");
    }

    #[test]
    fn test_get_encrypted_path_with_name_encryption() {
        let path = get_encrypted_path("ssh/config", true);
        assert!(path.ends_with(".age"));
        assert!(!path.contains("ssh"));
        assert!(!path.contains("config"));
    }

    #[test]
    fn test_encrypt_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("binary.bin");
        let encrypted = temp_dir.path().join("encrypted.age");
        let decrypted = temp_dir.path().join("decrypted.bin");

        // Binary content with non-UTF8 bytes
        let binary_content: Vec<u8> = (0..=255).collect();
        fs::write(&source, &binary_content).unwrap();

        let password = "binary-test";
        encrypt_file(&source, &encrypted, password).unwrap();
        decrypt_file(&encrypted, &decrypted, password).unwrap();

        let decrypted_content = fs::read(&decrypted).unwrap();
        assert_eq!(decrypted_content, binary_content);
    }
}
