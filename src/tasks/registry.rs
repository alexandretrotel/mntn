use crate::cli::{RegistryActions, RegistryArgs};
use crate::logger::log;
use crate::registry::{LinkRegistry, RegistryEntry};
use crate::utils::paths::get_registry_path;

/// Run the registry management command
pub fn run(args: RegistryArgs) {
    match args.action {
        RegistryActions::List {
            category,
            enabled_only,
        } => {
            list_entries(category, enabled_only);
        }
        RegistryActions::Add {
            id,
            name,
            source,
            target,
            category,
            description,
        } => {
            add_entry(id, name, source, target, category, description);
        }
        RegistryActions::Remove { id } => {
            remove_entry(id);
        }
        RegistryActions::Toggle { id, enable } => {
            toggle_entry(id, enable);
        }
    }
}

/// List registry entries
fn list_entries(filter_category: Option<String>, enabled_only: bool) {
    let registry_path = get_registry_path();
    let registry = match LinkRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load registry: {}", e);
            log(&format!("Failed to load registry: {}", e));
            return;
        }
    };

    println!("📋 Registry Entries");
    println!("==================");

    let entries_by_category = registry.list_by_category();
    let mut sorted_categories: Vec<_> = entries_by_category.keys().collect();
    sorted_categories.sort();

    for category in sorted_categories {
        // Skip categories that don't match the filter
        if let Some(ref filter) = filter_category {
            if category != filter {
                continue;
            }
        }

        let entries = &entries_by_category[category];
        let mut has_entries = false;

        for (id, entry) in entries {
            // Skip disabled entries if enabled_only is true
            if enabled_only && !entry.enabled {
                continue;
            }

            if !has_entries {
                println!("\n🏷️  {}", category.to_uppercase());
                println!("{}", "─".repeat(category.len() + 4));
                has_entries = true;
            }

            let status = if entry.enabled { "✅" } else { "❌" };
            println!("  {} {} ({})", status, entry.name, id);
            println!("     📁 Source: {}", entry.source_path);
            println!("     🔗 Target: {}", entry.target_path);

            if let Some(ref desc) = entry.description {
                println!("     💬 {}", desc);
            }
            println!();
        }

        if !has_entries && filter_category.is_some() {
            println!("\nNo entries found in category '{}'", category);
        }
    }

    let total_entries = registry.entries.len();
    let enabled_entries = registry.get_enabled_entries().count();

    println!(
        "Summary: {} total entries, {} enabled",
        total_entries, enabled_entries
    );
}

/// Add a new entry to the registry
fn add_entry(
    id: String,
    name: String,
    source: String,
    target: String,
    category: String,
    description: Option<String>,
) {
    let registry_path = get_registry_path();
    let mut registry = match LinkRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load registry: {}", e);
            log(&format!("Failed to load registry: {}", e));
            return;
        }
    };

    // Check if ID already exists
    if registry.get_entry(&id).is_some() {
        println!("❌ Entry with ID '{}' already exists", id);
        return;
    }

    let entry = RegistryEntry {
        name: name.clone(),
        source_path: source,
        target_path: target,
        category: category.clone(),
        enabled: true,
        description,
    };

    registry.add_entry(id.clone(), entry);

    if let Err(e) = registry.save(&registry_path) {
        println!("❌ Failed to save registry: {}", e);
        log(&format!("Failed to save registry: {}", e));
        return;
    }

    println!("✅ Added entry '{}' to registry", name);
    println!("   ID: {}", id);
    println!("   Category: {}", category);
    log(&format!("Added registry entry: {} ({})", name, id));
}

/// Remove an entry from the registry
fn remove_entry(id: String) {
    let registry_path = get_registry_path();
    let mut registry = match LinkRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load registry: {}", e);
            log(&format!("Failed to load registry: {}", e));
            return;
        }
    };

    match registry.remove_entry(&id) {
        Some(entry) => {
            if let Err(e) = registry.save(&registry_path) {
                println!("❌ Failed to save registry: {}", e);
                log(&format!("Failed to save registry: {}", e));
                return;
            }

            println!("✅ Removed entry '{}' from registry", entry.name);
            log(&format!("Removed registry entry: {} ({})", entry.name, id));
        }
        None => {
            println!("❌ Entry with ID '{}' not found", id);
        }
    }
}

/// Toggle an entry's enabled status
fn toggle_entry(id: String, enable: bool) {
    let registry_path = get_registry_path();
    let mut registry = match LinkRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            println!("❌ Failed to load registry: {}", e);
            log(&format!("Failed to load registry: {}", e));
            return;
        }
    };

    match registry.set_entry_enabled(&id, enable) {
        Ok(()) => {
            if let Err(e) = registry.save(&registry_path) {
                println!("❌ Failed to save registry: {}", e);
                log(&format!("Failed to save registry: {}", e));
                return;
            }

            let entry = registry.get_entry(&id).unwrap();
            let action = if enable { "enabled" } else { "disabled" };
            println!("✅ {} entry '{}'", action, entry.name);
            log(&format!(
                "{} registry entry: {} ({})",
                action, entry.name, id
            ));
        }
        Err(e) => {
            println!("❌ {}", e);
        }
    }
}
