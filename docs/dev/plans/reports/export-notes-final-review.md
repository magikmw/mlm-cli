# export-notes — final review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Baseline**: `main` at `b25bef7`
- **Branch**: `export-notes` at `11e472d` (clone `/tmp/cd-scratch/export-notes-final`, HEAD verified)
- **Spec**: `docs/dev/specs/2026-10-24-export-notes.md` §3-§8; SPEC.md §1.2, §3.8, §6.1, §6.2, §7.5, §8 (F15, E28-E31); NOTES.md 125, 133-136
- **Reviewed**: `src/export.rs`, `src/date.rs`, `src/cli.rs`, `src/main.rs`, `src/lib.rs`, `tests/export_notes_cli.rs`, README.md, AGENTS.md, SPEC.md, NOTES.md diffs
- **Ship verdict**: ship-with-followups

## 1. Verification set

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` (box local TZ, Europe/Warsaw) | 6 failures, all outside export (see F1) |
| `TZ=UTC cargo test --no-fail-fast` | 1 failure (`dst_transition_is_shown_correctly_end_to_end_via_status`, which forces its own Warsaw TZ; F1) |
| `TZ=America/New_York cargo test --no-fail-fast` | same single F1 failure, all else green |

The review ran on 2026-10-25 at 02:17 CEST. That falls inside the hour that
repeats when Europe falls back from CEST to CET. Every failure is the same
panic at `src/main.rs:39` and none comes from this changeset (F1). All
export tests, unit and binary, pass.

## 2. Manual runs (TZ=UTC, `MLM_DB_PATH` inside the clone)

The notes were seeded through the real `mlm note -d`, so the runs follow
F15 and the spec's §5 example.

- `mlm export notes --week 2026-43` printed the four §5 lines in order, plus today's (2026-10-25) note. Same for `mlm ex n` (default week) and `mlm ex n -w 43`.
- `--date 2026-10-19` printed the two 10-19 lines in entry order. `-d -1` printed 10-24 and `-d -6` printed 10-19.
- `--month 2026-10` included 10-18 and excluded 09-30. `-m 10` printed the same 6 lines.
- ISO week 1 across a year boundary: `-w 2026-01` covered 2025-12-29 to 2026-01-04, and 2025-12-28 and 2026-01-05 were excluded.
- ISO week 53: `-w 2020-53` covered 2020-12-28 to 2021-01-03, and the neighbours were excluded. `-w 2026-53` was accepted. `-w 2027-53` gave `2027 has only 52 ISO weeks`.
- Leap February: `-m 2024-2` reached 02-29 and excluded 03-01. `-m 2023-02` ended at 02-28.
- December: `-m 2020-12` included 12-27 and 12-28. Unit tests cover 2025-12 and 9999-12.
- Future ranges (`-d 9999-12-31`, `-m 9999-12`) printed nothing and exited 0. Year 0000 was accepted, as spec'd.
- Value errors: `-m 0`, `13`, `2026-13`, `abc`, `-1`, `-d`, `2026-1-1`, `99999-1`, `-w -1`, `-w 2026-54`, `-d 2026-02-30` and `-d -0` each printed the exact spec message, wrote nothing to stdout and exited 1.
- Usage errors: bare `export` (help listing `notes`), `-d` with no value, `-d … -w …`, `-m 3 -m 4` and a stray positional each exited 2.
- `--help`, `export --help` and `export notes --help` match the spec strings byte for byte.
- A non-ASCII body passed through unchanged.
- `mlm note` folds an embedded newline or CR into a space when it stores the note, so a new note cannot break the one-line layout. Pre-v0.3.0 rows can still hold newlines, and NOTES 133 F5 accepts that.
- Running against a missing DB file creates and migrates it, prints nothing and exits 0. `status` behaves the same way.

## 3. Mutation tests

Each mutant was applied with `sed`/`perl`, run with `TZ=UTC cargo test --no-fail-fast`, then reverted. The unmutated baseline has 1 failure (F1).

| Mutant | Failures |
|---|---|
| `last_day` drops `pred_opt` (off by one) | 7 |
| December wraps to the wrong year/month | 3 |
| one space instead of two in the line | 13 |
| bare month takes the ISO year | 3 |
| a year of any digit count is accepted | 3 |
| `-m` loses `allow_hyphen_values` | 4 |
| default range becomes today only | 3 |
| `#[group(multiple = false)]` removed | 3 |
| `--month` ignored | 10 |
| Month year domain narrowed to 1000+ | 3 |

Every mutant was caught. The assertions are exact-string equality on stdout and stderr together with the exit code, so they can fail. The same-date order test seeds a later `id` with an earlier `created_at_utc`, which proves the ordering instead of passing by accident.

## 4. Security

- `notes_in_range` binds both dates as parameters, so no SQL is built from input.
- The command builds no shell string. It does not echo a recreate command the way `delete` does.
- Bodies print raw, so an ESC byte stored in a note reaches the terminal. `status` and `week --notes` already do this, and `mlm note` is the only way to write a note, so this is not new exposure. Not a finding.

## 5. Code quality

- `src/export.rs` is small and fits the surrounding command modules. It has one resolver, one renderer and one run function.
- `Month` follows `WeekId`, with a validated constructor and `span()`.
- `month_parts` is tight and rejects every shape the spec lists.
- The `ArgKind::Month` arms in `Display` come before the generic `(_, OutOfRange)` week arm, so match order is correct.
- The docs fold matches the code. README, SPEC §1.2/§3.8/§6.1/§6.2/§7.4/§7.5/§8 and AGENTS.md are all updated, and none of them points readers at `plans/` or `specs/`.
- The export-notes spec and plans have no `Archived` banner yet. That is the phase-close step, not a finding.

## 6. Findings

### F1 — medium, pre-existing (not introduced here): every command panics in the repeated DST hour

- **Location**: `src/main.rs:37-41`
- **Cause**: `Local::now().with_second(0).unwrap().with_nanosecond(0).unwrap()` panics for one hour each year. chrono's `with_*` on `DateTime<Local>` re-resolves the local time and returns `None` when that time is ambiguous, which is what happens in the hour that repeats at fall-back.
- **Effect**: Every command exits 101 with a panic trace, `mlm export notes` included. This was reproduced live at 02:17 CEST on 2026-10-25 with TZ Europe/Warsaw and Europe/London. America/New_York and Asia/Tokyo ran fine. The same bug makes the verification set fail in that hour: `ascii_sweep` and 5 `status_cli` tests failed under local TZ, and `dst_transition_is_shown_correctly_end_to_end_via_status` failed under any TZ because it forces Warsaw.
- **Fix**: Truncate without re-resolving local time. One way is `let now = Local::now(); let now = now - TimeDelta::seconds(now.second().into()) - TimeDelta::nanoseconds(now.nanosecond().into());`. Another is `DurationRound::duration_trunc(TimeDelta::minutes(1))`. Add a test that builds an ambiguous local instant, for example with an injected clock or a helper over `DateTime<Tz>`.
- **Scope**: Separate follow-up. It does not block export-notes, but it is user-facing on the user's own timezone.

## 7. Notes (not counted)

- `-w 0999-01` prints `week number must be 1 or greater`. The real problem is the year, so the message is misleading. The text comes from WEEK_ID's existing `(_, OutOfRange)` arm and predates this changeset. The spec says export reuses the existing WEEK_ID messages, so it is in scope only as a follow-up candidate.
- A broken pipe on very large output panics (exit 101), and so does a full stdout (`>/dev/full`). The spec review dropped this explicitly (NOTES 133, F3), so it is not reported.

## 8. Verdict

**ship-with-followups.** The changeset meets the export-notes spec and SPEC §3.8/§6.1/§6.2/§7.5 in every case checked, and its tests catch every mutant tried. The one follow-up (F1) is a pre-existing DST panic in `main.rs`.
