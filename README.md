# mntn

A Rust-based CLI tool for system maintenance.

## Features

- **Backup**: Saves global package lists (e.g., brew, npm, cargo, bun, uv) and configuration files using registry-based management.
- **Biometric Sudo [macOS only]**: Configures Touch ID authentication for sudo commands.
- **Clean**: Removes system junk (caches, logs, etc) and runs package manager cleanup across all platforms.
- **Delete [macOS only]**: Removes applications and their related files with interactive selection.
- **Install**: Sets up automated services for backups, cleaning, and system updates on macOS (LaunchAgents), Linux (systemd), and Windows (Task Scheduler).
- **Link**: Creates symlinks for dotfiles (e.g., .zshrc, .vimrc, .config, VSCode settings) from your backup directory.
- **Package Registry**: Centralized management of package managers for backup operations with platform-specific support.
- **Purge**: Deletes unused services with user confirmation across all platforms.
- **Registry**: Centralized management of configuration files and directories using absolute paths for backup and linking.
- **Restore**: Restores configuration files from backups using the registry system.
- **Sync**: Git integration for synchronizing configurations across machines with automatic commit/push/pull operations.

## Installation

```bash
cargo install mntn
```

## Quick Start

```bash
# Create your first backup
mntn backup

# Set up automated maintenance
mntn install --with-clean

# Link your dotfiles to your system files (requires ~/.mntn/backup to exist)
mntn link

# Clean system junk
mntn clean

# Enable Touch ID for sudo (macOS only)
mntn biometric-sudo

# Sync your configurations with a git repository
mntn sync --init --remote-url https://github.com/yourusername/dotfiles.git
mntn sync --sync --auto-link
```

## Platform Support

mntn supports **macOS**, **Linux**, and **Windows** with platform-specific features:

- **All platforms**: backup, clean, install, link, purge, restore, registry management, sync
- **macOS only**: biometric-sudo, delete command, Homebrew cask support
- **Linux/Windows**: systemd services (Linux) and Task Scheduler (Windows) for automation

## Guides

### Backup and Restore Guide

#### Creating Backups

The `backup` command saves your system's package lists and configuration files to `~/.mntn/backup/`:

```bash
mntn backup
```

**What gets backed up:**
- **Package lists**: Managed through the package registry system:
  - Homebrew packages (`brew.txt`) - macOS/Linux only
  - Homebrew casks (`brew-cask.txt`) - macOS only  
  - npm global packages (`npm.txt`)
  - Yarn global packages (`yarn.txt`)
  - pnpm global packages (`pnpm.txt`)
  - Bun global packages (`bun.txt`)
  - uv packages (`uv.txt`)
  - Cargo installed packages (`cargo.txt`)
  - pip packages (`pip.txt`) - disabled by default
- **Configuration files**: Managed through the configuration registry:
  - Shell configurations (.zshrc, .vimrc, .gitconfig)
  - VSCode settings and keybindings
  - Ghostty terminal config (with platform-specific paths)
  - General config directory (~/.config)
  - Other registered dotfiles and application configs

**Backup location**: `~/.mntn/backup/`

#### Restoring from Backups

To restore your configuration files from a previous backup:

```bash
mntn restore
```

This will restore VS Code settings, keybindings, and Ghostty configuration from your backup.

**Note**: Package restoration must be done manually using the generated package lists. The package registry system ensures only enabled and platform-compatible package managers are backed up. For example:
```bash
# Restore Homebrew packages
brew install $(cat ~/.mntn/backup/brew.txt)

# Restore npm global packages (parse npm ls output format)
npm install -g $(cat ~/.mntn/backup/npm.txt | grep -E '^[├└]' | sed 's/^[├└]── //' | cut -d'@' -f1 | tr '\n' ' ')

# Restore cargo packages
while read -r line; do
  cargo install "$(echo "$line" | cut -d' ' -f1)"
done < ~/.mntn/backup/cargo.txt
```

### Configuration Management with Version Control

#### Setting up Version Control for Your Configurations

1. **Initialize git repository in the mntn directory:**
   ```bash
   # Create your first backup to set up the folder structure
   mntn backup
   
   # Initialize git repository in the mntn directory (includes full context)
   cd ~/.mntn
   git init
   git remote add origin https://github.com/yourusername/dotfiles.git
   
   # mntn will automatically create a .gitignore with mntn.log excluded
   ```

2. **Your mntn directory structure will look like:**
```
~/.mntn/
├── .git/               # Git repository
├── .gitignore          # Automatically created (excludes mntn.log)
├── registry.json       # Configuration registry
├── package_registry.json # Package manager registry
├── mntn.log           # Log file (ignored by git)
├── symlinks/          # Backup of original files before linking
└── backup/            # Your dotfiles and configs
    ├── .zshrc         # Shell configuration
    ├── .vimrc         # Vim configuration  
    ├── .gitconfig     # Git configuration
    ├── config/        # General config directory
    │   └── ...        # Various application configs
    ├── vscode/
    │   ├── settings.json
    │   └── keybindings.json
    ├── ghostty/
    │   └── config     # Terminal configuration
    ├── brew.txt       # Homebrew packages (macOS/Linux)
    ├── brew-cask.txt  # Homebrew casks (macOS only)
    ├── npm.txt        # npm global packages
    ├── yarn.txt       # Yarn global packages
    ├── pnpm.txt       # pnpm global packages
    ├── bun.txt        # Bun global packages
    ├── cargo.txt      # Cargo packages
    └── uv.txt         # uv tools
```3. **Commit and push your configurations:**
   ```bash
   cd ~/.mntn
   git add .
   git commit -m "Initial mntn setup with full context"
   git push -u origin main
   ```

#### Using mntn for Configuration Management

Once your backup repository is set up, use `mntn link` to create symlinks:

```bash
mntn link
```

**What it does:**
- Links files from `~/.mntn/backup/` to their target system locations based on the registry
- Examples for macOS:
  - `~/.mntn/backup/.zshrc` → `~/.zshrc`
  - `~/.mntn/backup/.vimrc` → `~/.vimrc`  
  - `~/.mntn/backup/config` → `~/.config`
  - `~/.mntn/backup/vscode/settings.json` → `~/Library/Application Support/Code/User/settings.json`
  - `~/.mntn/backup/vscode/keybindings.json` → `~/Library/Application Support/Code/User/keybindings.json`
  - `~/.mntn/backup/ghostty/config` → `~/Library/Application Support/com.mitchellh.ghostty/config` (macOS) or `~/.config/ghostty/config` (Linux)

**Safety features:**
- Automatically backs up existing files to `~/.mntn/symlinks/`
- If source doesn't exist but target does, copies target to source first
- Won't overwrite existing correct symlinks

#### Setting up on a New Machine

```bash
# Install mntn
cargo install mntn

# Clone your mntn repository (includes full context and registries)
git clone https://github.com/yourusername/dotfiles.git ~/.mntn

# Create symlinks for your configurations
mntn link

# Run backup to update with any new package installations
mntn backup

# The repository now includes your registries and full mntn context
```

### Git Integration and Sync Guide

The `sync` command provides seamless git integration for managing your mntn configurations across multiple machines. It automates the common git operations needed to keep your dotfiles synchronized.

#### Setting up Git Integration

**Initialize a new git repository:**
```bash
# Create your first backup to set up folder structure
mntn backup

# Initialize git repository in ~/.mntn with remote
mntn sync --init --remote-url https://github.com/yourusername/dotfiles.git

# This automatically:
# - Initializes git repository in ~/.mntn
# - Adds the remote origin
# - Creates a default .gitignore (excludes mntn.log)
# - Sets up main branch
```

**If you already have a repository:**
```bash
# Clone existing repository directly to ~/.mntn
git clone https://github.com/yourusername/dotfiles.git ~/.mntn

# The sync command will ensure .gitignore exists
mntn sync --pull
```

#### Sync Operations

**Pull latest changes:**
```bash
# Pull changes from remote repository
mntn sync --pull

# Pull and automatically re-link configurations
mntn sync --pull --auto-link
```

**Push local changes:**
```bash
# Push changes with automatic commit message
mntn sync --push

# Push with custom commit message
mntn sync --push --message "Update VS Code settings"
```

**Bidirectional sync:**
```bash
# Pull latest changes, then push any local changes
mntn sync --sync

# Same as above but also re-link after pulling
mntn sync --sync --auto-link
```

#### Automated Workflow Examples

**Daily workflow on main machine:**
```bash
# After making configuration changes
mntn backup                          # Update backup files
mntn sync --push --message "Daily backup"  # Push to remote
```

**Setting up a new machine:**
```bash
# Install mntn
cargo install mntn

# Clone your configurations
git clone https://github.com/yourusername/dotfiles.git ~/.mntn

# Link configurations to system locations
mntn link

# Keep in sync
mntn sync --pull --auto-link
```

**Working across multiple machines:**
```bash
# Before starting work (pull latest)
mntn sync --pull --auto-link

# After finishing work (push changes)
mntn backup
mntn sync --push --message "Work session updates"
```

#### Git Repository Structure

The sync command works with the entire `~/.mntn` directory as a git repository:

```
~/.mntn/                    # Git repository root
├── .git/                   # Git metadata
├── .gitignore             # Excludes mntn.log and temporary files
├── backup/                # Your dotfiles and configs
│   ├── .zshrc
│   ├── .vimrc
│   ├── vscode/
│   └── ...
├── registry.json          # Configuration registry
├── package_registry.json  # Package manager registry
├── symlinks/              # Backup of original files
└── mntn.log               # Ignored by git
```

**Benefits of this approach:**
- **Full context**: Registry files and all configurations are versioned together
- **Machine-independent**: Works the same way on any machine
- **Safe**: Automatic .gitignore prevents log files from being committed
- **Flexible**: Can organize backup/ directory however you prefer

#### Sync Command Options

```bash
# Initialize new repository
mntn sync --init --remote-url <URL>

# Pull operations
mntn sync --pull                    # Pull changes only
mntn sync --pull --auto-link        # Pull and re-link configs

# Push operations  
mntn sync --push                    # Push with auto-generated message
mntn sync --push -m "Custom msg"    # Push with custom message

# Bidirectional sync
mntn sync --sync                    # Pull then push
mntn sync --sync --auto-link        # Pull, re-link, then push
```

#### Troubleshooting Sync Issues

**Repository not found:**
```bash
# If you see "No git repository found"
mntn sync --init --remote-url https://github.com/yourusername/dotfiles.git
```

**Merge conflicts:**
```bash
# Handle conflicts manually in ~/.mntn
cd ~/.mntn
git status
# Edit conflicted files
git add .
git commit -m "Resolve merge conflicts"
mntn sync --push
```

**Authentication issues:**
```bash
# Set up SSH keys or use personal access tokens
# See GitHub documentation for authentication setup
```

### Package Registry Management

The `package-registry` command provides centralized management of package managers used during backup operations. This system allows you to configure which package managers to include, customize their commands, and control platform-specific behavior.

#### Viewing Package Manager Entries

```bash
# List all package manager entries
mntn package-registry list

# List only enabled entries
mntn package-registry list --enabled-only

# List only entries compatible with current platform
mntn package-registry list --platform-only
```

**Default Package Managers:**
- `brew` - Homebrew packages (macOS/Linux) - uses `brew leaves`
- `brew_cask` - Homebrew casks/applications (macOS only) - uses `brew list --cask`  
- `npm` - npm global packages (all platforms) - uses `npm ls -g`
- `yarn` - Yarn global packages (all platforms) - uses `yarn global list`
- `pnpm` - pnpm global packages (all platforms) - uses `pnpm ls -g`
- `bun` - Bun global packages (all platforms) - uses `bun pm ls -g`
- `cargo` - Cargo installed packages (all platforms) - uses `cargo install --list`
- `uv` - uv installed tools (all platforms) - uses `uv tool list`
- `pip` - pip packages (disabled by default) - uses `pip list --format=freeze`

#### Adding Custom Package Managers

```bash
# Add a new package manager
mntn package-registry add pipx \
  --name "pipx Applications" \
  --command "pipx" \
  --args "list" \
  --output-file "pipx.txt" \
  --description "pipx installed Python applications"

# Add with platform restrictions
mntn package-registry add winget \
  --name "Windows Package Manager" \
  --command "winget" \
  --args "list" \
  --output-file "winget.txt" \
  --platforms "windows"
```

#### Managing Package Manager Entries

```bash
# Enable or disable a package manager
mntn package-registry toggle npm --enable
mntn package-registry toggle pip --disable

# Remove a package manager from the registry
mntn package-registry remove custom_manager
```

#### Package Registry File Location

The package registry is stored as JSON at `~/.mntn/package_registry.json`. You can edit it manually if needed, but using the CLI commands is recommended for consistency.

**Example package manager entry:**
```json
{
  "name": "Homebrew Packages",
  "command": "brew",
  "args": ["leaves"],
  "output_file": "brew.txt",
  "enabled": true,
  "description": "Homebrew installed packages (leaves only)",
  "platforms": ["macos", "linux"]
}
```

### Configuration Registry Management

The `registry` command provides a centralized way to manage what configuration files and folders are backed up and linked. The registry stores metadata about each configuration entry including source paths, target locations, and categories.

#### Viewing Registry Entries

```bash
# List all entries in the registry
mntn registry list

# List only enabled entries
mntn registry list --enabled-only

# List entries in a specific category
mntn registry list --category editor
```

**Registry Categories:**
- `shell` - Shell configuration files (.zshrc, .bashrc, etc.)
- `editor` - Text editors and IDEs (vim, vscode, etc.)  
- `terminal` - Terminal emulators and related tools
- `system` - System-wide configuration
- `development` - Development tools and environments
- `application` - Application-specific configs

#### Adding New Entries

```bash
# Add a new configuration file to track
mntn registry add my_app_config \
  --name "My App Config" \
  --source "myapp/config.json" \
  --target "/Users/username/.config/myapp/config.json" \
  --category application \
  --description "Configuration for My App"
```

**Target Path:**
- Uses absolute paths to the actual system location where files should be linked
- Automatically resolves platform-specific paths (e.g., `~/Library/Application Support` on macOS, `~/.config` on Linux)
- Examples: `/Users/username/.zshrc`, `/Users/username/Library/Application Support/Code/User/settings.json`

#### Managing Entries

```bash
# Enable or disable an entry
mntn registry toggle my_app_config --enable
mntn registry toggle my_app_config --disable

# Remove an entry from the registry
mntn registry remove my_app_config
```

#### Registry File Location

The registry is stored as JSON at `~/.mntn/registry.json`. You can edit it manually if needed, but using the CLI commands is recommended for consistency.

**Example registry entry:**
```json
{
  "name": "Zsh Configuration",
  "source_path": ".zshrc",
  "target_path": "/Users/username/.zshrc",
  "category": "shell",
  "enabled": true,
  "description": "Main Zsh shell configuration file"
}
```

### Automated Maintenance Setup

The `install` command sets up automated maintenance tasks using your system's scheduler:

```bash
# Basic installation (backup every hour)
mntn install

# Include daily cleaning
mntn install --with-clean
```

**What gets installed:**

- **macOS**: Creates LaunchAgents in `~/Library/LaunchAgents/`
- **Linux**: Creates systemd user services and timers in `~/.config/systemd/user/`
- **Windows**: Creates scheduled tasks using Task Scheduler

**Scheduled tasks:**
- `mntn-backup`: Runs `mntn backup` every hour
- `mntn-clean`: Runs `mntn clean` daily (with `--with-clean` flag)
- `mntn-topgrade`: Runs `topgrade` daily (if topgrade is installed)

**Task logs:** 
- **macOS**: `/tmp/mntn-*.out` and `/tmp/mntn-*.err`
- **Linux**: Use `journalctl --user -u mntn-*.service` or `journalctl --user -u mntn-*.timer`
- **Windows**: Task Scheduler history and event logs

### System Cleaning Guide

The `clean` command removes unnecessary files and frees up disk space:

```bash
# Clean user-level files only
mntn clean

# Also clean system files (requires sudo)
mntn clean --system

# Preview what would be cleaned without actually deleting
mntn clean --dry-run
```

**What gets cleaned:**

**User-level cleanup (default):**
- Cache directories:
  - macOS: `~/Library/Caches`
  - Linux: `~/.cache`
- Temporary files and directories
- Application logs and saved states (macOS: `~/Library/Logs`, `~/Library/Saved Application State`)
- Quick Look cache reset (macOS only)

**System-level cleanup (with `--system`):**
- System caches:
  - macOS: `/Library/Caches`, `/System/Library/Caches`
  - Linux: `/var/cache`, `/tmp`
- System logs:
  - macOS: `/private/var/log`
  - Linux: `/var/log`
- Platform-specific cleanup:
  - macOS: Diagnostic reports, volume trash folders
  - Linux: Additional temp directories

**Package manager cleanup:**
- Homebrew: `brew cleanup` (macOS/Linux)
- npm: `npm cache clean --force`
- pnpm: `pnpm cache delete`
- Yarn: cache cleanup
- Other package managers as available

**Safety features:**
- Skips files modified in the last 24 hours
- Skips symbolic links
- Skips system-critical directories (`.X11-unix`, `systemd-private`, etc.)

### Service Management with Purge

Remove unused services and startup programs interactively:

```bash
# List and remove user services
mntn purge

# Include system services (requires sudo)
mntn purge --system

# Preview what would be removed
mntn purge --dry-run
```

**What it manages:**

- **macOS**: LaunchAgents (`.plist` files) in `~/Library/LaunchAgents/` and `/Library/LaunchAgents/`
- **Linux**: systemd user services in `~/.config/systemd/user/` and autostart programs in `~/.config/autostart/`
- **Windows**: Windows services and startup programs

**Interactive selection:**
- Lists all found services/programs
- Multi-select interface to choose what to delete
- Shows full paths for transparency
- Confirmation before deletion

### Biometric Sudo Setup (macOS)

Enable Touch ID authentication for sudo commands:

```bash
mntn biometric-sudo
```

**What it does:**
1. Backs up `/etc/pam.d/sudo` to `/etc/pam.d/sudo.bak`
2. Adds Touch ID PAM module (`pam_tid.so`) to the sudo configuration
3. Enables Touch ID authentication for all sudo commands

**After setup:**
- Use Touch ID instead of typing your password for sudo commands
- Fallback to password if Touch ID fails
- Works with Terminal, VS Code integrated terminal, and other applications

**Requirements:**
- macOS with Touch ID capability
- Administrator privileges (will prompt for password during setup)

## Troubleshooting

### Backup Issues
- **Permission denied**: Ensure you have read access to config directories
- **Missing package managers**: Commands will be skipped if tools aren't installed

### Link Issues
- **Symlink conflicts**: Use `mntn purge` to clean up old services, then retry
- **Permission issues**: Ensure write access to target directories

### Clean Issues
- **System clean fails**: Use `mntn clean --system` and enter password when prompted
- **Space not freed**: Some applications may recreate caches immediately

### Restore Issues
- **Files not found**: Run `mntn backup` first to create initial backups
- **Permission denied**: Ensure write access to target config directories

### Sync Issues
- **No git repository found**: Use `mntn sync --init --remote-url <URL>` to initialize
- **Authentication failed**: Set up SSH keys or GitHub personal access tokens
- **Merge conflicts**: Resolve manually in `~/.mntn` directory using standard git commands
- **Permission denied on push**: Check repository permissions and authentication
- **Remote URL required**: Use `--remote-url` flag when initializing with `--init`

## License

MIT
