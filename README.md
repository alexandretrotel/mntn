# mntn

mntn is built to keep your dotfiles organized, safe, and consistent across machines using profiles.

A profile is a named set of configuration choices that represents a context, like work, personal, or minimal. With profiles, you can keep multiple setups and switch between them so the right settings are active for the situation.

At a high level, mntn helps you manage these configurations, keep them in sync, and recover them when needed.

![Demo Video](./assets/mntn.gif)

## Quick Start

```bash
mntn backup
mntn restore
mntn validate
```

Switch profiles:

```bash
mntn profile create work --description "Work setup"
mntn use work
```

## Core Commands

- `backup` - copy tracked configs into `~/.mntn/backup/`
- `restore` - restore configs from backup
- `validate` - check registry files and config drift
- `profile` - list/create/delete profiles
- `use` - switch active profile
- `git` - run any git command inside `~/.mntn`
- `sync` - run `git add .`, commit with default message `chore: sync mntn (YYYY-MM-DD HH:MM:SS UTC)` (use `--message` to override), then `git push` inside `~/.mntn`

## Directory Layout

```text
~/.mntn/
├── backup/
│   ├── common/
│   │   └── encrypted/          # optional: encrypted bundle + legacy per-file .age
│   └── profiles/
│       └── <name>/
│           └── encrypted/
├── profiles.json
├── .active-profile
├── config.registry.json
├── package.registry.json
└── encrypted.registry.json
```

Registry notes:
- `config.registry.json` tracks regular dotfiles and their targets.
- `package.registry.json` tracks package managers and how to export package lists.
- `encrypted.registry.json` tracks sensitive files that are stored encrypted.

### Encrypted backups

After `mntn backup`, sensitive files are stored under `backup/common/encrypted/` or `backup/profiles/<profile>/encrypted/` as **`mntn-encrypted-bundle.age`**: one age-encrypted tar containing all backed-up entries. Restore and validate use this file when it exists. Older layouts with separate `<path>.age` files next to each logical path are still recognized for restore and validation until you remove them.

Package list exports under `backup/packages/` are generated in parallel when multiple package managers are enabled.

## License

GNU General Public License v3.0 or later (GPL-3.0-or-later), published by the Free Software Foundation.
