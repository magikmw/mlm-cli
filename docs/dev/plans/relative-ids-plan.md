# Plan: relative-ids

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

Spec: `docs/dev/specs/2026-10-25-relative-ids.md` (locked). Review: `docs/dev/plans/reports/relative-ids-spec-review.md` (folded, NOTES.md entry 152). Decision log: NOTES.md entries 151-153.

**Altitude**: scope of work only. Task plans choose identifiers and mechanisms.

**Read for this plan**: the spec in full; the review report; NOTES.md entries 150-153; `docs/dev/plans/panic-fixes-plan.md` (shape reference); `src/date.rs` (the error type and its messages, `DATE`'s `-N` resolver and its year clamp, the week id type and its invariant, the `WEEK_ID` and `MONTH` parsers, the T57/T59/T65 and `MONTH` bad-shape tests); `src/cli.rs` (the `status` `DATE` positional, the `week` positional and its help, the `week target` hand-written help and variadic positional, the `export notes` flags and help, the parse-failure classifier, the `<ARGS>` rename, the swapped-target detector); `src/week_target.rs` (parse order, the one- versus two-token split); `src/week_view.rs` (where `WEEK_ID` is parsed); `src/export.rs` (range resolution, the `-w -1` test); `src/time.rs` (the `DURATION` grammar); `tests/week_target_cli.rs` (the help goldens, the missing-`DURATION` test, the swapped-order tests); `tests/export_notes_cli.rs` (the help golden, the `-w -1`/`-m -1` test, its `TZ=UTC` clock-rollover pattern); `Cargo.toml` (`rust-version = "1.85"`); `clippy.toml`; `coverage-baseline.json`; SPEC.md §1.2, §3.6-§3.8, §6.1, §6.3, §8 (F7, E3, E10, E29); README.md's Planned list and its `week`, `week target` and `export notes` sections.

## Goal

Every `WEEK_ID` takes `-N` for N ISO weeks before the current one, and `MONTH` takes `-N` for N calendar months before the current one (spec §1, §3). This reaches `mlm week`, `mlm week target` and `mlm export notes --week/--month`. The grammar is `DATE`'s, so `-0` is a shape error and leading zeros are tolerated. `-N` crosses year boundaries and week 53. A `-N` that would land before `1000-01` (weeks) or `0000-01` (months), or whose digits overflow `u64`, gets a new offset error. The two shape messages change everywhere to name `-N`, and a `+` plus digits value gets a hint suffix pointing at the absolute forms (spec §4). A lone `week target` token that is a valid `WEEK_ID` becomes clap's missing-`DURATION` usage error (spec §3). There is no `+N`, no change to `DATE`, and no other relative form (spec §1.1).

## Architecture

The `WEEK_ID` and `MONTH` parsers in `src/date.rs` already take today's date, and all four call sites (`mlm week`, `week target`, `export notes -w` and `-m`, plus the swapped-target detector) go through them. So the relative form lands in the parsers, and every call site gains it with no change of its own. The parsers recognise `-` plus ASCII digits before the absolute grammar, the same way `DATE`'s resolver does. They resolve the offset from the week or month containing today. Resolution must not step one week at a time, because the week type's step-back asserts at the year-1000 floor and a huge N would loop for ages. Resolution must also never build a week id outside the type's documented `1000..=9999` invariant, since `week target` would store it and every later ranged target read would reject the row as corrupt (review F2). The `+` hint lives in the error's rendering. The `WEEK_ID` and `MONTH` shape arms append the suffix when the input is `+` plus digits, so the cause stays `Shape` and the existing `+1` cause assertions keep their meaning (plan review F2).

The error type gains a way to say "offset out of range" for `WEEK_ID` and `MONTH`. It must render spec §4's text and must not fall into the existing week arm that prints `week number must be 1 or greater`. Every other `WEEK_ID` and `MONTH` message stays byte-identical.

Three changes land in `src/cli.rs`. The `mlm week` positional takes negative-number tokens the way `status`'s `DATE` does, so `-1` reaches the parser while `-1x`, `-x` and `--bogus` stay clap usage errors with exit `2` (review clap table, F1). `week target` and `export notes` already pass hyphen values and need no clap change. The four help strings take spec §3's text. And after a successful parse, a `week target` with one token that parses as a `WEEK_ID` is turned into the same missing-argument usage error bare `mlm week target` gives, `<DURATION>` naming included. This keeps it a parse failure with exit `2`, ahead of any database work. The swapped-target reminder needs no edit, because its week check calls the `WEEK_ID` parser and so accepts `-1` once the parser does.

No schema change, no new dependency, no new top-level module, no MSRV change. `src/storage.rs`, `src/history.rs` and the renderers do not change. `mlm week -1`'s header already shows the resolved id.

## Global constraints

- Verification set: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- Cognitive complexity is 15 or less per function (`clippy.toml`, enforced by the pre-commit hook). The parsers are near that already, so the relative branch belongs in its own helper.
- Coverage ratchet against `coverage-baseline.json` (currently 99.51%). Every new branch needs a test: the `-N` path for both kinds, `-0`, the offset floor, `u64` overflow, the `+` hint, and the lone-`WEEK_ID` usage error.
- `Cargo.toml` declares `rust-version = "1.85"` and stays there. New code and tests use no std API that clippy's `incompatible_msrv` lint flags.
- Targets: Linux and Windows MSVC, CI also on macOS. No new platform-gated behavior.
- Tests never touch `~/.local/share/mlm`. Binary tests set `MLM_DB_PATH` to a temp-dir database, and `TZ=UTC` as `tests/export_notes_cli.rs` does.
- Tests that depend on the real clock derive expected ids from the test's own `Utc::now()` and rerun when the date rolls over mid-test, as `tests/export_notes_cli.rs` does. Fixed-today cases (year boundaries, week 53, floors) are unit tests with an injected today.
- Commits carry no Claude attribution. No `Co-Authored-By` or `Claude-Session` lines.
- Existing output, messages and help stay byte-identical apart from contracts 2-5 and 8. Existing tests keep their meaning, apart from the flips spec §5 lists and the one named in Risks.
- All new output is ASCII.

## Ordering

Two tasks, run in parallel in separate worktrees. They share no file, only the contracts below.

## Interface contracts (pinned before worktrees open)

1. **Grammar** (spec §3). `-N` is `-` followed by one or more ASCII digits, N at least 1, leading zeros tolerated (`-01` equals `-1`, `-07` is seven weeks ago). The current week is the ISO week of today's local date. The current month is today's calendar month. On 2026-01-05 (week 2026-02), `-2` is `2025-52`. In January 2026, `-m -1` is `2025-12`. On 2027-01-04 (week 2027-01), `-1` is `2026-53` and `-2` is `2026-52`.
2. **Shape messages** (spec §4), replacing the old text everywhere. `-0` gets these.
   - `invalid WEEK_ID "<input>": expected YYYY-WW, WW or -N (N >= 1)`
   - `invalid MONTH "<input>": expected YYYY-MM, MM or -N (N >= 1)`
3. **Offset messages** (spec §4). A `-N` resolving before `1000-01` (weeks) or `0000-01` (months), or a digit run too large for `u64`, gets these.
   - `invalid WEEK_ID "<input>": week offset is out of range`
   - `invalid MONTH "<input>": month offset is out of range`
4. **Future hint** (spec §4). `+` followed by one or more ASCII digits (`+0` included) gets the shape message plus a suffix.
   - `invalid WEEK_ID "+1": expected YYYY-WW, WW or -N (N >= 1); a future week takes YYYY-WW or WW`
   - `invalid MONTH "+1": expected YYYY-MM, MM or -N (N >= 1); a future month takes YYYY-MM or MM`
   Any other `+` input (`+1x`, `+`) gets the plain shape message.
5. **Help lines** (spec §3), verbatim. Clap renders each on one line after the argument name, as the current goldens show.
   - `mlm week`, `[WEEK_ID]`: ``Week to show: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week``
   - `mlm week target`, `[WEEK_ID]`: ``Week to set: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week``
   - `mlm export notes`, `-w, --week <WEEK_ID>`: ``Week to export: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week)``
   - `mlm export notes`, `-m, --month <MONTH>`: ``Month to export: YYYY-MM (e.g. 2026-03), a bare month number (e.g. 3), or `-N` for N months before this one (e.g. `-1` = last month)``
6. **Exit codes** (spec §3, §4). Every contract 2-4 error is `error: ...` on stderr, empty stdout, nothing written, exit `1`. On `mlm week`, tokens clap reads as negative numbers (`-1`, `-01`, `-1.5`, `-1e3`, `-1.`) reach the parser, and only `-` plus digits resolves, so `mlm week -1.5` gets contract 2's message with exit `1`. `mlm week -1x`, `-x` and `--bogus` stay clap usage errors with exit `2`, as `mlm status -1x` is today. On `week target`, `-w` and `-m`, `-1x` gets contract 2's message with exit `1`.
7. **Flag order** (spec §3). `mlm week -1 -n` and `mlm week -n -1` both show last week with notes.
8. **`week target` shapes** (spec §3).
   - `mlm week target -1 35h` writes last week's target and echoes the normalized id (`Week <YYYY-WW> target: 35h 00m`).
   - `mlm week -1 target 35h` prints `error: put WEEK_ID after target: mlm week target -1 35h`, exit `2`.
   - `mlm week target -1`, `mlm week target 7` and `mlm week target 2026-07` alone print stderr byte-identical to bare `mlm week target` and exit `2`. Stdout is empty and nothing is written.
   - `mlm week target 35h` still sets this week.
9. **SPEC placement** (spec "Interaction"). §1.2 drops the relative-week non-goal and lists `+N` as one. §3.6, §3.7 and §3.8 gain the `-N` form. §3.8's `-w -1`/`-m -1` sentence is rewritten and its quoted help lines take contract 5. §6.1's `WEEK_ID` bullet gains contracts 2 and 3 for weeks. Its `MONTH` bullet's shape message is replaced and the offset message added. §8 gains happy-path and error flows.

## Task 1: code and tests

**Spec**: §3 surface and help, §4 errors, §5 testing; non-goals §1.1.

The `WEEK_ID` and `MONTH` parsers in `src/date.rs` gain the relative form (contract 1), the offset error (contract 3), the `+` hint (contract 4) and the new shape text (contract 2). The error type gets an offset cause or arm that renders spec §4's text for both kinds and leaves the `DATE` and other arms alone. `src/cli.rs` lets `mlm week`'s positional take negative numbers, carries contract 5's four help strings, and turns a lone valid `WEEK_ID` after `week target` into the missing-`DURATION` usage error (contract 8). Unit tests in `src/date.rs` cover the fixed-today cases. Binary tests for the CLI shapes go in a new `tests/relative_ids_cli.rs`. Existing tests that assert `-N` is rejected flip to acceptance tests or move to inputs that stay malformed (spec §5). That covers `src/date.rs` T57 (`-5`), T59 (`-07`), T65 (`-1`), the `MONTH` bad-shape list's `-1`, the `-1` `MONTH` message test, `src/export.rs`'s `-w -1` test and `tests/export_notes_cli.rs`'s `-w -1`/`-m -1` test. Every assertion of the old shape text takes contract 2's text (2 assertions plus the 2 `Display` arms in `src/date.rs`, 8 assertions in `tests/export_notes_cli.rs`). The help goldens in `tests/week_target_cli.rs` and `tests/export_notes_cli.rs` take contract 5.

Acceptance criteria. Unit cases use an injected today. CLI cases assert exact stdout, stderr and exit code, with ids that depend on the clock computed from the test's own UTC today.
- `-1`, `-2` and `-01` resolve correctly for `WEEK_ID` and `MONTH` against a fixed today, and an N that crosses a year boundary does too (contract 1's examples).
- A `-N` landing on week 53 gives `2026-53`. One stepping over it lands on `2026-52` and then `2026-51`.
- `-0` gives contract 2's message for both kinds.
- The last N that reaches `1000-01` (weeks) and `0000-01` (months) from a fixed today is accepted. The next N gives contract 3's message. A 25-digit run gives contract 3's message for both kinds. From 2026-02-xx, `-m -24313` is `0000-01` and `-m -24314` is out of range. No resolution panics or produces a week id below `1000-01`.
- `+1` and `+0` give contract 4's messages for `WEEK_ID` and `MONTH`. `+1x` gives the plain shape message.
- Binary, `mlm week -1` shows last week's header with exit `0`. `mlm week -1 -n` and `mlm week -n -1` show last week with notes.
- Binary, `mlm week -1x` and `mlm week -x` exit `2` with clap's usage error, empty stdout. `mlm week -1.5` prints contract 2's message, exit `1`.
- Parse tests: `mlm week -01`, `-07`, `-2` and `-0` land in the `WEEK_ID` positional verbatim.
- Binary, `mlm week target -1x 35h`, `mlm ex n -w -1x` and `mlm ex n -m -1x` print contract 2's message, exit `1`.
- Binary, `mlm week +1`, `mlm week target +1 35h`, `mlm ex n -w +1` and `mlm ex n -m +1` print contract 4's message, exit `1`.
- Binary, `mlm week target -1 35h` writes last week's row and echoes its normalized id. An existing row for this week is untouched.
- Binary, `mlm week -1 target 35h` prints contract 8's reminder, exit `2`.
- Binary, `mlm week target -1`, `7` and `2026-07` alone print the same stderr as bare `mlm week target`, exit `2`, empty stdout, no row written. `mlm week target 35h` still writes this week. The same holds for `mlm w target -1`, `mlm -v week target -1`, `mlm week target -v 7` and `mlm week target -- -1`.
- `DATE` `+1` keeps its exact message, `invalid DATE "+1": expected YYYY-MM-DD or -N (N >= 1)`, with no hint.
- Binary, `mlm ex n -w -1` and `mlm ex n -m -1` print the notes seeded in last week and last month and nothing outside them.
- `mlm week -h`, `mlm week target -h`/`--help` and `mlm export notes -h` match goldens carrying contract 5.
- Every other existing test passes unchanged.

**Out of scope**: `+N` resolution. Any `DATE` change. Week ranges near year 9999. A relative spelling for the current week or month. Storage, history, rendering and the seed tool. Any `.md` file.

**Owns exclusively**: `src/date.rs`, `src/cli.rs`, `src/export.rs`, `src/week_target.rs`, `src/week_view.rs`, `tests/week_target_cli.rs`, `tests/export_notes_cli.rs`, the new `tests/relative_ids_cli.rs`, and `coverage-baseline.json`, which the pre-commit hook ratchets and stages. `src/export.rs`, `src/week_target.rs` and `src/week_view.rs` are expected to change only in their tests. If the work needs `src/main.rs` or `src/lib.rs`, that file also belongs to this task, since Task 2 owns no source file. Any other file needs a coordinator decision first.

## Task 2: docs

**Spec**: §3, §4, §6 user-visible surface, "Interaction with the project spec"; non-goals §1.1.

This task folds the behavior into SPEC.md and README.md so both stand alone, with no pointer to the working spec.

Acceptance criteria:
- SPEC §1.2's bullet on `+N`/`-N` relative week notation becomes a `+N`-only non-goal that names the absolute forms as the way to reach a future week or month.
- SPEC §3.6 and §3.7 describe `-N` for `WEEK_ID` (contract 1), the `mlm week` exit-`2` rule for other hyphen tokens (contract 6), `mlm week -1 -n` (contract 7), and the `week target` shapes (contract 8). §3.7's reminder text shows `-1` echoed as typed.
- SPEC §3.8 describes `-N` for `--week` and `--month`, including the year crossing. Its sentence saying `-w -1` and `-m -1` reach their own errors is rewritten, and its quoted `-w`/`-m` help lines take contract 5.
- SPEC §6.1's `WEEK_ID` bullet quotes contract 2's and 3's week messages and the `+` hint (contract 4). Its `MONTH` bullet replaces `expected YYYY-MM or MM` with contract 2's text and adds contract 3's and 4's month messages. The floors `1000-01` and `0000-01` and the `u64` overflow rule are stated.
- SPEC §8.1 gains happy paths for `mlm week -1`, `mlm week target -1 35h`, `mlm ex n -w -1` and `-m -1`. §8.2 gains the error flows (`-0`, offset out of range, `+1`, `mlm week -1x`, lone `week target -1`). E29's `-m -1` and `-w -1` text is rewritten, and E10 names the lone-`WEEK_ID` case.
- §3.7 or E10 states the boundary of that case. Only a lone token that parses as a `WEEK_ID` gives the usage error. Any other lone token, `-0` and an out-of-range `-N` included, keeps `invalid duration`, exit `1`.
- §3.6 states that `mlm week -1.5` reaches the parser and gets the shape error, exit `1` (contract 6).
- README's Planned bullet at `README.md:42-43` becomes a `+N`-only bullet. The `mlm week`, `week target` and `export notes` sections each gain the `-N` form with an example.
- AGENTS.md is checked and left alone unless a statement in it goes stale. With no new module, none is expected.
- Every quoted message, help line and exit code matches the contracts byte for byte.

**Out of scope**: `--help` text and source doc comments (Task 1). NOTES.md. Archiving the working spec and plans.

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`. NOTES.md and the archive banners stay with the coordinator.

## Risks and ambiguities

- **Flipped CLI test outside spec §5's list.** `tests/export_notes_cli.rs` has a test asserting `-w -1` and `-m -1` are value errors. Spec §5 names only the `src/export.rs` unit test. Task 1 flips both, since the spec's intent covers every `-N` rejection test.
- **Lone `week target` tokens that fail as `WEEK_ID`.** Spec §3 converts only a token that is a valid `WEEK_ID`. So `mlm week target -0`, `mlm week target -99999999999999999999` and `mlm week target 53` in a 52-week year keep today's `invalid duration "<token>"` error, exit `1`. This plan takes that literal reading. No `WEEK_ID` is ever a well-formed `DURATION` (a duration needs an `h` or `m` suffix), so the spec's "and not a valid `DURATION`" clause never changes the outcome.
- **SPEC §8 needs rewrites beyond "gains".** E29 says `-m -1` and `-w -1` fail, and E10 describes only the zero-token case. Both become false or incomplete, so Task 2 rewrites them rather than only adding flows.
- **Midnight and Monday rollover in binary tests.** A relative id computed by the test can differ from the binary's if the date rolls over between the two clock reads. Task 1 uses the existing rerun-on-rollover pattern from `tests/export_notes_cli.rs`.
- **Week floor arithmetic.** Subtracting `7 * N` days overflows for large N before chrono sees it. Task 1's plan has to reject the overflow as contract 3, the same way `DATE`'s resolver treats an offset chrono cannot represent.
