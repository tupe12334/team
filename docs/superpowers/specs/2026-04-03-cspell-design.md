---
title: Add cspell spell checking with Husky pre-commit hook
date: 2026-04-03
status: approved
centy_issue: "#2"
---

# cspell + Husky Pre-commit Setup

## Goal

Add spell checking to the repo so typos in Rust source, Markdown, TOML, and JSON files are caught automatically before each commit.

## Stack

- **cspell** — spell checker
- **Husky** — git hooks manager
- **lint-staged** — runs checks only on staged files (fast commits)
- **pnpm** — package manager, version-pinned via Corepack

## Files to Add / Modify

### `package.json` (new)

```json
{
  "packageManager": "pnpm@10.33.0",
  "private": true,
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "cspell": "latest",
    "husky": "latest",
    "lint-staged": "latest"
  },
  "lint-staged": {
    "*.{rs,md,toml,json}": "cspell lint --no-progress"
  }
}
```

### `cspell.json` (new, at repo root)

```json
{
  "version": "0.2",
  "language": "en",
  "files": ["**/*.{rs,md,toml,json}"],
  "ignorePaths": [
    ".centy/",
    "target/",
    "node_modules/"
  ],
  "words": [
    "allowedStates",
    "centy",
    "centyVersion",
    "createdAt",
    "displayNumber",
    "priorityColors",
    "priorityLevels",
    "schemaVersion",
    "stateColors",
    "updatedAt"
  ]
}
```

Word list seeded from `.centy/cspell.json`. Additional project-specific words (Rust crate names, proto identifiers, etc.) will be added here as needed.

### `.husky/pre-commit` (new)

```sh
pnpm lint-staged
```

### `.gitignore` (modify)

Add `node_modules/` entry.

## Contributor Workflow

First-time setup:

```bash
corepack enable   # once per machine
pnpm install      # installs husky, cspell, lint-staged; runs prepare → husky
```

After that, every `git commit` automatically spell-checks staged files.

## Out of Scope

- Checking files in CI (can be added later as a separate issue)
- Enforcing pnpm-only via `preinstall` script (not needed yet)
