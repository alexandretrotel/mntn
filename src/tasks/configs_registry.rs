use std::str::FromStr;

use crate::cli::{ConfigsRegistryActions, ConfigsRegistryArgs};
use crate::logger::{log, log_error, log_success};
use crate::registries::configs_registry::{Category, ConfigsRegistry, RegistryEntry};
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::paths::get_registry_path;

/// Configs registry management task
pub struct ConfigsRegistryTask {
    args: ConfigsRegistryArgs,
}

impl ConfigsRegistryTask {
    pub fn new(args: ConfigsRegistryArgs) -> Self {
        Self { args }
    }
}

impl Task for ConfigsRegistryTask {
    fn name(&self) -> &str {
        "Configs Registry"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.args.action {
            ConfigsRegistryActions::List {
                category,
                enabled_only,
            } => {
                list_entries(category.clone(), *enabled_only);
            }
            ConfigsRegistryActions::Add {
                id,
                name,
                source,
                target,
                category,
                description,
            } => {
                add_entry(
                    id.clone(),
                    name.clone(),
                    source.clone(),
                    target.clone(),
                    category.clone(),
                    description.clone(),
                );
            }
            ConfigsRegistryActions::Remove { id } => {
                remove_entry(id.clone());
            }
            ConfigsRegistryActions::Toggle { id, enable } => {
                toggle_entry(id.clone(), *enable);
            }
        }
        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let registry_path = get_registry_path();

        match &self.args.action {
            ConfigsRegistryActions::List { .. } => {
                operations.push(PlannedOperation::new("List registry entries"));
            }
            ConfigsRegistryActions::Add {
                id,
                name,
                source,
                target,
                ..
            } => {
                operations.push(PlannedOperation::with_target(
                    format!("Add registry entry '{}' ({})", name, id),
                    format!("Source: {}, Target: {}", source, target),
                ));
                operations.push(PlannedOperation::with_target(
                    "Save registry".to_string(),
                    registry_path.display().to_string(),
                ));
            }
            ConfigsRegistryActions::Remove { id } => {
                operations.push(PlannedOperation::with_target(
                    format!("Remove registry entry ({})", id),
                    registry_path.display().to_string(),
                ));
                operations.push(PlannedOperation::with_target(
                    "Save registry".to_string(),
                    registry_path.display().to_string(),
                ));
            }
            ConfigsRegistryActions::Toggle { id, enable } => {
                let action = if *enable { "enable" } else { "disable" };
                operations.push(PlannedOperation::with_target(
                    format!("{} registry entry ({})", action, id),
                    registry_path.display().to_string(),
                ));
                operations.push(PlannedOperation::with_target(
                    "Save registry".to_string(),
                    registry_path.display().to_string(),
                ));
            }
        }

        operations
    }
}

/// Run with CLI args
pub fn run_with_args(args: ConfigsRegistryArgs) {
    let dry_run = args.dry_run;
    let mut task = ConfigsRegistryTask::new(args);
    TaskExecutor::run(&mut task, dry_run);
}

/// List registry entries
fn list_entries(filter_category: Option<String>, enabled_only: bool) {
    let registry_path = get_registry_path();
    let registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load registry", e);
            return;
        }
    };

    println!("📋 Registry Entries");
    println!("==================");

    let entries_by_category = registry.list_by_category();
    let mut sorted_categories: Vec<_> = entries_by_category.keys().collect();
    sorted_categories.sort();

    for category in sorted_categories {
        if let Some(ref filter) = filter_category
            && format!("{:?}", category).to_lowercase() != *filter
        {
            continue;
        }

        let entries = &entries_by_category[category];
        let mut has_entries = false;

        for (id, entry) in entries {
            if enabled_only && !entry.enabled {
                continue;
            }

            if !has_entries {
                let category_str = format!("{:?}", category).to_uppercase();
                println!("\n🏷️  {}", category_str);
                println!("{}", "─".repeat(category_str.len() + 4));
                has_entries = true;
            }

            let status = if entry.enabled { "✅" } else { "❌" };
            println!("  {} {} ({})", status, entry.name, id);
            println!("     📁 Source: {}", entry.source_path);
            println!("     🔗 Target: {}", entry.target_path.display());

            if let Some(ref desc) = entry.description {
                println!("     💬 {}", desc);
            }
            println!();
        }

        if !has_entries && filter_category.is_some() {
            println!(
                "\nNo entries found in category '{}'",
                format!("{:?}", category).to_lowercase()
            );
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
    let mut registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load registry", e);
            return;
        }
    };

    if registry.get_entry(&id).is_some() {
        println!("❌ Entry with ID '{}' already exists", id);
        return;
    }

    let parsed_category = match Category::from_str(&category) {
        Ok(cat) => cat,
        Err(_) => {
            println!(
                "❌ Invalid category '{}'. Valid categories are: shell, editor, terminal, system, development, application",
                category
            );
            return;
        }
    };

    let target_path = std::path::PathBuf::from(target);

    let entry = RegistryEntry {
        name: name.clone(),
        source_path: source,
        target_path,
        category: parsed_category,
        enabled: true,
        description,
    };

    registry.add_entry(id.clone(), entry);

    if let Err(e) = registry.save(&registry_path) {
        log_error("Failed to save registry", e);
        return;
    }

    log_success(&format!("Added entry '{}' to registry", name));
    println!("   ID: {}", id);
    println!("   Category: {}", category);
    log(&format!("Added registry entry: {} ({})", name, id));
}

/// Remove an entry from the registry
fn remove_entry(id: String) {
    let registry_path = get_registry_path();
    let mut registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load registry", e);
            return;
        }
    };

    match registry.remove_entry(&id) {
        Some(entry) => {
            if let Err(e) = registry.save(&registry_path) {
                log_error("Failed to save registry", e);
                return;
            }

            log_success(&format!("Removed entry '{}' from registry", entry.name));
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
    let mut registry = match ConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load registry", e);
            return;
        }
    };

    match registry.set_entry_enabled(&id, enable) {
        Ok(()) => {
            if let Err(e) = registry.save(&registry_path) {
                log_error("Failed to save registry", e);
                return;
            }

            let entry = registry.get_entry(&id).unwrap();
            let action = if enable { "enabled" } else { "disabled" };
            log_success(&format!("{} entry '{}'", action, entry.name));
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
