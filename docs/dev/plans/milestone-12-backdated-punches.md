# Milestone 12 — Backdated punches (implementation plan)

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `-d, --date <DATE>` to `start`, `stop`, `note` (with `-N`
relative shorthand), and the same `-N` shorthand to `status`'s existing
positional `DATE`, so punches/notes can be recorded against a past date.

**Architecture:** A new core date resolver (`date::resolve_date`) handles
both absolute `YYYY-MM-DD` and `-N` shorthand, with no future-date
opinion; a thin wrapper (`date::resolve_future_checked_date`) adds the
future-date rejection that only `start`/`stop`/`note` need. `status`
calls the core resolver directly. `commands.rs` resolves the date first,
then enforces a new "TIME required when backdated" rule, before any
existing TIME/NOTE validation — preserving the existing error-precedence
convention. No `storage.rs` or schema changes: every write function
already takes an explicit `date: NaiveDate` parameter.

**Tech Stack:** Rust, `chrono` (`NaiveDate::checked_sub_days`, `Days`),
`clap` (derive), `rusqlite`.

**Spec:** `docs/dev/specs/2026-09-13-backdated-punches.md` (source of
truth — read in full before starting; also skim `docs/dev/SPEC.md` §3–§6
for the pre-existing hard-error/precedence conventions this plan
extends).

## Global Constraints

- Baseline: `v0.1.5`. `parse_date` (absolute-only) stays untouched and
  keeps being used internally by the new resolvers — it is not replaced.
- `-N` shape: `-` followed by one or more ASCII digits only, N ≥ 1
  (`-0` rejected), leading zeros tolerated (`-01` == `-1`). Anything else
  shaped-but-not-`-N` and not `YYYY-MM-DD` is `Cause::Shape`.
- `-N` overflow (parses to no representable date, or the digit run
  itself overflows `u64`) is `Cause::OutOfRange`, produced via
  `NaiveDate::checked_sub_days` — never the panicking `Sub<Days>`
  operator, and never any other panic path (`.unwrap()`/`.expect()` on
  user input is forbidden here, matching `date.rs`'s existing
  `junk_input_never_panics` convention).
- Future-date rejection (`Cause::Future`, new `Cause` variant) applies
  **only** to `start`/`stop`/`note`'s resolved date, never to `status`.
- `--date`/`-d` must be declared `allow_hyphen_values = true` on both
  `PunchArgs` and `NoteArgs` so `-N` parses as the flag's value.
- `--date`/`-d` must appear before `NOTE` text on the command line or it
  is silently absorbed into the note body (documented clap limitation,
  not fixed by this plan — locked down by a test instead).
- Error precedence, extending the existing TIME-before-NOTE rule: date
  errors → TIME errors (including the new "required" case) → NOTE
  errors. Date resolution always happens first, before any TIME
  handling.
- `TIME` becomes required on `start`/`stop` whenever the resolved date is
  not today; omitted `--date` (or `--date` resolving to today) keeps
  today's default-to-now behavior exactly as before.
- `created_at_utc` (on both `punches` and `notes`) is always the real
  wall-clock instant the command ran — never backdated, regardless of
  `--date`.
- No `storage.rs` signature changes: `insert_punch_with_note`,
  `insert_note`, `punches_for_date`, `notes_for_date` already take an
  explicit `date: NaiveDate`.
- `week` / `WEEK_ID` is out of scope — untouched by this plan.

## Parallelization

```
Batch A (fully independent, start immediately):
  Task 1 (src/date.rs core resolver)
  Task 3 (src/time.rs Required variant)
  Task 4 (src/cli.rs --date flag)

Batch B (each needs Task 1's resolve_date signature; independent of each other):
  Task 2 (src/date.rs future-checked wrapper)   -- needs Task 1
  Task 5 (src/status.rs swap to resolve_date)   -- needs Task 1

Batch C (needs Task 2's wrapper, Task 3's Required variant, and Task 4's
CLI field all landed):
  Task 6 (src/commands.rs wiring)
```

Tasks 1, 3, 4 touch disjoint files with no shared interface between
them — safe to hand to three parallel workers at once. Tasks 2 and 5
both only *consume* `date::resolve_date`'s signature from Task 1 (they
don't touch each other's files: `date.rs` vs `status.rs`), so once
Task 1 lands they too can run in parallel. Task 6 is the integration
point — it calls `resolve_future_checked_date` (Task 2), matches on
`TimeParseError::Required` (Task 3), and reads `args.date` (Task 4), so
it must wait for all three.

---

### Task 1: `date.rs` — core `resolve_date` resolver

**Files:**
- Modify: `src/date.rs` (add `Cause::OutOfRange`'s reuse is unchanged;
  add the new function near `parse_date`, roughly after line 113)
- Test: `src/date.rs`'s existing `#[cfg(test)] mod tests` (same file)

**Interfaces:**
- Consumes: existing `DateWeekError`, `ArgKind::Date`, `Cause::Shape`,
  `Cause::OutOfRange`, `parse_date(s: &str) -> Result<NaiveDate, DateWeekError>`
  (all already in `src/date.rs`, read above).
- Produces: `pub fn resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
  — used directly by Task 5 (`status.rs`) and by Task 2's wrapper.
  Also produces a new `Display` arm distinguishing `(ArgKind::Date,
  Cause::OutOfRange)` from the existing generic `(_, Cause::OutOfRange)`
  arm (which stays for `WeekId`'s "week number must be 1 or greater").

- [ ] **Step 1: Write the failing tests**

Add to `src/date.rs`'s `mod tests` (near the other `parse_date`/
`Cause` tests):

```rust
    // --- resolve_date (backdated-punches spec §2, §5) ------------------

    #[test]
    fn resolve_date_passes_through_absolute_dates() {
        let today = d(2026, 2, 12);
        assert_eq!(resolve_date("2026-02-12", today).unwrap(), d(2026, 2, 12));
        assert_eq!(resolve_date("2026-01-05", today).unwrap(), d(2026, 1, 5));
    }

    #[test]
    fn resolve_date_accepts_relative_shorthand() {
        let today = d(2026, 2, 12);
        assert_eq!(resolve_date("-1", today).unwrap(), d(2026, 2, 11));
        assert_eq!(resolve_date("-7", today).unwrap(), d(2026, 2, 5));
        assert_eq!(resolve_date("-30", today).unwrap(), d(2026, 1, 13));
    }

    #[test]
    fn resolve_date_shorthand_crosses_a_leap_year_boundary() {
        // 2027-03-01 minus 1 day is 2026-02-28 (2026 is not a leap year).
        let today = d(2027, 3, 1);
        assert_eq!(resolve_date("-1", today).unwrap(), d(2026, 2, 28));
    }

    #[test]
    fn resolve_date_zero_padded_shorthand_matches_unpadded() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_date("-01", today).unwrap(),
            resolve_date("-1", today).unwrap()
        );
        assert_eq!(
            resolve_date("-007", today).unwrap(),
            resolve_date("-7", today).unwrap()
        );
    }

    #[test]
    fn resolve_date_rejects_minus_zero_as_shape() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_date("-0", today).unwrap_err(),
            date_err("-0", Cause::Shape)
        );
        assert_eq!(
            resolve_date("-00", today).unwrap_err(),
            date_err("-00", Cause::Shape)
        );
    }

    #[test]
    fn resolve_date_rejects_malformed_shorthand_as_shape() {
        let today = d(2026, 2, 12);
        for input in ["-1.5", "-abc", "+1", "- 1", "-1 ", "-1\n", "--1"] {
            assert_eq!(
                resolve_date(input, today).unwrap_err(),
                date_err(input, Cause::Shape),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn resolve_date_absurdly_large_n_is_out_of_range_not_panic() {
        let today = d(2026, 2, 12);
        for input in [
            "-999999999999999999999999", // 24 digits: overflows the u64 parse itself
            "-99999999999999",           // 14 digits: parses as u64, overflows checked_sub_days
            "-18446744073709551615",     // u64::MAX exactly: parses as u64 (the largest
                                          // value that can), still overflows
                                          // checked_sub_days -- exercises the boundary
                                          // right at the parse/arithmetic seam rather
                                          // than deep in unrepresentable territory.
        ] {
            let err = resolve_date(input, today).unwrap_err();
            assert_eq!(err, date_err(input, Cause::OutOfRange), "input {input}");
        }
    }

    #[test]
    fn resolve_date_today_and_future_are_both_accepted_by_the_core_resolver() {
        let today = d(2026, 2, 12);
        assert_eq!(resolve_date("2026-02-12", today).unwrap(), today);
        assert_eq!(resolve_date("2026-02-13", today).unwrap(), d(2026, 2, 13));
        assert_eq!(resolve_date("2030-01-01", today).unwrap(), d(2030, 1, 1));
    }

    #[test]
    fn resolve_date_out_of_range_message_is_date_specific() {
        let today = d(2026, 2, 12);
        let msg = resolve_date("-99999999999999", today)
            .unwrap_err()
            .to_string();
        assert!(!msg.contains("week number"), "{msg}");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib date:: -- resolve_date`
Expected: FAIL with "cannot find function `resolve_date` in this scope"
(or similar) for every new test.

- [ ] **Step 3: Add the `Cause::OutOfRange` Date-specific `Display` arm**

In `src/date.rs`'s `impl std::fmt::Display for DateWeekError`, insert a
new arm **before** the existing generic `(_, Cause::OutOfRange)` arm
(match arms are checked top-down, so ordering matters):

```rust
            (ArgKind::Date, Cause::OutOfRange) => write!(f, "date offset is out of range"),
            (_, Cause::OutOfRange) => write!(f, "week number must be 1 or greater"),
```

- [ ] **Step 4: Implement `resolve_date`**

Add after `parse_date` (after line 113, before `format_date`):

```rust
/// Core `DATE` resolver (backdated-punches spec §2, §4): accepts the
/// existing absolute `YYYY-MM-DD` grammar, or a relative `-N` shorthand
/// (`-` + one or more ASCII digits, N >= 1, leading zeros tolerated)
/// meaning N days before `today`. No future-date opinion — `status`
/// calls this directly; `start`/`stop`/`note` go through
/// `resolve_future_checked_date` instead, which adds that check.
pub fn resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError> {
    let err = |cause| DateWeekError::new(ArgKind::Date, s, cause);
    if let Some(digits) = s.strip_prefix('-') {
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            let n: u64 = match digits.parse() {
                Ok(n) => n,
                // The digit run itself doesn't fit in a u64 -- as
                // unrepresentable as any other offset chrono can't
                // handle, so the same Cause applies.
                Err(_) => return Err(err(Cause::OutOfRange)),
            };
            if n == 0 {
                return Err(err(Cause::Shape));
            }
            return today
                .checked_sub_days(Days::new(n))
                .ok_or_else(|| err(Cause::OutOfRange));
        }
    }
    parse_date(s)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib date::`
Expected: PASS for all new tests and all pre-existing `date.rs` tests
(no regressions).

- [ ] **Step 6: Commit**

```bash
git add src/date.rs
git commit -m "feat(date): add resolve_date core resolver for -N shorthand"
```

---

### Task 2: `date.rs` — future-checked wrapper + `Cause::Future`

**Depends on:** Task 1 (`resolve_date`, `Cause`, `DateWeekError`).

**Files:**
- Modify: `src/date.rs` (the `Cause` enum around line 28, its `Display`
  impl around line 63, and a new function next to `resolve_date`)
- Test: same file's `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
  from Task 1.
- Produces: `pub fn resolve_future_checked_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
  — consumed by Task 6 (`commands.rs`). Also produces the new
  `Cause::Future` variant, which downstream code may match on via
  `DateWeekError.cause` (Task 6 doesn't need to match on it directly —
  it only needs the `Result` to be `Err` — but it is part of this
  task's public surface).

- [ ] **Step 1: Write the failing tests**

```rust
    // --- resolve_future_checked_date ------------------------------------

    #[test]
    fn future_checked_rejects_future_absolute_date() {
        let today = d(2026, 2, 12);
        let err = resolve_future_checked_date("2026-02-13", today).unwrap_err();
        assert_eq!(err, date_err("2026-02-13", Cause::Future));
    }

    #[test]
    fn future_checked_shorthand_is_never_mistaken_for_future() {
        // A positive N-days-before-today shorthand can never itself
        // resolve to the future, so exercise this via an already-future
        // *absolute* date fed alongside a shorthand test of the boundary:
        // -N always resolves to today or earlier, so there is no -N
        // input that reaches the future branch -- this test instead
        // pins that -1/-N shorthand is always accepted (never mistakenly
        // rejected as "future").
        let today = d(2026, 2, 12);
        assert!(resolve_future_checked_date("-1", today).is_ok());
        assert!(resolve_future_checked_date("-1000", today).is_ok());
    }

    #[test]
    fn future_checked_accepts_today_and_past() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_future_checked_date("2026-02-12", today).unwrap(),
            today
        );
        assert_eq!(
            resolve_future_checked_date("2026-01-05", today).unwrap(),
            d(2026, 1, 5)
        );
        assert_eq!(resolve_future_checked_date("-1", today).unwrap(), d(2026, 2, 11));
    }

    #[test]
    fn future_checked_still_propagates_shape_and_range_errors() {
        let today = d(2026, 2, 12);
        assert_eq!(
            resolve_future_checked_date("abc", today).unwrap_err(),
            date_err("abc", Cause::Shape)
        );
        assert_eq!(
            resolve_future_checked_date("-0", today).unwrap_err(),
            date_err("-0", Cause::Shape)
        );
    }

    #[test]
    fn future_error_message_says_future() {
        let today = d(2026, 2, 12);
        let msg = resolve_future_checked_date("2026-02-13", today)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("future"), "{msg}");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib date:: -- future_checked`
Expected: FAIL — `resolve_future_checked_date`/`Cause::Future` don't
exist yet.

- [ ] **Step 3: Add `Cause::Future` and its `Display` arm**

In the `Cause` enum (around line 28):

```rust
pub enum Cause {
    Shape,
    OutOfRange,
    NoSuchCalendarDate,
    NoSuchIsoWeek { iso_year: i32, weeks_in_year: u32 },
    /// A resolved `DATE` is later than today's local calendar date.
    /// Only ever paired with `ArgKind::Date` (backdated-punches spec §3).
    Future,
}
```

In `Display for DateWeekError`, add an arm (order relative to the other
arms doesn't matter here, since `Future` doesn't overlap any other
pattern):

```rust
            (_, Cause::Future) => write!(f, "date is in the future"),
```

- [ ] **Step 4: Implement `resolve_future_checked_date`**

Add directly after `resolve_date`:

```rust
/// Thin wrapper around [`resolve_date`] adding the future-date rejection
/// `start`/`stop`/`note` need (backdated-punches spec §3). `status` uses
/// `resolve_date` directly and keeps accepting future dates.
pub fn resolve_future_checked_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError> {
    let date = resolve_date(s, today)?;
    if date > today {
        return Err(DateWeekError::new(ArgKind::Date, s, Cause::Future));
    }
    Ok(date)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib date::`
Expected: PASS for all tests in the module, no regressions.

- [ ] **Step 6: Commit**

```bash
git add src/date.rs
git commit -m "feat(date): add resolve_future_checked_date + Cause::Future"
```

---

### Task 3: `time.rs` — `TimeParseError::Required`

**Files:**
- Modify: `src/time.rs` (the `TimeParseError` enum around line 118 and
  its `Display` impl around line 139)
- Test: same file's `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing new (existing `TimeParseError`, already
  `std::error::Error`).
- Produces: `TimeParseError::Required` variant — consumed by Task 6
  (`commands.rs`), which returns it (via `anyhow`'s `?`/`.into()`) when
  `TIME` is omitted on a backdated `start`/`stop`.

- [ ] **Step 1: Write the failing test**

Add to `src/time.rs`'s `mod tests`:

```rust
    #[test]
    fn required_variant_renders_the_backdated_reason() {
        let err = TimeParseError::Required;
        let msg = err.to_string();
        assert_eq!(
            msg,
            "TIME is required when --date targets a day other than today"
        );
        assert!(msg.is_ascii());
        assert!(!msg.contains('\n'));
    }

    #[test]
    fn required_variant_is_distinguishable_from_the_other_variants() {
        assert_ne!(TimeParseError::Required, TimeParseError::InvalidFormat("x".to_string()));
        assert_ne!(
            TimeParseError::Required,
            TimeParseError::OutOfRange {
                input: "x".to_string(),
                hour: 1,
                minute: 1
            }
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib time:: -- required_variant`
Expected: FAIL with "no variant `Required` on `TimeParseError`".

- [ ] **Step 3: Add the variant and its `Display` arm**

In `pub enum TimeParseError` (around line 118):

```rust
pub enum TimeParseError {
    InvalidFormat(String),
    OutOfRange {
        input: String,
        hour: u32,
        minute: u32,
    },
    /// No `TIME` given where one was required: `--date` resolved to a
    /// day other than today, so "default to now" has no meaning
    /// (backdated-punches spec §3). This is a "missing input" error,
    /// not a parse failure -- it carries no offending string.
    Required,
}
```

In `impl std::fmt::Display for TimeParseError` (around line 139), add:

```rust
            Self::Required => write!(
                f,
                "TIME is required when --date targets a day other than today"
            ),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib time::`
Expected: PASS, no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/time.rs
git commit -m "feat(time): add TimeParseError::Required for backdated punches"
```

---

### Task 4: `cli.rs` — `-d, --date` on `PunchArgs`/`NoteArgs`

**Files:**
- Modify: `src/cli.rs` (`PunchArgs` around line 48, `NoteArgs` around
  line 65)
- Test: same file's `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: nothing from Tasks 1–3 (clap-level plumbing only; the field
  is a plain `Option<String>`, not yet parsed by `date::resolve_*`).
- Produces: `PunchArgs.date: Option<String>` and `NoteArgs.date: Option<String>`
  — consumed by Task 6 (`commands.rs`), which resolves them via
  `date::resolve_future_checked_date`.

- [ ] **Step 1: Write the failing tests**

Add to `src/cli.rs`'s `mod tests`:

```rust
    // --- backdated-punches: --date/-d plumbing --------------------------

    #[test]
    fn start_accepts_long_and_short_date_flag_with_hyphen_value() {
        let a = start_args(parse(&["mlm", "start", "--date", "-1", "9:00"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("-1"));
        assert_eq!(a.time.as_deref(), Some("9:00"));

        let a = start_args(parse(&["mlm", "start", "-d", "-1", "9:00"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("-1"));
    }

    #[test]
    fn start_date_flag_accepts_absolute_date_before_or_after_time() {
        let a = start_args(parse(&["mlm", "start", "--date", "2026-01-05", "9:00"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("2026-01-05"));
        assert_eq!(a.time.as_deref(), Some("9:00"));

        let a = start_args(parse(&["mlm", "start", "9:00", "--date", "2026-01-05"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("2026-01-05"));
        assert_eq!(a.time.as_deref(), Some("9:00"));
    }

    #[test]
    fn start_omitted_date_flag_is_none() {
        let a = start_args(parse(&["mlm", "start", "9:00"]).unwrap());
        assert_eq!(a.date, None);
    }

    #[test]
    fn note_accepts_date_flag_before_body() {
        let cli = parse(&["mlm", "note", "--date", "-2", "fixed a bug"]).unwrap();
        match cli.command {
            Command::Note(a) => {
                assert_eq!(a.date.as_deref(), Some("-2"));
                assert_eq!(a.body, vec!["fixed".to_string(), "a".to_string(), "bug".to_string()]);
            }
            other => panic!("expected Command::Note, got {other:?}"),
        }
    }

    /// Locks down the §2.1 clap footgun: once `--date` appears after
    /// NOTE tokens have started, clap's trailing_var_arg no longer
    /// re-scans for named flags, so `--date -1` is silently absorbed
    /// into the note body instead of being parsed as the date flag.
    /// This test exists so a future clap upgrade or arg refactor that
    /// changes this behavior gets caught, not silently shipped.
    #[test]
    fn date_flag_after_note_text_is_absorbed_into_the_note_body() {
        let a = start_args(
            parse(&[
                "mlm", "start", "9:00", "kicked", "off", "migration", "--date", "-1",
            ])
            .unwrap(),
        );
        assert_eq!(a.date, None, "the flag was swallowed, not parsed");
        assert_eq!(
            a.note,
            vec![
                "kicked".to_string(),
                "off".to_string(),
                "migration".to_string(),
                "--date".to_string(),
                "-1".to_string(),
            ]
        );
    }

    /// Same §2.1 clap footgun as `date_flag_after_note_text_is_absorbed_into_the_note_body`,
    /// but for `note`: `NoteArgs.body` has the identical
    /// `trailing_var_arg = true, allow_hyphen_values = true` shape as
    /// `PunchArgs.note`, so it carries the identical risk and needs its
    /// own lock-down rather than relying on `start`'s test to stand in
    /// for it.
    #[test]
    fn note_date_flag_after_body_text_is_absorbed_into_the_note_body() {
        let cli = parse(&["mlm", "note", "fixed", "a", "bug", "--date", "-2"]).unwrap();
        match cli.command {
            Command::Note(a) => {
                assert_eq!(a.date, None, "the flag was swallowed, not parsed");
                assert_eq!(
                    a.body,
                    vec![
                        "fixed".to_string(),
                        "a".to_string(),
                        "bug".to_string(),
                        "--date".to_string(),
                        "-2".to_string(),
                    ]
                );
            }
            other => panic!("expected Command::Note, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib cli::`
Expected: FAIL to compile — `PunchArgs`/`NoteArgs` have no `date` field
yet.

- [ ] **Step 3: Add the `date` field and doc comments**

Replace `PunchArgs` (around line 48–61):

```rust
#[derive(Args, Debug)]
pub struct PunchArgs {
    /// Time of day (HH:MM, HHMM or HH, 24h). Defaults to now when
    /// recording for today; required when `--date` targets another day.
    #[arg(value_name = "TIME")]
    pub time: Option<String>,

    /// Date to record against: YYYY-MM-DD, or `-N` for N days before
    /// today (e.g. `-1` = yesterday). Defaults to today. Must come
    /// before NOTE text on the command line, or it is silently absorbed
    /// into the note body instead of being parsed as this flag -- see
    /// docs/dev/specs/2026-09-13-backdated-punches.md §2.1.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,

    /// Optional work-log note recorded alongside the punch, against
    /// today or, with `--date`, the resolved date.
    #[arg(
        value_name = "NOTE",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub note: Vec<String>,
}
```

Replace `NoteArgs` (around line 65–76):

```rust
#[derive(Args, Debug)]
pub struct NoteArgs {
    /// Date to record against: YYYY-MM-DD, or `-N` for N days before
    /// today (e.g. `-1` = yesterday). Defaults to today. Must come
    /// before NOTE text on the command line -- see
    /// docs/dev/specs/2026-09-13-backdated-punches.md §2.1.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,

    /// Work-log note text, against today or, with `--date`, the resolved
    /// date.
    #[arg(
        value_name = "NOTE",
        required = true,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub body: Vec<String>,
}
```

- [ ] **Step 3a: Update `Status`'s `DATE` doc comment**

`status`'s positional `DATE` also gains `-N` shorthand support (Task 5),
but its clap doc comment (the `--help` text) still only mentions the
absolute form. Update it here, since Task 4 is the task that owns
`cli.rs` edits (Task 5 only touches `status.rs`).

In `Command::Status` (around line 34–39), change:

```rust
    /// Show a date's stints, notes and totals (defaults to today).
    #[command(visible_alias = "d")]
    Status {
        /// Date to show, YYYY-MM-DD. Defaults to today.
        date: Option<String>,
    },
```

to:

```rust
    /// Show a date's stints, notes and totals (defaults to today).
    #[command(visible_alias = "d")]
    Status {
        /// Date to show: YYYY-MM-DD, or `-N` for N days before today
        /// (e.g. `-1` = yesterday). Defaults to today.
        date: Option<String>,
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib cli::`
Expected: PASS, no regressions (all pre-existing `cli.rs` tests still
green, including `parse_start_variants`, `parse_note_joins_tokens`,
`note_can_start_with_hyphen`).

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add -d/--date to start/stop/note"
```

---

### Task 5: `status.rs` — swap to `resolve_date`

**Depends on:** Task 1 (`date::resolve_date`).

**Files:**
- Modify: `src/status.rs:317` (inside `resolve()`)
- Test: same file's `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `date::resolve_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
  from Task 1.
- Produces: nothing new for other tasks — this is a leaf change. `status`'s
  public `resolve`/`run` signatures are unchanged (`date_arg: Option<&str>`).

- [ ] **Step 1: Write the failing tests**

Add to `src/status.rs`'s `mod tests` (near `resolve_f10_past_date_different_closed_week`):

```rust
    #[test]
    fn resolve_shorthand_matches_equivalent_absolute_date() {
        let conn = test_db();
        storage::insert_punch(&conn, PunchKind::Start, d(2026, 1, 5), t(8, 30), &Local)
            .expect("insert");
        storage::insert_punch(&conn, PunchKind::End, d(2026, 1, 5), t(14, 45), &Local)
            .expect("insert");

        // now_thu_1800() is 2026-02-12; 2026-01-05 is 38 days earlier.
        let via_absolute = resolve(Some("2026-01-05"), now_thu_1800(), &conn).expect("resolve");
        let via_shorthand = resolve(Some("-38"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(via_absolute.header, via_shorthand.header);
        assert_eq!(
            via_absolute.day_total_minutes,
            via_shorthand.day_total_minutes
        );
        assert_eq!(via_shorthand.header, "Mon 2026-01-05");
    }

    #[test]
    fn resolve_future_absolute_date_still_succeeds_and_renders_empty() {
        let conn = test_db();
        // now_thu_1800() is 2026-02-12; this stays permissive, unlike
        // start/stop/note's future-date rejection.
        let view = resolve(Some("2026-03-01"), now_thu_1800(), &conn).expect("resolve");
        assert_eq!(view.header, date::format_date_with_weekday(d(2026, 3, 1)));
        assert!(view.stints.is_empty());
        assert!(view.notes.is_empty());
        assert_eq!(view.day_total_minutes, 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib status:: -- resolve_shorthand_matches`
Expected: `resolve_shorthand_matches_equivalent_absolute_date` FAILs
(malformed-date error from `-38` since `parse_date` doesn't accept it
yet); `resolve_future_absolute_date_still_succeeds_and_renders_empty`
already passes (no behavior change needed for that one — it's a
regression guard, confirm it's green both before and after Step 3).

- [ ] **Step 3: Swap the resolver call**

In `resolve()` (around line 315–318), change:

```rust
    let target_date = match date_arg {
        None => today,
        Some(s) => date::parse_date(s)?,
    };
```

to:

```rust
    let target_date = match date_arg {
        None => today,
        Some(s) => date::resolve_date(s, today)?,
    };
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib status::`
Expected: PASS, no regressions (including
`resolve_malformed_date_is_a_hard_error`, which must keep failing on
`"2026-02-30"`/`"13/02/2026"` — both still shape/calendar errors under
`resolve_date`).

- [ ] **Step 5: Commit**

```bash
git add src/status.rs
git commit -m "feat(status): accept -N shorthand via date::resolve_date"
```

---

### Task 6: `commands.rs` — wire date resolution into `punch()`/`note()`

**Depends on:** Task 2 (`resolve_future_checked_date`), Task 3
(`TimeParseError::Required`), Task 4 (`PunchArgs.date`/`NoteArgs.date`).

**Files:**
- Modify: `src/commands.rs` (`note()` around line 30, `punch()` around
  line 45)
- Test: same file's `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes:
  - `date::resolve_future_checked_date(s: &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>`
    (Task 2)
  - `TimeParseError::Required` (Task 3)
  - `args.date: Option<String>` on both `PunchArgs` and `NoteArgs`
    (Task 4)
  - Unchanged: `storage::insert_punch_with_note(conn, kind, date, time_of_day, &Local, note_text, now_utc)`,
    `storage::insert_note(conn, date, &body, now_utc)` (both already
    take `date: NaiveDate` — no signature change needed).
  - Test-only: `status::resolve(date_arg: Option<&str>, now: DateTime<Local>, conn: &Connection) -> anyhow::Result<StatusView>`
    (already in `src/status.rs`, unmodified by this plan except by
    Task 5) — used by this task's own retroactive-recompute test only,
    not by `commands.rs`'s non-test code.
- Produces: nothing new for other tasks — `start`/`stop`/`note`'s
  public signatures (`fn start/stop/note(conn, now, args) -> anyhow::Result<()>`)
  are unchanged.

- [ ] **Step 1: Write the failing tests**

Add to `src/commands.rs`'s `mod tests`. First, four small helpers next
to the existing `today()`/`punches()`/`notes()` fixtures. `d()` mirrors
the identical fixture already in `date.rs`'s and `status.rs`'s own test
modules (same name, same signature, same body — kept consistent across
all three rather than reinvented here); `stop_args()` mirrors
`punch_args()` but parses a `stop` command line instead of `start`, so
tests that need backdated `stop` args (this task also needs at least
one — see below) don't have to hand-roll the `Cli::try_parse_from`
match arm inline the way the pre-existing `stop_all_shapes` test does:

```rust
    /// A known date; panics on a typo in the test itself. (Same
    /// fixture as `date.rs`'s and `status.rs`'s `mod tests`.)
    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("test fixture is a real date")
    }

    fn stop_args(argv: &[&str]) -> PunchArgs {
        // argv excludes "mlm" and the subcommand name.
        let mut full = vec!["mlm", "stop"];
        full.extend_from_slice(argv);
        match Cli::try_parse_from(full).expect("parse").command {
            crate::cli::Command::Stop(a) => a,
            other => panic!("expected Stop, got {other:?}"),
        }
    }

    fn punches_for(conn: &Connection, date: NaiveDate) -> Vec<Punch> {
        punches_for_date(conn, date).expect("read punches")
    }

    fn notes_for(conn: &Connection, date: NaiveDate) -> Vec<Note> {
        notes_for_date(conn, date).expect("read notes")
    }
```

Then the new tests:

```rust
    // --- backdated punches (backdated-punches spec §5) ------------------

    // Successful backdated punch with explicit TIME.
    #[test]
    fn backdated_start_with_explicit_time_succeeds() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-1", "09:00"]),
        )
        .expect("ok");
        let yesterday = d(2026, 2, 11);
        let p = punches_for(&conn, yesterday);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].date, yesterday);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 11, 9, 0, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(punches(&conn).is_empty(), "nothing written against today");
    }

    // Missing TIME with a backdated --date is rejected; nothing written.
    #[test]
    fn backdated_start_without_time_is_rejected() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["--date", "-1"]));
        let err = r.expect_err("expected error");
        assert!(
            matches!(
                err.downcast_ref::<crate::time::TimeParseError>(),
                Some(crate::time::TimeParseError::Required)
            ),
            "expected TimeParseError::Required, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 11)).is_empty());
        assert!(punches(&conn).is_empty());
    }

    // Future --date is rejected; nothing written.
    #[test]
    fn future_dated_start_is_rejected() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["--date", "2026-02-13", "09:00"]));
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected DateWeekError, got {err:?}"
        );
        assert!(punches_for(&conn, d(2026, 2, 13)).is_empty());
    }

    // --date omitted behaves exactly as before (regression guard).
    #[test]
    fn omitted_date_flag_behaves_like_before() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["09:00"])).expect("ok");
        let p = punches(&conn);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].date, today());
    }

    // An anomaly-producing pairing on a backdated date surfaces the same
    // as it would for today: two starts, no end, is still just two
    // "start" rows for that date -- anomaly *rendering* is status's job,
    // this only proves storage.rs's per-date bookkeeping isn't disturbed
    // by a backdated date.
    #[test]
    fn backdated_anomaly_producing_pairing_is_stored_like_today() {
        let mut conn = test_db();
        start(&mut conn, fixed_now(), &punch_args(&["--date", "-1", "09:00"])).expect("ok");
        start(&mut conn, fixed_now(), &punch_args(&["--date", "-1", "10:00"])).expect("ok");
        let p = punches_for(&conn, d(2026, 2, 11));
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|x| x.kind == PunchKind::Start));
    }

    // An inline NOTE alongside a backdated punch lands on the *resolved*
    // date, not today.
    #[test]
    fn backdated_inline_note_lands_on_resolved_date() {
        let mut conn = test_db();
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-1", "09:00", "kicked off migration"]),
        )
        .expect("ok");
        let yesterday = d(2026, 2, 11);
        let n = notes_for(&conn, yesterday);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].date, yesterday);
        assert_eq!(n[0].body, "kicked off migration");
        assert!(notes(&conn).is_empty(), "nothing written against today");
    }

    // A simultaneous bad --date + bad/missing TIME reports the date
    // error (precedence: date -> TIME -> NOTE).
    #[test]
    fn bad_date_precedes_bad_time_error() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["--date", "not-a-date", "25:00"]));
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected the date error to win, got {err:?}"
        );
        assert!(err.downcast_ref::<crate::time::TimeParseError>().is_none());
        assert!(punches(&conn).is_empty());
    }

    #[test]
    fn bad_date_precedes_missing_time_error() {
        let mut conn = test_db();
        let r = start(&mut conn, fixed_now(), &punch_args(&["--date", "not-a-date"]));
        let err = r.expect_err("expected error");
        assert!(
            err.downcast_ref::<crate::date::DateWeekError>().is_some(),
            "expected the date error to win over the missing-TIME error, got {err:?}"
        );
    }

    // `stop` shares `punch()` with `start`, but every backdated test
    // above exercises it only via `start(...)`. `stop_all_shapes` (the
    // pre-existing test this mirrors) is the only place `stop` itself is
    // exercised at all, and it never touches `--date` -- so nothing
    // today actually proves the shared helper resolves `--date`
    // correctly on the `stop` path specifically, only that it compiles
    // against `PunchArgs`. This closes that gap.
    #[test]
    fn backdated_stop_with_explicit_time_succeeds() {
        let mut conn = test_db();
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "-1", "17:30"]),
        )
        .expect("ok");
        let yesterday = d(2026, 2, 11);
        let p = punches_for(&conn, yesterday);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].kind, PunchKind::End);
        assert_eq!(p[0].date, yesterday);
        assert_eq!(
            p[0].at_utc,
            Local
                .with_ymd_and_hms(2026, 2, 11, 17, 30, 0)
                .unwrap()
                .with_timezone(&Utc)
        );
        assert!(punches(&conn).is_empty(), "nothing written against today");
    }

    // --- backdated note (backdated-punches spec §5) ---------------------

    #[test]
    fn backdated_note_stored_against_resolved_date() {
        let mut conn = test_db();
        note(
            &mut conn,
            fixed_now(),
            &note_args(&["--date", "-3", "fixed a bug"]),
        )
        .expect("ok");
        let target = d(2026, 2, 9);
        let n = notes_for(&conn, target);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].date, target);
        assert_eq!(n[0].body, "fixed a bug");
        // created_at_utc still reflects real now, not the backdated date.
        assert_eq!(n[0].created_at_utc, fixed_now().with_timezone(&Utc));
        assert!(notes(&conn).is_empty(), "nothing written against today");
    }

    // --- retroactive week recompute (backdated-punches spec §3.1) -------
    //
    // §3.1 calls this the whole point of the feature: backdating a punch
    // into an already-"closed" past week must change that week's, and a
    // later week's, owed/carry figures on the next status view. Nothing
    // above proves this -- every test up to here only inspects rows via
    // `punches_for`/`notes_for`, never a computed week figure. This
    // drives `status::resolve` (src/status.rs) directly against the same
    // in-memory connection `commands::start`/`stop` just wrote to, so it
    // is a genuine end-to-end check of storage -> week accounting, not a
    // restatement of either module's own unit tests.
    //
    // Fixture: `fixed_now()` is 2026-02-12 (Thursday, ISO week 2026-07).
    // 2026-01-27 is a Tuesday in ISO week 2026-05 (Mon 2026-01-26 .. Sun
    // 2026-02-01) and is exactly 16 days before `fixed_now()`'s date, so
    // `--date -16` reaches the same day as `--date 2026-01-27`. ISO week
    // 2026-06 (Mon 2026-02-02 .. Sun 2026-02-08) sits between 2026-05 and
    // the current week 2026-07, so it is already "closed" (in the past,
    // per `render::week_framing`) both before and after the backdated
    // punch lands -- exactly the "already-closed past week" §3.1
    // describes, not the current week's own live-updating figure.
    #[test]
    fn backdated_punch_retroactively_changes_a_later_closed_weeks_owed() {
        let mut conn = test_db();

        // Seed 4h in week 2026-05 (2026-01-27, 09:00-13:00).
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "2026-01-27", "09:00"]),
        )
        .expect("ok");
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "2026-01-27", "13:00"]),
        )
        .expect("ok");

        // Before the fix: week 2026-05 worked 240m against a 2400m
        // default target, so it owes 2160m and carries -2160m forward
        // through the idle week 2026-06 (which itself then owes its own
        // full 2400m on top): 2160 + 2400 = 4560m = 76h 00m.
        let before =
            crate::status::resolve(Some("2026-02-02"), fixed_now(), &conn).expect("resolve");
        assert!(
            before.week_line.contains("Total still owed: 76h 00m"),
            "before: {}",
            before.week_line
        );

        // A forgotten 2h stint is now backdated into week 2026-05 via
        // the -N shorthand (2026-01-27 is 16 days before fixed_now()'s
        // date).
        start(
            &mut conn,
            fixed_now(),
            &punch_args(&["--date", "-16", "14:00"]),
        )
        .expect("ok");
        stop(
            &mut conn,
            fixed_now(),
            &stop_args(&["--date", "-16", "16:00"]),
        )
        .expect("ok");

        // After: week 2026-05 now worked 360m, owes 2040m; week 2026-06
        // owes 2040 + 2400 = 4440m = 74h 00m -- 2 hours less, exactly the
        // backdated stint's length, with no `status`/`week` action taken
        // beyond re-reading the same view.
        let after =
            crate::status::resolve(Some("2026-02-02"), fixed_now(), &conn).expect("resolve");
        assert!(
            after.week_line.contains("Total still owed: 74h 00m"),
            "after: {}",
            after.week_line
        );
        assert_ne!(before.week_line, after.week_line);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib commands::`
Expected: FAIL TO COMPILE, not a runtime test failure — Step 1's new
tests read `args.date` on `PunchArgs`/`NoteArgs` (added by Task 4, a
declared dependency of this task) and match on
`TimeParseError::Required` (added by Task 3, also a dependency), but
`punch()`/`note()` in `commands.rs`'s non-test code have not been
updated yet to resolve `--date` at all — that only happens in Step 3
below. There is no intermediate state in which the crate compiles and
the new assertions merely fail at runtime: every new test in this batch
depends on behavior Step 3 introduces. (`d()`, `stop_args()`,
`punches_for()`, `notes_for()` — all added as part of Step 1 itself —
compile cleanly on their own; they only reference pre-existing
`storage`/`cli`/`chrono` items.)

- [ ] **Step 3: Wire date resolution into `note()` and `punch()`**

Update the imports at the top of `src/commands.rs`:

```rust
use crate::cli::{NoteArgs, PunchArgs};
use crate::date::resolve_future_checked_date;
use crate::storage::{self, PunchKind};
use crate::time::{TimeParseError, parse_time};
```

Replace `note()`:

```rust
/// Record a standalone work-log note (SPEC §3.4), against today or, with
/// `--date`, a resolved past date (backdated-punches spec §4).
pub fn note(conn: &mut Connection, now: DateTime<Local>, args: &NoteArgs) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = match &args.date {
        Some(s) => resolve_future_checked_date(s, today)?,
        None => today,
    };
    let body = args.body.join(" ");
    storage::insert_note(conn, target_date, &body, now.with_timezone(&Utc))?;
    Ok(())
}
```

Replace the `punch()` helper:

```rust
/// Shared `start`/`stop` implementation, parameterised by punch kind.
///
/// Validation order (backdated-punches spec §3, extending §4.1's
/// existing rule): date resolution happens first (a malformed or
/// future `--date` is a hard error here, before TIME is even looked
/// at), then TIME is parsed -- required when the resolved date isn't
/// today (E1/new "required" case exits here, nothing written) -- then
/// the note body is handed to `storage::insert_punch_with_note`, which
/// itself validates empty/whitespace bodies *before* opening a
/// transaction (E5) and wraps the punch+note pair in one transaction so
/// a failure at either insert rolls back both (§6.1: a rejected note
/// leaves no orphaned punch).
fn punch(
    conn: &mut Connection,
    now: DateTime<Local>,
    kind: PunchKind,
    args: &PunchArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = match &args.date {
        Some(s) => resolve_future_checked_date(s, today)?,
        None => today,
    };

    let time_of_day = match &args.time {
        Some(s) => parse_time(s)?,
        None if target_date == today => now.time(),
        None => return Err(TimeParseError::Required.into()),
    };

    let note_text = join_note(&args.note);

    storage::insert_punch_with_note(
        conn,
        kind,
        target_date,
        time_of_day,
        &Local,
        note_text.as_deref(),
        now.with_timezone(&Utc),
    )?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib commands::`
Expected: PASS for all new tests and all pre-existing `commands.rs`
tests (in particular `start_no_args_uses_now`, `malformed_time_rejected`,
`time_error_precedes_note_error` — these must keep passing unchanged
since `args.date` is `None` in all of them, so `target_date == today`
and behavior is identical to before).

- [ ] **Step 5: Run the full test suite**

Run: `cargo test`
Expected: PASS, zero failures, across `date.rs`, `time.rs`, `cli.rs`,
`status.rs`, `commands.rs`, and every other module.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: clean (matches this repo's existing CRAP-ish pre-commit gate
per `AGENTS.md`; no new lint suppressions should be needed for this
change).

- [ ] **Step 7: Commit**

```bash
git add src/commands.rs
git commit -m "feat(commands): resolve --date in start/stop/note, require TIME when backdated"
```

---

## Self-Review Notes (for the plan author, kept for the record)

- **Spec coverage**: §2 CLI surface → Tasks 1, 2, 4. §2.1 ordering
  footgun → Task 4's locked-down tests (both `start` and `note`). §3
  validation (malformed date, `-N` overflow, future-date, TIME-required,
  error precedence) → Tasks 1, 2, 3, 6. §3.1 (retroactive week figures)
  needs no new production code — it falls out of the existing
  live-computation model untouched — but it is still this feature's
  central claim, so Task 6 carries one dedicated end-to-end test
  (`backdated_punch_retroactively_changes_a_later_closed_weeks_owed`)
  proving a backdated punch actually moves a later closed week's owed
  figure through `status::resolve`, not just that storage accepts the
  write. §4 touch points → Tasks 1, 2, 3, 4, 5, 6 map 1:1 onto the
  spec's own file list. §5 testing plan → each bullet has a
  corresponding test in Tasks 1, 2, 4, 5, 6, plus the §3.1
  retroactive-recompute test above (not itself a separate §5 bullet,
  but implied by §3.1's "whole point of the feature" framing). §6 out
  of scope → no task touches `week`, editing/deleting, or `NOTE`'s
  `trailing_var_arg` capture.
- **Placeholder scan**: no TBDs; every step has literal code, not a
  description of code.
- **Type/signature consistency checked across tasks**: `resolve_date(s:
  &str, today: NaiveDate) -> Result<NaiveDate, DateWeekError>` (Task 1)
  is the exact signature Task 2 wraps and Task 5 calls.
  `resolve_future_checked_date` (Task 2) is the exact name/signature
  Task 6 imports and calls. `TimeParseError::Required` (Task 3) is the
  exact variant Task 6 matches on and constructs. `PunchArgs.date` /
  `NoteArgs.date: Option<String>` (Task 4) is the exact field Task 6
  reads as `&args.date`. Task 6's retroactive test calls
  `status::resolve(date_arg: Option<&str>, now: DateTime<Local>, conn: &Connection) -> anyhow::Result<StatusView>`
  and reads `StatusView.week_line: String` — both exactly as declared in
  `src/status.rs` today (unmodified by Task 5 except for the internal
  resolver swap), verified directly against the file rather than
  assumed.
- **Adversarial review round (fixed inline, not re-reviewed)**: Task 6's
  `d()` fixture was missing entirely (compile-error blocker) — added,
  matching `date.rs`/`status.rs`'s existing convention exactly; Task 6's
  "expected fail" step text now says COMPILE failure, not a runtime one;
  added the §3.1 end-to-end retroactive-recompute test; added the
  `Status` `DATE` doc-comment update to Task 4; added a `stop`-specific
  backdated test (`backdated_stop_with_explicit_time_succeeds`) and a
  `stop_args()` fixture to Task 6; extended Task 4's doc-comment step to
  also fix `PunchArgs.note`/`NoteArgs.body`'s stale "today" wording;
  renamed Task 2's misnamed `future_checked_rejects_future_shorthand` to
  `future_checked_shorthand_is_never_mistaken_for_future`; added a
  matching `note` lock-down test for the §2.1 clap footgun to Task 4;
  added a third, concretely-computed overflow case
  (`-18446744073709551615`, i.e. `-u64::MAX`) to Task 1's overflow test.
</content>
