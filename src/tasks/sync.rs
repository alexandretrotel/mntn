use crate::cli::SyncArgs;
use crate::logger::{log_info, log_success};
use crate::tasks::core::{PlannedOperation, Task, TaskExecutor};
use crate::utils::paths::get_mntn_dir;
use crate::utils::system::run_cmd_in_dir;
use chrono::Utc;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Sync task that synchronizes configurations with a git repository
pub struct SyncTask {
    pub init: bool,
    pub remote_url: Option<String>,
    pub pull: bool,
    pub push: bool,
    pub sync: bool,
    pub message: Option<String>,
    pub auto_restore: bool,
    pub dry_run: bool,
    pub status: bool,
}

impl SyncTask {
    pub fn from_args(args: &SyncArgs) -> Self {
        Self {
            init: args.init,
            remote_url: args.remote_url.clone(),
            pull: args.pull,
            push: args.push,
            sync: args.sync,
            message: args.message.clone(),
            auto_restore: args.auto_restore,
            dry_run: args.dry_run,
            status: args.status,
        }
    }
}

impl Task for SyncTask {
    fn name(&self) -> &str {
        "Sync"
    }

    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let args = SyncArgs {
            init: self.init,
            remote_url: self.remote_url.clone(),
            pull: self.pull,
            push: self.push,
            sync: self.sync,
            message: self.message.clone(),
            auto_restore: self.auto_restore,
            dry_run: self.dry_run,
            status: self.status,
        };

        if args.status {
            show_git_status()?;
            return Ok(());
        }

        sync_with_git(args)?;

        Ok(())
    }

    fn dry_run(&self) -> Vec<PlannedOperation> {
        let mut operations = Vec::new();
        let mntn_dir = get_mntn_dir();

        if self.init {
            operations.push(PlannedOperation::with_target(
                "Initialize git repository".to_string(),
                mntn_dir.display().to_string(),
            ));
            if let Some(url) = &self.remote_url {
                operations.push(PlannedOperation::with_target(
                    "Add remote origin".to_string(),
                    url.clone(),
                ));
            }
        }

        if self.pull || self.sync {
            operations.push(PlannedOperation::new("Pull latest changes from remote"));
            if self.auto_restore {
                operations.push(PlannedOperation::new(
                    "Auto-restore configurations after pull",
                ));
            }
        }

        if self.push || self.sync {
            operations.push(PlannedOperation::new("Stage all changes"));
            operations.push(PlannedOperation::new("Commit changes"));
            operations.push(PlannedOperation::new("Push to remote repository"));
        }

        operations
    }
}

/// Run with CLI args
pub fn run_with_args(args: SyncArgs) {
    let mut task = SyncTask::from_args(&args);
    TaskExecutor::run(&mut task, args.dry_run);
}

fn sync_with_git(args: SyncArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mntn_dir = get_mntn_dir();

    if !mntn_dir.join(".git").exists() {
        if args.init {
            if args.remote_url.is_none() {
                return Err("Remote URL is required when using --init".into());
            }
            initialize_git_repo(&mntn_dir, args.remote_url.as_ref().unwrap())?;
            create_default_gitignore(&mntn_dir)?;
        } else {
            return Err(
                "No git repository found. Use --init with --remote-url to initialize.".into(),
            );
        }
    } else {
        if args.init {
            log_info("A git repository already exists in this directory. Skipping initialization.");
        }
        ensure_gitignore_exists(&mntn_dir)?;
    }

    if (args.pull || args.sync) && !args.init {
        println!("Pulling latest changes...");
        // Explicitly pull from origin/<branch> to avoid relying on tracking info
        let branch = crate::utils::system::get_current_git_branch(&mntn_dir)?;
        run_cmd_in_dir("git", &["pull", "origin", &branch], &mntn_dir)?;
        log_success("Successfully pulled latest changes");

        if args.auto_restore {
            println!("Auto-restoring configurations...");
            crate::tasks::restore::run_with_args(crate::cli::RestoreArgs { dry_run: false });
        }
    }

    if args.push || args.sync {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let commit_msg = args
            .message
            .unwrap_or_else(|| format!("Update dotfiles - {}", timestamp));

        run_cmd_in_dir("git", &["add", "."], &mntn_dir)?;

        let status = run_cmd_in_dir("git", &["status", "--porcelain"], &mntn_dir)?;
        if !status.trim().is_empty() {
            run_cmd_in_dir("git", &["commit", "-m", &commit_msg], &mntn_dir)?;
            run_cmd_in_dir("git", &["push"], &mntn_dir)?;
            log_success("Changes pushed to remote repository");
        } else {
            log_info("No changes to commit");
        }
    }

    Ok(())
}

fn initialize_git_repo(
    mntn_dir: &Path,
    remote_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing git repository in {}", mntn_dir.display());

    run_cmd_in_dir("git", &["init"], mntn_dir)?;
    run_cmd_in_dir("git", &["remote", "add", "origin", remote_url], mntn_dir)?;
    run_cmd_in_dir("git", &["branch", "-M", "main"], mntn_dir)?;

    log_success(&format!(
        "Git repository initialized with remote: {}",
        remote_url
    ));
    Ok(())
}

fn create_default_gitignore(mntn_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = mntn_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let default_gitignore = "# mntn log files
mntn.log
*.log

# Temporary files
.DS_Store
Thumbs.db

# Editor files
.vscode/
.idea/

# OS generated files
*~
*.swp
*.swo

# Backup directories (optional - uncomment if you don't want to track these)
# symlinks/
";
        fs::write(&gitignore_path, default_gitignore)?;
        log_success("Created default .gitignore with mntn.log excluded");
    }
    Ok(())
}

fn show_git_status() -> Result<(), Box<dyn std::error::Error>> {
    let mntn_dir = get_mntn_dir();
    let output = run_cmd_in_dir("git", &["status", "--short", "--branch"], &mntn_dir)?;
    println!("{}", output);
    Ok(())
}

fn ensure_gitignore_exists(mntn_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = mntn_dir.join(".gitignore");
    if !gitignore_path.exists() {
        create_default_gitignore(mntn_dir)?;
    } else {
        let content = fs::read_to_string(&gitignore_path)?;
        if !content.contains("mntn.log") && !content.contains("*.log") {
            let mut file = fs::OpenOptions::new().append(true).open(&gitignore_path)?;
            writeln!(file, "\n# mntn log files\nmntn.log")?;
            log_success("Added mntn.log to existing .gitignore");
        }
    }
    Ok(())
}
