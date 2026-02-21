use crate::utils::system::sync_directory_contents;
use std::fs;
use std::path::Path;

pub fn backup_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file {} not found", source.display()),
        ));
    }

    if source.is_symlink()
        && let Ok(link_target) = fs::read_link(source)
    {
        let canonical_target = link_target.canonicalize().unwrap_or(link_target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.to_path_buf());

        if canonical_target == canonical_dest {
            let content = fs::read(&canonical_target)?;
            fs::remove_file(source)?;
            fs::write(source, &content)?;
            println!("Converted symlink to real file: {}", source.display());
            return Ok(());
        }
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, destination)?;
    Ok(())
}

pub fn backup_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source directory {} not found", source.display()),
        ));
    }

    if source.is_symlink()
        && let Ok(link_target) = fs::read_link(source)
    {
        let canonical_target = link_target
            .canonicalize()
            .unwrap_or_else(|_| link_target.clone());
        let canonical_dest = destination
            .canonicalize()
            .unwrap_or_else(|_| destination.to_path_buf());

        if canonical_target == canonical_dest {
            let file_type = fs::symlink_metadata(source)?.file_type();
            if file_type.is_dir() || (file_type.is_symlink() && fs::metadata(source)?.is_dir()) {
                fs::remove_dir(source)?;
            } else {
                fs::remove_file(source)?;
            }
            fs::create_dir_all(source)?;
            crate::utils::filesystem::copy_dir_recursive(&canonical_target, source)?;
            println!("Converted symlink to real directory: {}", source.display());
            return Ok(());
        }
    }

    fs::create_dir_all(destination)?;
    sync_directory_contents(source, destination)
}
