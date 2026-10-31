# doc-drift-sweep — review

- Tree: `main` at `341ceaa`, read on disk from the `msrv-fix` checkout (identical apart from the MSRV edit; Rust-version claims in `Cargo.toml` and `README.md:66` skipped as instructed)
- Live docs checked: `README.md`, `AGENTS.md`, `docs/dev/SPEC.md`, `docs/dev/NOTES.md` (Current state block, phase-0 table, and non-log sections), `docs/dev/README.md`, `docs/SIGNING.md`, `CHANGELOG.md`, every `--help` string in `src/cli.rs`
- Archived docs (`docs/dev/PLAN.md`, `docs/dev/specs/`, `docs/dev/plans/` including `plans/reports/`) checked for the banner only
- Gate run: `cargo build` clean, `cargo test` 1018 passed, 0 failed (matches NOTES entry 156); `coverage-baseline.json` 99.5166% (matches entry 156's 99.517%)
- Method note: the read-only hook blocks `cargo run` and running `target/debug/mlm`, and the scratch box cannot see the repo. Binary output was therefore checked by reading the rendering code and the CLI tests (`tests/*.rs` drive the real binary, e.g. the help goldens in `tests/week_target_cli.rs:205-264`), not by running the binary.

## Findings

### F1 — medium, user-facing: `delete punch` example prints a `--date` the code omits

- Claim: `README.md:707-712` lists today's punches (`mlm delete punch`, no `--date`), then `mlm delete punch 2` prints `deleted. to recreate: mlm stop 13:00 --date 2026-09-10`.
- Evidence: `src/commands.rs:244-246` appends ` --date <date>` only when `p.date != today`. A delete from today's listing prints `deleted. to recreate: mlm stop 13:00`.
- The two delete examples also disagree about what today is. `README.md:687-691` prints `--date 2026-09-10` in the note recreate line, so today is not 2026-09-10 there (`src/commands.rs:276-278`). `README.md:712` needs today to be 2026-09-10 for its listing to hold that date's punches, yet still prints `--date`.
- Fix: drop ` --date 2026-09-10` from `README.md:712`, or make the example `mlm delete punch --date 2026-09-10` and `mlm delete punch 2 --date 2026-09-10`.

### F2 — medium, user-facing: the local-dev build example fails

- Claim: `README.md:721-723` shows `cargo run -- start "working on mlm"` as the dev loop.
- Evidence: `TIME` is the first positional (`src/cli.rs:120-123`). `src/commands.rs:390-391` parses it with `parse_time`, which rejects it (`src/time.rs:18-38`). SPEC §3.2 (`docs/dev/SPEC.md:267-271`) says a non-`TIME` first positional is never read as `NOTE`. The command exits 1 with `error: invalid time "working on mlm": expected HH:MM, HHMM, or HH (24-hour)` (`src/time.rs:147-150`).
- Fix: use `cargo run -- start 9:00 "working on mlm"`, the form `AGENTS.md:101` already uses.

### F3 — medium, user-facing: broken `SIGNING.md` link

- Claim: `README.md:74` links `[SIGNING.md](SIGNING.md)`, relative to the repo root.
- Evidence: there is no root `SIGNING.md`. Commit `d26d16d` ("create a dedicated docs folder") moved it to `docs/SIGNING.md`. `SECURITY.md:40` already says `docs/SIGNING.md`. The link 404s on GitHub and on crates.io, where `README.md` is the package readme (`Cargo.toml:9`).
- Fix: link `docs/SIGNING.md`.

### F4 — medium, internal: SPEC.md has no `mlm delete` section

- Claim: `AGENTS.md:16-18` says every command, `delete` included, matches `docs/dev/SPEC.md`. `AGENTS.md:72-79` says SPEC.md must be self-contained.
- Evidence: SPEC's §3 has `start`, `stop`, `note`, `--date`, `status`, `week`, `week target` and `export notes` (`docs/dev/SPEC.md:258-495`), and nothing for `delete note|punch`. The only delete text is §1.2 (`SPEC.md:41-43`) and two sentences in §7.4 (`SPEC.md:1669-1673`). Shipped behavior that SPEC never states:
  - list format `<n>  <kind> <HH:MM>` and `<n>  <body>` (`src/commands.rs:196-218`)
  - `ID` errors `id must be 1 or greater (got 0); N entries for this date` and `id N is out of range; only N entries for this date` (`src/commands.rs:184-193`)
  - the recreate echo, including `stop` for an `end` punch, `--date` only for another day, single-quote shell quoting, and the backslash fallback `deleted note (<date>): <body>...` (`src/commands.rs:157-281`)
  - the stale-id rule (only in `README.md:694-700`)
- That detail lives only in the archived `docs/dev/specs/2026-09-13-delete-punches-notes.md` (§3-§5). Code comments cite its numbering as "spec §4/§5" (`src/commands.rs:181`, `:195`, `:229`, `:264`, `:292`), which collides with SPEC.md's own §4/§5. `docs/dev/NOTES.md:146-149` (decision 5) points readers into that archived spec.
- Fix: fold the delete surface into SPEC (§3.x for the command, §6.1 for the `ID` errors, §7.4 for list and echo formats), then repoint NOTES decision 5 at SPEC.

### F5 — medium, internal: SPEC never says `TIME` is required when backdating

- Claim: SPEC §3.2 (`docs/dev/SPEC.md:263`) says "`TIME` optional (§3.1). Defaults to now." §3.4a (`SPEC.md:286-298`) adds no exception.
- Evidence: `src/commands.rs:390-394` returns `TimeParseError::Required` when `--date` names another day, which prints `TIME is required when --date targets a day other than today` (`src/time.rs:155-158`). `README.md:157-158` and the `TIME` help (`src/cli.rs:120-121`) both state the rule. Only the archived `docs/dev/specs/2026-09-13-backdated-punches.md:95-102` specs it. §6.1 doesn't list the error either.
- Fix: add the rule to §3.2/§3.4a and the message to §6.1.

### F6 — medium, internal: 97 archived files have no archive banner

- Claim: `docs/dev/README.md:8-12` says each archived file under `PLAN.md`, `plans/` and `specs/` carries an `Archived` banner.
- Evidence: a scan of the first five lines of each file found no `Archived` banner in:
  - specs (6): `2026-09-20-boundary-context-cues.md`, `2026-09-23-status-wording-fixes.md`, `2026-09-24-current-week-ahead-wording.md`, `2026-09-26-open-punch-visibility.md`, `2026-10-03-live-eod.md`, `2026-10-03-point-in-time-week.md`
  - plans (15): `boundary-context-cues-plan.md`, `boundary-context-cues-task-1-status-render.md`, `current-week-ahead-wording-plan.md`, `live-eod-plan.md`, `open-punch-visibility-plan.md`, `open-punch-visibility-task-1..6-*.md` (6 files), `point-in-time-week-plan.md`, `status-wording-fixes-plan.md`, `status-wording-fixes-task-1-status-render.md`, `status-wording-fixes-task-2-docs.md`
  - plans/reports (76 of 114): every report for boundary-context-cues, current-week-ahead-wording, export-notes, live-eod, open-punch-visibility, orphan-stop-warning, overnight-visibility, panic-fixes, point-in-time-week, possible-issues-triage, relative-ids, status-wording-fixes and week-notes, plus `unwrap-audit-review.md`. The 38 bannered reports are the older ones, which use the form `> **Archived — historical review/report record.** ...`.
- The six unbannered changesets (boundary-context-cues through point-in-time-week) lost the banner on their spec and plans too. From overnight-visibility on, the spec and plans carry it but the reports do not, although the NOTES log says "Spec, plan and task plans archived" for those changesets (e.g. `NOTES.md:2063`, `:2147`), which implies reports were never part of the close-out step.
- Fix: banner all 97, and add reports to the close-out checklist. Another option is to change `docs/dev/README.md` so `plans/reports/` is exempt, if that is the intent.

### F7 — low, internal: the banner rule names the wrong target

- Claim: `docs/dev/README.md:11-12` says the banner points "back here" (`docs/dev/README.md`).
- Evidence: every banner found points at `docs/dev/SPEC.md`, e.g. `docs/dev/specs/2026-10-25-relative-ids.md:3`, `docs/dev/plans/reports/boundary-stint-pairing-review-round2.md:3`.
- Fix: change the README sentence to "pointing to `SPEC.md`".

### F8 — low, user-facing (`--help`): the `delete` ID help leaks internal wording

- Claim: `src/cli.rs:205-208`, rendered as the `[ID]` help of `mlm delete note|punch`: "1-based position from the most recent listing for this date ... Position 0 and anything past the current count are rejected once the app resolves this against a fresh listing, not here."
- Evidence: "not here" refers to the clap layer, which a user cannot see. "From the most recent listing" also reads as the listing the user last saw. `README.md:694-697` says the opposite: the ID resolves against a fresh listing taken when `delete` runs, "not whatever listing you last looked at" (`src/commands.rs:310-311`).
- Fix: "Position in this date's list, numbered from 1, taken fresh when this runs (run with no ID to see it)."

### F9 — low, user-facing (`--help`): glossary defines an orphaned end too narrowly

- Claim: the glossary at `src/cli.rs:42-44` says "An end punch with no start before it."
- Evidence: under LIFO pairing an end is orphaned when no open start remains to pair with (`docs/dev/SPEC.md:664-668`). `start 09:00`, `stop 10:00`, `stop 11:00` orphans the 11:00 end, and a start does come before it. `README.md:209-210` gets this right ("an end with no start to pair with").
- Fix: "An end punch with no open start to pair with."

### F10 — low, internal: AGENTS.md lists fewer target platforms than README

- Claim: `AGENTS.md:93` says "Target platforms: Linux and Windows (MSVC)."
- Evidence: `README.md:27-28` and `:69-72` list Linux x86_64 and ARM64, Windows MSVC, and macOS Intel and Apple Silicon. `dist-workspace.toml` `targets` lists all five.
- Fix: list the five targets in AGENTS.md.

### F11 — low, user-facing: a bare `WW` defaults to the ISO year, not the calendar year

- Claim: `README.md:427-428` says "a bare week number for the current year". `README.md:645` says "a bare `43` for the current year".
- Evidence: `src/date.rs:418` uses `today.iso_week().year()`. SPEC §3.8 (`SPEC.md:508-509`) says "current ISO year". The two differ between 29 December and 3 January: on 2025-12-30, `mlm week 5` is 2026-05.
- Fix: say "the current ISO year" in both README lines.

### F12 — low, user-facing: README implies the lookback line has the date-page shape

- Claim: `README.md:297-300` says today's `status` "appends one such line", meaning the line shown at `README.md:274`: `[!] unclosed start at 09:00, 5 days old (no stop recorded)`.
- Evidence: a lookback line has a different shape, `[!] unclosed start on Mon 2026-09-21 at 09:00 (5 days ago)` (`src/lookback.rs:158-160`, SPEC §7.3 `SPEC.md:1599`). README never shows that form.
- Fix: show the lookback form under that paragraph.

### F13 — low, user-facing and internal: two examples of week 2026-43 disagree

- Claim: the export example of week 2026-43 includes `2026-10-24  refactored the parser` (`README.md:656-661`, SPEC §7.5 `SPEC.md:1770-1775`).
- Evidence: the `mlm week --notes` example of the same week shows no note under `Sat 2026-10-24` (`README.md:516-538`, SPEC §7.2 `SPEC.md:1447-1469`). Both examples are illustrative, but a reader comparing them sees one note vanish.
- Fix: add the note under Sat's row in the `--notes` example, or drop it from the export example.

### F14 — low, user-facing: the alias claim misses `week target`

- Claim: `README.md:146` says every command has a 1-character alias except `delete` (`del`) and `export` (`ex`).
- Evidence: `week target` has no alias (`src/cli.rs:253-256`).
- Fix: say "every top-level command", or add the exception.

### F15 — low, internal: stale document status headers

- `docs/dev/SPEC.md:3-8` reads "Sections land one at a time for review — this doc grows over several passes" and "Status: draft, complete through section 8". SPEC now describes shipped behavior through v0.4.0 plus five unreleased changesets.
- `docs/dev/NOTES.md:3-4` reads "Running log of background/story before formal spec exists ... Formal spec comes later as its own doc." SPEC.md exists.
- Fix: one line each, saying what the doc is now.

### F16 — low, internal: SPEC data-model wording behind the code

- `docs/dev/SPEC.md:161` reads "Index on `date` (and probably `at_utc` for ordering within a date)." The schema has composite indexes `punches(date, at_utc, id)` and `notes(date, created_at_utc, id)` (`src/db.rs:51-62`). SPEC never mentions the notes index.
- `docs/dev/SPEC.md:877-879` reads "see `src/db.rs` in the scaffold". The scaffold is long gone (`src/db.rs:37-41` drops its leftover `entries` table).
- `docs/dev/SPEC.md:140` reads "A `schema_migrations` table (or equivalent)". `rusqlite_migration` tracks the version in SQLite's `user_version` (`src/db.rs:37-39`, `:174-179`).

### F17 — low, internal: the SPEC §6.1 read-failure list misses two `status` reads

- Claim: `docs/dev/SPEC.md:880-884` lists the reads whose failure is a hard error: the earliest data week, target overrides, carry-walk punches, `--notes` notes, and the export note read.
- Evidence: `status` also fails hard on its own date's note read (`src/status.rs:473`) and, for today, on the lookback punch read (`src/status.rs:487-488`, `src/lookback.rs:229`).
- Fix: add both reads to the list.

### F18 — low, internal: SPEC miscounts the glossary entries

- Claim: `docs/dev/SPEC.md:332` says "Three entries precede the `[!], [i]` one."
- Evidence: twelve entries precede it (`src/cli.rs:12-56`). The sentence means the three directly before it.
- Fix: say "The three entries just before the `[!], [i]` one are ...".

### F19 — low, internal: AGENTS Layout descriptions out of date

- `AGENTS.md:38-42` describes `storage.rs` as insert/read plus two ranged reads. It omits `delete_punch`/`delete_note` (`src/storage.rs:443`, `:528`) and the per-date reads `punches_for_date`/`notes_for_date` (`:393`, `:477`) that `delete` and `status` use.
- `AGENTS.md:68` says `main.rs` handles "exit code". The exit-code mapping, including 141, is `mlm::exit_code` in `src/lib.rs:31-38`, along with `OutputError` and `emit`. The Layout entry for `lib.rs` (`AGENTS.md:27-30`) doesn't mention them.

### F20 — low, internal: stale code comments (outside the live-doc list, reported since the sweep touched them)

- `src/time.rs:5-11` cites "PLAN.md interface contract 11" and says "nothing in `main` calls them yet" above `#![allow(dead_code)]`. `main` calls them now, so the comment is false and the allow may hide real dead code.
- `src/main.rs:12` cites "PLAN.md interface contract 7", and `src/cli.rs:228` cites "PLAN.md's canonical shape note". Both point into the archived PLAN.md.
- `src/main.rs:25` cites "SPEC §4.1" for the default log level. That rule is in SPEC §3 (`SPEC.md:243-246`).
- `src/date.rs:433-434` says `Month` has "the same four-digit domain `parse_date` accepts". `parse_date` accepts years 1000-9999 (`src/date.rs:177`), and `Month` accepts 0-9999 (`src/date.rs:444`).
- `src/commands.rs` "spec §4/§5" comments (see F4).

### F21 — low, code/doc gap: misleading `WEEK_ID` message for years below 1000

- Evidence: `WeekId::new` maps a year outside 1000-9999 to `Cause::OutOfRange` (`src/date.rs:272-274`). That cause renders as `week number must be 1 or greater` (`src/date.rs:97`). So `mlm week 0999-05` reports a bad week number, though the year is what's wrong. SPEC §6.1 (`SPEC.md:805-829`) names 1000-01 as the floor but gives no message for this case.
- This is the user's call (it may be a non-issue under the triage rules). Report only.

### Checked, no drift found

- AGENTS Layout lists every `pub mod` in `src/lib.rs:8-24`. The history, anomalies, carry_scan, lookback, render, export and stop-warning descriptions match the source (`src/history.rs:83-93`, `src/commands.rs:119-130`, `src/week_view.rs:310-316`, `src/export.rs:48-58`).
- AGENTS "Current state" command list, the seed tool flags (`examples/seed_test_data.rs:95-104`, `:200-203`, `:257-259`), the pre-commit description (`.githooks/pre-commit`), and `clippy.toml` threshold 15.
- NOTES Current state block: the `main` pointer `3e9213f` is the merge commit that `341ceaa` (which edits only that line) follows, five unreleased changesets, next entry 159. Phase-0 table paths (`scripts/prep_release.sh`, `scripts/release.sh`) exist.
- Every `--help` string SPEC quotes (§3.6, §3.7, §3.8) matches `src/cli.rs` and the goldens in `tests/week_target_cli.rs`, `tests/week_notes_cli.rs:317-325` and `tests/export_notes_cli.rs:572`.
- Every error message SPEC §6.1 and E29-E37 quote matches `src/date.rs:73-107`, `src/week_target.rs:21-35` and `src/status.rs:1577`.
- Anomaly, lookback, warning, footer and carry-marker strings in README and SPEC §7.3/§7.4 match `src/anomalies.rs`, `src/lookback.rs`, `src/render.rs` and `src/commands.rs:133-145`.
- Arithmetic in every README/SPEC worked example sums correctly (row totals, fulfillment, gap, est. EOD 21:25, SPEC §5 table), and every weekday/date pair is a real calendar match.
- Panic-fixes claims (minute-floor `now` at `src/main.rs:39-54`, 168h cap at `src/week_target.rs:19`/`:97`, exit 141 and `cannot write output` at `src/lib.rs:28-71`) match. The §1.2a/README possible issue about a stored target above 168h holds, because `src/status.rs:370` still adds `ChronoDuration::minutes(base_gap)` unchecked.
- `docs/SIGNING.md`: the pubkey matches `Cargo.toml:50`, and the `sign-artifacts.yml` workflow exists.
- `CHANGELOG.md`: top entry 0.4.0 matches `Cargo.toml:3`. Unreleased changesets are absent by design, because `scripts/prep_release.sh:88` generates them with git-cliff at release.
- No live doc or `--help` string sends a reader into `docs/dev/specs/` or `docs/dev/plans/` for behavior detail. The one exception, `NOTES.md:148`, sits in the numbered decision list and is covered under F4.

## Verdict

ship-with-followups

No live doc claim blocks anything, but three README errors (F1-F3) are wrong in front of users and cheap to fix. F4 and F5 mean SPEC.md is not self-contained for `delete` and for backdated `TIME`, which breaks the AGENTS.md rule. F6 leaves most review reports, plus six whole changesets, without the banner `docs/dev/README.md` promises.
