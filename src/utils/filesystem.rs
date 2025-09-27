use chrono::Local;
use std::{
    ffi::OsString,
    fs::{self},
    io,
    path::Path,
};

/// Recursively calculates the total size in bytes of the given directory or file path.
///
/// Symlinks are ignored and contribute zero to the total size to avoid cycles.
pub fn calculate_dir_size(path: &Path) -> Option<u64> {
    let metadata = fs::symlink_metadata(path).ok()?;

    if metadata.file_type().is_symlink() {
        return Some(0);
    } else if metadata.is_file() {
        return Some(metadata.len());
    } else if metadata.is_dir() {
        let mut size = 0;
        for entry in fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            let entry_path = entry.path();
            size += calculate_dir_size(&entry_path).unwrap_or(0);
        }
        return Some(size);
    }

    Some(0)
}

/// Copies an existing directory from `target` to `source`.
pub fn copy_dir_to_source(target: &Path, source: &Path) -> io::Result<()> {
    if let Some(parent) = source.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_dir = source.with_extension("tmp_copy_dir");
    fs::create_dir_all(&tmp_dir)?;
    copy_dir_recursive(target, &tmp_dir)?;
    fs::rename(tmp_dir, source)?;

    Ok(())
}

/// Recursively copies the contents of `src` to `dst` (not the root directory itself).
/// Creates directories as needed, handles nested files.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let metadata = fs::symlink_metadata(&src_path)?;
        if metadata.file_type().is_symlink() {
            continue; // ignoring symlinks here
        } else if metadata.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if metadata.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Backs up an existing file or directory to a timestamped location inside `backup_dir`.
pub fn backup_existing_target(target: &Path, backup_dir: &Path) -> io::Result<()> {
    if !backup_dir.exists() {
        fs::create_dir_all(backup_dir)?;
    }

    let filename = target
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("backup"));
    let timestamp = Local::now().format("%Y%m%d_%H%M%S%3f").to_string();

    let mut backup_name = OsString::from(filename);
    backup_name.push("_");
    backup_name.push(timestamp);

    let backup_path = backup_dir.join(backup_name);

    match fs::rename(target, &backup_path) {
        Ok(_) => Ok(()),
        Err(_) => {
            if target.is_dir() {
                copy_dir_recursive(target, &backup_path)?;
                fs::remove_dir_all(target)?;
            } else {
                fs::copy(target, &backup_path)?;
                fs::remove_file(target)?;
            }
            Ok(())
        }
    }
}
