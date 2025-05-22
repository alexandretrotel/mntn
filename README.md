# mntn

A Rust-based CLI tool for macOS system maintenance.

## Features

- **Backup**: Saves global package lists (e.g., brew, npm, cargo, bun, go).
- **Clean**: Removes system junk (caches, logs, trash).
- **Purge**: Deletes unused launch agents/daemons with user confirmation.
- **Install**: Sets up launch agents for automated backups and cleaning.
- **Link**: Creates symlinks for dotfiles (e.g., .zshrc, .vimrc, .config, lporg).

## Installation

```bash
cargo install mntn
```

## License

MIT
