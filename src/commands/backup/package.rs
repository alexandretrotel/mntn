use crate::registry::package::PackageRegistry;
use crate::utils::paths::get_package_registry_path;
use crate::utils::system::{run_cmd, strip_ansi_codes};
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn backup_packages(packages_path: &Path) {
    let package_registry_path = get_package_registry_path();
    let package_registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!(
                "Failed to load package registry, skipping package backup: {}",
                e
            );
            return;
        }
    };

    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    if compatible_entries.is_empty() {
        println!("No package managers found to backup");
        return;
    }

    println!(
        "Backing up {} package managers...",
        compatible_entries.len()
    );

    let results: Vec<_> = compatible_entries
        .par_iter()
        .map(|(id, entry)| {
            let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
            let result = match run_cmd(&entry.command, &args, None) {
                Ok(content) => Ok(content),
                Err(e) => Err(e.to_string()),
            };
            ((*id).clone(), (*entry).clone(), result)
        })
        .collect();

    for (id, entry, result) in results {
        match result {
            Ok(content) => {
                let content = strip_ansi_codes(&content);
                let output_path = packages_path.join(&entry.output_file);
                let tmp_path = output_path.with_extension("tmp");

                match fs::File::create(&tmp_path).and_then(|mut f| f.write_all(content.as_bytes()))
                {
                    Ok(_) => {
                        if let Err(e) = fs::rename(&tmp_path, &output_path) {
                            eprintln!("Failed to atomically move {}: {}", entry.output_file, e);
                        } else {
                            println!("Backed up {} ({})", entry.name, id);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to write temp file for {}: {}", entry.output_file, e);
                        let _ = fs::remove_file(&tmp_path);
                    }
                }
            }
            Err(e) => {
                eprintln!("Command for {} failed: {}", entry.name, e);
            }
        }
    }
}
