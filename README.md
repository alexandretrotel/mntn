# mntn

A Rust-based CLI tool for system maintenance and dotfiles management with a profile-based architecture.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Platform Support](#platform-support)
- [Guides](#guides)
  - [Interactive Setup Wizard](#interactive-setup-wizard)
  - [Profile-Based Configuration](#profile-based-configuration)
    - [Directory Structure](#directory-structure)
    - [Layer Priority](#layer-priority)
    - [Creating and Managing Profiles](#creating-and-managing-profiles)
    - [Switching Between Profiles](#switching-between-profiles)
  - [Backup and Restore Guide](#backup-and-restore-guide)
  - [Migration Guide](#migration-guide)
  - [Configuration Management with Version Control](#configuration-management-with-version-control)
  - [Git Integration and Sync Guide](#git-integration-and-sync-guide)
  - [Validation Guide](#validation-guide)
  - [Package Registry Management](#package-registry-management)
  - [Configuration Registry Management](#configuration-registry-management)
  - [Automated Maintenance Setup](#automated-maintenance-setup)
  - [System Cleaning Guide](#system-cleaning-guide)
  - [Service Management with Purge](#service-management-with-purge)
  - [Biometric Sudo Setup (macOS)](#biometric-sudo-setup-macos)
- [Troubleshooting](#troubleshooting)
- [License](#license)

## Features

- **Setup**: Interactive wizard to configure mntn with guided profile creation.
- **Profile-Based Dotfiles**: Flexible profile system for different contexts (work, personal, gaming, etc.).
- **Backup**: Copies configuration files and package lists to layered backup directories.
- **Restore**: Restores configuration files from backups using profile-aware resolution.
- **Use**: Quickly switch between different profiles for different workflows or machines.
- **Profile**: Manage profiles (list, create, delete).
- **Biometric Sudo [macOS only]**: Configures Touch ID authentication for sudo commands.
- **Clean**: Removes system junk (caches, logs, etc) and runs package manager cleanup.
- **Delete [macOS only]**: Removes applications and their related files with interactive selection.
- **Install**: Sets up automated services for backups, cleaning, and system updates.
- **Migrate**: Moves configuration files from legacy structure to the new profile-based system.
- **Package Registry**: Centralized management of package managers for backup operations.
- **Purge**: Deletes unused services with user confirmation.
- **Registry**: Centralized management of configuration files and directories.
- **Sync**: Git integration for synchronizing configurations across machines.
- **Validate**: Checks configuration files and layer resolution status.

## Installation

```bash
cargo install mntn
```

## Quick Start

```bash
# Run the interactive setup wizard (recommended for new users)
mntn setup

# Or manually configure:
mntn backup                    # Create your first backup
mntn restore                   # Restore configurations from backup
mntn validate                  # Check configuration status

# After editing config files, save your changes:
mntn backup                    # Sync your changes to the backup

# Enable Touch ID for sudo (macOS only)
mntn biometric-sudo

# Sync with git repository
mntn sync --init --remote-url https://github.com/yourusername/dotfiles.git
mntn sync --sync
```

## Platform Support

mntn supports **macOS**, **Linux**, and **Windows** with platform-specific features:

- **All platforms**: setup, backup, clean, install, migrate, purge, restore, registry management, switch, sync, validate
- **macOS only**: biometric-sudo, delete command, Homebrew cask support
- **Linux/Windows**: systemd services (Linux) and Task Scheduler (Windows) for automation

## Guides

### Interactive Setup Wizard

The `setup` command provides an interactive wizard for configuring mntn:

```bash
mntn setup
```

**The wizard guides you through:**

1. **Profile Creation**: Create a profile or use common configuration only
2. **Legacy Migration**: Automatically detect and migrate existing configs to the layered structure
3. **Initial Backup**: Optionally run first backup
4. **Scheduled Tasks**: Optionally install automated backups

**Example output:**
```text
Welcome to mntn interactive setup!
   This wizard will help you configure your dotfiles management.

Profile Setup
   Profiles let you maintain different configurations for different contexts
   (e.g., 'work', 'personal', 'minimal', 'gaming')

? Create a profile now? Yes
? Profile name: work
   ✓ Profile: work

Legacy Files Detected
   Found files in ~/.mntn/backup/ that aren't in the layered structure.
? Migrate legacy files to common/ layer? Yes

Setup Summary:
   Profile: work
   ✓ Migrate legacy files to common/
   ✓ Run initial backup

? Proceed with setup? Yes

Setup complete!

Quick reference:
   mntn backup          - Backup your configurations
   mntn restore         - Restore configurations from backup
   mntn use <name>      - Switch to a different profile
   mntn profile         - List and manage profiles
   mntn validate        - Check configuration status

   Remember: Run 'mntn backup' after editing config files!
```

### Profile-Based Configuration

mntn uses a **profile-based architecture** that simplifies configuration management by focusing on profiles rather than machines or environments. This makes it easy to switch between different setups on any computer and share configurations across devices.

#### Directory Structure

```
~/.mntn/
├── backup/
│   ├── common/                    # Shared across all profiles
│   │   ├── .zshrc
│   │   ├── .vimrc
│   │   └── config/
│   ├── profiles/
│   │   ├── work/                  # Profile-specific configs
│   │   │   ├── .gitconfig
│   │   │   └── config/zed/settings.json
│   │   ├── personal/
│   │   │   └── .gitconfig
│   │   └── gaming/
│   │       └── .zshrc
│   └── packages/                  # Package lists
│       ├── brew.txt
│       └── npm.txt
├── profile.json                   # Profile definitions
├── .active-profile                # Currently active profile
├── configs_registry.json          # Tracked config files
└── package_registry.json          # Package managers to backup
```

#### Layer Priority

When restoring, mntn resolves sources in this order (highest priority first):

1. **Profile** (`profiles/<profile-name>/`) - Profile-specific (if active)
2. **Common** (`common/`) - Shared across all profiles
3. **Legacy** (root `backup/`) - Backwards compatibility

**Example:** If you have:
- `common/.zshrc` - Base shell config
- `profiles/work/.zshrc` - Work-specific shell config

With profile `work` active, the work version is used. Without an active profile, the common version is used.

#### Creating and Managing Profiles

```bash
# List all profiles and show current
mntn profile

# Create a new profile
mntn profile create work --description "Work laptop configuration"

# List all profiles
mntn profile list

# Delete a profile
mntn profile delete old-profile
```

**Profile Examples:**
- `work` - Work-specific configurations (corporate git, work-specific tools)
- `personal` - Personal machine configurations
- `minimal` - Minimal setup for servers or lightweight environments
- `gaming` - Gaming-focused configuration with different shell aliases
- `presentation` - Clean setup for presentations or demos

#### Switching Between Profiles

```bash
# Switch to a profile
mntn use work

# View current profile and available profiles
mntn profile

# Switch back to common only (no profile)
mntn use common
```

**Note:** After switching profiles, run `mntn restore` to apply the profile's configurations.

### Backup and Restore Guide

mntn uses a **copy-based** approach for managing dotfiles. This means:
- **Backup** copies files from your system to the backup location
- **Restore** copies files from the backup location to your system
- Changes to config files require running `mntn backup` to sync

#### Creating Backups

```bash
# Backup to active profile (or common if no profile is active)
mntn backup

# Backup to a specific profile
mntn backup --profile work

# Preview what would be backed up
mntn backup --dry-run
```

**What gets backed up:**
- **Package lists**: brew, npm, cargo, bun, uv, etc. (stored in `backup/packages/`)
- **Configuration files**: Based on registry entries (stored in active profile or common)

#### Restoring from Backups

```bash
# Restore using active profile
mntn restore

# Preview what would be restored
mntn restore --dry-run
```

Restore uses layer resolution to find the best source for each config file.

#### Workflow

The recommended workflow with mntn is:

1. **Edit** your config files normally (e.g., `~/.zshrc`)
2. **Backup** your changes: `mntn backup`
3. **Commit** to git: `cd ~/.mntn && git add . && git commit -m "Update configs"`
4. **Push** to remote: `git push`

On another machine:
1. **Pull** latest: `cd ~/.mntn && git pull` (or `mntn sync --pull`)
2. **Restore** configs: `mntn restore`

### Migration Guide

The `migrate` command moves files from the legacy location (flat backup directory) to the layered structure:

```bash
# Preview what would be migrated
mntn migrate --dry-run

# Migrate legacy files to common layer
mntn migrate
```

**When to migrate:**
- After upgrading from an older version of mntn
- When you have files in `~/.mntn/backup/` that aren't in `common/` or `profiles/`
- When you have legacy symlinks that need to be converted to real files

**What it does:**
- Moves files from `backup/` to `backup/common/`
- Converts symlinks to real files
- Preserves your data while updating the structure

### Configuration Management with Version Control

#### Setting up Version Control

```bash
# Create your first backup
mntn backup

# Initialize git repository
cd ~/.mntn
git init
git remote add origin https://github.com/yourusername/dotfiles.git

# Commit everything
git add .
git commit -m "Initial mntn setup"
git push -u origin main
```

#### Repository Structure

```
~/.mntn/                    # Git repository root
├── .git/
├── .gitignore              # Excludes mntn.log
├── profile.json            # Profile definitions (versioned)
├── .active-profile         # Current profile (may want to .gitignore)
├── configs_registry.json   # Configuration registry
├── package_registry.json   # Package manager registry
└── backup/
    ├── common/             # Shared configs
    └── profiles/           # Profile-specific configs
```

**Tip:** Add `.active-profile` to `.gitignore` if you want each machine to maintain its own active profile.

#### Setting up a New Machine

```bash
# Install mntn
cargo install mntn

# Clone your configurations
git clone https://github.com/yourusername/dotfiles.git ~/.mntn

# Run setup wizard or manually switch profile
mntn setup
# or
mntn use work

# Restore configurations
mntn restore
```

### Git Integration and Sync Guide

```bash
# Initialize new repository
mntn sync --init --remote-url https://github.com/yourusername/dotfiles.git

# Pull latest changes
mntn sync --pull

# Push local changes
mntn sync --push --message "Update configs"

# Bidirectional sync
mntn sync --sync

# Auto-restore after pull
mntn sync --pull --auto-restore
```

### Validation Guide

The `validate` command checks your configuration status:

```bash
mntn validate
```

**What it validates:**
- **Registry files**: JSON syntax and consistency
- **Layer resolution**: Shows which layer each config comes from
- **JSON configs**: Validates VS Code, Zed settings syntax
- **Legacy symlinks**: Detects old symlink-based configurations

**Example output:**
```
Validating configuration...
   Profile: profile=work

 Registry Files OK
 Layer Resolution OK
 JSON Configuration Files OK
 Legacy Symlink Check OK

Validation complete: 0 error(s), 0 warning(s)
```

### Package Registry Management

```bash
# List all package managers
mntn registry-packages list

# List platform-compatible entries
mntn registry-packages list --platform-only

# Add custom package manager
mntn registry-packages add pipx \
  --name "pipx Applications" \
  --command "pipx" \
  --args "list" \
  --output-file "pipx.txt"

# Enable/disable entries
mntn registry-packages toggle pip --enable
```

### Configuration Registry Management

```bash
# List all entries
mntn registry-configs list

# Add new entry
mntn registry-configs add my_config \
  --name "My Config" \
  --source "myapp/config.json" \
  --target "/Users/alex/.config/myapp/config.json"

# Enable/disable entries
mntn registry-configs toggle my_config --enable
```

### Automated Maintenance Setup

```bash
# Install hourly backup task
mntn install

# Include daily cleaning
mntn install --with-clean

# Preview what would be installed
mntn install --dry-run
```

### System Cleaning Guide

```bash
# Clean user-level files
mntn clean

# Include system files (requires sudo)
mntn clean --system

# Preview what would be cleaned
mntn clean --dry-run
```

### Service Management with Purge

```bash
# Remove unused user services
mntn purge

# Include system services
mntn purge --system

# Preview what would be removed
mntn purge --dry-run
```

### Biometric Sudo Setup (macOS)

```bash
mntn biometric-sudo
```

Enables Touch ID authentication for sudo commands.

## Troubleshooting

### Setup Issues
- **Profile not saving**: Ensure `~/.mntn/` directory exists and is writable
- **Can't create profile**: Check profile name contains only letters, numbers, hyphens, and underscores

### Backup Issues
- **Wrong profile used**: Check active profile with `mntn profile`.
- **Permission denied**: Ensure read access to config directories

### Changes Not Saved
- **Symptom**: Edited config file but changes not in backup
- **Solution**: Run `mntn backup` after editing config files to sync changes

### Restore Issues
- **Wrong version restored**: Check active profile with `mntn use` and layer priority with `mntn validate`
- **Permission denied**: Ensure write access to target directories

### Migration Issues
- **Files not detected**: Only registry entries are migrated; add entries first with `mntn registry-configs add`
- **Already migrated**: Files in `common/` or `profiles/` are skipped
- **Legacy symlinks**: Run `mntn migrate` to convert symlinks to real files

### Sync Issues
- **No git repository**: Run `mntn sync --init --remote-url <URL>`
- **Merge conflicts**: Resolve in `~/.mntn` using standard git commands

### Profile Issues
- **Can't switch profiles**: Ensure profile exists with `mntn profile list`
- **Profile not found**: Create it with `mntn profile create <name>`
- **Can't delete active profile**: Switch to another profile first with `mntn use <other-profile>`

### Validation Issues
- **Legacy files warning**: Run `mntn migrate` to update structure
- **Legacy symlinks warning**: Run `mntn backup` or `mntn migrate` to convert to real files
- **Layer conflicts**: Multiple layers with same file is intentional for overrides

## License

MIT
