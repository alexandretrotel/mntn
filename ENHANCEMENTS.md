# Suggested Enhancements for mntn

## 1. Code Mutualization and Architecture Improvements

### Command Pattern for Tasks

Implement a command pattern to standardize task execution:

```rust
// In src/tasks/core.rs
pub trait Task {
    type Args;
    type Error;
    
    fn execute(&self, args: Self::Args) -> Result<(), Self::Error>;
    fn validate(&self, args: &Self::Args) -> Result<(), String>;
    fn dry_run(&self, args: &Self::Args) -> Vec<String>; // Show what would be done
}

pub struct TaskExecutor {
    logger: Arc<dyn Logger>,
}

impl TaskExecutor {
    pub fn run<T: Task>(&self, task: T, args: T::Args) -> Result<(), T::Error> {
        task.validate(&args)?;
        self.logger.log(&format!("Starting task: {}", std::any::type_name::<T>()));
        let result = task.execute(args);
        match &result {
            Ok(_) => self.logger.log("Task completed successfully"),
            Err(e) => self.logger.log(&format!("Task failed: {:?}", e)),
        }
        result
    }
}
```

## 2. Enhanced Package Management

### Smart Package Restoration

Add a `mntn restore-packages` command that uses the package registry for intelligent restoration:

```rust
// In src/tasks/restore_packages.rs
pub fn run(args: RestorePackagesArgs) {
    let package_registry = PackageRegistry::load_or_create(&get_package_registry_path())?;
    let backup_dir = get_backup_path();
    let current_platform = PackageRegistry::get_current_platform();
    
    for (id, entry) in package_registry.get_platform_compatible_entries(&current_platform) {
        if !args.package_managers.is_empty() && !args.package_managers.contains(id) {
            continue; // Skip if specific package managers requested
        }
        
        let backup_file = backup_dir.join(&entry.output_file);
        if !backup_file.exists() {
            println!("⚠️ Backup file {} not found, skipping {}", backup_file.display(), entry.name);
            continue;
        }
        
        match entry.command.as_str() {
            "brew" => restore_homebrew_packages(&backup_file, &entry.args),
            "npm" => restore_npm_packages(&backup_file),
            "cargo" => restore_cargo_packages(&backup_file),
            _ => restore_generic_packages(&backup_file, &entry),
        }
    }
}

fn restore_npm_packages(backup_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(backup_file)?;
    let packages: Vec<String> = content.lines()
        .filter_map(|line| {
            // Parse npm ls output to extract package names without versions
            if line.starts_with("├──") || line.starts_with("└──") {
                line.split_whitespace().nth(1)
                    .and_then(|pkg| pkg.split('@').next())
                    .map(|name| name.to_string())
            } else {
                None
            }
        })
        .collect();
    
    if !packages.is_empty() {
        run_cmd("npm", &["install", "-g"].iter().chain(packages.iter().map(|s| s.as_str())).collect::<Vec<_>>())?;
    }
    Ok(())
}
```

### Package Manager Detection

Add automatic detection of available package managers:

```rust
// In src/package_registry.rs
impl PackageRegistry {
    pub fn detect_available_package_managers() -> Vec<String> {
        let mut available = Vec::new();
        let managers = [("brew", "brew --version"), ("npm", "npm --version"), ("cargo", "cargo --version")];
        
        for (name, check_cmd) in managers {
            if Command::new(check_cmd.split_whitespace().next().unwrap())
                .args(check_cmd.split_whitespace().skip(1))
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
            {
                available.push(name.to_string());
            }
        }
        available
    }
    
    pub fn enable_detected_managers(&mut self) {
        let available = Self::detect_available_package_managers();
        for (id, entry) in self.entries.iter_mut() {
            entry.enabled = available.contains(&entry.command);
        }
    }
}
```

## 3. VS Code Extensions Backup and Restore

### Extension Management Integration

Add VS Code extensions to the package registry and implement backup/restore functionality:

```rust
// In src/registries/package_registry.rs - Add VS Code extensions entry
impl PackageRegistry {
    pub fn add_vscode_extensions_entry(&mut self) {
        let entry = PackageEntry {
            name: "VS Code Extensions".to_string(),
            command: "code".to_string(),
            args: vec!["--list-extensions".to_string()],
            output_file: "vscode-extensions.txt".to_string(),
            enabled: true,
            platforms: vec!["macos".to_string(), "linux".to_string(), "windows".to_string()],
        };
        self.entries.insert("vscode-extensions".to_string(), entry);
    }
}
```

### Enhanced VS Code Extension Backup

```rust
// In src/tasks/backup_vscode.rs
use std::process::Command;
use crate::utils::filesystem::ensure_dir_exists;

pub fn backup_vscode_extensions(backup_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Check if VS Code is installed
    if !is_vscode_installed() {
        println!("ℹ️ VS Code not found, skipping extension backup");
        return Ok(());
    }

    let extensions_file = backup_dir.join("vscode-extensions.txt");
    let settings_backup_dir = backup_dir.join("vscode");
    ensure_dir_exists(&settings_backup_dir)?;

    // Backup extension list
    println!("📦 Backing up VS Code extensions...");
    let output = Command::new("code")
        .args(&["--list-extensions"])
        .output()?;

    if output.status.success() {
        let extensions = String::from_utf8_lossy(&output.stdout);
        fs::write(&extensions_file, extensions.as_bytes())?;
        
        let extension_count = extensions.lines().count();
        println!("✅ Backed up {} VS Code extensions to {}", 
                extension_count, extensions_file.display());
    } else {
        println!("⚠️ Failed to list VS Code extensions");
        return Err("Failed to execute 'code --list-extensions'".into());
    }

    // Also backup VS Code settings and keybindings
    backup_vscode_settings(&settings_backup_dir)?;
    
    Ok(())
}

fn backup_vscode_settings(backup_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let vscode_config_dir = get_vscode_config_dir();
    
    let files_to_backup = vec![
        ("settings.json", "User settings"),
        ("keybindings.json", "Keybindings"),
        ("snippets", "User snippets"), // Directory
    ];

    for (file_name, description) in files_to_backup {
        let source = vscode_config_dir.join(file_name);
        let dest = backup_dir.join(file_name);
        
        if source.exists() {
            if source.is_dir() {
                copy_directory_recursive(&source, &dest)?;
                println!("✅ Backed up VS Code {}", description);
            } else {
                fs::copy(&source, &dest)?;
                println!("✅ Backed up VS Code {}", description);
            }
        }
    }
    
    Ok(())
}

fn get_vscode_config_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");
    match std::env::consts::OS {
        "macos" => PathBuf::from(home).join("Library/Application Support/Code/User"),
        "linux" => PathBuf::from(home).join(".config/Code/User"),
        "windows" => PathBuf::from(std::env::var("APPDATA").unwrap_or_default())
            .join("Code/User"),
        _ => PathBuf::from(home).join(".config/Code/User"), // Default to Linux path
    }
}

fn is_vscode_installed() -> bool {
    Command::new("code")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
```

### VS Code Extensions Restoration

```rust
// In src/tasks/restore_vscode.rs
pub fn restore_vscode_extensions(backup_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let extensions_file = backup_dir.join("vscode-extensions.txt");
    let settings_backup_dir = backup_dir.join("vscode");

    if !extensions_file.exists() {
        println!("ℹ️ No VS Code extensions backup found");
        return Ok(());
    }

    if !is_vscode_installed() {
        println!("⚠️ VS Code not installed, cannot restore extensions");
        return Err("VS Code not found".into());
    }

    // Restore extensions
    println!("🔄 Restoring VS Code extensions...");
    let extensions = fs::read_to_string(&extensions_file)?;
    let mut installed_count = 0;
    let mut failed_count = 0;

    for extension in extensions.lines() {
        let extension = extension.trim();
        if extension.is_empty() {
            continue;
        }

        print!("Installing {}... ", extension);
        let result = Command::new("code")
            .args(&["--install-extension", extension])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                println!("✅");
                installed_count += 1;
            }
            Ok(_) => {
                println!("❌");
                failed_count += 1;
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                failed_count += 1;
            }
        }
    }

    println!("📦 Extension restoration complete: {} installed, {} failed", 
            installed_count, failed_count);

    // Restore settings and keybindings
    restore_vscode_settings(&settings_backup_dir)?;

    Ok(())
}

fn restore_vscode_settings(backup_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !backup_dir.exists() {
        println!("ℹ️ No VS Code settings backup found");
        return Ok(());
    }

    let vscode_config_dir = get_vscode_config_dir();
    ensure_dir_exists(&vscode_config_dir)?;

    let files_to_restore = vec![
        ("settings.json", "User settings"),
        ("keybindings.json", "Keybindings"),
        ("snippets", "User snippets"),
    ];

    for (file_name, description) in files_to_restore {
        let source = backup_dir.join(file_name);
        let dest = vscode_config_dir.join(file_name);
        
        if source.exists() {
            if source.is_dir() {
                copy_directory_recursive(&source, &dest)?;
                println!("✅ Restored VS Code {}", description);
            } else {
                fs::copy(&source, &dest)?;
                println!("✅ Restored VS Code {}", description);
            }
        }
    }
    
    Ok(())
}
```

### Integration with Main Backup Command

```rust
// In src/tasks/backup.rs - Add VS Code extensions to main backup
pub fn run() {
    let backup_dir = get_backup_path();
    ensure_dir_exists(&backup_dir).expect("Failed to create backup directory");

    // Existing package manager backups
    backup_package_managers(&backup_dir);
    
    // Add VS Code extensions backup
    if let Err(e) = backup_vscode_extensions(&backup_dir) {
        println!("⚠️ Failed to backup VS Code extensions: {}", e);
    }
    
    // Existing file backups
    backup_files(&backup_dir);
}
```

## 4. Advanced Dotfiles Management

### Machine-Specific and Environment-Based Configurations

```rust
// In src/tasks/link.rs - Enhanced linking with machine profiles
pub fn run_with_profile(profile: Option<String>) {
    let backup_dir = get_backup_path();
    let machine_id = profile.unwrap_or_else(|| get_machine_identifier());
    
    // Link order: common -> machine-specific -> environment-specific
    let link_paths = vec![
        backup_dir.join("common"),
        backup_dir.join("machines").join(&machine_id),
        backup_dir.join("environments").join(&get_environment()), // dev, work, personal
    ];
    
    for path in link_paths {
        if path.exists() {
            link_directory_contents(&path)?;
        }
    }
}

fn get_machine_identifier() -> String {
    // Try to read from ~/.mntn/.machine-id first
    if let Ok(id) = fs::read_to_string(get_mntn_dir().join(".machine-id")) {
        return id.trim().to_string();
    }
    
    // Generate based on hostname and user
    format!("{}-{}",
        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
        gethostname::gethostname().to_string_lossy().to_lowercase()
    )
}
```

## 4. Enhanced Validation and Health Checks

### Configuration Validation Framework

```rust
// In src/tasks/validate.rs
pub trait Validator {
    fn validate(&self) -> Result<(), Vec<ValidationError>>;
    fn name(&self) -> &str;
}

pub struct ValidationError {
    pub severity: Severity,
    pub message: String,
    pub fix_suggestion: Option<String>,
}

pub enum Severity {
    Error,
    Warning,
    Info,
}

pub struct ConfigValidator {
    validators: Vec<Box<dyn Validator>>,
}

impl ConfigValidator {
    pub fn new() -> Self {
        let mut validators: Vec<Box<dyn Validator>> = vec![
            Box::new(JsonConfigValidator::new()),
            Box::new(ShellConfigValidator::new()),
            Box::new(SymlinkValidator::new()),
            Box::new(RegistryValidator::new()),
        ];
        
        Self { validators }
    }
    
    pub fn run_all(&self) -> ValidationReport {
        let mut report = ValidationReport::new();
        
        for validator in &self.validators {
            match validator.validate() {
                Ok(_) => report.add_success(validator.name()),
                Err(errors) => report.add_errors(validator.name(), errors),
            }
        }
        
        report
    }
}

struct JsonConfigValidator;

impl Validator for JsonConfigValidator {
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let backup_dir = get_backup_path();
        
        let json_files = [
            ("vscode/settings.json", "VS Code settings"),
            ("vscode/keybindings.json", "VS Code keybindings"),
        ];
        
        for (file_path, description) in json_files {
            let full_path = backup_dir.join(file_path);
            if full_path.exists() {
                if let Err(e) = fs::read_to_string(&full_path)
                    .and_then(|content| serde_json::from_str::<serde_json::Value>(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))) {
                    errors.push(ValidationError {
                        severity: Severity::Error,
                        message: format!("Invalid JSON in {}: {}", description, e),
                        fix_suggestion: Some(format!("Check syntax in {}", full_path.display())),
                    });
                }
            }
        }
        
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
    
    fn name(&self) -> &str {
        "JSON Configuration Files"
    }
}
```

## 5. Improved User Experience

### Interactive Setup Wizard

```rust
// In src/tasks/setup.rs
pub fn run_interactive_setup() {
    println!("🚀 Welcome to mntn interactive setup!");
    
    // Step 1: Choose what to manage
    let features = select_features();
    
    // Step 2: Configure package managers
    if features.contains("packages") {
        configure_package_managers();
    }
    
    // Step 3: Set up git repository
    if features.contains("git") {
        setup_git_repository();
    }
    
    // Step 4: Initial backup and link
    if confirm("Run initial backup?") {
        crate::tasks::backup::run();
    }
    
    if confirm("Link configurations now?") {
        crate::tasks::link::run();
    }
    
    println!("✅ Setup complete! Run 'mntn --help' to see available commands.");
}

fn select_features() -> Vec<String> {
    let options = vec![
        ("packages", "Package manager backup", true),
        ("dotfiles", "Dotfile management", true),
        ("git", "Git repository integration", false),
        ("automation", "Automated maintenance tasks", false),
    ];
    
    // Use inquire or similar crate for interactive selection
    // This is pseudocode - would need actual interactive library
    MultiSelect::new("Select features to enable:", options)
        .prompt()
        .unwrap_or_default()
}
```

## 6. Performance and Reliability

### Parallel Operations

```rust
// In src/tasks/backup.rs - Enhanced with parallelization
use rayon::prelude::*;

pub fn backup_package_managers_parallel(backup_dir: &PathBuf) {
    let package_registry = PackageRegistry::load_or_create(&get_package_registry_path())?;
    let current_platform = PackageRegistry::get_current_platform();
    let compatible_entries: Vec<_> = package_registry
        .get_platform_compatible_entries(&current_platform)
        .collect();

    // Run package manager commands in parallel
    let results: Vec<_> = compatible_entries
        .par_iter()
        .map(|(id, entry)| {
            let args: Vec<&str> = entry.args.iter().map(|s| s.as_str()).collect();
            let result = run_cmd(&entry.command, &args);
            (id, entry, result)
        })
        .collect();

    // Write results sequentially to avoid file conflicts
    for (id, entry, result) in results {
        match result {
            Ok(content) => {
                if let Err(e) = fs::write(backup_dir.join(&entry.output_file), content) {
                    println!("⚠️ Failed to write {}: {}", entry.output_file, e);
                } else {
                    println!("✅ Backed up {} ({})", entry.name, id);
                }
            }
            Err(e) => {
                println!("⚠️ Command for {} failed: {}", entry.name, e);
                let _ = fs::write(backup_dir.join(&entry.output_file), "");
            }
        }
    }
}
```