# Task plan: relative-ids, Task 1 (code and tests)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

- Changeset plan: `docs/dev/plans/relative-ids-plan.md` (Task 1, contracts 1-8)
- Spec: `docs/dev/specs/2026-10-25-relative-ids.md` (locked)
- Branch: `relative-ids` at `13e0f80`
- Altitude: task plan. Exact signatures and code sketches. No source has been edited to write this plan.

## Read for this plan

- `docs/dev/plans/relative-ids-plan.md`, in full
- `docs/dev/specs/2026-10-25-relative-ids.md`, in full
- `docs/dev/plans/reports/relative-ids-spec-review.md` and `relative-ids-plan-review.md`, in full
- `docs/dev/NOTES.md` entries 151-154 (lines 2066-2111)
- Sibling task plans: none exist yet. `docs/dev/plans/` holds no `relative-ids-task-2-*.md`. Task 2 owns docs only, so no signature has to agree with it.
- `src/date.rs`, `src/cli.rs`, `src/export.rs`, `src/main.rs`, in full
- `src/week_target.rs:1-140` plus a grep of its tests; `src/week_view.rs:315-335` plus a grep for `parse_week_id`
- `src/time.rs:60-90` and `:170` (the `DURATION` grammar and its message)
- `src/status.rs:427-434` (the `DATE` error path)
- `tests/week_target_cli.rs`, `tests/export_notes_cli.rs`, in full; `tests/week_notes_cli.rs:1-60`; `tests/panic_fixes_cli.rs:36-50`
- `clippy.toml`, `.githooks/pre-commit`, `coverage-baseline.json`, `Cargo.toml` (`rust-version = "1.85"` at line 5, clap 4.6.6 at line 32)

## Scope

| Owner | Files |
|---|---|
| Task 1 (this plan) | `src/date.rs`, `src/cli.rs`, `src/export.rs` (tests only), `tests/week_target_cli.rs`, `tests/export_notes_cli.rs`, new `tests/relative_ids_cli.rs`, `coverage-baseline.json` (hook-staged) |
| Task 1, no change expected | `src/week_target.rs`, `src/week_view.rs`, `src/main.rs`, `src/lib.rs` |
| Task 2 (docs, parallel) | `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` |
| Coordinator | `docs/dev/NOTES.md`, archive banners |

Task 1 needs nothing from Task 2. Task 2 quotes contracts 2-6 and 8 from the changeset plan. This plan emits exactly those strings, so the handoff is the contract text itself.

## 1. Facts gathered

### 1.1 `src/date.rs`

- `use chrono::{Datelike, Days, NaiveDate, Weekday};` at :15. `Days` and `Datelike` are already imported.
- `Cause` enum at :31-48 has `Shape`, `OutOfRange`, `NoSuchCalendarDate`, `NoSuchIsoWeek {..}`, `Future`. It derives `Debug, Clone, PartialEq, Eq`.
- `DateWeekError { arg, input, cause }` at :52-57, with `new(arg, &str, cause)` at :60.
- `Display::fmt` at :69-95. The shape arms are at :79 (`expected YYYY-WW or WW`) and :80 (`expected YYYY-MM or MM`). The catch-all `(_, Cause::OutOfRange)` at :83 prints `week number must be 1 or greater`. A new offset cause must not land there (review F2).
- `resolve_date` at :132-162 is the model for `-N`. It does `strip_prefix('-')`, checks for non-empty all-ASCII digits, maps a `u64` parse failure to `OutOfRange`, maps `0` to `Shape`, then uses `checked_sub_days` plus a year-domain check.
- `MIN_YEAR = 1000` and `MAX_YEAR = 9999` at :215-216.
- `WeekId::from_date` at :260-266 does not check the year invariant. `WeekId::prev` asserts at the year-1000 floor (:355-358).
- `parse_week_id` at :377-393. `split_once('-')` makes any `-...` input a shape error today, because the year part is empty.
- `Month::new` at :411-416 accepts years `0..=9999`. `parse_month` at :444-448 delegates to `month_parts` (:452-467).
- Tests that flip or change text:
  - `parse_week_id_rejects_wrong_shapes` at :735-760 lists `"-5"` (T57, :743), `"-07"` (T59, :745), `"+1"` and `"-1"` (T65, :751-752).
  - `parse_month_rejects_wrong_shapes` at :1150-1162 lists `"-1"` and `"+1"` (:1153).
  - `month_error_messages_match_the_spec` at :1204-1218 asserts the old text at :1207 (`abc`) and :1208 (`-1`).
- Unit test fixture `TODAY()` is 2026-02-12, a Thursday in ISO week 2026-07 (:521). Helpers `d`, `wk`, `date_err`, `week_err`, `mo`, `month_err` exist.

### 1.2 Cognitive complexity today

I ran clippy with `CLIPPY_CONF_DIR` pointing at a scratch `clippy.toml` with threshold 1, so every function reports its score. The relevant scores are:

| Function | Location | Score |
|---|---|---|
| `DateWeekError::fmt` | `src/date.rs:70` | 3 |
| `resolve_date` | `src/date.rs:132` | 2 |
| `parse_week_id` | `src/date.rs:377` | 2 |
| `month_parts` | `src/date.rs:452` | 4 |
| `classify` | `src/cli.rs:384` | 2 |
| `rename_args_context` | `src/cli.rs:402` | 5 |
| `swapped_week_target` | `src/cli.rs:423` | 3 |

Every function in this plan stays far under 15. Each new helper below has at most two branch points.

### 1.3 `src/cli.rs`

- `Status.date` uses `#[arg(allow_negative_numbers = true)]` at :103. That is the parity model for `mlm week`.
- `WeekArgs` at :235-249 has `#[command(args_conflicts_with_subcommands = true)]`. The doc comment of `week_id` (:241-242) is the `mlm week` help line. Its attribute at :243 is `#[arg(value_name = "WEEK_ID")]`.
- `WeekTargetArgs` at :272-290 has a hand-written `before_help` whose `[WEEK_ID]` line is :278. The single positional `args` has `num_args = 1..=2`, `allow_hyphen_values = true` and `hide = true`.
- `ExportNotesArgs` at :316-332. The `-w` doc is :324 and the `-m` doc is :328-329. Both already set `allow_hyphen_values = true`.
- `parse` at :377-382 maps any clap error through `classify` (:384-396). `classify` returns help and version untouched, runs the swap check, then runs `rename_args_context` (:402-417), which turns `<ARGS>` into `<DURATION>`.
- `swapped_week_target` at :423-440 calls `is_week_token` (:443-445), which calls `parse_week_id`. It accepts `-1` once the parser does.
- `WeekTargetArgs::split` at :460-466 returns `(None, token)` for one token.
- The test module's `parse` helper (:473-475) is `Cli::try_parse_from`, so it bypasses `super::parse`. Tests `os`, `strs`, `swapped`, `clap_text` exist at :839-850 and :980-985.

### 1.4 Call sites (no change needed)

- `src/week_view.rs:327-331` does `parse_week_id(s, today)` with `today = now.date_naive()`.
- `src/week_target.rs:89-92` does `parse_week_id(tok, today)`. `main.rs:77` passes `now.date_naive()`.
- `src/export.rs:29-34` calls `parse_week_id` and `parse_month`.
- `src/main.rs:19-22` parses before `db::connect` (:58). A parse failure writes nothing and creates no database file.
- `src/storage.rs:578` maps a `from_key` error to `CorruptRow`, so `DateWeekError`'s shape text never reaches the user from storage.
- Outside `src/date.rs`, no code matches `Cause` exhaustively. There are only `matches!` calls at `status.rs:429` and `commands.rs:1122`.

### 1.5 Tests and goldens

- `tests/week_target_cli.rs`: `WEEK_SHORT` golden `[WEEK_ID]` line at :229, `week_target_help_text` golden `[WEEK_ID]` line at :262, and `week_target_missing_duration_names_duration` at :272-284. No existing test runs `week target` with a lone token that is a valid `WEEK_ID`. The lone tokens in use are `20h`, `-5h`, `--help` and `-h`.
- `tests/export_notes_cli.rs`: `minus_one_week_and_month_are_value_errors` at :249-264 flips. `VALUE_ERRORS` has old text at :380, :384, :388, :392, :396 and :400. `EXPORT_NOTES_HELP` has `-w` and `-m` lines at :543-544. `on_stable_today` is at :86-94. `seed_on` and `line` helpers exist.
- `src/export.rs:157-176` (`range_errors_carry_their_arg_kind`) asserts `-w -1` is `(WeekId, Shape)` at :168-171.
- The `DURATION` grammar needs an `h` or `m` suffix (`src/time.rs:70-82`), so no `WEEK_ID` is a valid `DURATION`. Its message is `invalid duration "<input>": expected a form like 20h, 33h30m, or 45m` (`src/time.rs:170`).

### 1.6 Constants derived from the spec's examples

I computed these with Python's proleptic Gregorian `date` in a scratch box.

| Today | Input | Result | Note |
|---|---|---|---|
| 2026-02-12 (W07) | `-1`, `-01` / `-2` / `-07` | 2026-06 / 2026-05 / 2025-52 | `-07` is seven weeks ago |
| 2026-01-05 (W02) | `-1` / `-2` | 2026-01 / 2025-52 | contract 1 |
| 2027-01-04 (W01) | `-1` / `-2` / `-3` | 2026-53 / 2026-52 / 2026-51 | contract 1, week 53 |
| 2027-01-10 (Sun, W01) | `-1` | 2026-53 | same week, other weekday |
| 2026-02-12 | `-53540` / `-53541` | 1000-01 / out of range | week floor |
| 2026-02-09 (Mon, W07) | `-53540` | 1000-01 | resolves to 0999-12-30, calendar year 999 |
| 2026-02-12 | `-m -1` / `-2` / `-14` | 2026-01 / 2025-12 / 2024-12 | |
| 2026-01-15 | `-m -1` | 2025-12 | contract 1 |
| 2026-02-12 | `-m -24313` / `-24314` | 0000-01 / out of range | 2026*12 + 1 = 24313 |

ISO week 1000-01 runs from 0999-12-30 to 1000-01-05. So the week floor must check the ISO year. A calendar-year check would wrongly reject `-53540` from a Monday.

## 2. Design

### 2.1 Error type (`src/date.rs`)

I add one `Cause` variant after `OutOfRange` (:36):

```rust
/// A `-N` WEEK_ID or MONTH that resolves before the earliest id its
/// absolute form accepts (`1000-01` weeks, `0000-01` months), or whose
/// digits overflow a u64. `DATE` offsets keep `OutOfRange`.
OffsetOutOfRange,
```

A separate variant keeps the `(_, Cause::OutOfRange)` week arm (:83) and every existing `OutOfRange` assertion unchanged.

`Display::fmt` (:77-93) becomes the match below. Only the shape arms change, the `Date` offset arm gains an or-pattern, and two arms are new. Every other line stays byte-identical.

```rust
match (&self.arg, &self.cause) {
    (ArgKind::Date, Cause::Shape) => write!(f, "expected YYYY-MM-DD or -N (N >= 1)"),
    (ArgKind::WeekId, Cause::Shape) => {
        write!(f, "expected YYYY-WW, WW or -N (N >= 1)")?;
        write_future_hint(f, &self.input, "week", "YYYY-WW or WW")
    }
    (ArgKind::Month, Cause::Shape) => {
        write!(f, "expected YYYY-MM, MM or -N (N >= 1)")?;
        write_future_hint(f, &self.input, "month", "YYYY-MM or MM")
    }
    (ArgKind::Month, Cause::OutOfRange) => write!(f, "month must be 1-12"),
    (ArgKind::Date, Cause::OutOfRange | Cause::OffsetOutOfRange) => {
        write!(f, "date offset is out of range")
    }
    (ArgKind::WeekId, Cause::OffsetOutOfRange) => write!(f, "week offset is out of range"),
    (ArgKind::Month, Cause::OffsetOutOfRange) => write!(f, "month offset is out of range"),
    (_, Cause::OutOfRange) => write!(f, "week number must be 1 or greater"),
    // NoSuchCalendarDate, NoSuchIsoWeek, Future arms unchanged
}
```

`Date` never carries `OffsetOutOfRange`. The or-pattern gives that pair a sensible text on a line tests already cover, so there is no unreachable arm and no uncovered line.

The new free function `write_future_hint` goes next to the `Display` impl:

```rust
/// Appends `; a future <unit> takes <forms>` when `input` is `+` plus
/// ASCII digits (relative-ids spec §4).
fn write_future_hint(
    f: &mut std::fmt::Formatter<'_>,
    input: &str,
    unit: &str,
    forms: &str,
) -> std::fmt::Result {
    if signed_digits(input, '+').is_some() {
        write!(f, "; a future {unit} takes {forms}")?;
    }
    Ok(())
}
```

The cause stays `Shape` for `+1`, so T65's `"+1"` and the `MONTH` list's `"+1"` keep their `Cause::Shape` assertions (plan review F2).

### 2.2 Relative helpers (`src/date.rs`)

These go in a new section, `// Relative WEEK_ID / MONTH (-N)`, between the MONTH section (:395-467) and "Week iteration" (:469).

```rust
/// The digits after `sign` when `s` is `sign` followed by one or more
/// ASCII digits.
fn signed_digits(s: &str, sign: char) -> Option<&str> {
    s.strip_prefix(sign)
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// N from a `-N` digit run. `0` is a shape error, and a run too large
/// for a u64 is out of range.
fn offset_count(digits: &str, s: &str, arg: ArgKind) -> Result<u64, DateWeekError> {
    let n: u64 = digits
        .parse()
        .map_err(|_| DateWeekError::new(arg, s, Cause::OffsetOutOfRange))?;
    if n == 0 {
        return Err(DateWeekError::new(arg, s, Cause::Shape));
    }
    Ok(n)
}

/// The ISO week N weeks before the one containing `today`. Stepping 7N
/// days back from `today` lands in that week on today's weekday, so no
/// week-by-week walk is needed.
fn resolve_week_offset(digits: &str, s: &str, today: NaiveDate) -> Result<WeekId, DateWeekError> {
    let out_of_range = || DateWeekError::new(ArgKind::WeekId, s, Cause::OffsetOutOfRange);
    let n = offset_count(digits, s, ArgKind::WeekId)?;
    let resolved = n
        .checked_mul(7)
        .and_then(|days| today.checked_sub_days(Days::new(days)))
        .ok_or_else(out_of_range)?;

    // ISO year 1000 starts on 0999-12-30, so the floor reads the ISO year.
    if resolved.iso_week().year() < MIN_YEAR {
        return Err(out_of_range());
    }
    Ok(WeekId::from_date(resolved))
}

/// The calendar month N months before `today`'s, counted from 0000-01,
/// the earliest month `Month` holds.
fn resolve_month_offset(digits: &str, s: &str, today: NaiveDate) -> Result<Month, DateWeekError> {
    let n = offset_count(digits, s, ArgKind::Month)?;
    let current = i64::from(today.year()) * 12 + i64::from(today.month0());
    let target = u64::try_from(current)
        .ok()
        .and_then(|m| m.checked_sub(n))
        .ok_or_else(|| DateWeekError::new(ArgKind::Month, s, Cause::OffsetOutOfRange))?;
    Month::new((target / 12) as i32, (target % 12) as u32 + 1, s)
}
```

Notes on these helpers:

- `WeekId::from_date` is only reached after the ISO-year check. The upper bound is today's own ISO year. So no `WeekId` outside `1000..=9999` is ever built (review F2), and `prev()` is never called.
- `n.checked_mul(7)` overflows for `n > u64::MAX / 7`, and `checked_sub_days` returns `None` past chrono's range. Both become `OffsetOutOfRange` (changeset plan Risks, "Week floor arithmetic").
- `target <= current <= 9999 * 12 + 11`, so both `as` casts are lossless. `Month::new` can only fail if an injected `today` has a year above 9999, and then it returns its usual `OutOfRange`.
- `i64::from(today.month0())` is `month - 1`, so `current` counts months since 0000-01.

### 2.3 Parser entry points (`src/date.rs`)

`parse_week_id` (:377) gains a first branch. The rest of the body is unchanged.

```rust
/// Parse a `WEEK_ID` argument (§3.6): full `YYYY-WW`, a bare `WW` whose
/// year defaults to `today`'s **ISO** year, or `-N` for N ISO weeks
/// before `today`'s. Unpadded weeks are normalized.
pub fn parse_week_id(s: &str, today: NaiveDate) -> Result<WeekId, DateWeekError> {
    if let Some(digits) = signed_digits(s, '-') {
        return resolve_week_offset(digits, s, today);
    }
    // existing body from :378 on, unchanged
}
```

`parse_month` (:444) changes the same way:

```rust
/// Parse a `MONTH` argument (§3.8): `YYYY-MM`, `YYYY-M`, a bare `M`/`MM`
/// whose year defaults to `today`'s **calendar** year, or `-N` for N
/// months before `today`'s.
pub fn parse_month(s: &str, today: NaiveDate) -> Result<Month, DateWeekError> {
    if let Some(digits) = signed_digits(s, '-') {
        return resolve_month_offset(digits, s, today);
    }
    let (year, month) =
        month_parts(s, today).ok_or_else(|| DateWeekError::new(ArgKind::Month, s, Cause::Shape))?;
    Month::new(year, month, s)
}
```

Inputs that are not `-` plus digits fall through to the old grammar, which already makes them shape errors. That covers `-1x`, `-1.5`, `-1e3`, `--1`, `-` and `-d`. In each case the part before the first `-` is empty, so it fails the four-digit year check. `resolve_date` and `parse_date` are not touched (spec §1.1).

The module doc (:1-8) gains the `-N` forms in its first line: "`DATE`, ISO week ids (`WEEK_ID`) and calendar months (`MONTH`), each with a `-N` relative form: ...". Only the doc comment changes.

### 2.4 `mlm week` positional (`src/cli.rs:241-244`)

```rust
/// Week to show: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or
/// `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to
/// the current week.
#[arg(value_name = "WEEK_ID", allow_negative_numbers = true)]
pub week_id: Option<String>,
```

Clap joins the lines with spaces and drops the final period, as `status`'s `DATE` doc (:101-102) shows in the `STATUS_SHORT` golden. The rendered line equals contract 5. The review harness confirmed this setting under `args_conflicts_with_subcommands`. With it, `-1`, `-1 -n` and `-n -1` parse. `-1.5`, `-1e3` and `-1.` reach the parser. `-1x`, `-x` and `--bogus` stay exit 2. `-1 target 35h` becomes `ArgumentConflict`, which `classify` routes to the swap check.

### 2.5 Help strings (`src/cli.rs`)

- `before_help` line :278 becomes:
  ``  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week``
  This is three spaces after `[WEEK_ID]`, as now, and stays on one line.
- `-w` doc (:324):
  ```rust
  /// Week to export: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7),
  /// or `-N` for N weeks before this one (e.g. `-1` = last week)
  ```
- `-m` doc (:328-329):
  ```rust
  /// Month to export: YYYY-MM (e.g. 2026-03), a bare month number (e.g.
  /// 3), or `-N` for N months before this one (e.g. `-1` = last month)
  ```
- No trailing period on either, which matches the existing `-d` doc (:319-320).

### 2.6 Lone `WEEK_ID` after `week target` (`src/cli.rs`)

This uses the re-parse mechanism the plan review verified in the clap 4.6.6 harness. Parsing argv again without the token produces the exact error bare `mlm week target` gives, so it is byte-identical by construction. `classify` then renames `<ARGS>` to `<DURATION>`.

`parse` (:377-382) becomes:

```rust
/// `Cli` from raw arguments (argv[0] included).
pub fn parse(args: &[OsString]) -> Result<Cli, ParseFailure> {
    let cli = command()
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
        .map_err(|err| classify(args, err))?;
    let today = chrono::Local::now().date_naive();
    match lone_week_id_failure(args, &cli, today) {
        Some(failure) => Err(failure),
        None => Ok(cli),
    }
}
```

These new private functions go after `rename_args_context`:

```rust
/// A one-token `week target` whose token parses as a WEEK_ID is missing
/// its DURATION (relative-ids spec §3). Returns the error bare
/// `mlm week target` gives, by parsing `args` again without the token.
/// No WEEK_ID is a valid DURATION, which needs an `h` or `m` suffix.
fn lone_week_id_failure(
    args: &[OsString],
    cli: &Cli,
    today: chrono::NaiveDate,
) -> Option<ParseFailure> {
    let token = lone_target_token(cli).filter(|t| is_week_token(t, today))?;
    let rest = without_last(args, token);
    command()
        .try_get_matches_from(&rest)
        .err()
        .map(|err| classify(&rest, err))
}

/// The token of a one-token `week target`.
fn lone_target_token(cli: &Cli) -> Option<&str> {
    let Command::Week(WeekArgs {
        action: Some(WeekAction::Target(target)),
        ..
    }) = &cli.command
    else {
        return None;
    };
    match target.split() {
        (None, token) => Some(token),
        (Some(_), _) => None,
    }
}

/// `args` without its last element equal to `token`.
fn without_last(args: &[OsString], token: &str) -> Vec<OsString> {
    let mut rest = args.to_vec();
    if let Some(i) = rest.iter().rposition(|a| a == token) {
        rest.remove(i);
    }
    rest
}
```

How this behaves:

- The token always comes from argv, because clap copies the value verbatim. `rposition` removes the right element even when argv[0] or another token shares its text. The token cannot equal `week`, `w`, `target`, `-v`, `--verbose` or `--`, since none of those is a valid `WEEK_ID`.
- `.err().map(...)` has no dead branch. A re-parse that succeeded, which cannot happen, would yield `None`, and the original `Cli` would go through unchanged.
- On the remainder, `classify` runs `swapped_week_target`. It never matches, because the remainder has no four-token shape. Then `rename_args_context` gives `<DURATION>`.
- `a == token` compares `&OsString` with `&str` through std's `impl PartialEq<str> for OsString` and the blanket reference impl. If it does not compile, use `a.as_os_str() == token`.
- Scores are about 2 for `parse`, about 2 for `lone_week_id_failure`, 2 for `lone_target_token` and 2 for `without_last`.
- `parse` reads `Local::now()` once more on success. `classify` already reads it on failure. The cost is negligible. See Risk R3 for the midnight race.

Doc touch-ups:

- `WeekTargetArgs` doc (:257-271) gains one sentence: "A lone token that parses as a WEEK_ID is a missing DURATION; `parse` turns it into the same usage error."
- `ParseFailure::Clap`'s doc stays as it is.

### 2.7 What does not change

- `src/week_target.rs`, `src/week_view.rs`, `src/main.rs`, `src/lib.rs`, `src/storage.rs` and the renderers do not change.
- `swapped_week_target` and `is_week_token` are unchanged. They pick up `-1` through `parse_week_id`.
- `resolve_date` and `parse_date` are unchanged, and so is every `DATE` message.

## 3. Steps

1. **`src/date.rs` error type.** Add `Cause::OffsetOutOfRange`, the new `Display` arms, and `write_future_hint` (§2.1).
2. **`src/date.rs` resolvers.** Add `signed_digits`, `offset_count`, `resolve_week_offset` and `resolve_month_offset`. Add the first branch of `parse_week_id` and `parse_month`, and update the doc comments (§2.2, §2.3).
3. **`src/date.rs` tests.** Flip and update the existing tests, then add the new unit tests (§4.1, §4.2).
4. **`src/cli.rs`.** Apply the `week_id` attribute and doc (§2.4), the help strings (§2.5), and the lone-token conversion (§2.6). Add the unit tests (§4.3).
5. **`src/export.rs` tests.** Flip `range_errors_carry_their_arg_kind` and add the `-N` range test (§4.4).
6. **Goldens and flips in `tests/week_target_cli.rs` and `tests/export_notes_cli.rs`** (§4.5).
7. **New `tests/relative_ids_cli.rs`** (§4.6).
8. **Verify** with `cargo fmt`, `cargo build`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`. Commit through the hook, which ratchets and stages `coverage-baseline.json`. Do not add Claude attribution lines.

## 4. Test changes

### 4.1 Existing `src/date.rs` tests that change

| Test | Line | Change |
|---|---|---|
| `parse_week_id_rejects_wrong_shapes` | :743, :745, :752 | Remove `"-5"`, `"-07"` and `"-1"`. Add the still-malformed `"-1x"`, `"-1.5"`, `"--1"`, `"-"` and `"- 1"`. Re-comment `"+1"` as `// T65 (no +N)` |
| `parse_month_rejects_wrong_shapes` | :1153 | Replace `"-1"` with `"-1x"` and `"-1.5"`. `"-d"` and `"+1"` stay |
| `month_error_messages_match_the_spec` | :1207-1208 | `abc` takes the new text. `msg("-1")` becomes `msg("-0")` with `invalid MONTH "-0": expected YYYY-MM, MM or -N (N >= 1)` |

T59 (`"-07"`) and T57 (`"-5"`) become acceptance cases in §4.2.

### 4.2 New `src/date.rs` unit tests

The new tests go in a `// --- relative WEEK_ID / MONTH (relative-ids spec §3-§5) ---` block after the `resolve_date` tests. Each subsection below maps to one acceptance criterion of the changeset plan.

| Test | Input → expected |
|---|---|
| `relative_week_counts_back_from_the_current_week` | today 2026-02-12: `-1` → 2026-06, `-01` → 2026-06, `-2` → 2026-05, `-5` → 2026-02 (old T57), `-07` → 2025-52 (old T59) |
| `relative_week_crosses_the_year_boundary` | today 2026-01-05: `-1` → 2026-01, `-2` → 2025-52. Today 2026-01-11 (Sunday): `-2` → 2025-52 |
| `relative_week_lands_on_and_steps_over_week_53` | today 2027-01-04: `-1` → 2026-53, `-2` → 2026-52, `-3` → 2026-51. Today 2027-01-10: `-1` → 2026-53 |
| `relative_month_counts_back_from_the_current_month` | today 2026-02-12: `-1` → 2026-01, `-01` → 2026-01, `-2` → 2025-12, `-14` → 2024-12. Today 2026-01-15: `-1` → 2025-12. Bare-year rule check: today 2025-12-30, `-1` → 2025-11 (calendar, not ISO) |
| `relative_minus_zero_is_a_shape_error` | `-0`, `-00` give `week_err(.., Shape)` and `month_err(.., Shape)` |
| `relative_week_floor_is_1000_01` | today 2026-02-12: `-53540` → 1000-01, `-53541` → `week_err(.., OffsetOutOfRange)`. Today 2026-02-09: `-53540` → 1000-01, with a guard that `d(2026,2,9) - 53540*7 days == d(999,12,30)` |
| `relative_week_never_builds_a_week_below_1000` | For today in [2026-02-09, 2026-02-12, 2026-02-15, 1000-01-05] and N in 53530..=53560, plus `-1` from 1000-01-01: every result is `Ok(w)` with `w.iso_year() >= 1000`, or `Err` with `OffsetOutOfRange`. Nothing panics |
| `relative_month_floor_is_0000_01` | today 2026-02-12: `-24313` → `mo(0, 1)`, `-24314` → `month_err(.., OffsetOutOfRange)` |
| `relative_offset_overflow_is_out_of_range` | For both kinds: `-9999999999999999999999999` (25 digits, fails the u64 parse), `-18446744073709551615` (u64::MAX; week fails `checked_mul`, month fails `checked_sub`), `-99999999999999` (14 digits; week fails `checked_sub_days`). All give `OffsetOutOfRange` |
| `relative_offset_messages_match_the_spec` | `invalid WEEK_ID "-53541": week offset is out of range`, `invalid MONTH "-24314": month offset is out of range`. Neither contains `week number` |
| `week_shape_message_names_minus_n` | `abc` → `invalid WEEK_ID "abc": expected YYYY-WW, WW or -N (N >= 1)`. `-0` and `-1x` → the same text with their own input |
| `plus_digits_get_the_future_hint` | `+1` and `+0` for WEEK_ID → `invalid WEEK_ID "+1": expected YYYY-WW, WW or -N (N >= 1); a future week takes YYYY-WW or WW`. For MONTH → `...; a future month takes YYYY-MM or MM`. `+01` also gets the hint |
| `other_plus_inputs_get_the_plain_shape_message` | `+1x`, `+` and `+-1` give the plain shape text for both kinds |
| `date_plus_one_keeps_its_message` | `resolve_date("+1", today).unwrap_err().to_string()` == `invalid DATE "+1": expected YYYY-MM-DD or -N (N >= 1)` (plan review F6) |

### 4.3 `src/cli.rs` unit tests

The existing tests `swapped_rejects_other_shapes` (:934-954) and `parse_reports_swapped_order` (:956-970) stay as they are. `["mlm", "week", "-x", "target", "20h"]` still returns `None`.

| Test | Asserts |
|---|---|
| `week_positional_takes_negative_numbers_verbatim` | For `-1`, `-01`, `-07`, `-2`, `-0`, `-1.5`, `-1e3` and `-99999999999999999999`, `week_view_args(parse(["mlm","week",tok]))` gives `week_id == Some(tok)` and `!notes` (plan review F4) |
| `week_minus_n_works_with_flags_in_either_order` | `week -1 -n`, `week -n -1`, `w -1 --notes`, `week -v -1` and `week -1 -v` all give `week_id == Some("-1")`, with `notes` set where given |
| `week_non_numeric_hyphen_tokens_stay_usage_errors` | For `-1x`, `-x` and `--bogus`, `super::parse` fails as `ParseFailure::Clap` with exit code 2 and kind `UnknownArgument` |
| `swapped_accepts_a_relative_week` | `swapped(["mlm","week","-1","target","35h"])` == `Some("error: put WEEK_ID after target: mlm week target -1 35h")`. `super::parse` of the same argv is `SwappedWeekTarget` with exit 2 |
| `lone_week_id_after_target_is_a_missing_duration` | today 2026-10-04. For each of `[week target -1]`, `[week target 7]`, `[week target 2026-07]`, `[week target -01]`, `[w target -1]`, `[-v week target -1]`, `[week target -v 7]` and `[week target -- -1]`: build `cli` with `Cli::try_parse_from`, call `lone_week_id_failure`, and expect `Some`. Its exit code is 2, its `clap_text` kind is `MissingRequiredArgument`, and its text equals `clap_text(super::parse(os(["mlm","week","target"])))`'s text. The text contains `<DURATION>` and not `<ARGS>` |
| `other_lone_target_tokens_are_left_alone` | today 2027-06-15, a 52-week year. `lone_week_id_failure` returns `None` for `[week target 35h]`, `[week target -0]`, `[week target 53]`, `[week target -99999999999999999999]`, `[week target 2026-07 35h]`, `[week 7]` and `[status -1]` |
| `parse_turns_a_lone_week_id_into_the_usage_error` | `super::parse(os(["mlm","week","target","-1"]))` is `Clap` with exit 2. `super::parse(os(["mlm","week","target","35h"]))` is `Ok` |

### 4.4 `src/export.rs` tests

- `range_errors_carry_their_arg_kind` at :168-171 changes `Some("-1")` to `Some("-0")`, still `(WeekId, Shape)`. A new assertion checks that `args(None, Some("-9999999999999999999999999"), None)` gives `(WeekId, OffsetOutOfRange)`.
- New `minus_n_week_and_month_ranges`, with today 2026-10-24 (W43). `-w -1` gives `(2026-10-12, 2026-10-18)` and `-w -01` gives the same. `-m -1` gives `(2026-09-01, 2026-09-30)`. With today 2026-01-07, `-m -1` gives `(2025-12-01, 2025-12-31)`.

### 4.5 `tests/week_target_cli.rs` and `tests/export_notes_cli.rs`

- `WEEK_SHORT` line :229:
  ``  [WEEK_ID]  Week to show: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week``
- `week_target_help_text` line :262:
  ``  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week``
- `EXPORT_NOTES_HELP` lines :543-544:
  ```
    -w, --week <WEEK_ID>  Week to export: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week)
    -m, --month <MONTH>   Month to export: YYYY-MM (e.g. 2026-03), a bare month number (e.g. 3), or `-N` for N months before this one (e.g. `-1` = last month)
  ```
- In `VALUE_ERRORS`, lines :380, :384, :388, :392, :396 and :400 swap `expected YYYY-WW or WW` for `expected YYYY-WW, WW or -N (N >= 1)`, and `expected YYYY-MM or MM` for `expected YYYY-MM, MM or -N (N >= 1)`. The array length stays 12.
- `minus_one_week_and_month_are_value_errors` (:249-264) is replaced by `minus_one_week_and_month_are_last_week_and_month`, which runs under `on_stable_today`. Seed the day before last week's Monday, last week's Monday, its Sunday, and this week's Monday. `-w -1` prints exactly the two inside lines. Then seed the day before last month's 1st, the 1st, the last day, and this month's 1st. `-m -1` prints exactly the two inside lines. Compute last month from `today` with the same 12-to-1 wrap `bare_month_uses_the_current_calendar_year` uses (:281-288). Use separate fresh DBs for the week and month halves so the two seed sets cannot overlap.

### 4.6 New `tests/relative_ids_cli.rs`

- Module doc: "Binary-level tests for relative `WEEK_ID` and `MONTH` ids (relative-ids spec §3-§5). Drives the binary with `MLM_DB_PATH` and `TZ=UTC`. Ids that depend on the clock come from the test's own UTC today, and the test reruns when the date rolls over mid-test."
- Helpers copied from `tests/export_notes_cli.rs`: `mlm` (with `env_remove("RUST_LOG")`), `stdout`, `stderr`, `fresh` (via `mlm::db::connect_at`), `seed_on`, `ok_stdout`, `fails` and `on_stable_today`.
- Helper `target_rows` copied from `tests/panic_fixes_cli.rs:40-49`.
- Helper `last_week(today) -> WeekId` is `WeekId::from_date(today - Days::new(7))`.
- Constants:
  - `WEEK_SHAPE = "expected YYYY-WW, WW or -N (N >= 1)"`
  - `MONTH_SHAPE = "expected YYYY-MM, MM or -N (N >= 1)"`
  - `HUGE = "-9999999999999999999999999"`

Each subsection below is one acceptance criterion of the changeset plan (Task 1) and cites its spec case.

#### 4.6.1 `-N` resolves for each kind against a fixed today (spec §5 bullet 1, contract 1)

- Covered by the unit tests in §4.2 and §4.4.
- The binary checks are §4.6.6, §4.6.11 and §4.6.14.

#### 4.6.2 Week 53 landing and stepping over (spec §5 bullet 2)

- Covered by `relative_week_lands_on_and_steps_over_week_53` (§4.2).

#### 4.6.3 `-0` gives the shape errors (spec §4 table rows 1-2, §5 bullet 3)

- Unit: `relative_minus_zero_is_a_shape_error`.
- Binary, `minus_zero_is_a_shape_error`:
  - `fails(["week","-0"], 1, "error: invalid WEEK_ID \"-0\": {WEEK_SHAPE}\n")`
  - `fails(["ex","n","-w","-0"], 1, the same text)`
  - `fails(["ex","n","-m","-0"], 1, "error: invalid MONTH \"-0\": {MONTH_SHAPE}\n")`

#### 4.6.4 Offset floors and u64 overflow (spec §4 rows 3-4 and the floor bullets, §5 bullet 4)

- Unit: the floor and overflow tests in §4.2.
- Binary, `huge_offsets_are_out_of_range`:
  - `fails(["week",HUGE], 1, "error: invalid WEEK_ID \"{HUGE}\": week offset is out of range\n")`
  - `fails(["ex","n","-w",HUGE], 1, the same text)`
  - `fails(["ex","n","-m",HUGE], 1, "error: invalid MONTH \"{HUGE}\": month offset is out of range\n")`
  - `fails(["week","target",HUGE,"35h"], 1, the WEEK_ID offset text)`
  - `target_rows` is empty afterwards.

#### 4.6.5 `+` hint (spec §4 rows 5-6, §5 bullet 9; contract 4)

- Unit: `plus_digits_get_the_future_hint` and `other_plus_inputs_get_the_plain_shape_message`.
- Binary, `plus_digits_get_the_future_hint`:
  - `["week","+1"]`, `["week","target","+1","35h"]` and `["ex","n","-w","+1"]` → exit 1, `error: invalid WEEK_ID "+1": {WEEK_SHAPE}; a future week takes YYYY-WW or WW\n`
  - `["ex","n","-m","+1"]` → exit 1, `error: invalid MONTH "+1": {MONTH_SHAPE}; a future month takes YYYY-MM or MM\n`
  - `["week","+0"]` → exit 1, the WEEK_ID hint text for `+0`
  - `["ex","n","-w","+1x"]` → exit 1, `error: invalid WEEK_ID "+1x": {WEEK_SHAPE}\n` with no hint

#### 4.6.6 `mlm week -1`, `-1 -n` and `-n -1` (spec §3 bullets 3-4, §5 bullet 6; contract 7)

`week_minus_one_shows_last_week` runs under `on_stable_today` with a fresh DB:

- `ok_stdout(["week","-1"])` starts with `format!("Week {last} (")`.
- It equals `ok_stdout(["week", &last.to_string()])` exactly.
- `["week","-01"]` and `["w","-1"]` give the same stdout.

`week_minus_one_with_notes_in_either_order` runs under `on_stable_today`:

- Seed `last week` on `last.start()` and `this week` on `today`.
- `a = ok_stdout(["week","-1","-n"])`, `b = ok_stdout(["week","-n","-1"])`, `c = ok_stdout(["week",&last.to_string(),"-n"])`.
- `a == b == c`.
- `a` contains `"      - last week\n"` and does not contain `this week`.

#### 4.6.7 `mlm week -1x`, `-x` and `-1.5` (spec §3 bullet 4, §5 bullet 3; contract 6)

`week_hyphen_junk_is_a_usage_error`, for `-1x`, `-x` and `--bogus`:

- exit 2 with empty stdout
- stderr starts with `error: unexpected argument '`
- stderr contains `Usage: mlm week`
- The database file does not exist afterwards. Use a `TempDir` path that is never created, because parse fails before `db::connect` (`src/main.rs:19-22,58`).

`week_float_shapes_reach_the_parser`, for `-1.5` and `-1e3`:

- `fails(["week",tok], 1, "error: invalid WEEK_ID \"{tok}\": {WEEK_SHAPE}\n")`

#### 4.6.8 Parse tests for `-01`, `-07`, `-2` and `-0` (plan review F4)

- Covered by `week_positional_takes_negative_numbers_verbatim` (§4.3).

#### 4.6.9 `-1x` after `week target`, `-w` and `-m` (spec §3 bullet 5, §5 bullet 3)

`hyphen_junk_after_target_and_export_flags`:

- `["week","target","-1x","35h"]` → exit 1, `error: invalid WEEK_ID "-1x": {WEEK_SHAPE}\n`
- `["ex","n","-w","-1x"]` → exit 1, the same text
- `["ex","n","-m","-1x"]` → exit 1, `error: invalid MONTH "-1x": {MONTH_SHAPE}\n`
- `target_rows` is empty afterwards.

#### 4.6.10 `+1` on all three commands

- Covered by §4.6.5.

#### 4.6.11 `mlm week target -1 35h` (spec §3 bullet 6, §5 bullet 7; contract 8)

`week_target_minus_one_writes_last_week` runs under `on_stable_today` with a fresh DB:

- `ok_stdout(["week","target","20h"]) == "Week {this} target: 20h 00m\n"`
- `ok_stdout(["week","target","-1","35h"]) == "Week {last} target: 35h 00m\n"`
- `target_rows == [(last, 2100), (this, 1200)]`. The row order is by `week_id`.

#### 4.6.12 Swapped reminder (spec §3 bullet 6, §5 bullet 8)

`swapped_relative_week_gets_the_reminder`:

- `fails(["week","-1","target","35h"], 2, "error: put WEEK_ID after target: mlm week target -1 35h\n")`
- The same for `["w","-1","target","35h"]`.

#### 4.6.13 Lone `WEEK_ID` after `week target` (spec §3 bullet 7, §5 bullet 9; contract 8; plan review F5)

`lone_week_id_after_target_is_a_missing_duration`:

- `bare = mlm(["week","target"])`. Assert exit 2 and capture its stderr.
- For each of `[week target -1]`, `[week target 7]`, `[week target 2026-07]`, `[w target -1]`, `[-v week target -1]`, `[week target -v 7]` and `[week target -- -1]`:
  - exit 2, stdout empty
  - stderr == bare stderr, byte for byte
  - `target_rows` empty
- Then, under `on_stable_today` with a fresh DB, `ok_stdout(["week","target","35h"]) == "Week {this} target: 35h 00m\n"`.

`lone_non_week_tokens_keep_invalid_duration` covers changeset plan Risks, "Lone `week target` tokens that fail as `WEEK_ID`":

- `fails(["week","target","-0"], 1, "error: invalid duration \"-0\": expected a form like 20h, 33h30m, or 45m\n")`
- The same with `-99999999999999999999`.

#### 4.6.14 `DATE` `+1` keeps its message (spec §1.1; plan review F6)

`date_plus_one_has_no_hint`:

- `fails(["status","+1"], 1, "error: invalid DATE \"+1\": expected YYYY-MM-DD or -N (N >= 1)\n")`
- Also unit-tested in §4.2.

#### 4.6.15 `ex n -w -1` and `-m -1` print the right notes (spec §5 bullet 1, row 3-4 of the §3 table)

- Covered by `minus_one_week_and_month_are_last_week_and_month` in `tests/export_notes_cli.rs` (§4.5).

#### 4.6.16 Help goldens (spec §3 help text, §5 last bullet; contract 5)

- Covered by the golden edits in §4.5.
- `week_target_help_text` already asserts that `-h` and `--help` match.
- `help_lines_match_the_contract` covers `export notes -h` and `--help`.
- `short_help_goldens` covers `week -h`.

#### 4.6.17 Every other existing test passes unchanged

Run the full `cargo test`. Every other existing `-N` or `+1` assertion keeps its meaning:

- `tests/status_cli.rs:232`
- `tests/start_hint_cli.rs:198`
- `tests/overnight_visibility_cli.rs:133`
- `src/cli.rs:1032-1045`, where `-w -1` still parses into `week`

## 5. Coverage

Every new line has a test:

| Code | Covered by |
|---|---|
| `signed_digits` | both prefixes, see §4.2 |
| `offset_count` | parse failure, zero, ok |
| `resolve_week_offset` | `checked_mul` / `checked_sub_days` failure, floor failure, ok |
| `resolve_month_offset` | `checked_sub` failure, ok |
| new `Display` arms | each kind |
| `write_future_hint` | both paths |
| `lone_week_id_failure` | `None` from the filter, `None` from the token, `Some` |
| `lone_target_token` | the `else` branch via `[week 7]` and `[status -1]`, plus both `split` arms |
| `without_last` | always reached |

The or-pattern `Date` arm shares a line with the existing `-99999999999999` `DATE` test.

## 6. Risks, ambiguities, disagreements

- **R1. The `--` remainder is not verified.** `mlm week target -- -1` re-parses as `mlm week target --`. Clap should report `MissingRequiredArgument` with the fixed `override_usage`, which would be byte-identical to bare. I could not run a modified binary or clap harness from this planner's sandbox: there is no offline crate registry in the scratch box, and `cargo run` is outside my allowed commands. The tests in §4.3 and §4.6.13 decide it. If the text differs, `without_last` should also drop a `--` directly before the removed token. That is a small, local change.
- **R2. `-1x` kind.** The harness showed `mlm week -1x` printing `unexpected argument '-1' found`, which is `ErrorKind::UnknownArgument`'s text. If clap reports a different kind, relax §4.3's assertion to exit 2 plus `ParseFailure::Clap`. The binary test only checks the text prefix and `Usage:`.
- **R3. Midnight race in `parse`.** `parse` reads today for the lone-token check, and dispatch reads it again later. A lone `53` typed at the instant the ISO year changes could be judged against a different year than `week_target::apply` would use. That case cannot occur for `-N`, a bare week of 52 or less, or a full `YYYY-WW`. I accept it.
- **A1. The spec's "and not a valid `DURATION`" clause is not coded.** No `WEEK_ID` is a valid `DURATION` (`src/time.rs:70-82`), so a check would be dead logic and an uncovered branch. The doc comment of `lone_week_id_failure` states the reason. This follows the changeset plan's Risks entry.
- **A2. `+0` and `+01` get the hint**, per contract 4 ("`+0` included"). Plan review U3 was ruled a non-issue (NOTES entry 154).
- **A3. `WeekId::from_key` shares the new shape text.** Its errors only reach `CorruptRow` in `src/storage.rs:578`, which prints its own message, so no user ever sees `-N` named for a stored key.
- **A4. The new cause is named `OffsetOutOfRange`.** `DATE` offsets keep `Cause::OutOfRange`, so existing `DATE` assertions (`src/date.rs:1302,1326`) are untouched (spec §1.1).
- **D1. Minor disagreement with the changeset plan on assertion counts.** The plan says "8 assertions in `tests/export_notes_cli.rs`" take contract 2's text. Two of the eight sit in the flipped `-w -1`/`-m -1` test, which is rewritten as an acceptance test (§4.5). So 6 assertions change text, and that test is replaced. Likewise, `src/date.rs:1208` (`-1`) moves to `-0`.
- **D2. The changeset plan expects `src/week_target.rs` and `src/week_view.rs` to change in their tests.** Neither has a test that asserts `-N` is rejected (grep of both files), so this plan leaves both alone.
