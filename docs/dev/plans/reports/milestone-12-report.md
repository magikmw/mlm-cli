# Milestone 12 — Documentation: completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


## Scope

Documented the real, final CLI behavior (`start`, `stop`, `note`, `status`,
`week`, `week target`) — no new behavior implemented. Per PLAN.md's
Milestone 12 acceptance criteria.

## What changed

### `README.md`

- Removed the stale "Status" section ("Scaffolding stage... commands are
  stubs") — false as of this milestone; all six commands are implemented.
- Replaced the "Usage (planned)" section (three placeholder lines,
  including a `log` command that never existed) with a full "Usage"
  section: a short usage-model paragraph (daily start/stop/note loop,
  checking `status`, adjusting a week's target with `week target`), then
  one subsection per real command with its exact syntax (pulled from
  `src/cli.rs`'s actual arg shapes) and at least one real, captured example
  each.
- Noted explicitly that `start`/`stop`/`note`/`week target` are silent on
  success (verified by running them) — worth calling out since a first-time
  reader would otherwise expect some confirmation output.
- **Data location fix**: the Windows path was wrong — README said
  `%APPDATA%\mlm\mlm.db`, but `src/db.rs`'s own doc comment (and the
  `directories` crate's actual Windows behavior of nesting data under a
  `data` subfolder) says `%APPDATA%\mlm\data\mlm.db`. Fixed to match
  `src/db.rs`. Also added a one-line mention of the `MLM_DB_PATH` override,
  which was previously undocumented in the README (only in code comments).
- "Build" section: left the `cargo build` / `cargo run` shape alone, just
  swapped the leftover scaffold-era note text (`"working on scaffold"`) for
  something that isn't anachronistic.

### `AGENTS.md`

- "Verifying changes" block: replaced `cargo run -- log` (a command that
  has never existed post-scaffold) with a real, verified sequence:
  `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  then a real `MLM_DB_PATH`-scoped `start` + `status` round trip. Ran this
  exact sequence for real before writing it down.
- "Current state" section: was still scaffold-era ("command bodies are
  `TODO` stubs", listing already-resolved design questions like "entries
  table vs. arbitrary punches"). Updated to state plainly that all six
  commands are implemented/tested/spec-conformant, and trimmed the
  "remaining decisions" list down to the two stretch items that are
  actually still open (dashboard, shell-prompt integration).
- "What this is" now points at `README.md` for the command reference and
  `SPEC.md` for detailed behavior, instead of duplicating a shorter,
  slightly-off description ("review a log" — there's no `log` command).

### `src/cli.rs` doc-comment audit

Read every `#[command]`/`#[arg]` doc comment across all six commands.
Findings:

- Terminology was already consistent across commands added in different
  milestones: `TIME`'s accepted forms ("Time of day (HH:MM, HHMM or HH,
  24h). Defaults to now.") are defined once on the shared `PunchArgs`
  struct and used verbatim by both `start` and `stop`, so there was no
  drift to fix there. `WEEK_ID`'s description, "work-log note" phrasing,
  etc. were likewise consistent. No leftover `log`-command wording
  anywhere in `cli.rs` (confirmed via `mlm --help` / every `mlm <cmd>
  --help`, all read clean).
- One minimal wording edit made: `Cli::verbose`'s doc comment was
  `"Enable debug-level logging (PLAN.md interface contract 12)."` — an
  internal planning-doc reference leaking into user-facing `--help` text
  (every subcommand's `--help` shows this line, since it's a global flag).
  Trimmed to `"Enable debug-level logging."`. This is a doc-comment-only
  change; the flag's parsing/behavior is untouched.
- Ran `mlm --help` and `mlm <command> --help` for all six commands after
  the edit to confirm the reference reads as one coherent document.

## README examples: real captured output, not hand-transcribed

Built the binary (`cargo build`) and ran it against a scratch
`MLM_DB_PATH` (a `mktemp -d` temp file, deleted afterward). Scenario:

- Today's punches/note inserted via the real CLI: `start 09:00 "reviewed
  open PRs"`, `stop 13:00`, `start 14:05`, `stop 17:30`, `start 17:45`
  (left open/ongoing), `note "fixed migration runner bug"`.
- Prior weekdays of the same ISO week (Mon–Fri) seeded directly into the
  `punches` table via `sqlite3` (matching the real schema) so `week` had a
  realistic full week to render — `status`/`week` output itself is 100%
  real binary output, computed from that data by the actual code, not
  copied from SPEC.md.
- Captured real output for: `status` (today, with ongoing stint and
  daily-target/est.-EOD lines), `status <past date>` (no daily-target
  lines, week line still framed against "today" since that week was still
  ongoing), `week` (current week, ahead of target), and `week target`
  (confirmed silent, then `week` again showing the new target applied).
- Every README code block under "Usage" is one of these real captures,
  verbatim. Scratch directory removed after each session (`rm -rf`).

## Worth flagging for the reviewer

- **README Windows data path was wrong before this milestone** — it said
  `%APPDATA%\mlm\mlm.db`, missing the `\data\` segment that `src/db.rs`'s
  own doc comment (and the `directories` crate's real Windows layout)
  uses. Not a SPEC.md issue (SPEC.md doesn't document data-dir paths at
  all), but a real README bug predating Milestone 12, fixed here since the
  acceptance criteria only said to leave Data location alone "if already
  accurate" — it wasn't.
- **SPEC.md §7's rendered examples are illustrative, confirmed not
  byte-identical to real output** — e.g. §7.1's sample header is `Thu
  2026-02-12` with `Week 2026-07`, and its `status`/`week` numbers are a
  hand-authored scenario. The real binary's output format (column
  alignment, wording, ordering) matches SPEC.md's structure exactly, but
  the actual numbers/dates in this report's captures differ because they
  come from a different (real, run-for-real) scenario, not SPEC's
  fictional one. No formatting discrepancies were found between SPEC.md's
  described layout rules (§7) and the real output — just the expected
  fact that specific sample values differ.
- `week target` (and `start`/`stop`/`note`) print nothing on success —
  confirmed by capturing stdout directly. This isn't stated explicitly in
  SPEC.md §3.7/§7, so it's now spelled out in the README's usage-model
  paragraph to avoid users expecting confirmation output.
- `src/week.rs` still has a stale `TODO(integration): drop this once
  Milestones 9/10/11 consume the module` comment, left untouched — out of
  this milestone's scope (`cli.rs` doc comments only), but likely dead
  now that those milestones are merged; flagging for a future cleanup
  pass.

## Verification

- `cargo build` — clean.
- `cargo test` — 307 + 1 + 2 + 4 tests passed, 0 failed (full suite,
  including integration test binaries), after the `cli.rs` doc-comment
  edit.
- `cargo clippy --all-targets -- -D warnings` — clean, no warnings.
