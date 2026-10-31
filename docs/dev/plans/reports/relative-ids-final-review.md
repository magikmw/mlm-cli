# relative-ids — final review

- Baseline: `main` at `77fb19f`
- Branch: `relative-ids`, tip `0580e6f` (clone `/tmp/cd-scratch/relative-ids-final`, HEAD verified)
- Spec: `docs/dev/specs/2026-10-25-relative-ids.md` (locked)
- Plan: `docs/dev/plans/relative-ids-plan.md`, contracts 1-9
- Decisions: NOTES.md entries 151-156
- Today on the machine: Sat 2026-10-31, ISO week 2026-44

## Verdict

ship-with-followups. The code matches every contract, and the tests that matter fail under mutation. One low-severity README imprecision is the only finding.

## Verification set (run in the clone)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | all green (853 lib tests plus 16 integration binaries, 0 failed) |
| `cargo clippy --all-targets -- -D warnings` | clean |

`coverage-baseline.json` moved up, from 99.505% to 99.517%.

## Findings

### F1 (low) README overstates which bare numbers become the usage error

- Location: `README.md:586-587`
- The text says "A bare number counts as a week id, so `mlm week target 40` fails the same way." Only a bare number that is a valid ISO week for the current year counts.
- Observed in the clone on 2026-10-31:
  - `mlm week target 60` prints `error: invalid duration "60": ...`, exit 1
  - `mlm week target 0` prints `error: invalid duration "0": ...`, exit 1
  - `mlm week target 40` gives the usage error, exit 2, as documented
- SPEC §3.7 (lines 484-493) and E10 state the boundary correctly. The README omits it, so a reader expects exit 2 for every bare number.
- Fix: change the README sentence to "A bare number that is a valid week number counts as a week id, so `mlm week target 40` fails the same way, while `mlm week target 60` keeps the `invalid duration` error." Or add one sentence that names `0` and numbers past the year's last week.

## What was checked

### Correctness, by hand against the binary

These runs used `MLM_DB_PATH` under the clone, today 2026-10-31.

| Command | Result |
|---|---|
| `mlm week -1`, `-01`, `-1 -n`, `-n -1` | `Week 2026-43 (2026-10-19 - 2026-10-25)`, notes under rows with `-n` |
| `mlm week -1.5`, `-1e3`, `-1.` | WEEK_ID shape error, exit 1 |
| `mlm week -1x`, `-x`, `--bogus` | clap usage error, exit 2 |
| `mlm week -- -1x` | shape error, exit 1 |
| `mlm week -0` | shape error, exit 1 |
| `mlm week +1` | shape error plus future-week hint, exit 1 |
| `mlm week -99999999999999999999`, `-18446744073709551616`, 25 digits | week offset error, exit 1 |
| `mlm week -53577` / `-53578` | `1000-01 (0999-12-30 - 1000-01-05)` / week offset error |
| `mlm week target -1 35h` | `Week 2026-43 target: 35h 00m`, row `2026-43\|2100`; this week's row untouched |
| `mlm week target -53577 10h` | stores `1000-01`; `mlm week` and `mlm status` still load fine afterwards |
| `mlm week target -1`, `7`, `2026-07`, `40`, `53` (2026 has 53 weeks) | stderr identical to bare `mlm week target`, exit 2 |
| `mlm week target -0`, `0`, `60` | `invalid duration`, exit 1 |
| `mlm week -1 target 35h`, `mlm w -01 target 35h` | reminder echoing `-1` / `-01` as typed, exit 2 |
| `mlm week target -1x 35h`, `+1 35h` | shape error (with hint for `+1`), exit 1 |
| `mlm ex n -w -1` | notes on 2026-10-19 and 2026-10-25 only; 2026-10-18 and 2026-10-26 excluded |
| `mlm ex n -m -1`, `-m -01` | the 2026-09-30 note only; 2026-10-01 excluded |
| `mlm ex n -m -24321` / `-24322` | `0000-01`, exit 0 / month offset error |
| `mlm ex n -m -0`, `+1`, `-1x`, `-w -1x` | shape errors as in contracts 2 and 4, exit 1 |
| `--help` for `week`, `week target`, `export notes` | the four contract-5 lines, verbatim |

### Worked examples with an injected today

A throwaway example program in the clone (since deleted) called `mlm::date::parse_week_id` and `parse_month` directly.

- 2026-01-05: `-1` is `2026-01`, `-2` is `2025-52`.
- 2027-01-04: `-1` is `2026-53`, `-2` is `2026-52`. 2027-01-10 (Sunday) gives `-1` = `2026-53` too.
- 2021-01-03 (Sunday of 2020-53): `-1` is `2020-52`. 2021-01-04: `-1` is `2020-53`.
- 2026-02-12 and 2026-02-09 (Monday): `-53540` is `1000-01`. `-53541` is out of range. This matches E36.
- January 2026: `-m -1` is `2025-12`. 2025-12-30 gives `-m -1` = `2025-11`, the calendar month, not the ISO one.
- February 2026: `-m -24313` is `0000-01`, and `-24314` is out of range.
- A sweep over every day of 1000-1003 and 2020-2028 tried N in {1, 2, 3, 52, 53, 54, 105, 300, 53600}, 42,732 cases. Each result either errored with the offset message or returned a week whose Monday-to-Sunday span holds `today - 7N`, with ISO year at least 1000. Nothing panicked.

### Week floor and panics

- `resolve_week_offset` (`src/date.rs`) uses `checked_mul(7)` and `checked_sub_days`. It checks `iso_week().year() < MIN_YEAR` before it calls `WeekId::from_date`, so no path builds a week below `1000-01`. A digit run past `u64` maps to `OffsetOutOfRange`.
- `resolve_month_offset` counts months from `0000-01` and uses `checked_sub`. The `as` casts are bounded by today's year.

### Lone-WEEK_ID re-parse (`src/cli.rs`)

- The check runs only after a successful parse, and only for a one-token `week target` whose token passes `parse_week_id`.
- `without_last` removes the last argv element equal to that token. The re-parse then gives clap's own missing-argument error, and the `<ARGS>` rename applies.
- `rposition` versus `position` is an equivalent mutant. No earlier argv token (`mlm`, `week`/`w`, `target`, `-v`, `--`) can equal a valid WEEK_ID. This is not a finding.

### Mutation tests (each applied in the clone, run against `cargo test`, then reverted)

| Mutation | Outcome |
|---|---|
| floor reads calendar year instead of ISO year | killed |
| floor check removed | killed |
| `checked_mul(7)` replaced by plain `* 7` | killed |
| `n == 0` shape check removed | killed |
| month `checked_sub` replaced by `saturating_sub` | killed |
| month off by one (`% 12` without `+ 1`) | killed |
| `month0()` replaced by `month()` | killed |
| `allow_negative_numbers = false` on the week positional | killed |
| lone-WEEK_ID check disabled | killed |
| future hint keyed on `-` instead of `+` | killed |
| `signed_digits` accepts an empty digit run | killed |
| `rposition` replaced by `position` | survived (equivalent, see above) |

### Assertion strength

- The floor tests assert exact `Ok(wk(1000, 1))` and exact `Err(... OffsetOutOfRange)` on both sides of the boundary. They also cover the Monday case, where the resolved date falls in calendar year 999.
- `relative_week_never_builds_a_week_below_1000` sweeps N 53530-53560 from four todays, and asserts either ISO year at least 1000 or the offset cause.
- The binary tests assert exact stderr, empty stdout and the exit code, and check that the DB has no target rows. The tests that depend on the clock compare `mlm week -1` byte-for-byte against `mlm week <absolute id>` and use the rerun-on-rollover loop.
- The lone-token tests compare stderr byte-for-byte against bare `mlm week target`, in both the unit and binary layers.

### Security

- The swapped-target reminder echoes two tokens. It fires only when the week token passes `parse_week_id`, so the token holds only digits and `-`. The duration token must have the DURATION shape (digits with `h`/`m`, optional sign). No shell metacharacter or quote can reach the echoed `mlm week target ...` line.
- No new SQL. The week id stored by `week target -N` is the normalized `WeekId` display, never the typed string.

### Spec and docs compliance

- SPEC §1.2, §3.6, §3.7, §3.8, §6.1, §8 (F16, F17, E10, E24, E29, E35-E38) match the contracts and the binary's output byte-for-byte where they quote text. §6.3 needed no change.
- No stale "WEEK_ID takes no `-N`" statement remains in SPEC.md, README.md or AGENTS.md.
- README matches the binary except for F1.
- Source comments cite "relative-ids spec §N". That follows the existing convention (backdated-punches, export-notes, panic-fixes) and is not a finding.

### Out of scope, observed and not filed

- `mlm week target 7 --verbose` reads `--verbose` as the DURATION (`invalid duration "--verbose"`, exit 1). This comes from the hyphen-value rule `week target`'s argument list had before this changeset. The spec and SPEC claim only `-v` before the token.
- Items the user already ruled on, and the fresh-eyes findings, are not re-filed.
