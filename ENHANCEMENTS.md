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

## 2. Advanced Dotfiles Management

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

## 3. Improved User Experience

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

## 4. Performance and Reliability

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
