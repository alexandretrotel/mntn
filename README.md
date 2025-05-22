# mntn

A Rust-based CLI tool for macOS system maintenance.

## Features

- **Backup**: Saves global package lists (e.g., brew, npm, cargo, bun, go).
- **Clean**: Removes system junk (caches, logs, trash).
- **Purge**: Deletes unused launch agents/daemons with user confirmation.
- **Install**: Sets up launch agents for automated backups and cleaning.

## Installation

```bash
cargo install mntn
```

## Usage

```bash
mntn [COMMAND]
```

Commands:

- `backup`: Back up global packages.
- `clean`: Clean system junk.
- `purge`: Remove unused launch agents/daemons.
- `install`: Install launch agents and run backup+clean (default).

## License

MIT
