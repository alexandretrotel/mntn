# Suggested Enhancements for mntn

## 1. Code Mutualization and Architecture Improvements

### Registry Core Logic Abstraction

The current implementation has two separate registry systems (`LinkRegistry` and `PackageRegistry`) with similar functionality. We should create a generic registry trait to reduce code duplication:

```rust
// In src/registry/core.rs
pub trait Registry<T> {
    fn load_or_create(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
    fn save(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>>;
    fn get_enabled_entries(&self) -> impl Iterator<Item = (&String, &T)>;
    fn add_entry(&mut self, id: String, entry: T);
    fn remove_entry(&mut self, id: &str) -> Option<T>;
    fn set_entry_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String>;
    fn get_entry(&self, id: &str) -> Option<&T>;
}

// Common registry operations
pub struct RegistryManager<T> {
    pub version: String,
    pub entries: HashMap<String, T>,
}

impl<T> Registry<T> for RegistryManager<T>
where
    T: Serialize + DeserializeOwned + HasEnabled,
{
    // Implementation shared by both registries
}

trait HasEnabled {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}
```

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

## 3. Advanced Dotfiles Management

### Git Integration and Sync Command

Add `mntn sync` command for seamless git integration:

```rust
// In src/tasks/sync.rs
pub fn run(args: SyncArgs) {
    let mntn_dir = get_mntn_dir(); // ~/.mntn instead of ~/.mntn/backup
    
    // Ensure git repository exists
    if !mntn_dir.join(".git").exists() {
        if args.init {
            initialize_git_repo(&mntn_dir, &args.remote_url)?;
            create_default_gitignore(&mntn_dir)?;
        } else {
            return Err("No git repository found. Use --init to initialize.".into());
        }
    } else {
        // Ensure .gitignore exists even if repo already exists
        ensure_gitignore_exists(&mntn_dir)?;
    }
    
    if args.pull || args.sync {
        println!("🔄 Pulling latest changes...");
        run_cmd_in_dir("git", &["pull"], &mntn_dir)?;
        
        // Re-link configurations after pull
        if args.auto_link {
            crate::tasks::link::run();
        }
    }
    
    if args.push || args.sync {
        // Auto-commit changes with timestamp
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let commit_msg = args.message.unwrap_or_else(|| format!("Update dotfiles - {}", timestamp));
        
        run_cmd_in_dir("git", &["add", "."], &mntn_dir)?;
        
        // Check if there are changes to commit
        let status = run_cmd_in_dir("git", &["status", "--porcelain"], &mntn_dir)?;
        if !status.trim().is_empty() {
            run_cmd_in_dir("git", &["commit", "-m", &commit_msg], &mntn_dir)?;
            run_cmd_in_dir("git", &["push"], &mntn_dir)?;
            println!("✅ Changes pushed to remote repository");
        } else {
            println!("ℹ️ No changes to commit");
        }
    }
}

fn create_default_gitignore(mntn_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = mntn_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let default_gitignore = "# mntn log files\nmntn.log\n*.log\n\n# Temporary files\n.DS_Store\nThumbs.db\n\n# Editor files\n.vscode/\n.idea/\n\n# OS generated files\n*~\n.swp\n.swo\n";
        fs::write(&gitignore_path, default_gitignore)?;
        println!("✅ Created default .gitignore with mntn.log excluded");
    }
    Ok(())
}

fn ensure_gitignore_exists(mntn_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = mntn_dir.join(".gitignore");
    if !gitignore_path.exists() {
        create_default_gitignore(mntn_dir)?;
    } else {
        // Check if mntn.log is in .gitignore, add if missing
        let content = fs::read_to_string(&gitignore_path)?;
        if !content.contains("mntn.log") && !content.contains("*.log") {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&gitignore_path)?;
            writeln!(file, "\n# mntn log files\nmntn.log")?;
            println!("✅ Added mntn.log to existing .gitignore");
        }
    }
    Ok(())
}
```

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