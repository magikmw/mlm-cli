# Task 1 plan: orphan-stop-warning, code and tests

Changeset plan: `docs/dev/plans/orphan-stop-warning-plan.md` (Task 1).
Spec: `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (§3, §4, §5, §6; non-goals §1.1).

## Read for this plan

- `docs/dev/plans/orphan-stop-warning-plan.md` (full)
- `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (full)
- `docs/dev/plans/orphan-stop-warning-task-2-docs.md` did not exist when this plan was written. Task 2 shares no file and no signature with this task, so nothing here depends on it.
- `src/commands.rs` (full)
- `src/anomalies.rs` (full)
- `src/history.rs` (full)
- `src/storage.rs` (lines 1-80, 330-430: formats, `Punch`, `punch_from_row`, `punches_for_date`, `punches_in_range`; the rest surveyed by grep)
- `src/stint.rs` (lines 1-340: `DayStints`, `classify`, `close_group`, `classify_at`)
- `src/status.rs:455-500` (how `status` builds D's facts), `src/date.rs:175` (`format_date_with_weekday`), `src/render.rs:152` (`local_time`), `src/main.rs:1-60` (dispatch)
- `tests/start_hint_cli.rs` (full), `tests/ascii_sweep.rs` (grep), `tests/week_target_cli.rs:130-140`, `tests/status_cli.rs:60-100`, `tests/overlap_cli.rs:140-165`
- `clippy.toml`, `.githooks/pre-commit` (first 80 lines), `AGENTS.md`

## Scope

| File | Owner | This task |
|---|---|---|
| `src/commands.rs` | Task 1 | edit |
| `src/anomalies.rs` | Task 1 | edit |
| `tests/orphan_stop_cli.rs` | Task 1 | new |
| `src/storage.rs`, `src/history.rs`, `src/stint.rs` | nobody (read only) | call, never edit |
| `docs/dev/SPEC.md`, `README.md`, `AGENTS.md` | Task 2 | do not touch |
| `docs/dev/NOTES.md`, spec archive banner | coordinator | do not touch |

## Dependencies

| Dependency | State | Use |
|---|---|---|
| `storage::punches_in_range(conn, from, to) -> Result<Vec<Punch>, StorageError>` (`src/storage.rs:398`) | shipped | the one ranged read |
| `history::PunchBuckets::from_punches(&[Punch])`, `.trio(date) -> DateBuckets<'_>` (`src/history.rs:27`, `:41`) | shipped | three-date bucket view |
| `anomalies::date_facts(&DateBuckets, date, today, now_utc) -> DateAnomalies` (`src/anomalies.rs:436`) | shipped | splice-aware facts; `orphaned_end_times` is ascending |
| `date::format_date_with_weekday(NaiveDate) -> String` (`src/date.rs:175`) | shipped | header date |
| Task 2 | parallel, no interface | none |

There is no unlanded dependency, so there are no stubs.

## Facts gathered

### `src/commands.rs`

- Line 10 imports `chrono::{DateTime, Local, TimeZone, Utc}`. Lines 14-16 import `crate::date::{format_date, resolve_future_checked_date}`, `crate::lookback::{self, LookbackEntry, LookbackKind}` and `crate::storage::{self, Note, Punch, PunchKind}`. The file spells `chrono::NaiveDate` fully qualified in signatures (lines 51, 174, 201, 291).
- `start` (lines 24-34) writes through `punch`, then calls `warn_stale_starts(..., &mut std::io::stderr())`.
- `stop` (lines 39-42) calls `punch(conn, now, PunchKind::End, args)?` and discards the result.
- `warn_stale_starts` (lines 49-60) is generic over `W: std::io::Write`. It runs `let Ok(..) = .. else { return; }` and hands off to `emit_warnings` (lines 72-85), which writes each line with `let _ = writeln!(out, ...)`. That is the pattern for "best-effort, write errors ignored".
- `punch` (lines 315-342) returns `(target_date, punch_id)` only after `storage::insert_punch_with_note` commits. Every hard error (date, TIME, empty note) returns before the write.
- Test module (line 357 on): `test_db()`, `fixed_now()` = Thu 2026-02-12 14:23 Local, `today()`, `d()`, `t()` (line 2108), `stop_args(&[..])` (line 416), `punches_for(conn, date)`. `read_failure_after_write_is_best_effort` (line 1974) drops the `punches` table to force a read failure.

### `src/anomalies.rs`

- `e8_lines` (lines 272-286) is private. It formats `d = format_date(self.date)` and, per orphaned time, pushes the header and the two hint lines:
  - `"    fix: mlm start HH:MM --date {d}  (a time before {time})"`
  - `"     or: mlm delete punch --date {d}  (lists punches; rerun with the end's number)"`
  with `time = t.format("%H:%M")`.
- `detail_lines` (lines 201-209) calls `e8_lines` third.
- `date_facts` (lines 436-444) runs `classify_context` then `build_facts`. `orphaned_end_times` comes from `ctx.day`, which is `stint::classify_at(prev, day, next, now)` (line 319). `classify_at` drops a spliced orphan from `day` (`src/stint.rs:309-311`).
- `orphaned_end_times` is documented ascending (line 53). `classify` sorts by `(at_utc, kind, id)` (`src/stint.rs:145`), so insertion order never matters.
- Existing pins on E8 text: `t_e8_gains_fix_lines_text_unchanged` (line 558), `t_ordering_all_kinds_at_once` (line 602), `block_order_puts_overlap_between_e8_and_refused_joins` (line 1116).

### Reads and buckets

- `punches_in_range` parses every row in `[from, to]` and returns `StorageError::CorruptRow { column: "at_utc", .. }` on a bad `at_utc` (`src/storage.rs:363-375`, `:398-421`). A malformed D-1 row therefore fails the whole read.
- `PunchBuckets::trio` (`src/history.rs:41-47`) returns empty slices for dates without punches.
- `status` builds D's facts as `history.buckets.trio(target_date)` then `classify_context` + `build_facts` (`src/status.rs:472-483`). Building from `trio(D)` over a D-1..D+1 read gives the same three slices, so `stop`'s classification equals `status`'s.

### Tests and gates

- `tests/start_hint_cli.rs` runs the binary with `MLM_DB_PATH` and `TZ=UTC`, boots the schema with one `status` run, and seeds raw rows with `format!("{date}T{hhmm}:00Z")`.
- `tests/ascii_sweep.rs` checks stdout only. The new test file asserts ASCII on `stop`'s stderr itself.
- `tests/week_target_cli.rs:132` runs `start 9:00` then a bare `stop` and checks stdout only. It keeps passing.
- No existing test asserts empty stderr after a `stop`. `tests/status_cli.rs:94` asserts empty stderr on `status`, which this task does not change.
- `clippy.toml` sets `cognitive-complexity-threshold = 15`. The hook also runs `cargo fmt --check` and the coverage ratchet.

## Design

### Step 1. Share the E8 hint lines (`src/anomalies.rs`)

Add one public free function next to `refused_join_line` (after line 313):

```rust
/// `status`'s two E8 hint lines for an orphaned end at `time` on `date`
/// (§7.3). `mlm stop`'s warning prints the same two lines.
pub fn orphan_hint_lines(date: NaiveDate, time: NaiveTime) -> [String; 2] {
    let d = format_date(date);
    let time = time.format("%H:%M");
    [
        format!("    fix: mlm start HH:MM --date {d}  (a time before {time})"),
        format!(
            "     or: mlm delete punch --date {d}  (lists punches; rerun with the end's number)"
        ),
    ]
}
```

Rewrite `e8_lines` (lines 272-286) over it:

```rust
fn e8_lines(&self) -> Vec<String> {
    let mut out = Vec::new();
    for t in &self.orphaned_end_times {
        out.push(format!(
            "[!] orphaned end at {} (no matching start)",
            t.format("%H:%M")
        ));
        out.extend(orphan_hint_lines(self.date, *t));
    }
    out
}
```

The strings are copied byte for byte from the current lines 279-283, so `status` output does not change. Complexity is 1-2 for each function.

### Step 2. The `stop` warning step (`src/commands.rs`)

Imports:

- line 14 becomes `use crate::date::{format_date, format_date_with_weekday, resolve_future_checked_date};`
- add `use crate::anomalies;` and `use crate::history::PunchBuckets;`
- `storage::StorageError` is reached as `storage::StorageError` (the `storage` module is already imported).

Replace `stop` (lines 36-42), keeping its doc comment and adding one sentence:

```rust
/// Record an end punch for today, optionally with an inline note
/// (SPEC §3.3 — "same shape as start"), or against another day via
/// `--date` (backdated-punches spec §4). After a successful write, warns
/// on stderr about every orphaned end on the written date (§7.4) —
/// best-effort, like `start`'s warning.
pub fn stop(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> anyhow::Result<()> {
    let (date, _) = punch(conn, now, PunchKind::End, args)?;
    warn_orphaned_ends(
        conn,
        date,
        now.date_naive(),
        now.with_timezone(&Utc),
        &mut std::io::stderr(),
    );
    Ok(())
}
```

Add three private functions after `emit_warnings` (after line 85):

```rust
/// §7.4: after a successful `stop` write to `date`, warn once per orphaned
/// end on `date`, classified with the splice as `date`'s status page does.
/// A failed read prints nothing; a failed write to `out` is ignored.
fn warn_orphaned_ends<W: std::io::Write>(
    conn: &Connection,
    date: chrono::NaiveDate,
    today: chrono::NaiveDate,
    now_utc: DateTime<Utc>,
    out: &mut W,
) {
    let Ok(times) = orphaned_end_times(conn, date, today, now_utc) else {
        return;
    };
    for line in orphan_warning_lines(date, &times) {
        let _ = writeln!(out, "{line}");
    }
}

/// `date`'s orphaned-end times, ascending, from one read of `date` and its
/// two neighbours. Any read failure is an error, never an empty date.
fn orphaned_end_times(
    conn: &Connection,
    date: chrono::NaiveDate,
    today: chrono::NaiveDate,
    now_utc: DateTime<Utc>,
) -> Result<Vec<chrono::NaiveTime>, storage::StorageError> {
    let from = date.pred_opt().unwrap_or(date);
    let to = date.succ_opt().unwrap_or(date);
    let buckets = PunchBuckets::from_punches(&storage::punches_in_range(conn, from, to)?);
    Ok(anomalies::date_facts(&buckets.trio(date), date, today, now_utc).orphaned_end_times)
}

/// One three-line block per orphaned end: a header naming the date, then
/// `status`'s two E8 hint lines.
fn orphan_warning_lines(date: chrono::NaiveDate, times: &[chrono::NaiveTime]) -> Vec<String> {
    let day = format_date_with_weekday(date);
    let mut out = Vec::new();
    for t in times {
        out.push(format!(
            "warning: orphaned end on {day} at {} (no matching start)",
            t.format("%H:%M")
        ));
        out.extend(anomalies::orphan_hint_lines(date, *t));
    }
    out
}
```

Notes on the sketch:

- `writeln!` on `W` resolves through the generic bound, as in `emit_warnings`. No `use std::io::Write` is needed.
- `today` and `now_utc` do not affect `orphaned_end_times`. They are passed so the call matches `status`'s exactly.
- Each function stays at cognitive complexity 3 or less.
- Nothing is printed between blocks (contract 2). The header uses `format_date_with_weekday`, the form `start`'s warnings use (contract 1).

## Test plan

Unit tests go in `src/commands.rs`'s test module, after the §7.4 start-warning section (after line 2106), under a `// --- §7.4 stop orphaned-end warning ---` banner. They drive `punch` and `warn_orphaned_ends` with a `Vec<u8>` writer, the same way `warnings_after` (line 1769) drives `warn_stale_starts`. Add one helper:

```rust
fn stop_warnings(conn: &Connection, date: NaiveDate) -> String {
    let mut out = Vec::new();
    warn_orphaned_ends(conn, date, today(), fixed_now().with_timezone(&Utc), &mut out);
    String::from_utf8(out).expect("utf8")
}
```

CLI tests go in `tests/orphan_stop_cli.rs`. Copy `mlm_utc`, `stdout`, `stderr`, `today`, `boot` and `seed` from `tests/start_hint_cli.rs:15-60`, add `days_ago(n)`, and add one shared assertion:

```rust
fn assert_stop(o: &Output, expected_stderr: &str) {
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(stdout(o), "");
    assert_eq!(stderr(o), expected_stderr);
    assert!(stderr(o).is_ascii());
}

fn block(date: NaiveDate, hhmm: &str) -> String {
    let d = format_date(date);
    format!(
        "warning: orphaned end on {} at {hhmm} (no matching start)\n    fix: mlm start HH:MM --date {d}  (a time before {hhmm})\n     or: mlm delete punch --date {d}  (lists punches; rerun with the end's number)\n",
        format_date_with_weekday(date)
    )
}
```

`block` restates contract 1 in the test file on purpose, so a drift in the shared hint lines fails here. Every `stop` passes an explicit TIME. Only the output of the final `stop` is asserted. Earlier setup commands may print `start`'s own warnings, which these tests ignore.

### AC1. FE5 prints one block naming today (spec §6 "FE5 → one orphan block", §4)

- CLI `fe5_stop_today_after_a_start_two_days_back`: boot; `mlm start 22:00 --date -2`; `mlm stop 14:16`; `assert_stop(&o, &block(today(), "14:16"))`.
- CLI `spec_example_block_verbatim`: boot; `mlm start 22:00 --date 2026-10-03`; `mlm stop 14:16 --date 2026-10-05`; assert stderr equals spec §4's three lines as a string literal (`warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)` and the two hint lines). This pins contract 4 without depending on the clock.
- Unit `stop_warns_for_an_orphan_two_days_after_an_open_start`: `punch` a start `22:00 --date 2026-02-10`, then an end `14:16` today; `stop_warnings(&conn, today())` equals the three lines for `Thu 2026-02-12` / `2026-02-12` / `14:16`.

### AC2. A clean pair is silent (spec §6 "clean pair → silent")

- CLI `clean_pair_is_silent`: boot; `start 09:00`; `stop 17:00`; `assert_stop(&o, "")`.
- Unit `stop_is_silent_after_a_clean_pair`: same punches; `stop_warnings` is `""`.

### AC3. A splice with yesterday's open start is silent (spec §6 "splice with yesterday's open start → silent", §3 second bullet)

- CLI `splice_with_yesterdays_open_start_is_silent`: boot; `seed(yesterday, "22:00", "start")`; `mlm stop 01:00`; `assert_stop(&o, "")`. The stop is today's first punch, so `classify_at` drops the orphan.
- Unit `stop_is_silent_when_the_end_splices_with_yesterday`: same shape through `punch`.

### AC4. A displaced end warns for 17:00 (spec §6 "displaced end", §3 first bullet)

- CLI `displaced_end_warns_for_the_later_end`: boot; `start 09:00`; `stop 17:00`; `stop 12:00`; `assert_stop(&o, &block(today(), "17:00"))`. LIFO pairs 09:00 with 12:00, so 17:00 is orphaned.

### AC5. A pre-existing orphan is reported (spec §6 "pre-existing orphan, then a clean pair", §3 first bullet)

- CLI `pre_existing_orphan_is_reported_after_a_clean_pair`: boot; `seed(today, "08:00", "end")`; `start 09:00`; `stop 17:00`; `assert_stop(&o, &block(today(), "08:00"))`.

### AC6. Two orphans print two blocks, ascending (spec §6 "two orphans on one date → two blocks, ascending", contract 2)

- CLI `two_orphans_print_two_blocks_ascending`: boot; `seed(today, "16:00", "end")`; `mlm stop 10:00`; `assert_stop(&o, &(block(today(), "10:00") + &block(today(), "16:00")))`. The later-inserted punch is the earlier one, so only instant order can produce this output. No blank line sits between the blocks.
- Unit `orphan_warning_lines_one_block_per_time_in_order`: `orphan_warning_lines(d(2026, 2, 12), &[t(10, 0), t(16, 0)])` has six lines, and lines 0 and 3 are the two headers.

### AC7. `--date` to a past date names that date (spec §6 "`--date` to a past date → block names that date", §3 third bullet)

- CLI `past_date_block_names_that_date`: boot; `mlm stop 18:00 --date <days_ago(5)>`; `assert_stop(&o, &block(days_ago(5), "18:00"))`. `block` includes the weekday through `format_date_with_weekday`.
- `spec_example_block_verbatim` (AC1) also covers this with a literal weekday.

### AC8. A read failure prints nothing and the punch is stored (spec §6 "read failure after the write", §5)

- CLI `read_failure_on_the_previous_date_prints_nothing`: boot; insert a raw row `('garbage', <yesterday>, 'start')` with rusqlite; `mlm stop 17:00`; `assert_stop(&o, "")`. Then open the db and assert `SELECT COUNT(*) FROM punches WHERE "date" = ?1 AND kind = 'end'` with today's date is `1`. `status` cannot verify the row here, because its own read fails on the same garbage row.
- Unit `stop_warning_read_failure_prints_nothing`: write an orphaned end through `punch`, `DROP TABLE punches`, and `stop_warnings` is `""`. This mirrors `read_failure_after_write_is_best_effort` (line 1974) and covers the `else { return; }` arm for the coverage ratchet.
- Unit `stop_warning_ignores_a_failing_writer`: a local `struct Broken;` whose `write` returns `Err(std::io::Error::other("broken"))` (clippy's `io_other_error` lint rejects the `ErrorKind::Other.into()` form) and whose `flush` returns `Ok(())`; `warn_orphaned_ends` with a seeded orphan returns without panicking (contract 3).

### AC9. `status`'s E8 output stays byte-identical (changeset plan AC; spec §4 "word for word")

- The existing tests `t_e8_gains_fix_lines_text_unchanged`, `t_ordering_all_kinds_at_once` and `block_order_puts_overlap_between_e8_and_refused_joins` in `src/anomalies.rs`, and all of `tests/status_cli.rs`, pass without edits.
- Unit in `src/anomalies.rs` `orphan_hint_lines_are_the_e8_block_hint_lines`: for `DateAnomalies { orphaned_end_times: vec![t(18, 0)], ..none(d(2026, 9, 21)) }`, `detail_lines()[1..3]` equals `orphan_hint_lines(d(2026, 9, 21), t(18, 0))`.
- CLI `stop_hint_lines_match_status_page`: after the AC7 setup, the last two lines of `stop`'s stderr each appear as a whole line in `mlm status <date>`'s stdout.

### AC10. `stop`'s stderr is ASCII (changeset plan AC; global constraint)

- `assert_stop` asserts `stderr.is_ascii()` in every CLI case above.

## Verification

```sh
cargo fmt
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

The coverage ratchet needs the read-failure unit test from AC8. All new functions are exercised by unit tests, so coverage should not drop.

## Risks and ambiguities

- **Midnight race in CLI tests.** Relative-date cases compute `today()` in the test and again in the child. A run that crosses UTC midnight can fail. `tests/start_hint_cli.rs` carries the same risk. `spec_example_block_verbatim` pins dates and is immune.
- **Explicit future TIME today.** `mlm stop 14:16` on today is accepted even before 14:16, because `punch` does no chronology check (`src/commands.rs:324-328`). The CLI tests rely on that.
- **Header location.** The header is built in `src/commands.rs`, and only the two hint lines are shared from `src/anomalies.rs`. The changeset plan requires sharing only the hint lines, and the header is `stop`-specific.
- **Existing stop tests now print to stderr.** Unit tests that call `stop()` with orphaned ends (for example `stop_all_shapes`, `stop_before_its_start_accepted`) now write a warning to the real stderr of the test process. That is noise only, the same as `start` produces today.
- **D+1 read.** A malformed row on D+1 also suppresses the warning. The changeset plan accepts this under spec §5's "any read". No test pins it, because D+1 is always in the future for a today write and is an odd setup for a past write.
- No disagreement with the changeset plan or the spec.
