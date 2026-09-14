# Changelog

All notable changes to this project are documented in this file.

## [0.3.0] - 2026-09-14

### 🚀 Features

- *(cli)* Add `delete note`/`delete punch` (aliases `del`, `n`, `p`) — list a date's entries with `-d/--date`, delete by number, prints a ready-to-run command to recreate what was deleted

### 🐛 Bug Fixes

- Collapse embedded `\r`/`\n` in note bodies to a single space, keeping every stored note on one line

## [0.2.0] - 2026-09-13

### 🚀 Features

- *(feat)* Enable backdating punches and notes
- *(cli)* Add -d/--date to start/stop/note, including -N shorthand and 'yesterday'
- *(cli)* Accept -N shorthand with status

## [0.1.5] - 2026-09-13

### 💼 Other Changes

- (release) strip debug symbols from release binaries
- (ci) filter Release commits and fix changelog spacing in git-cliff
- (ci) bump actions/checkout to v6 in ci.yml
- (docs) add SECURITY.md

## [0.1.4] - 2026-09-13

### 💼 Other Changes

- (ci) split release script for changelog review
 
## [0.1.3] - 2026-09-13

### 🚀 Features

- (ux) command aliases

### 💼 Other Changes

- (docs) clean impl plans from project's root
- (ci) prepend, don't overwrite the changelog but actually

## [0.1.2] - 2026-09-13

### 🐛 Bug Fixes

- *(ci)* fix github releases

## [0.1.1] - 2026-09-13

### 🐛 Bug Fixes

- *(ci)* Install minisign from upstream binary, not apt
- *(ci)* Pin sha256 of downloaded minisign binary

## [0.1.0] - 2026-09-12

Initial release.

### 🚀 Features

- `start`/`stop`: log a time punch for today, with an optional note in the same call
- `note`: log a work-log entry independent of any punch
- `status`: today's (or a given date's) stints, notes, and day/week totals — including an estimated end-of-day time while a stint is open
- `week`: a week's per-day totals, carried-over balance, and running total against target
- `week target`: override a week's target hours (defaults to 40h/week, with shortfall/surplus carried into the next week)
- All data stored locally in SQLite (via an embedded, migration-managed schema) — no external service, no account
