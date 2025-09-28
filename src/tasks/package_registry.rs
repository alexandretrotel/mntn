use crate::cli::{PackageRegistryActions, PackageRegistryArgs};
use crate::logger::log;
use crate::package_registry::{PackageManagerEntry, PackageRegistry};
use crate::utils::paths::get_package_registry_path;

/// Run the package registry management command
pub fn run(args: PackageRegistryArgs) {
    match args.action {
        PackageRegistryActions::List {
            enabled_only,
            platform_only,
        } => {
            list_entries(enabled_only, platform_only);
        }
        PackageRegistryActions::Add {
            id,
            name,
            command,
            args,
            output_file,
            description,
            platforms,
        } => {
            add_entry(id, name, command, args, output_file, description, platforms);
        }
        PackageRegistryActions::Remove { id } => {
            remove_entry(id);
        }
        PackageRegistryActions::Toggle { id, enable } => {
            toggle_entry(id, enable);
        }
    }
}

/// List package manager registry entries
fn list_entries(enabled_only: bool, platform_only: bool) {
    let package_registry_path = get_package_registry_path();
    let registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load package registry: {}", e);
            log(&format!("Failed to load package registry: {}", e));
            return;
        }
    };

    println!("📦 Package Manager Registry");
    println!("===========================");

    let current_platform = PackageRegistry::get_current_platform();
    let entries: Vec<_> = if platform_only {
        registry
            .get_platform_compatible_entries(&current_platform)
            .collect()
    } else {
        registry.entries.iter().collect()
    };

    let filtered_entries: Vec<_> = if enabled_only {
        entries
            .into_iter()
            .filter(|(_, entry)| entry.enabled)
            .collect()
    } else {
        entries
    };

    if filtered_entries.is_empty() {
        println!("No package manager entries found.");
        return;
    }

    println!("Current platform: {}", current_platform);
    println!();

    for (id, entry) in filtered_entries {
        let status = if entry.enabled { "✅" } else { "❌" };
        let platform_info = match &entry.platforms {
            Some(platforms) => {
                if platforms.contains(&current_platform) {
                    format!(" ({})", platforms.join(", "))
                } else {
                    format!(" ({}) [INCOMPATIBLE]", platforms.join(", "))
                }
            }
            None => " (all platforms)".to_string(),
        };

        println!("{} {} ({})", status, entry.name, id);
        println!("   Command: {} {}", entry.command, entry.args.join(" "));
        println!("   Output: {}", entry.output_file);
        if let Some(description) = &entry.description {
            println!("   Description: {}", description);
        }
        println!("   Platforms: {}", platform_info);
        println!();
    }

    println!(
        "Total entries: {} (enabled: {})",
        registry.entries.len(),
        registry.get_enabled_entries().count()
    );
}

/// Add a new package manager entry
fn add_entry(
    id: String,
    name: String,
    command: String,
    args_str: String,
    output_file: String,
    description: Option<String>,
    platforms_str: Option<String>,
) {
    let package_registry_path = get_package_registry_path();
    let mut registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load package registry: {}", e);
            log(&format!("Failed to load package registry: {}", e));
            return;
        }
    };

    // Check if entry already exists
    if registry.get_entry(&id).is_some() {
        println!("❌ Entry '{}' already exists in the package registry", id);
        return;
    }

    // Parse args
    let args: Vec<String> = args_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Parse platforms
    let platforms = platforms_str.map(|s| {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });

    let entry = PackageManagerEntry {
        name: name.clone(),
        command,
        args,
        output_file,
        enabled: true,
        description,
        platforms,
    };

    registry.add_entry(id.clone(), entry);

    match registry.save(&package_registry_path) {
        Ok(_) => {
            println!("✅ Added package manager entry '{}' ({})", name, id);
            log(&format!("Added package manager entry: {}", id));
        }
        Err(e) => {
            println!("❌ Failed to save package registry: {}", e);
            log(&format!("Failed to save package registry: {}", e));
        }
    }
}

/// Remove a package manager entry
fn remove_entry(id: String) {
    let package_registry_path = get_package_registry_path();
    let mut registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load package registry: {}", e);
            log(&format!("Failed to load package registry: {}", e));
            return;
        }
    };

    match registry.remove_entry(&id) {
        Some(entry) => match registry.save(&package_registry_path) {
            Ok(_) => {
                println!("✅ Removed package manager entry '{}' ({})", entry.name, id);
                log(&format!("Removed package manager entry: {}", id));
            }
            Err(e) => {
                println!("❌ Failed to save package registry: {}", e);
                log(&format!("Failed to save package registry: {}", e));
            }
        },
        None => {
            println!("❌ Entry '{}' not found in package registry", id);
        }
    }
}

/// Toggle entry enabled/disabled state
fn toggle_entry(id: String, enable: bool) {
    let package_registry_path = get_package_registry_path();
    let mut registry = match PackageRegistry::load_or_create(&package_registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load package registry: {}", e);
            log(&format!("Failed to load package registry: {}", e));
            return;
        }
    };

    match registry.set_entry_enabled(&id, enable) {
        Ok(_) => match registry.save(&package_registry_path) {
            Ok(_) => {
                let action = if enable { "enabled" } else { "disabled" };
                println!("✅ {} package manager entry '{}'", action, id);
                log(&format!("{} package manager entry: {}", action, id));
            }
            Err(e) => {
                println!("❌ Failed to save package registry: {}", e);
                log(&format!("Failed to save package registry: {}", e));
            }
        },
        Err(e) => {
            println!("❌ {}", e);
        }
    }
}
