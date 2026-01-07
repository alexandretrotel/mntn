use crate::cli::{EncryptedRegistryActions, EncryptedRegistryArgs};
use crate::logger::{log, log_error, log_success};
use crate::registries::encrypted_configs_registry::{
    EncryptedConfigsRegistry, EncryptedRegistryEntry,
};
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::paths::get_encrypted_registry_path;

/// Encrypted configs registry management task
pub struct EncryptedConfigsRegistryTask {
    args: EncryptedRegistryArgs,
}

impl EncryptedConfigsRegistryTask {
    pub fn new(args: EncryptedRegistryArgs) -> Self {
        Self { args }
    }
}

impl Task for EncryptedConfigsRegistryTask {
    fn name(&self) -> &str {
        "Encrypted Configs Registry"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.args.action {
            EncryptedRegistryActions::List { enabled_only } => {
                list_entries(*enabled_only);
            }
            EncryptedRegistryActions::Add {
                id,
                name,
                source,
                target,
                description,
                encrypt_filename,
            } => {
                add_entry(
                    id.clone(),
                    name.clone(),
                    source.clone(),
                    target.clone(),
                    description.clone(),
                    *encrypt_filename,
                );
            }
            EncryptedRegistryActions::Remove { id } => {
                remove_entry(id.clone());
            }
            EncryptedRegistryActions::Toggle { id, enable } => {
                toggle_entry(id.clone(), *enable);
            }
        }
        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let registry_path = get_encrypted_registry_path();

        match &self.args.action {
            EncryptedRegistryActions::List { .. } => {
                operations.push(PlannedOperation::new("List encrypted registry entries"));
            }
            EncryptedRegistryActions::Add {
                id,
                name,
                source,
                target,
                ..
            } => {
                operations.push(PlannedOperation::with_target(
                    format!("Add encrypted registry entry '{}' ({})", name, id),
                    format!("Source: {}, Target: {}", source, target),
                ));
                operations.push(PlannedOperation::with_target(
                    "Save encrypted registry".to_string(),
                    registry_path.display().to_string(),
                ));
            }
            EncryptedRegistryActions::Remove { id } => {
                operations.push(PlannedOperation::with_target(
                    format!("Remove encrypted registry entry ({})", id),
                    registry_path.display().to_string(),
                ));
                operations.push(PlannedOperation::with_target(
                    "Save encrypted registry".to_string(),
                    registry_path.display().to_string(),
                ));
            }
            EncryptedRegistryActions::Toggle { id, enable } => {
                let action = if *enable { "Enable" } else { "Disable" };
                operations.push(PlannedOperation::with_target(
                    format!("{} encrypted registry entry ({})", action, id),
                    registry_path.display().to_string(),
                ));
                operations.push(PlannedOperation::with_target(
                    "Save encrypted registry".to_string(),
                    registry_path.display().to_string(),
                ));
            }
        }

        operations
    }
}

/// Run with CLI args
pub fn run_with_args(args: EncryptedRegistryArgs) {
    let dry_run = args.dry_run;
    let mut task = EncryptedConfigsRegistryTask::new(args);
    TaskExecutor::run(&mut task, dry_run);
}

/// List encrypted registry entries
fn list_entries(enabled_only: bool) {
    let registry_path = get_encrypted_registry_path();
    let registry = match EncryptedConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load encrypted registry", e);
            return;
        }
    };

    println!("Encrypted Registry Entries");
    println!("==========================\n");

    let mut entries: Vec<_> = registry.entries.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    for (id, entry) in entries {
        if enabled_only && !entry.enabled {
            continue;
        }

        let status = if entry.enabled { "[x]" } else { "[ ]" };
        let filename_status = if entry.encrypt_filename {
            " [encrypted filename]"
        } else {
            ""
        };
        println!("{} {} ({}){}", status, entry.name, id, filename_status);
        println!("    Source: {}", entry.source_path);
        println!("    Target: {}", entry.target_path.display());

        if let Some(ref desc) = entry.description {
            println!("    {}", desc);
        }
        println!();
    }

    let total_entries = registry.entries.len();
    let enabled_entries = registry.get_enabled_entries().count();

    println!(
        "Summary: {} total entries, {} enabled",
        total_entries, enabled_entries
    );
}

/// Add a new entry to the encrypted registry
fn add_entry(
    id: String,
    name: String,
    source: String,
    target: String,
    description: Option<String>,
    encrypt_filename: bool,
) {
    let registry_path = get_encrypted_registry_path();
    let mut registry = match EncryptedConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load encrypted registry", e);
            return;
        }
    };

    if registry.get_entry(&id).is_some() {
        log_error("Entry already exists", &id);
        return;
    }

    let target_path = std::path::PathBuf::from(target);

    let entry = EncryptedRegistryEntry {
        name: name.clone(),
        source_path: source,
        target_path,
        enabled: true,
        description,
        encrypt_filename,
    };

    registry.add_entry(id.clone(), entry);

    if let Err(e) = registry.save(&registry_path) {
        log_error("Failed to save encrypted registry", e);
        return;
    }

    log_success(&format!("Added entry '{}' to encrypted registry", name));
    println!("   ID: {}", id);
    log(&format!(
        "Added encrypted registry entry: {} ({})",
        name, id
    ));
}

/// Remove an entry from the encrypted registry
fn remove_entry(id: String) {
    let registry_path = get_encrypted_registry_path();
    let mut registry = match EncryptedConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load encrypted registry", e);
            return;
        }
    };

    match registry.remove_entry(&id) {
        Some(entry) => {
            if let Err(e) = registry.save(&registry_path) {
                log_error("Failed to save encrypted registry", e);
                return;
            }

            log_success(&format!(
                "Removed entry '{}' from encrypted registry",
                entry.name
            ));
            log(&format!(
                "Removed encrypted registry entry: {} ({})",
                entry.name, id
            ));
        }
        None => {
            log_error("Entry not found", &id);
        }
    }
}

/// Toggle an entry's enabled status
fn toggle_entry(id: String, enable: bool) {
    let registry_path = get_encrypted_registry_path();
    let mut registry = match EncryptedConfigsRegistry::load_or_create(&registry_path) {
        Ok(registry) => registry,
        Err(e) => {
            log_error("Failed to load encrypted registry", e);
            return;
        }
    };

    match registry.set_entry_enabled(&id, enable) {
        Ok(()) => {
            if let Err(e) = registry.save(&registry_path) {
                log_error("Failed to save encrypted registry", e);
                return;
            }

            let entry = registry.get_entry(&id).unwrap();
            let action = if enable { "enabled" } else { "disabled" };
            log_success(&format!("{} entry '{}'", action, entry.name));
            log(&format!(
                "{} encrypted registry entry: {} ({})",
                action, entry.name, id
            ));
        }
        Err(e) => {
            log_error("Failed to toggle entry", e);
        }
    }
}
