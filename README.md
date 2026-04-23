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
- `secret` - store the encryption passphrase in the OS keychain (`secret set`) so `backup` / `restore` / `validate` can reuse it without prompting
- `profile` - list/create/delete profiles
- `use` - switch active profile
- `git` - run any git command inside `~/.mntn`
- `sync` - run `git add .`, commit with default message `chore: sync mntn (YYYY-MM-DD HH:MM:SS UTC)` (use `--message` to override), then `git push` inside `~/.mntn`

Encrypted configs: run `mntn secret set` after you know your passphrase to persist it. Use `--ask-password` on `backup`, `restore`, or `validate` if you want to type it for that run only.

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

## License

GNU General Public License v3.0 or later (GPL-3.0-or-later), published by the Free Software Foundation.
