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

## Pre-commit quality gate (CRAP-ish: complexity + coverage)

There's a git hook that blocks commits on two things:

- **Complexity**: any function whose cognitive complexity (via
  `cargo clippy`'s `clippy::cognitive_complexity` lint) exceeds the
  threshold in `clippy.toml` (`cognitive-complexity-threshold`,
  currently **15** — clippy's own default is 25; tune it there, not in
  the hook script).
- **Coverage regression**: overall line coverage (via `cargo llvm-cov`)
  is compared against `coverage-baseline.json`. Equal or improved
  coverage passes and — when it improves — the baseline is ratcheted
  up automatically. A drop fails the commit. The baseline is never
  lowered automatically.

Both checks degrade to a clean pass (not an error) when there's
nothing to measure yet — an empty/near-empty codebase, or missing
optional tooling (`cargo-llvm-cov`, `jq`) just produces a `SKIP` line
for that check rather than blocking the commit.

One-time setup per clone (hooks live in `.githooks/`, not `.git/hooks`,
so this has to be opted into explicitly):

```sh
git config core.hooksPath .githooks
```

Coverage requires `cargo-llvm-cov` and the `llvm-tools-preview`
rustup component:

```sh
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

In a genuine emergency the gate can be skipped with
`git commit --no-verify` — use sparingly.
