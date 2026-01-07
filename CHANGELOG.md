# Changelog

All notable changes to this project are documented in this file.

## v2.1.0

### Added
- **Encrypted Configuration Registry:** Added secure password-based encryption for sensitive configuration files (SSH keys, credentials) with age encryption, filename encryption support, and seamless integration with backup/restore commands.

## v2.0.0

### Added
- **Interactive Setup Wizard:** Introduced a user-friendly wizard for configuring dotfiles management.
- **Profile Management:** Added support for multiple profiles, including default profile saving and migration tasks for layered backup structures.
- **Validation Command:** New `validate` command to check configuration integrity.
- **Dry-Run Functionality:** Added dry-run flags and execution for biometric sudo, configs registry, package registry, delete, and validate tasks.
- **Parallel Backup:** Enabled parallel operations for backup tasks.
- **Sync Command:** Added a `sync` command with Git repository management options, including initialization and auto-commit features.
- **VS Code Extensions:** Backup and restore functionality for VS Code extensions.
- **Zed Settings:** Added registry entry and path helper for Zed editor settings.
- **Git Configuration:** Added Git configuration entry to ConfigsRegistry.
- **Cross-Platform Trash Cleaning:** Implemented trash cleaning for macOS, Linux, and enhanced Windows support.
- **Startup Program Listing:** Implemented listing of startup programs on Windows.
- **Comprehensive Logging:** Enhanced logging with error, success, and warning messages.
- **CI Workflow:** Added GitHub Actions CI workflow.

### Changed
- **Refactored CLI Arguments:** Improved argument structures for backup, migration, and link tasks.
- **Registry Management:** Refactored to use `ConfigsRegistry` and improved registry structure.
- **Category Handling:** Optimized category filtering, parsing, and display logic.
- **Code Structure:** Major refactor of core modules, including moving and renaming files for better organization (e.g., registry modules, tasks).
- **Documentation:** Expanded and improved README with detailed guides and usage examples.
- **Error Handling:** Improved error handling and logging throughout the codebase.
- **Platform Compatibility:** Enhanced platform-specific code for better cross-OS support.
- **Project Formatting:** Applied consistent formatting and code cleanup.

### Removed
- **Redundant Registry Logic:** Removed redundant abstractions and unused methods.
- **Git Integration in Enhancements Doc:** Removed outdated documentation about git integration and sync command.

### Fixed
- **Windows Support:** Fixed and improved Windows-specific logic and tests.
- **CI and Linting:** Fixed CI workflow permissions and linting errors.
- **Code Cleanups:** Removed unused variables, simplified control flows, and improved consistency in method signatures.
- **Platform-Specific Bugs:** Addressed various platform-specific bugs and improved compatibility.
