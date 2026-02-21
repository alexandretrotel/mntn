use crate::registry::package::PackageRegistry;
use crate::utils::paths::get_package_registry_path;
use crate::utils::system::{run_cmd, strip_ansi_codes};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn backup_packages(packages_path: &Path) -> Result<()> {
    let package_registry_path = get_package_registry_path();
    let package_registry = PackageRegistry::load_or_create(&package_registry_path)
        .with_context(|| format!("Load package registry: {}", package_registry_path.display()))?;

    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    if compatible_entries.is_empty() {
        println!("No package managers found to backup");
        return Ok(());
    }

    println!(
        "Backing up {} package managers...",
        compatible_entries.len()
    );

    for (id, entry) in compatible_entries {
        let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
        let content = run_cmd(&entry.command, &args, None)
            .with_context(|| format!("Command for {} failed", entry.name))?;

        let content = strip_ansi_codes(&content);
        let output_path = packages_path.join(&entry.output_file);
        let tmp_path = output_path.with_extension("tmp");

        let mut tmp_file = fs::File::create(&tmp_path)
            .with_context(|| format!("Create temp file for {}", entry.output_file))?;
        tmp_file
            .write_all(content.as_bytes())
            .with_context(|| format!("Write temp file for {}", entry.output_file))?;

        fs::rename(&tmp_path, &output_path)
            .with_context(|| format!("Move {} into place", entry.output_file))?;

        println!("Backed up {} ({})", entry.name, id);
    }

    Ok(())
}
