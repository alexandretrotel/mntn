# mntn

A Rust-based CLI tool for macOS system maintenance.

## Features

- **Backup**: Saves global package lists (e.g., brew, npm, cargo, bun, go, uv).
- **Biometric Sudo**: Configures Touch ID authentication for sudo commands.
- **Clean**: Removes system junk (caches, logs, trash) and runs brew cleanup.
- **Delete**: Removes applications and their related files with interactive selection.
- **Install**: Sets up launch agents for automated backups, cleaning, and system updates (if topgrade is installed).
- **Link**: Creates symlinks for dotfiles (e.g., .zshrc, .vimrc, .config, lporg, VSCode settings).
- **Purge**: Deletes unused launch agents/daemons with user confirmation.
- **Restore**: Reinstalls packages from backup files and restores editor configuration files (VSCode settings/keybindings, Ghostty config).

## Installation

```bash
cargo install mntn
```

## License

MIT
