# mntn

A Rust-based CLI tool for system maintenance and dotfiles management.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Platform Support](#platform-support)
- [Guides](#guides)
  - [Interactive Setup Wizard](#interactive-setup-wizard)
  - [Layered Dotfiles Management](#layered-dotfiles-management)
    - [Directory Structure](#directory-structure)
    - [Layer Priority](#layer-priority)
    - [Profile Configuration](#profile-configuration)
    - [Using Profiles](#using-profiles)
  - [Migration Guide](#migration-guide)
  - [Backup and Restore Guide](#backup-and-restore-guide)
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

- **Setup**: Interactive wizard to configure mntn for your system with guided machine/environment setup.
- **Layered Dotfiles**: Machine-specific and environment-based configuration management with automatic layer resolution.
- **Backup**: Copies configuration files and package lists to layered backup directories.
- **Restore**: Restores configuration files from backups using profile-aware resolution.
- **Biometric Sudo [macOS only]**: Configures Touch ID authentication for sudo commands.
- **Clean**: Removes system junk (caches, logs, etc) and runs package manager cleanup.
- **Delete [macOS only]**: Removes applications and their related files with interactive selection.
- **Install**: Sets up automated services for backups, cleaning, and system updates.
- **Migrate**: Moves configuration files between layers (common, machine, environment) and cleans up legacy symlinks.
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

- **All platforms**: setup, backup, clean, install, migrate, purge, restore, registry management, sync, validate
- **macOS only**: biometric-sudo, delete command, Homebrew cask support
- **Linux/Windows**: systemd services (Linux) and Task Scheduler (Windows) for automation

## Guides

### Interactive Setup Wizard

The `setup` command provides an interactive wizard for configuring mntn:

```bash
mntn setup
```

**The wizard guides you through:**

1. **Machine Identifier**: Set a custom name or use auto-detected hostname
2. **Environment Selection**: Choose from default, work, personal, dev, or custom
3. **Legacy Migration**: Automatically detect and migrate existing configs to the layered structure
4. **Initial Backup**: Optionally run first backup
5. **Scheduled Tasks**: Optionally install automated backups

**Example output:**
```
Welcome to mntn interactive setup!
   This wizard will help you configure your dotfiles management.

Machine Identifier
   Auto-detected: alex-macbook-pro
? Set a custom machine identifier? Yes
? Enter machine identifier: work-laptop
   Saved machine ID: work-laptop

Environment
? Select your environment: work
   Environment: work

Legacy Files Detected
   Found files in ~/.mntn/backup/ that aren't in the layered structure.
? Migrate legacy files to common/ layer? Yes

Setup Summary:
   Machine ID: work-laptop
   Environment: work
   Migrate legacy files to common/
   Run initial backup

? Proceed with setup? Yes

Setup complete!

Quick reference:
   mntn backup          - Backup your configurations
   mntn restore         - Restore configurations from backup
   mntn validate        - Check configuration status
   mntn migrate         - Move files between layers
   mntn sync --help     - Git sync options

   Remember: Run 'mntn backup' after editing config files!
```

### Layered Dotfiles Management

mntn supports a layered approach to dotfiles management, allowing you to have:
- **Common configs** shared across all machines
- **Machine-specific configs** for individual computers
- **Environment-specific configs** for different contexts (work, personal, dev)

#### Directory Structure

```
~/.mntn/
├── backup/
│   ├── common/                    # Shared across all machines
│   │   ├── .zshrc
│   │   ├── .vimrc
│   │   └── config/
│   ├── machines/
│   │   ├── work-laptop/           # Machine-specific overrides
│   │   │   └── .gitconfig
│   │   └── home-desktop/
│   │       └── .zshrc
│   ├── environments/
│   │   ├── work/                  # Environment-specific overrides
│   │   │   └── config/git/config
│   │   └── personal/
│   │       └── .gitconfig
│   ├── brew.txt                   # Package lists (at root level)
│   └── npm.txt
├── profile.json                   # Profile definitions
├── .machine-id                    # Current machine identifier
├── configs_registry.json          # Configuration registry
└── package_registry.json          # Package manager registry
```

#### Layer Priority

When restoring, mntn resolves sources in this order (highest priority first):

1. **Environment** (`environments/<env>/`) - Most specific
2. **Machine** (`machines/<machine-id>/`)
3. **Common** (`common/`)
4. **Legacy** (root `backup/`) - Lowest priority, for backwards compatibility

**Example:** If you have:
- `common/.zshrc` - Base shell config
- `machines/work-laptop/.zshrc` - Machine-specific overrides
- `environments/work/.zshrc` - Environment-specific overrides

With `machine=work-laptop` and `env=work`, the environment version is used.

#### Profile Configuration

Create named profiles in `~/.mntn/profile.json`:

```json
{
  "version": "1.0.0",
  "default_profile": "work-laptop-work",
  "profiles": {
    "work-laptop-work": {
      "machine_id": "work-laptop",
      "environment": "work",
      "description": "Work laptop in work environment"
    },
    "home": {
      "machine_id": "home-desktop",
      "environment": "personal",
      "description": "Home desktop configuration"
    }
  }
}
```

#### Using Profiles

**Override via CLI flags:**
```bash
# Use specific environment
mntn restore --env work

# Use specific machine
mntn restore --machine-id work-laptop

# Use both
mntn backup --env work --machine-id work-laptop --to-machine

# Use a named profile
mntn restore --profile home
```

**Environment variable:**
```bash
export MNTN_ENV=work
mntn restore  # Uses 'work' environment
```

**Machine ID file:**
```bash
echo "work-laptop" > ~/.mntn/.machine-id
mntn restore  # Uses 'work-laptop' machine
```

### Migration Guide

The `migrate` command moves files from the legacy location to the layered structure and cleans up legacy symlinks:

```bash
# Preview what would be migrated
mntn migrate --dry-run

# Migrate to common layer (recommended for shared configs)
mntn migrate --to-common

# Migrate to machine-specific layer
mntn migrate --to-machine

# Migrate to environment-specific layer
mntn migrate --to-environment --env work
```

**When to use each layer:**

| Layer | Use Case |
|-------|----------|
| `--to-common` | Configs shared across all machines (default) |
| `--to-machine` | Hardware-specific settings, paths with machine names |
| `--to-environment` | Context-specific: work email, personal git config |

#### Migrating from Symlinks

If you previously used mntn with symlinks, the `migrate` command will automatically convert them to real files. Your data is preserved - the content from the backup location becomes a real file at the system location.

After migration, remember to run `mntn backup` after editing config files to keep them in sync.

### Backup and Restore Guide

mntn uses a **copy-based** approach for managing dotfiles. This means:
- **Backup** copies files from your system to the backup location
- **Restore** copies files from the backup location to your system
- Changes to config files require running `mntn backup` to sync

#### Creating Backups

```bash
# Backup to common layer (default)
mntn backup

# Backup to machine-specific layer
mntn backup --to-machine

# Backup to environment layer
mntn backup --to-environment --env work

# Preview what would be backed up
mntn backup --dry-run
```

**What gets backed up:**
- **Package lists**: brew, npm, cargo, bun, uv, etc. (stored at backup root)
- **Configuration files**: Based on registry entries (stored in specified layer)

#### Restoring from Backups

```bash
# Restore using current profile settings
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

### Configuration Management with Version Control

#### Setting up Version Control

```bash
# Create your first backup
mntn backup

# Initialize git repository
cd ~/.mntn
git init
git remote add origin https://github.com/yourusername/dotfiles.git

# The setup wizard or validate command will create profile.json
mntn validate

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
├── .machine-id             # Machine identifier (may want to .gitignore)
├── configs_registry.json   # Configuration registry
├── package_registry.json   # Package manager registry
└── backup/
    ├── common/             # Shared configs
    ├── machines/           # Machine-specific
    └── environments/       # Environment-specific
```

**Tip:** Add `.machine-id` to `.gitignore` if you want each machine to auto-detect its own name.

#### Setting up a New Machine

```bash
# Install mntn
cargo install mntn

# Clone your configurations
git clone https://github.com/yourusername/dotfiles.git ~/.mntn

# Run setup wizard to configure machine/environment
mntn setup

# Or manually set machine ID
echo "new-laptop" > ~/.mntn/.machine-id

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

**Example output:**
```
Validating configuration...
   Profile: machine=work-laptop, env=work

 Registry Files OK
 Layer Resolution
 ! Some configs are still in legacy location (/Users/alex/.mntn/backup)
 Fix: Run 'mntn migrate --to-common' to migrate to the layered structure
 JSON Configuration Files OK
 Legacy Symlink Check OK

Validation complete: 0 error(s), 1 warning(s)
```

### Package Registry Management

```bash
# List all package managers
mntn registry packages list

# List platform-compatible entries
mntn registry packages list --platform-only

# Add custom package manager
mntn registry packages add pipx \
  --name "pipx Applications" \
  --command "pipx" \
  --args "list" \
  --output-file "pipx.txt"

# Enable/disable entries
mntn registry packages toggle pip --enable
```

### Configuration Registry Management

```bash
# List all entries
mntn registry configs list

# Add new entry
mntn registry configs add my_config \
  --name "My Config" \
  --source "myapp/config.json" \
  --target "/Users/alex/.config/myapp/config.json"

# Enable/disable entries
mntn registry configs toggle my_config --enable
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
- **Machine ID not detected**: Set manually with `echo "my-machine" > ~/.mntn/.machine-id`

### Backup Issues
- **Wrong layer**: Use `--to-common`, `--to-machine`, or `--to-environment` flags
- **Permission denied**: Ensure read access to config directories

### Changes Not Saved
- **Symptom**: Edited config file but changes not in backup
- **Solution**: Run `mntn backup` after editing config files to sync changes

### Restore Issues
- **Wrong version restored**: Check layer priority with `mntn validate`
- **Permission denied**: Ensure write access to target directories

### Migration Issues
- **Files not detected**: Only registry entries are migrated; add entries first with `mntn registry add`
- **Already migrated**: Files in `common/`, `machines/`, or `environments/` are skipped
- **Legacy symlinks**: Run `mntn migrate` to convert symlinks to real files

### Sync Issues
- **No git repository**: Run `mntn sync --init --remote-url <URL>`
- **Merge conflicts**: Resolve in `~/.mntn` using standard git commands

### Validation Issues
- **Legacy files warning**: Run `mntn migrate --to-common` to update structure
- **Legacy symlinks warning**: Run `mntn backup` or `mntn migrate` to convert to real files
- **Layer conflicts**: Intentional overrides show as info, not errors

## License

MIT
