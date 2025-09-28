# mntn

A Rust-based CLI tool for system maintenance.

## Features

- **Backup**: Saves global package lists (e.g., brew, npm, cargo, bun, uv).
- **Biometric Sudo [macOS]**: Configures Touch ID authentication for sudo commands.
- **Clean**: Removes system junk (caches, logs, etc) and runs brew cleanup.
- **Delete [macOS]**: Removes applications and their related files with interactive selection.
- **Install**: Sets up services for automated backups, cleaning, and system updates (if topgrade is installed).
- **Link**: Creates symlinks for dotfiles (e.g., .mntn, .zshrc, .vimrc, .config, VSCode settings).
- **Purge**: Deletes unused services with user confirmation.
- **Restore**: Restores configuration files (VSCode settings/keybindings, Ghostty config).

## Installation

```bash
cargo install mntn
```

## License

MIT
