# Suggested Enhancements for Dotfiles Workflow

## 1. Package Restoration Command

Add a `mntn restore-packages` command that automatically reinstalls packages:

```rust
// In src/tasks/restore_packages.rs
pub fn run() {
    let backup_dir = get_backup_path();
    
    // Restore Homebrew packages
    if let Ok(packages) = fs::read_to_string(backup_dir.join("brew.txt")) {
        run_cmd("brew", &["install"].iter().chain(packages.lines()).collect::<Vec<_>>());
    }
    
    // Restore npm packages
    if let Ok(packages) = fs::read_to_string(backup_dir.join("npm.txt")) {
        let package_names: Vec<&str> = packages.lines()
            .filter_map(|line| line.split('@').next())
            .collect();
        run_cmd("npm", &["install", "-g"].iter().chain(package_names.iter()).collect::<Vec<_>>());
    }
    
    // Similar for other package managers...
}
```

## 2. Dotfiles Sync Command

Add `mntn sync` command to push/pull dotfiles changes:

```rust
// In src/tasks/sync.rs
pub fn run(push: bool, pull: bool) {
    let backup_dir = get_backup_dir();
    
    if pull {
        run_cmd_in_dir("git", &["pull"], &backup_dir)?;
        // Re-run link command after pull
        crate::tasks::link::run();
    }
    
    if push {
        // show changes before comitting, etc
        run_cmd_in_dir("git", &["add", "."], &backup_dir)?;
        run_cmd_in_dir("git", &["commit", "-m", "Update dotfiles"], &backup_dir)?;
        run_cmd_in_dir("git", &["push"], &backup_dir)?;
    }
}
```

## 3. Machine-Specific Configurations

Support for machine-specific dotfiles:

```rust
// Directory structure:
// ~/.mntn/
// ├── common/          # Shared configs
// ├── machines/
// │   ├── work-laptop/ # Machine-specific configs
// │   └── home-desktop/
// └── .mntn-machine    # Contains current machine identifier

pub fn link_with_machine_specific() {
    let machine_id = get_machine_identifier();
    let backup_dir = get_backup_dir();
    let common_dir = backup_dir.join("common");
    let machine_dir = backup_dir.join("machines").join(&machine_id);
    
    // Link common files first
    link_directory(&common_dir);
    
    // Override with machine-specific files
    if machine_dir.exists() {
        link_directory(&machine_dir);
    }
}
```

## 4. Configuration Validation

Add validation for config files before linking:

```rust
// In src/tasks/validate.rs
pub fn validate_config_files() -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let backup_dir = get_backup_dir();
    
    // Check if essential files exist
    if !backup_dir.join(".zshrc").exists() && !backup_dir.join(".bashrc").exists() {
        errors.push("No shell configuration found (.zshrc or .bashrc)".to_string());
    }
    
    // Validate JSON files
    if let Ok(content) = fs::read_to_string(backup_dir.join("vscode/settings.json")) {
        if serde_json::from_str::<serde_json::Value>(&content).is_err() {
            errors.push("Invalid JSON in vscode/settings.json".to_string());
        }
    }

     if let Ok(content) = fs::read_to_string(backup_dir.join("vscode/keybindings.json")) {
        if serde_json::from_str::<serde_json::Value>(&content).is_err() {
            errors.push("Invalid JSON in vscode/settings.json".to_string());
        }
    }
    
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```