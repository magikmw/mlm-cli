# AGENTS.md

Notes for AI agents (and future-me) working on this repo.

## What this is

`mlm` — minimal CLI time tracker. Core loop: fire a `start`/`stop` point
for the current date, optionally attach a short note, later review a
log. Everything persists in SQLite in the platform app-data dir.

## Current state

Scaffolding only. Structure and deps are in place; command bodies are
`TODO` stubs. Don't build out full features until design is agreed —
see project memory / conversation history for the latest decisions on:

- schema shape (entries table, single start/stop pairs vs. arbitrary
  punches, how notes attach)
- overnight/cross-midnight handling in `time.rs`
- exact CLI surface (subcommands, flags, output format)
- dashboard layout/widgets (ratatui: `Chart`/`Sparkline`/`BarChart`)
- stretch: shell prompt integration — needs a fast, side-effect-free
  "status" query (e.g. `mlm status --short`) cheap enough for PS1/starship

## Layout

- `src/cli.rs` — clap arg definitions (`Cli`, `Command`)
- `src/db.rs` — SQLite connection + schema, app-data path resolution
  via `directories::ProjectDirs`
- `src/time.rs` — time-of-day parsing and duration math (chrono)
- `src/main.rs` — wires the above together

## Conventions

- Keep platform paths going through `directories`, don't hardcode
  `~/.local/share` or `%APPDATA%` anywhere else.
- Target platforms: Linux and Windows (MSVC). `rusqlite` uses the
  `bundled` feature so SQLite compiles from source — no external
  system dependency needed on either platform.
- Cargo.lock is committed (this is a binary, not a library).

## Verifying changes

```sh
cargo build
cargo run -- start "note"
cargo run -- log
```
