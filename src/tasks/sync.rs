use crate::cli::SyncArgs;
use crate::utils::paths::get_mntn_dir;
use crate::utils::system::run_cmd_in_dir;
use chrono::Utc;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Run git sync command with the provided arguments
pub fn run(args: SyncArgs) {
    if let Err(e) = sync_with_git(args) {
        eprintln!("❌ Sync failed: {}", e);
        std::process::exit(1);
    }
}

fn sync_with_git(args: SyncArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mntn_dir = get_mntn_dir();

    // Ensure git repository exists
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
        // Ensure .gitignore exists even if repo already exists
        ensure_gitignore_exists(&mntn_dir)?;
    }

    if args.pull || args.sync {
        println!("🔄 Pulling latest changes...");
        run_cmd_in_dir("git", &["pull"], &mntn_dir)?;
        println!("✅ Successfully pulled latest changes");

        // Re-link configurations after pull
        if args.auto_link {
            println!("🔗 Auto-linking configurations...");
            crate::tasks::link::run();
        }
    }

    if args.push || args.sync {
        // Auto-commit changes with timestamp
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let commit_msg = args
            .message
            .unwrap_or_else(|| format!("Update dotfiles - {}", timestamp));

        run_cmd_in_dir("git", &["add", "."], &mntn_dir)?;

        // Check if there are changes to commit
        let status = run_cmd_in_dir("git", &["status", "--porcelain"], &mntn_dir)?;
        if !status.trim().is_empty() {
            run_cmd_in_dir("git", &["commit", "-m", &commit_msg], &mntn_dir)?;
            run_cmd_in_dir("git", &["push"], &mntn_dir)?;
            println!("✅ Changes pushed to remote repository");
        } else {
            println!("ℹ️  No changes to commit");
        }
    }

    Ok(())
}

fn initialize_git_repo(
    mntn_dir: &PathBuf,
    remote_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Initializing git repository in {}", mntn_dir.display());

    // Initialize git repository
    run_cmd_in_dir("git", &["init"], mntn_dir)?;

    // Add remote origin
    run_cmd_in_dir("git", &["remote", "add", "origin", remote_url], mntn_dir)?;

    // Set default branch to main
    run_cmd_in_dir("git", &["branch", "-M", "main"], mntn_dir)?;

    println!("✅ Git repository initialized with remote: {}", remote_url);
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
        println!("✅ Created default .gitignore with mntn.log excluded");
    }
    Ok(())
}

fn ensure_gitignore_exists(mntn_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore_path = mntn_dir.join(".gitignore");
    if !gitignore_path.exists() {
        create_default_gitignore(mntn_dir)?;
    } else {
        // Check if mntn.log is in .gitignore, add if missing
        let content = fs::read_to_string(&gitignore_path)?;
        if !content.contains("mntn.log") && !content.contains("*.log") {
            let mut file = fs::OpenOptions::new().append(true).open(&gitignore_path)?;
            writeln!(file, "\n# mntn log files\nmntn.log")?;
            println!("✅ Added mntn.log to existing .gitignore");
        }
    }
    Ok(())
}
