# AGENTS.md

Notes for AI agents (and future-me) working on this repo.

## What this is

`mlm` — minimal CLI time tracker. Core loop: fire a `start`/`stop`
punch for the current date, optionally attach a short note, later
review a day's or week's totals via `status`/`week`. Everything
persists in SQLite in the platform app-data dir. See `README.md` for
the full command reference and `docs/dev/SPEC.md` for the detailed behavior
spec.

## Current state

All commands (`start`, `stop`, `note`, `delete`, `status`, `week`,
`week target`) are implemented, tested, and match `docs/dev/SPEC.md`.
Remaining stretch ideas (not implemented):

- dashboard layout/widgets (ratatui: `Chart`/`Sparkline`/`BarChart`)
- shell prompt integration — needs a fast, side-effect-free "status"
  query (e.g. `mlm status --short`) cheap enough for PS1/starship

## Layout

`mlm` is both a library (`src/lib.rs`) and a thin binary (`src/main.rs`)
in the same package, specifically so `examples/`/`tests/` can reuse the
real logic instead of re-deriving it — new top-level modules go in
`src/lib.rs`'s `pub mod` list.

- `src/cli.rs` — clap arg definitions (`Cli`, `Command`, per-command args)
- `src/db.rs` — SQLite connection + schema/migrations, app-data path
  resolution via `directories::ProjectDirs` (+ `MLM_DB_PATH` override)
- `src/time.rs` — TIME/DURATION parsing and the one duration formatter
- `src/date.rs` — DATE/WEEK_ID parsing, `WeekId`
- `src/storage.rs` — `Punch`/`PunchKind`/`Note`, punch/note insert+read
- `src/stint.rs` — LIFO stint-pairing algorithm (§4.3)
- `src/week.rs` — target/carry/fulfillment accounting (§2.4/§5)
- `src/render.rs` — shared rendering helpers used by both `status`/`week`
- `src/commands.rs`, `src/status.rs`, `src/week_target.rs`,
  `src/week_view.rs` — per-command wiring over the above
- `src/main.rs` — CLI entry point: parses args, dispatches, exit code
- `examples/seed_test_data.rs` — dev-only test-data generator (below)

## Documentation

`docs/dev/SPEC.md` and `docs/dev/NOTES.md` are the only live docs
under `docs/dev/` (`docs/SIGNING.md` is separately live, for release
signing) — SPEC.md/NOTES.md must be self-contained. Never send a
reader (user or agent) from `--help` text, `README.md`, or
`SPEC.md`/`NOTES.md` themselves out to `docs/dev/plans/`,
`docs/dev/specs/`, or any other in-dev working doc **for behavior
detail**. When a changeset's working spec/plan/report settles
something real, fold the actual content into `SPEC.md` (or `NOTES.md`
for process/background) directly, then archive the working doc
(banner it as historical — see `docs/dev/README.md` for the archive
convention itself; that meta-reference is process, not behavior
detail, so it's not what this rule forbids).

## Conventions

- Keep platform paths going through `directories`, don't hardcode
  `~/.local/share` or `%APPDATA%` anywhere else.
- Target platforms: Linux and Windows (MSVC). `rusqlite` uses the
  `bundled` feature so SQLite compiles from source — no external
  system dependency needed on either platform.
- Cargo.lock is committed (this ships a binary).

## Verifying changes

```sh
cargo fmt
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
MLM_DB_PATH=/tmp/mlm-check.db cargo run -- start "9:00" "note"
MLM_DB_PATH=/tmp/mlm-check.db cargo run -- status
```

## Test-data seeding (dev tool, not shipped)

`examples/seed_test_data.rs` populates a database with a few weeks of
randomized-but-plausible punches/notes ending today, for manually
poking at `status`/`week` without hand-typing entries. Writes through
the real `mlm::storage`/`mlm::db` insert functions (so it's exactly
what real punches would look like), just backdated — something the
real CLI deliberately never allows (§1.2). Not part of the shipped
command surface.

```sh
cargo run --example seed_test_data -- --db /tmp/mlm-seed.db --seed 42
MLM_DB_PATH=/tmp/mlm-seed.db cargo run -- status
```

Refuses to run without an explicit `--db PATH` or `MLM_DB_PATH` set —
never guesses/falls back to the real app-data path. `--weeks N`
(default 3) and `--seed N` (for a reproducible run) are also
available. Occasionally injects a deliberate anomaly (an orphaned
`end`) so `[!]` rendering has something to show too — printed to
stdout when it happens, along with a `try:` block naming the exact
commands to inspect the result.

Since it's a `crate::` package with both a `lib.rs` and `main.rs`
(added specifically so this tool and `examples`/`tests` can reuse the
real logic modules instead of re-deriving them), any new top-level
module goes in `src/lib.rs`'s `pub mod` list, not `src/main.rs`'s —
`main.rs` is just the thin CLI entry point now.

## Pre-commit quality gate (CRAP-ish: format + complexity + coverage)

There's a git hook that blocks commits on three things:

- **Formatting**: `cargo fmt --check` must be clean — run `cargo fmt`
  and re-stage if it fails. No threshold to tune.
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

All three checks degrade to a clean pass (not an error) when there's
nothing to measure yet — an empty/near-empty codebase, or missing
optional tooling (`rustfmt`, `cargo-llvm-cov`, `jq`) just produces a
`SKIP` line for that check rather than blocking the commit.

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
