# mntn

A Rust-based CLI tool for system maintenance.

## Features

- **Backup**: Saves global package lists (e.g., brew, npm, cargo, bun, uv) and configuration files.
- **Biometric Sudo [macOS]**: Configures Touch ID authentication for sudo commands.
- **Clean**: Removes system junk (caches, logs, etc) and runs package manager cleanup.
- **Delete [macOS]**: Removes applications and their related files with interactive selection.
- **Install**: Sets up automated services for backups, cleaning, and system updates.
- **Link**: Creates symlinks for dotfiles (e.g., .mntn, .zshrc, .vimrc, .config, VSCode settings).
- **Purge**: Deletes unused services with user confirmation.
- **Restore**: Restores configuration files from backups.

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
```

## Guides

### Backup and Restore Guide

#### Creating Backups

The `backup` command saves your system's package lists and configuration files to `~/.mntn/backups/`:

```bash
mntn backup
```

**What gets backed up:**
- **Package lists**: Homebrew packages (`brew.txt`, `brew-cask.txt`), npm global packages (`npm.txt`), Yarn global packages (`yarn.txt`), pnpm global packages (`pnpm.txt`), Bun global packages (`bun.txt`), and Cargo installed packages (`cargo.txt`)
- **Configuration files**: VS Code settings and keybindings, Ghostty terminal config

**Backup location**: `~/.mntn/backups/`

#### Restoring from Backups

To restore your configuration files from a previous backup:

```bash
mntn restore
```

This will restore VS Code settings, keybindings, and Ghostty configuration from your backup.

**Note**: Package restoration must be done manually using the generated package lists. For example:
```bash
# Restore Homebrew packages
brew install $(cat ~/.mntn/backups/brew.txt)

# Restore npm global packages
npm install -g $(cat ~/.mntn/backups/npm.txt | grep -o '^[^@]*' | tr '\n' ' ')
```

### Dotfiles Management with GitHub

#### Setting up a Dotfiles Repository

1. **Create a dotfiles repository on GitHub:**
   ```bash
   mkdir ~/dotfiles
   cd ~/dotfiles
   git init
   git remote add origin https://github.com/yourusername/dotfiles.git
   ```

2. **Organize your dotfiles structure:**
   ```
   ~/dotfiles/
   ├── .zshrc
   ├── .vimrc
   ├── config/          # This becomes ~/.config
   │   ├── nvim/
   │   └── git/
   └── vscode/
       ├── settings.json
       └── keybindings.json
   ```

3. **Add your existing config files:**
   ```bash
   # Copy existing configs to dotfiles repo
   cp ~/.zshrc ~/dotfiles/
   cp ~/.vimrc ~/dotfiles/
   cp -r ~/.config ~/dotfiles/config
   mkdir -p ~/dotfiles/vscode
   cp ~/Library/Application\ Support/Code/User/settings.json ~/dotfiles/vscode/
   cp ~/Library/Application\ Support/Code/User/keybindings.json ~/dotfiles/vscode/
   ```

4. **Commit and push:**
   ```bash
   cd ~/dotfiles
   git add .
   git commit -m "Initial dotfiles setup"
   git push -u origin main
   ```

#### Using mntn with Your Dotfiles

Once your dotfiles repository is set up, use `mntn link` to create symlinks:

```bash
mntn link
```

**What it does:**
- Links `~/dotfiles/.zshrc` → `~/.zshrc`
- Links `~/dotfiles/.vimrc` → `~/.vimrc`
- Links `~/dotfiles/config` → `~/.config`
- Links `~/dotfiles/vscode/settings.json` → `~/Library/Application Support/Code/User/settings.json`
- Links `~/dotfiles/vscode/keybindings.json` → `~/Library/Application Support/Code/User/keybindings.json`

**Safety features:**
- Automatically backs up existing files to `~/.mntn/backups/symlinks/`
- If source doesn't exist but target does, copies target to source first
- Won't overwrite existing correct symlinks

#### Cloning Your Dotfiles on a New Machine

```bash
# Clone your dotfiles
git clone https://github.com/yourusername/dotfiles.git ~/dotfiles

# Install mntn
cargo install mntn

# Create symlinks
mntn link

# Create your first backup
mntn backup
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
- **Linux**: Creates systemd user services and timers
- **Windows**: Creates scheduled tasks

**Scheduled tasks:**
- `mntn-backup`: Runs `mntn backup` every hour
- `mntn-clean`: Runs `mntn clean` daily (with `--with-clean` flag)
- `mntn-topgrade`: Runs `topgrade` daily (if topgrade is installed)

**Task logs:** 
- **macOS**: `/tmp/mntn-*.out` and `/tmp/mntn-*.err`
- **Linux**: Use `journalctl --user -u mntn-*.service`

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
- Cache directories (`~/Library/Caches` on macOS, `~/.cache` on Linux)
- Temporary files
- Application logs and saved states (macOS)
- Quick Look cache reset (macOS)

**System-level cleanup (with `--system`):**
- System caches (`/Library/Caches`, `/var/cache`)
- System logs (`/private/var/log`, `/var/log`)
- Diagnostic reports (macOS)
- Volume trash folders (macOS)

**Package manager cleanup:**
- Homebrew: `brew cleanup`
- npm: `npm cache clean --force`
- pnpm: `pnpm cache delete`

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
- **Linux**: systemd user services and autostart programs
- **Windows**: Windows services and startup programs (planned)

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

## License

MIT
