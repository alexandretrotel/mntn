use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn normalize_tar_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

pub(crate) fn write_entries_tar(tar_path: &Path, entries: &[(&str, &Path)]) -> Result<()> {
    let file = fs::File::create(tar_path)
        .with_context(|| format!("Create tar archive {}", tar_path.display()))?;
    let mut builder = tar::Builder::new(file);
    builder.mode(tar::HeaderMode::Deterministic);
    for (source_path, target_path) in entries {
        let mut input = fs::File::open(target_path).with_context(|| {
            format!(
                "Open {} for archiving as {}",
                target_path.display(),
                source_path
            )
        })?;
        builder
            .append_file(*source_path, &mut input)
            .with_context(|| format!("Append {} to tar", source_path))?;
    }
    builder.finish().context("Finish tar archive")?;
    Ok(())
}

pub(crate) fn load_tar_member_map(tar_path: &Path) -> Result<HashMap<String, Vec<u8>>> {
    let file = fs::File::open(tar_path).context("Open tar for reading")?;
    let mut archive = tar::Archive::new(file);
    let mut map = HashMap::new();
    for entry in archive.entries().context("Read tar entries")? {
        let mut entry = entry.context("Tar entry")?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let path = normalize_tar_path(&entry.path().context("Tar entry path")?);
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .with_context(|| format!("Read tar member {}", path))?;
        map.insert(path, data);
    }
    Ok(map)
}

pub(crate) fn set_private_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Set permissions on: {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

pub(crate) fn create_temp_path(label: &str) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for attempt in 0..10 {
        let name = format!("mntn-{label}-{pid}-{base}-{attempt}");
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }

    Err(std::io::Error::other("Failed to create temporary file"))
}
