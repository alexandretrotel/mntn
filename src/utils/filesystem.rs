use chrono::Local;
use std::{
    ffi::OsString,
    fs::{self},
    io,
    path::Path,
};

/// Recursively calculates the total size in bytes of the given directory or file path.
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

/// Backs up an existing file or directory to a timestamped location inside `symlinks_dir`.
pub fn backup_existing_target(target: &Path, symlinks_dir: &Path) -> io::Result<()> {
    if !symlinks_dir.exists() {
        fs::create_dir_all(symlinks_dir)?;
    }

    let filename = target
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("backup"));
    let timestamp = Local::now().format("%Y%m%d_%H%M%S%3f").to_string();

    let mut backup_name = OsString::from(filename);
    backup_name.push("_");
    backup_name.push(timestamp);

    let backup_path = symlinks_dir.join(backup_name);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_size_nonexistent_path() {
        let result = calculate_dir_size(Path::new("/nonexistent/path/that/does/not/exist"));
        assert_eq!(result, None);
    }

    #[test]
    fn test_calculate_size_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = calculate_dir_size(temp_dir.path());
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_calculate_size_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap(); // 11 bytes

        let result = calculate_dir_size(&file_path);
        assert_eq!(result, Some(11));
    }

    #[test]
    fn test_calculate_size_directory_with_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create file1 with 10 bytes
        let file1 = temp_dir.path().join("file1.txt");
        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(b"0123456789").unwrap();

        // Create file2 with 5 bytes
        let file2 = temp_dir.path().join("file2.txt");
        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(b"abcde").unwrap();

        let result = calculate_dir_size(temp_dir.path());
        assert_eq!(result, Some(15));
    }

    #[test]
    fn test_calculate_size_nested_directories() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested structure: root/subdir/file.txt
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let file = subdir.join("file.txt");
        let mut f = File::create(&file).unwrap();
        f.write_all(b"nested content").unwrap(); // 14 bytes

        // Also create a file at root level
        let root_file = temp_dir.path().join("root.txt");
        let mut rf = File::create(&root_file).unwrap();
        rf.write_all(b"root").unwrap(); // 4 bytes

        let result = calculate_dir_size(temp_dir.path());
        assert_eq!(result, Some(18));
    }

    #[test]
    #[cfg(unix)]
    fn test_calculate_size_symlink_ignored() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();

        // Create a real file with 10 bytes
        let real_file = temp_dir.path().join("real.txt");
        let mut f = File::create(&real_file).unwrap();
        f.write_all(b"0123456789").unwrap();

        // Create a symlink to the file
        let link_path = temp_dir.path().join("link.txt");
        symlink(&real_file, &link_path).unwrap();

        // Size should only count real file (10 bytes), symlink contributes 0
        let result = calculate_dir_size(temp_dir.path());
        assert_eq!(result, Some(10));
    }

    #[test]
    #[cfg(unix)]
    fn test_calculate_size_symlink_directly() {
        use std::os::unix::fs::symlink;

        let temp_dir = TempDir::new().unwrap();
        let real_file = temp_dir.path().join("real.txt");
        let mut f = File::create(&real_file).unwrap();
        f.write_all(b"content").unwrap();

        let link_path = temp_dir.path().join("link.txt");
        symlink(&real_file, &link_path).unwrap();

        // Calculating size of symlink itself should return 0
        let result = calculate_dir_size(&link_path);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_copy_dir_recursive_empty_directory() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        // Destination should still be empty (no contents to copy)
        assert!(fs::read_dir(dst_dir.path()).unwrap().next().is_none());
    }

    #[test]
    fn test_copy_dir_recursive_single_file() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create source file
        let src_file = src_dir.path().join("test.txt");
        fs::write(&src_file, "hello").unwrap();

        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        // Check file was copied
        let dst_file = dst_dir.path().join("test.txt");
        assert!(dst_file.exists());
        assert_eq!(fs::read_to_string(&dst_file).unwrap(), "hello");
    }

    #[test]
    fn test_copy_dir_recursive_nested_structure() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create nested structure
        let subdir = src_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("nested.txt"), "nested content").unwrap();
        fs::write(src_dir.path().join("root.txt"), "root content").unwrap();

        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        // Verify structure
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("root.txt")).unwrap(),
            "root content"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("subdir").join("nested.txt")).unwrap(),
            "nested content"
        );
    }

    #[test]
    fn test_copy_dir_recursive_preserves_content() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create file with specific content
        let content = "This is a test file with specific content!\nLine 2\nLine 3";
        fs::write(src_dir.path().join("data.txt"), content).unwrap();

        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("data.txt")).unwrap(),
            content
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_copy_dir_recursive_ignores_symlinks() {
        use std::os::unix::fs::symlink;

        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create a real file
        fs::write(src_dir.path().join("real.txt"), "real").unwrap();

        // Create a symlink
        symlink(
            src_dir.path().join("real.txt"),
            src_dir.path().join("link.txt"),
        )
        .unwrap();

        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        // Real file should be copied, symlink should not
        assert!(dst_dir.path().join("real.txt").exists());
        assert!(!dst_dir.path().join("link.txt").exists());
    }

    #[test]
    fn test_copy_dir_recursive_multiple_levels() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        // Create deep nesting: a/b/c/file.txt
        let deep_path = src_dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep_path).unwrap();
        fs::write(deep_path.join("file.txt"), "deep").unwrap();

        copy_dir_recursive(src_dir.path(), dst_dir.path()).unwrap();

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("a/b/c/file.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn test_copy_dir_to_source_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();

        // Create source directory with content
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("file.txt"), "content").unwrap();

        // Destination with non-existent parent
        let source = temp_dir.path().join("new_parent").join("new_dir");

        copy_dir_to_source(&target_dir, &source).unwrap();

        assert!(source.exists());
        assert_eq!(
            fs::read_to_string(source.join("file.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn test_copy_dir_to_source_copies_contents() {
        let temp_dir = TempDir::new().unwrap();

        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("a.txt"), "aaa").unwrap();
        fs::write(target_dir.join("b.txt"), "bbb").unwrap();

        let source = temp_dir.path().join("source");

        copy_dir_to_source(&target_dir, &source).unwrap();

        assert_eq!(fs::read_to_string(source.join("a.txt")).unwrap(), "aaa");
        assert_eq!(fs::read_to_string(source.join("b.txt")).unwrap(), "bbb");
    }

    #[test]
    fn test_copy_dir_to_source_atomic_rename() {
        let temp_dir = TempDir::new().unwrap();

        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("test.txt"), "test").unwrap();

        let source = temp_dir.path().join("source");

        copy_dir_to_source(&target_dir, &source).unwrap();

        // Verify temp directory was cleaned up
        let tmp_path = source.with_extension("tmp_copy_dir");
        assert!(!tmp_path.exists());
    }

    #[test]
    fn test_backup_existing_target_file() {
        let temp_dir = TempDir::new().unwrap();
        let symlinks_dir = temp_dir.path().join("symlinks");

        // Create target file
        let target = temp_dir.path().join("config.txt");
        fs::write(&target, "original content").unwrap();

        backup_existing_target(&target, &symlinks_dir).unwrap();

        // Original should be gone
        assert!(!target.exists());

        // Backup should exist in symlinks_dir
        let entries: Vec<_> = fs::read_dir(&symlinks_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);

        let backup_path = entries[0].as_ref().unwrap().path();
        assert!(
            backup_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("config.txt_")
        );
        assert_eq!(
            fs::read_to_string(&backup_path).unwrap(),
            "original content"
        );
    }

    #[test]
    fn test_backup_existing_target_directory() {
        let temp_dir = TempDir::new().unwrap();
        let symlinks_dir = temp_dir.path().join("symlinks");

        // Create target directory with content
        let target = temp_dir.path().join("config_dir");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("file.txt"), "dir content").unwrap();

        backup_existing_target(&target, &symlinks_dir).unwrap();

        // Original should be gone
        assert!(!target.exists());

        // Backup should exist
        let entries: Vec<_> = fs::read_dir(&symlinks_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);

        let backup_path = entries[0].as_ref().unwrap().path();
        assert!(backup_path.is_dir());
        assert_eq!(
            fs::read_to_string(backup_path.join("file.txt")).unwrap(),
            "dir content"
        );
    }

    #[test]
    fn test_backup_existing_target_creates_symlinks_dir() {
        let temp_dir = TempDir::new().unwrap();
        let symlinks_dir = temp_dir.path().join("new_symlinks_dir");

        // Ensure symlinks_dir doesn't exist
        assert!(!symlinks_dir.exists());

        let target = temp_dir.path().join("file.txt");
        fs::write(&target, "content").unwrap();

        backup_existing_target(&target, &symlinks_dir).unwrap();

        // symlinks_dir should now exist
        assert!(symlinks_dir.exists());
        assert!(symlinks_dir.is_dir());
    }

    #[test]
    fn test_backup_existing_target_timestamp_format() {
        let temp_dir = TempDir::new().unwrap();
        let symlinks_dir = temp_dir.path().join("symlinks");

        let target = temp_dir.path().join("myfile.txt");
        fs::write(&target, "test").unwrap();

        backup_existing_target(&target, &symlinks_dir).unwrap();

        let entries: Vec<_> = fs::read_dir(&symlinks_dir).unwrap().collect();
        let backup_name = entries[0]
            .as_ref()
            .unwrap()
            .file_name()
            .to_str()
            .unwrap()
            .to_string();

        // Should match pattern: myfile.txt_YYYYMMDD_HHMMSSmmm
        assert!(backup_name.starts_with("myfile.txt_"));
        let timestamp_part = &backup_name["myfile.txt_".len()..];
        // Timestamp should be 18 chars: YYYYMMDD_HHMMSSmmm
        assert_eq!(timestamp_part.len(), 18);
    }

    #[test]
    fn test_backup_multiple_targets_unique_names() {
        let temp_dir = TempDir::new().unwrap();
        let symlinks_dir = temp_dir.path().join("symlinks");

        // Create and backup first file
        let target1 = temp_dir.path().join("file.txt");
        fs::write(&target1, "first").unwrap();
        backup_existing_target(&target1, &symlinks_dir).unwrap();

        // Small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Create and backup second file with same name
        let target2 = temp_dir.path().join("file.txt");
        fs::write(&target2, "second").unwrap();
        backup_existing_target(&target2, &symlinks_dir).unwrap();

        // Should have two distinct backups
        let entries: Vec<_> = fs::read_dir(&symlinks_dir).unwrap().collect();
        assert_eq!(entries.len(), 2);
    }
}
