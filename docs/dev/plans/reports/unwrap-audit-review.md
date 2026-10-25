# Panic audit: `unwrap`/`expect` and other panic sources

- Tree: `main` at `dd91952`
- Scope: `src/**/*.rs` production code, `examples/seed_test_data.rs`
- Excluded: `#[cfg(test)]` modules (every one runs to end of file, except the
  `#[cfg(test)] fn build_rows` at `src/week_view.rs:241-251`), and `tests/`
- Findings: 7 `can panic` items (4 user-reachable, 3 not reachable through
  the shipped CLI)
- File name: the dispatch named `unwrap-audit.md`. The write hook only
  accepts `*-review.md` for reviewers, so this report lives at
  `unwrap-audit-review.md`.

## How claims were checked

- Every production `.unwrap()`, `.expect(`, `panic!`, `unreachable!`,
  `assert!`, indexing, slicing, division and date arithmetic site was read
  in context.
- chrono 0.4.45 behaviour was read from
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/chrono-0.4.45/src/`.
- The review hook blocks running `target/debug/mlm` and `cargo run`, and
  the sealed scratch box cannot see the repo or the cargo registry. So
  F1 was proven through `cargo test`. The existing integration test
  `status_on_a_fresh_empty_database_exits_zero` (`tests/status_cli.rs:248`)
  spawns the real binary and inherits `TZ`. The other items are proven by
  reading the code and chrono's source, and none was run.
- `faketime` is not installed. A POSIX TZ rule did the same job (see F1).

## 1. User-reachable, can panic

### F1. `Local::now()` truncation panics during the repeated autumn hour

- Site: `src/main.rs:37-41`
- Expression: `Local::now().with_second(0).unwrap().with_nanosecond(0).unwrap()`
- Class: can panic. The panic fires on line 39.
- Trigger: any subcommand run while local wall-clock time is inside a
  fall-back overlap. For Europe/Warsaw that is both passes of 02:00-02:59
  on the last Sunday of October, so two real hours (00:00-02:00 UTC).
  Only `--help`, `--version` and usage errors escape, because
  `cli::parse` runs first (`src/main.rs:18`).
- Mechanism: `DateTime::with_second` calls `map_local`
  (chrono `datetime/mod.rs:1009-1016`). That function rebuilds the
  instant with `from_local_datetime(..).single()`. In the overlap the
  local time maps to two instants, so `.single()` is `None`, and the
  unwrap panics. A spring-forward gap cannot trigger it, because
  `Local::now()` is always a real instant.
- Reachable: yes, by every user in a DST zone, once a year, for every
  command.
- Proof (run 2026-10-25 11:54 UTC). The rule `AAA0BBB-1,M3.5.0,M10.5.0/13:40`
  ends DST today at 13:40 local, so local 12:40-13:40 is ambiguous and
  "now" (12:54 local) sits inside it.

  ```
  TZ='AAA0BBB-1,M3.5.0,M10.5.0/13:40' cargo test -q --test status_cli status_on_a_fresh_empty_database_exits_zero
  stderr: "thread 'main' panicked at src/main.rs:39:10:
           called `Option::unwrap()` on a `None` value"   exit status 101
  ```

  The control run under `TZ=Europe/Warsaw` (12:54 CET, not ambiguous)
  passes.
- Windows: chrono's Windows `Local` also reports `Ambiguous` for overlap
  times, so the same panic is expected there. This was not verified.
- Suggested fix: truncate in UTC, where there is no DST, then convert.
  For example `Utc::now().with_second(0)?.with_nanosecond(0)?` (always
  `Some`), then `.with_timezone(&Local)`. Another option is
  `Local::now() - Duration::seconds(s) - Duration::nanoseconds(ns)`,
  which works on the instant instead of the wall clock.

### Shared failure class check for F1

No other call site shares the "rebuild a local time and demand one answer"
pattern.

| Site | Construction | Why it is safe |
|---|---|---|
| `src/storage.rs:210-214` | `NaiveTime::with_second(0)/with_nanosecond(0)` | Naive, no zone. Always `Some` for 0. |
| `src/storage.rs:217-226` | `tz.from_local_datetime` | Matches `Single`, `Ambiguous` (takes earliest) and `None` (returns `NonexistentLocalTime`) |
| `src/storage.rs:314-318` | `DateTime<Utc>::with_second(0)` | `map_local` on `Utc` is always `Single` |
| `src/anomalies.rs:379-385` | `Local.from_local_datetime(midnight).earliest()` | A gap falls back to naive-as-UTC through `map_or_else` |
| `src/cli.rs:387`, `examples/seed_test_data.rs:96,211-213` | `Local::now()` with no `with_*` | No local rebuild |
| `src/date.rs:123,223,308,420,431` | `from_ymd_opt`/`from_isoywd_opt` | Naive dates, no DST. See the year-bound table in section 3. |
| `.and_hms_opt`, `.single()`, `and_local_timezone` | none in production | — |
| `MLM_DB_PATH` / `ProjectDirs` (`src/db.rs:152-211`) | path resolution | Every step returns `DbError`. No unwrap. |

### F2. `est. EOD` overflows chrono on a huge week target

- Site: `src/status.rs:367`
- Expression: `open.start.at_utc.with_timezone(&Local) + ChronoDuration::minutes(base_gap)`
- Class: can panic, in release and debug builds. Two chrono panics are
  possible here.
  - `TimeDelta::minutes` panics with "TimeDelta::minutes out of bounds"
    when `base_gap` is above about 1.54e14 minutes (chrono
    `time_delta.rs:190-192`).
  - `DateTime + TimeDelta` panics with "`DateTime + TimeDelta` overflowed"
    when the sum passes year 262142, which is about 1.37e11 minutes from
    2026 (chrono `datetime/mod.rs:1529-1536`).
- Trigger: today's page has an open stint and `gap_minutes > 0`, and the
  week's required minutes exceed about 1.37e11. `parse_duration` accepts
  any digit run that fits in `i64` (`src/time.rs:92-104`), and the schema
  only checks `>= 0` (`src/db.rs:66`).

  ```
  mlm week target 99999999999h    # 6.0e12 min; Monday's slice alone is 1.2e12
  mlm start
  mlm status                      # panics in est_eod
  ```

  A huge target on an earlier week does the same thing through carry-in.
- Reachable: yes, through a typo or a joke target. No backdating is
  needed. Carry alone cannot reach the threshold, because a walk from year
  0000 builds up only about 2.5e8 minutes of deficit.
- Suggested fix: cap DURATION at parse time (for example 168h × some
  sane factor). Another option is `TimeDelta::try_minutes` plus
  `checked_add_signed`, showing no EOD line when either returns `None`.

### F3. `print!`/`println!`/`eprintln!` panic on a closed pipe

- Sites:
  - stdout: `src/status.rs:527`, `src/week_view.rs:329`,
    `src/export.rs:52`, `src/week_target.rs:79`,
    `src/commands.rs:198,211,296,305,328,337`
  - stderr: `src/main.rs:84`, `src/cli.rs:367`
- Class: can panic. The std macros panic when the write fails ("failed
  printing to stdout: Broken pipe (os error 32)", exit 101). Rust sets
  `SIGPIPE` to ignore on Unix, so a closed reader turns into an `EPIPE`
  error instead of a quiet signal death. A closed fd (`>&-`) is safe,
  because std treats `EBADF` as success.
- Trigger: the reader closes before mlm writes.
  - `mlm week | true` is a likely race, because `true` exits while mlm is
    still opening the DB.
  - `mlm export notes --month 2026-10 | head -1` panics once the output
    exceeds the 64 KiB pipe buffer.
  - `mlm week 7 target 5h 2>&1 | true` hits the same panic on the stderr
    path.
- Reachable: yes, in ordinary shell use. Not run, because the hook
  blocks running the binary.
- Note: the stderr warnings in `src/commands.rs:94,112` already use
  `let _ = writeln!`, and they are safe.
- Suggested fix: write through `std::io::stdout().lock()` with
  `write_all`, and treat `ErrorKind::BrokenPipe` as a clean exit (0, or
  the conventional 141).

### F4. Week accounting `i64` overflow (debug builds panic, release wraps)

- Sites: `src/week.rs:185-187` (`worked + carry_in`, `target - fulfillment`,
  `fulfillment - target`), `src/week.rs:255`, `src/status.rs:385`
  (`gap_minutes + live`), `src/render.rs:66,94,112` (`-gap`, `-acct.owed`,
  `fulfillment - carry_in`)
- Class: can panic in debug builds only. `Cargo.toml` leaves
  `overflow-checks` at the release default (off), and `dist` inherits
  `release`. Shipped binaries and `cargo install` wrap silently and print
  garbage totals.
- Trigger: a target near `i64::MAX` on a week earlier than the shown week,
  with data that anchors the walk at or before it.

  ```
  mlm note --date 2026-10-12 x
  mlm week target 2026-42 9223372036854775807m
  mlm week          # 2026-43: owed = 2400 - (0 - MAX) overflows at week.rs:186
  ```

- Reachable: yes, but it panics only for someone running a debug build
  (`cargo run`). In release the same input gives wrong numbers, and F2's
  panic may follow if the wrapped gap stays positive.
- Suggested fix: the same DURATION cap as F2 removes the input. Checked
  or saturating arithmetic would conflict with the comment at
  `src/week.rs:182-184`, which says nothing here is clamped. Capping the
  input fits that design better.

## 2. Not reachable through the shipped CLI, can panic

### F5. `WeekId::start` on a corrupt date row at chrono's minimum year

- Site: `src/date.rs:308-309`
- Expression: `NaiveDate::from_isoywd_opt(..).expect("WeekId's invariant guarantees the ISO week exists")`
- Class: can panic. `WeekId::from_date` (`src/date.rs:260-266`) does not
  enforce the `1000..=9999` invariant the type's doc claims
  (`src/date.rs:231-232`).
- Trigger: a hand-edited `punches."date"` or `notes."date"` value in
  `-262143-01-01` through `-262143-01-04`. `storage::parse_date`
  (`src/storage.rs:42-47`) uses `%Y`, which accepts a signed year of any
  width (chrono `format/parse.rs:408-417`). The value then sorts first
  under `MIN("date")` (`src/storage.rs:532-546`). Its ISO week starts on
  Monday -262144-12-29, which is before `NaiveDate::MIN`. The path is
  `history::load_history`, then `walk_start`, then `read_range`, then
  `start_week.start()` (`src/history.rs:90-91,117`). Both `status` and
  `week` panic.
- Reachable: no. The CLI only writes `0000`-`9999` dates
  (`src/date.rs:104-124`). The input must come from a hand-edited DB.
- Suggested fix: reject years outside `0..=9999` in `storage::parse_date`
  as `CorruptRow`.

### F6. Seeder `--weeks`/`--seed`/`--db` with no value

- Site: `examples/seed_test_data.rs:102-104`
- Expression: `args.next().expect("--weeks needs a value")` (and the
  same for `--seed` and `--db`)
- Class: can panic.
- Trigger: `cargo run --example seed_test_data -- --db`
- Reachable: dev tool only, not shipped.
- Suggested fix: `.ok_or_else(|| anyhow!(..))?`.

### F7. Seeder `--weeks` out of range

- Site: `examples/seed_test_data.rs:214`
- Expression: `today - Duration::days(weeks * 7 - 1)`
- Class: can panic.
- Trigger, three ways:
  - `weeks * 7` overflows. Examples build in debug.
  - `Duration::days` panics above about 1.07e14 days.
  - `NaiveDate - TimeDelta` panics past chrono's minimum year, starting
    around `--weeks 14000000`.
- Reachable: dev tool only.
- Suggested fix: bound `--weeks`, for example `1..=520`.

## 3. Cannot panic

| Site | Expression | Invariant |
|---|---|---|
| `src/storage.rs:211-214` | `NaiveTime::with_second(0).expect` / `with_nanosecond(0).expect` | 0 is always a valid second and nanosecond, and `NaiveTime` has no zone |
| `src/storage.rs:315-318` | `DateTime<Utc>::with_second(0).expect` / `with_nanosecond(0).expect` | `Utc` maps every local time to one instant, and "now" is inside `MIN_UTC..=MAX_UTC` |
| `src/stint.rs:320` | `next.orphaned_ends.first().expect(..)` | `splice_candidate` returned true, so `orphaned_ends.len() == 1` (`src/stint.rs:347`) |
| `src/stint.rs:310,316`, `src/stint.rs:353`, `src/status.rs:406` | `remove(0)`, `orphaned_ends[0]` | Same `splice_candidate` gate. It also requires `open.len() == 1` on the earlier side. |
| `src/week.rs:217` | `week_series(..).pop().expect(..)` | The loop pushes before its first `break` check (`src/week.rs:189-201`) |
| `src/cli.rs:461` | `unreachable!` | Clap `num_args = 1..=2`, `required = true` (`src/cli.rs:281-288`) |
| `src/date.rs:420-421` | `Month::first_day` `expect` | `Month::new` holds `1..=12` and `0..=9999` (`src/date.rs:412`) |
| `src/date.rs:431-433` | `Month::last_day` `expect` | The next month's 1st is at most `10000-01-01`, inside chrono's range |
| `src/date.rs:223-224` | `iso_weeks_in_year` `expect` | Callers pass `1000..=9999` (`WeekId::new`) or the year of a week at or before `shown` (`next()` in `week_series`). Dec 28 exists for every year below chrono's max. |
| `src/date.rs:342` | `assert!(iso_year <= MAX_YEAR)` in `next()` | Its only production caller is `week_series`, and only while `current < through`. `through` is a parsed `WeekId` (year ≤ 9999) or today's week. |
| `src/date.rs:356-359` | `assert!` in `prev()` | No production caller (`prev()` and `week_range` are lib-only API) |
| `src/date.rs:107-108,120-122,290,297,300`, `src/time.rs:32` | byte index, `s[a..b]` | The length is checked first in the same `&&` chain, and the shape check makes the string pure ASCII, so every slice lands on a char boundary |
| `src/commands.rs:186,304,336` | `id as usize - 1`, `rows[index]` | `validate_entry_id` rejects `id == 0` and `index >= count` |
| `src/week_view.rs:223,229`, `src/week_view.rs:302` | `dates[i]`, `WEEKDAY_ABBREVS[i]`, `rows[i]` | `from_fn` over a fixed 7, and `i` comes from `position` over the same 7-array |
| `src/anomalies.rs:146`, `src/stint.rs:158-164` | `spans[i + 1..]`, `sorted[i..group_end]` | `i < len` from `enumerate` / the `while` guard, and `group_end <= len` |
| `src/time.rs:113`, `src/week.rs:231` | `/ 60`, `% 60`, `div_euclid(5)` | Constant non-zero divisors, no runtime divisor anywhere |
| `src/time.rs:85,94-95`, `src/date.rs:147` | `-magnitude`, `checked_mul/add`, `checked_sub_days` | Checked, or negating a non-negative `i64` |
| `src/date.rs:314,325` | `start + Days::new(6)` / `Days::new(i)` | Weeks come from 4-digit-year inputs or real dates, so the sum stays far from chrono's max |
| `src/status.rs:392`, `src/lookback.rs:228,238` | `today - ChronoDuration::days(1..=15)` | `today` is a real current date |
| `src/history.rs:119-120`, `src/commands.rs:124-125`, `src/anomalies.rs:388` | `pred_opt`/`succ_opt` | `unwrap_or` fallbacks |
| `src/status.rs:82`, `src/commands.rs:266` | `unwrap_or_default` / `unwrap_or("")` | Non-panicking forms |
| `src/storage.rs:33-47,77-86,566` | stored `at_utc`, `date`, `kind`, `week_id` | Parse failures map to `StorageError::CorruptRow`, and `rusqlite` type mismatches propagate as `Err` |
| `src/main.rs:31` | `env_logger::Builder::init()` | Called once per process |
| `examples/seed_test_data.rs:181,274` | `from_hms_opt(..).unwrap()` | `rng.range(20,22)`, `(7,9)` and `(0,59)` are always valid. `f64()` is in `[0,1)`. |
| `examples/seed_test_data.rs:65` | `items[..]` in `pick` | Called only with non-empty constant slices |

`debug_assert!`s (debug-only, and the invariants hold on every traced path):
`src/anomalies.rs:254`, `src/commands.rs:260`, `src/lookback.rs:87,112`,
`src/render.rs:203`, `src/stint.rs:133,270-298`, `src/week_target.rs:70`.

## 4. Test-code calls skipped (`.unwrap()` + `.expect(`)

| File | Count | File | Count |
|---|---|---|---|
| `src/anomalies.rs` | 15 | `src/time.rs` | 24 |
| `src/carry_scan.rs` | 5 | `src/week.rs` | 7 |
| `src/cli.rs` | 42 | `src/week_target.rs` | 52 |
| `src/commands.rs` | 145 | `src/week_view.rs` | 89 |
| `src/date.rs` | 43 | `tests/ascii_sweep.rs` | 5 |
| `src/db.rs` | 64 | `tests/carry_marker_cli.rs` | 11 |
| `src/export.rs` | 5 | `tests/e6_db_failure.rs` | 6 |
| `src/history.rs` | 28 | `tests/export_notes_cli.rs` | 9 |
| `src/lookback.rs` | 44 | `tests/orphan_stop_cli.rs` | 10 |
| `src/main.rs` | 0 | `tests/overlap_cli.rs` | 5 |
| `src/render.rs` | 5 | `tests/overnight_visibility_cli.rs` | 5 |
| `src/status.rs` | 238 | `tests/start_hint_cli.rs` | 5 |
| `src/stint.rs` | 26 | `tests/status_cli.rs` | 13 |
| `src/storage.rs` | 186 | `tests/week_notes_cli.rs` | 11 |
| | | `tests/week_read_errors.rs` | 7 |
| | | `tests/week_target_cli.rs` | 2 |

## 5. Adjacent observations (not panics, not counted)

- `src/date.rs:149-154` says `parse_date` accepts only `1000..=9999`, but
  `has_ymd_shape` accepts `0000`-`9999`. So `mlm start --date 0001-01-01
  09:00` is accepted. Every later `status`/`week` then walks about
  105k weeks from year 1, and builds `WeekId`s below the documented
  1000 floor.
- `mlm week 9999-52` ends its read range on `10000-01-03`, which chrono
  formats as `+10000-01-03`. In text order `+` sorts before `2`, so
  `BETWEEN` (`src/storage.rs:405`) matches nothing, and that week shows
  no punches.
- chrono's Unix `Local` has its own `expect("unable to select local time
  type")` (chrono `offset/local/unix.rs:155,168`). No mlm input was found
  that reaches it, because stored `at_utc` values are bounded to 4-digit
  years by RFC 3339 parsing.
