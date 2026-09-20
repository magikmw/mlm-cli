# Milestone 14, Task 3 — low-level plan: `src/commands.rs` list/delete

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.

handlers and the recreate-echo

**Scope:** `src/commands.rs` only. New `delete_note`/`delete_punch`
handlers, a quoting helper, small factored-out helpers (list printer,
id validator, recreate-line builders), and their tests. No `cli.rs`/
`storage.rs`/`main.rs` changes (Tasks 1, 2, 4). This is a plan, not a
diff — nothing in `src/commands.rs` has been edited to produce it.

**Sources read in full:** `docs/dev/plans/milestone-14-delete-punches-notes.md`
(whole file), `docs/dev/specs/2026-09-13-delete-punches-notes.md`
(whole file, current post-review revision), `docs/dev/plans/milestone-14-task-1-storage-delete.md`,
`docs/dev/plans/milestone-14-task-2-cli.md`, current `src/commands.rs`,
`src/date.rs`, `src/storage.rs`, `src/status.rs`, `src/main.rs`,
`clippy.toml`, `.githooks/pre-commit`.

---

## 1. Facts gathered (so the steps below are unambiguous)

- `commands.rs` currently imports `crate::date::resolve_future_checked_date`
  (line 14) and nothing else from `date`; `crate::storage::{self,
  PunchKind}` (line 15). `resolve_date` (the permissive resolver) is
  never imported here — confirmed by `grep -n "resolve_date"
  src/commands.rs` matching only `resolve_future_checked_date` and the
  local `resolve_target_date` helper. Task 3 adds no new resolver.
- `resolve_future_checked_date(s: &str, today: NaiveDate) ->
  Result<NaiveDate, DateWeekError>` (`src/date.rs:161`) — exact
  signature to call, via the existing local `resolve_target_date`
  helper (line 45-53), reused as-is, no change needed.
- `storage::punches_for_date(conn: &Connection, date: NaiveDate) ->
  Result<Vec<Punch>, StorageError>` (`src/storage.rs:349`) and
  `storage::notes_for_date(conn: &Connection, date: NaiveDate) ->
  Result<Vec<Note>, StorageError>` (`src/storage.rs:394`) — both
  already correctly ordered (punches by `at_utc`, notes by
  `created_at_utc ASC, id ASC`), reused verbatim, no new storage read.
- Task 1's exact signatures (from `milestone-14-task-1-storage-delete.md`
  §5, must be used verbatim):
  - `pub fn delete_punch(conn: &Connection, id: i64) -> Result<Punch, StorageError>`
  - `pub fn delete_note(conn: &Connection, id: i64) -> Result<Note, StorageError>`
  - `StorageError::NotFound { id: i64 }` — new variant, `Display`:
    `"no row found with id {id}"`.
- Task 2's exact types (from `milestone-14-task-2-cli.md` §2-4, must be
  used verbatim):
  - `pub struct DeleteArgs { pub target: DeleteTarget }`
  - `pub enum DeleteTarget { Note(DeleteEntryArgs), Punch(DeleteEntryArgs) }`
  - `pub struct DeleteEntryArgs { pub id: Option<u32>, pub date:
    Option<String> }`
- `Punch` fields (`src/storage.rs:94-111`): `id: i64`, `at_utc:
  DateTime<Utc>`, `date: NaiveDate`, `kind: PunchKind`.
- `Note` fields (`src/storage.rs:114-130`): `id: i64`, `date:
  NaiveDate`, `body: String`, `created_at_utc: DateTime<Utc>`.
- `PunchKind::as_str(self) -> &'static str` (`src/storage.rs:61-66`):
  `"start"` / `"end"` — this is also the exact token `mlm start`/`mlm
  stop` are invoked as, so it doubles as the recreate line's verb.
- Per-instant local-time conversion pattern already established in
  `status.rs` (`src/status.rs:215,228,229,235`): `punch.at_utc
  .with_timezone(&Local).time()` — done per row, never a single offset
  captured once from `now`. Task 3's punch recreate-line builder must
  use exactly this pattern (`chrono::Local`, already imported in
  `commands.rs` line 10) against the *deleted row's own* `at_utc`, not
  against the injected `now`.
- Time formatting: `status.rs` (line 143-144) formats with
  `.format("%H:%M")` directly — no shared helper function exists to
  import; Task 3's punch recreate builder does the same
  `.format("%H:%M")` inline (no helper needed, this is a one-line
  `chrono` call, not logic worth its own function).
- `date::format_date(d: NaiveDate) -> String` (`src/date.rs:169-171`) —
  canonical `YYYY-MM-DD` form, reused for the recreate line's `--date`
  value (never hand-rolled with `.format(...)` a second time).
- `--verbose` (`src/cli.rs:12-13`) is the concrete registered global
  flag the quoting round-trip test's leading-flag-lookalike case must
  target (`"--verbose logging bug"`), per the milestone plan's and
  spec's explicit instruction.
- `main.rs`'s `dispatch` (line 38-53) matches on `cli.command` and
  calls handlers directly (`commands::start(&mut conn, now, a)` etc.)
  — Task 4 will add `Command::Delete(args) => match &args.target { ...
  }` arms calling this task's `delete_note`/`delete_punch`. Task 3's
  handlers must accept `&Connection` (read-then-write, no transaction
  needed — `storage::delete_punch`/`delete_note` are each already a
  single atomic statement per Task 1) rather than `&mut Connection`;
  confirm this against Task 1's exact signatures (`conn: &Connection`)
  — **the handler signatures in this task take `&Connection`, not
  `&mut Connection`**, unlike `start`/`stop`/`note`, since there is no
  multi-statement transaction here and `delete_punch`/`delete_note`
  themselves only require a shared reference.
- Existing test module fixtures to reuse, not reinvent
  (`src/commands.rs:118-183`): `test_db() -> Connection`, `fixed_now()
  -> DateTime<Local>` (2026-02-12T14:23:00 local, Thursday), `today()
  -> NaiveDate` (2026-02-12), `d(y, m, day) -> NaiveDate`, `punches(&conn)
  -> Vec<Punch>` / `notes(&conn) -> Vec<Note>` (scoped to `today()`),
  `punches_for(&conn, date)` / `notes_for(&conn, date)` (scoped to an
  arbitrary date), and the `Cli::try_parse_from` + `match
  ...command { Command::X(a) => a, other => panic!(...) }` extractor
  pattern (`punch_args`, `note_args`, `stop_args`). Task 3 adds
  `delete_note_args(argv: &[&str]) -> DeleteEntryArgs` and
  `delete_punch_args(argv: &[&str]) -> DeleteEntryArgs` following that
  exact pattern, matching on `Command::Delete(a) => match a.target {
  DeleteTarget::Note(e) => e, ... }` (panicking via the same
  `other => panic!("expected ..., got {other:?}")` shape).
- `clippy.toml`: `cognitive-complexity-threshold = 15`.
  `.githooks/pre-commit` runs `cargo clippy --all-targets --quiet
  --message-format=json -- -W clippy::cognitive_complexity` and
  compares each function's score against that threshold. This is the
  concrete reason this plan factors the handlers into several small
  helpers below rather than writing two large functions.

## 2. New imports

Add to the existing `use` block at the top of `src/commands.rs`:

```rust
use crate::cli::{DeleteEntryArgs, NoteArgs, PunchArgs};
use crate::date::format_date;
use crate::storage::{Note, Punch, PunchKind, StorageError};
```

(`storage::{self, PunchKind}` on line 15 becomes `storage::{self, Note,
Punch, PunchKind, StorageError}` — `self` is kept because
`storage::delete_punch`/`storage::delete_note`/`storage::punches_for_date`/
`storage::notes_for_date` are all still called qualified as
`storage::...`, matching the existing style for `storage::insert_punch_with_note`
etc.) `Local` is already imported (line 10); no change needed there.

## 3. Design overview — why this many small functions

The milestone plan's Task 3 acceptance criteria explicitly require
keeping `delete_note`/`delete_punch` under the cognitive-complexity
gate by delegating to helpers. The handler for each of `note`/`punch`
has five logically distinct steps (resolve date → fetch rows → branch
list/delete → validate id → build+print recreate line), so this plan
factors out one function per step that is nontrivial, shared where the
punch/note logic is identical, and kept separate where it differs
(quoting only applies to notes; per-instant TZ conversion only applies
to punches).

Function inventory (all new, all in `src/commands.rs`, all private
except the two top-level handlers):

1. `quote_shell_single(body: &str) -> String` — the quoting helper.
2. `validate_entry_id(id: u32, count: usize) -> anyhow::Result<usize>`
   — id-range validator, shared by both handlers, returns the
   zero-based index into the freshly-fetched `Vec` on success.
3. `print_punch_list(rows: &[Punch])` — list-mode printer for punches.
4. `print_note_list(rows: &[Note])` — list-mode printer for notes.
5. `punch_recreate_line(p: &Punch, today: NaiveDate) -> String` —
   recreate-line builder for a deleted punch.
6. `note_recreate_line(n: &Note, today: NaiveDate) -> String` —
   recreate-line builder for a deleted note, including the §5 legacy-
   newline defensive fallback.
7. `delete_note(conn: &Connection, now: DateTime<Local>, args:
   &DeleteEntryArgs) -> anyhow::Result<()>` — top-level handler.
8. `delete_punch(conn: &Connection, now: DateTime<Local>, args:
   &DeleteEntryArgs) -> anyhow::Result<()>` — top-level handler.

`print_punch_list`/`print_note_list` and
`punch_recreate_line`/`note_recreate_line` are deliberately **not**
generic/merged into one type-erased function: `Punch` and `Note` have
different fields and different formatting rules (quoting only applies
to note bodies; the DST-safe per-instant conversion only applies to
punches), so a shared abstraction would need its own branching inside
it — that just relocates complexity rather than removing it, and this
project has no existing precedent (`status.rs` also keeps punch/stint
and note rendering as separate functions) for merging these two
concerns.

## 4. `quote_shell_single` — the quoting helper

**Placement:** top-level free function in `src/commands.rs`, directly
above `note_recreate_line` (§6 below) since that is its only caller.
Kept general-purpose (not `note`-specific in name) in case a future
command needs it, matching this file's existing style of small
top-of-purpose-cluster helpers like `join_note`.

```rust
/// Wrap `body` as a single, safely-quoted POSIX/fish shell token:
/// single-quoted, with every embedded `'` escaped as `'"'"'` (close
/// the open single-quote, emit a double-quoted literal `'`, reopen the
/// single-quote). Single quotes, never double: double quotes still let
/// a shell interpolate `$(...)`/backticks/`$VAR` inside them before the
/// token reaches `mlm` on replay (spec §5). The output always starts
/// and ends with `'`, even for an empty or already-safe body.
fn quote_shell_single(body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 2);
    out.push('\'');
    for ch in body.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}
```

**Escaping algorithm, spelled out precisely:**
1. Start the output with a literal `'`.
2. Walk `body` one `char` at a time (not byte-at-a-time — bodies have
   no charset restriction per SPEC §2.3, so this must be Unicode-scalar
   safe; `String::chars()` already guarantees that).
3. For every character that is **not** `'`: append it to the output
   unchanged.
4. For every character that **is** `'`: append the four-character
   literal sequence `'"'"'` instead (close the currently-open single
   quote with `'`, open a double-quoted segment containing one literal
   `'`, close the double-quoted segment with `"`, reopen a single
   quote with `'` — four shell tokens concatenated with no separator,
   which is valid because shells silently concatenate adjacent quoted
   segments with no space between them).
5. End the output with a literal `'`.

No other character needs escaping inside single quotes in POSIX shells
or fish (this is exactly why single-quote wrapping was chosen over
double — per spec §5, single quotes suppress all interpolation, so `$`,
backtick, `\`, and `"` all pass through inert).

## 5. `validate_entry_id` — id-range validator

**Placement:** directly above `delete_note`/`delete_punch` (§7-8),
since both call it identically.

```rust
/// Validate a 1-based ephemeral `id` against `count` freshly-fetched
/// rows for the resolved date; on success, return the corresponding
/// zero-based index into that `Vec`. `id == 0` and `id > count` are
/// both application-level hard errors (spec §5) — clap has already
/// rejected negative/non-numeric/too-large values before this ever
/// runs (Task 2). The error message names both the requested position
/// and how many entries exist, per the milestone plan's Task 3
/// acceptance criteria.
fn validate_entry_id(id: u32, count: usize) -> anyhow::Result<usize> {
    if id == 0 {
        anyhow::bail!("id must be 1 or greater (got 0); {count} entr{} for this date",
            if count == 1 { "y" } else { "ies" });
    }
    let index = id as usize - 1;
    if index >= count {
        anyhow::bail!("id {id} is out of range; only {count} entr{} for this date",
            if count == 1 { "y" } else { "ies" });
    }
    Ok(index)
}
```

(Exact wording of the bail! message is an implementation detail the
milestone plan leaves open — "names both the requested position and
how many entries exist" is the hard requirement; the pluralization
branch is optional polish, drop it if it complicates the
cognitive-complexity score and hardcode "entries" instead — not
load-bearing for any acceptance criterion.)

## 6. List-mode printers

**Placement:** directly above `delete_punch`/`delete_note`, grouped
with the other new small helpers.

```rust
/// List-mode output for punches (spec §4): `<n>  <kind> <HH:MM>` per
/// row, 1-based. Pure — never touches storage.
fn print_punch_list(rows: &[Punch]) {
    for (i, p) in rows.iter().enumerate() {
        let local_time = p.at_utc.with_timezone(&Local).time();
        println!("{}  {} {}", i + 1, p.kind.as_str(), local_time.format("%H:%M"));
    }
}

/// List-mode output for notes (spec §4): `<n>  <body>` per row,
/// 1-based. Pure — never touches storage.
fn print_note_list(rows: &[Note]) {
    for (i, n) in rows.iter().enumerate() {
        println!("{}  {}", i + 1, n.body);
    }
}
```

Both take `&[T]` (a slice, not `&Vec<T>`) — matches this crate's usual
style of taking the loosest sufficient type; callers pass `&rows`.

The "nothing to delete" empty-list message is **not** inside these
printers — it belongs in the top-level handler (§7/§8) since it's a
single `println!` guarded by `rows.is_empty()`, not worth its own
function, and keeping it in the handler makes the handler's own
control flow ("empty → print + return" vs "non-empty → print list")
readable without an extra indirection.

## 7. Recreate-line builders

**Placement:** directly above the two top-level handlers, below
`quote_shell_single`.

```rust
/// Build the punch recreate-echo (spec §5/§7): `mlm start|stop
/// HH:MM[ --date YYYY-MM-DD]`. Converts the deleted row's own stored
/// `at_utc` to local time per-instant (`p.at_utc.with_timezone(&Local)`)
/// -- never reuses `now`'s offset, so a backdated punch on the far
/// side of a DST boundary from today still echoes its correct local
/// wall-clock time (spec §7, backdated-punches spec §2.1).
fn punch_recreate_line(p: &Punch, today: NaiveDate) -> String {
    let verb = p.kind.as_str(); // "start" | "end" -- same token as the CLI command name for End? see note below
    let local_time = p.at_utc.with_timezone(&Local).time();
    let mut line = format!("mlm {verb} {}", local_time.format("%H:%M"));
    if p.date != today {
        line.push_str(&format!(" --date {}", format_date(p.date)));
    }
    line
}
```

**Correction on `verb`:** `PunchKind::as_str()` returns `"start"` /
`"end"`, but the CLI command for an end-punch is `mlm stop`, not `mlm
end` (`Command::Stop`, `src/cli.rs`) — `as_str()` and the CLI verb
diverge for the End case. The builder must **not** call `as_str()`
directly for the command name; match on `p.kind` explicitly instead:

```rust
fn punch_recreate_line(p: &Punch, today: NaiveDate) -> String {
    let verb = match p.kind {
        PunchKind::Start => "start",
        PunchKind::End => "stop",
    };
    let local_time = p.at_utc.with_timezone(&Local).time();
    let mut line = format!("mlm {verb} {}", local_time.format("%H:%M"));
    if p.date != today {
        line.push_str(&format!(" --date {}", format_date(p.date)));
    }
    line
}
```

(Use this second version — the first was a first-draft trap worth
flagging explicitly since `PunchKind::as_str()` looks reusable here
but silently produces `mlm end 09:00`, which is not a real subcommand.
A test in §9 below pins this: `punch_recreate_line`'s output for an
`End` punch must contain `"mlm stop "`, never `"mlm end "`.)

```rust
/// Build the note recreate-echo (spec §5/§7): `mlm note[ --date
/// YYYY-MM-DD] '<quoted body>'`, `--date` always preceding the body
/// (backdated-punches spec §2.1 footgun -- this printed string is fed
/// back into `mlm note`, which does have that ordering requirement,
/// even though `delete`'s own args don't, Task 2 §5). Defensive
/// fallback (spec §5, legacy pre-Milestone-13 rows only): if the
/// deleted body still contains a literal `\n` -- checked here, at echo
/// time, never assumed away -- print a plain description instead of a
/// broken/multi-line "command".
fn note_recreate_line(n: &Note, today: NaiveDate) -> String {
    if n.body.contains('\n') {
        let first_line = n.body.lines().next().unwrap_or("");
        return format!("deleted note ({}): {first_line}...", format_date(n.date));
    }
    let mut line = "mlm note".to_string();
    if n.date != today {
        line.push_str(&format!(" --date {}", format_date(n.date)));
    }
    line.push(' ');
    line.push_str(&quote_shell_single(&n.body));
    line
}
```

**Legacy-newline fallback — exact trigger and exact output, per spec
§5's current wording:**
- **Trigger:** `n.body.contains('\n')`, checked on the deleted row's
  body at echo time (after `storage::delete_note` has already
  returned it) — not inferred from anything else, not skipped because
  Milestone 13 "should" have normalized it away. This is the literal
  "checked at echo time, not assumed away" instruction from spec §5.
- **Output, quoted verbatim from spec §5:** `deleted note
  (2026-09-10): <first line of body>...` — i.e. the literal string
  `"deleted note ("`, the entry's date in `YYYY-MM-DD` form, `"): "`,
  the first line of the body (everything before the first `\n`), then
  the literal `"..."`. This is printed **instead of** a `mlm note ...`
  command — never a broken or multi-line "command" attempt.
- `n.body.lines().next()` correctly yields the text up to (not
  including) the first `\n` or `\r\n`; `.unwrap_or("")` only matters
  for a body that is empty or starts with the newline itself (an edge
  case `EmptyNote` validation should already prevent for any row
  written through `note()`, but this fallback path is specifically for
  *rows not written through validated `note()` at all* — direct
  storage inserts in tests — so the empty-first-line degenerate case
  is handled defensively rather than assumed impossible).
- This fallback path does **not** call `quote_shell_single` at all —
  it is a plain description, not a shell command, so no quoting logic
  applies to it.

## 8. Top-level handlers

**Placement:** `delete_note` directly after the existing `note()`
function (after line 40, before `resolve_target_date`), `delete_punch`
directly after `delete_note` — keeping the "single-entry-recording"
handlers (`start`/`stop`/`note`) and the new delete handlers adjacent,
mirroring how `cli.rs`'s Task 2 plan places `Command::Delete` right
after `Command::Note`.

```rust
/// List (no `ID`) or delete (`ID` given) that date's notes (spec §4/
/// §5). `ID` is always resolved against a freshly re-run
/// `storage::notes_for_date` on every invocation -- never a cached
/// listing from an earlier call (spec §5's stale-id note). On delete,
/// prints a ready-to-run recreate line (spec §5/§7); the note-body
/// quoting/footgun/legacy-newline handling for that line lives in
/// `note_recreate_line`.
pub fn delete_note(
    conn: &Connection,
    now: DateTime<Local>,
    args: &DeleteEntryArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;
    let rows = storage::notes_for_date(conn, target_date)?;

    let Some(id) = args.id else {
        if rows.is_empty() {
            println!("nothing to delete for {}.", format_date(target_date));
        } else {
            print_note_list(&rows);
        }
        return Ok(());
    };

    let index = validate_entry_id(id, rows.len())?;
    let deleted = storage::delete_note(conn, rows[index].id)?;
    println!("deleted. to recreate: {}", note_recreate_line(&deleted, today));
    Ok(())
}

/// List (no `ID`) or delete (`ID` given) that date's punches (spec
/// §4/§5). Same freshness/no-caching rule as `delete_note`. On
/// delete, prints a ready-to-run recreate line built from the deleted
/// row's own `at_utc` converted to local time per-instant
/// (`punch_recreate_line`) -- never `now`'s offset.
pub fn delete_punch(
    conn: &Connection,
    now: DateTime<Local>,
    args: &DeleteEntryArgs,
) -> anyhow::Result<()> {
    let today = now.date_naive();
    let target_date = resolve_target_date(&args.date, today)?;
    let rows = storage::punches_for_date(conn, target_date)?;

    let Some(id) = args.id else {
        if rows.is_empty() {
            println!("nothing to delete for {}.", format_date(target_date));
        } else {
            print_punch_list(&rows);
        }
        return Ok(());
    };

    let index = validate_entry_id(id, rows.len())?;
    let deleted = storage::delete_punch(conn, rows[index].id)?;
    println!("deleted. to recreate: {}", punch_recreate_line(&deleted, today));
    Ok(())
}
```

Notes on this exact shape:
- Both handlers are ~12 executable lines with a single early-return
  branch (`let Some(id) = args.id else { ... return Ok(()); }`) and one
  more `?`-chained validation — well under the cognitive-complexity
  budget precisely because list-printing, id validation, and
  recreate-line construction are all delegated to the helpers above.
  This is the concrete mechanism by which this plan satisfies the
  milestone plan's "keep the new handlers small by delegating to
  helper functions rather than raising that threshold" instruction.
- `resolve_future_checked_date` is reached transitively through the
  existing `resolve_target_date` helper (line 45-53) — reused
  unchanged, no new resolver call written here, satisfying the
  Global Constraint verbatim.
- Order of operations matches spec §7 exactly: (1) resolve date — a
  malformed/future `--date` errors out here, before any listing or
  deletion; (2) fetch rows; (3) branch on `args.id`; (4) validate id;
  (5) delete + echo.
- `conn: &Connection` (not `&mut Connection`) on both handlers, since
  `storage::delete_punch`/`storage::delete_note` (Task 1) only need a
  shared reference and no transaction spans multiple statements here.
- The "nothing to delete" message format matches `date::format_date`'s
  `YYYY-MM-DD` output exactly, per spec §4: `nothing to delete for
  <YYYY-MM-DD>.`.
- The success message format matches spec §5 exactly: `deleted. to
  recreate: <line>` where `<line>` is exactly what
  `punch_recreate_line`/`note_recreate_line` returns (no extra
  wrapping quotes around the whole line — only the note body itself is
  quoted, by `quote_shell_single`, inside `note_recreate_line`).

## 9. Tests to write first (TDD), named per this file's convention

All new tests go in the existing `#[cfg(test)] mod tests` block
(`src/commands.rs:109-949`), appended after the last existing test
(`backdated_punch_retroactively_changes_a_later_closed_weeks_owed`),
under a new `// --- delete: list/delete handlers (Milestone 14) ---`
comment banner matching the file's existing banner style (e.g. `// ---
backdated punches (backdated-punches spec §5) ---`). Two new extractor
helpers, `delete_note_args`/`delete_punch_args`, are added alongside
`punch_args`/`note_args`/`stop_args` (§1 above) before the first new
test that needs them. New tests use `today()`/`fixed_now()`/`d()`/
`test_db()`/`punches_for`/`notes_for` throughout — no hand-rolled
fixtures.

1. **`delete_note_future_date_is_rejected`** /
   **`delete_punch_future_date_is_rejected`** — `delete_note`/
   `delete_punch` called with `delete_note_args(&["--date",
   "2026-02-13"])` (no `id`, list-mode shape, since the milestone plan
   is explicit this must reject *before* any listing/deletion is
   attempted, not "nothing to delete"). Assert
   `err.downcast_ref::<crate::date::DateWeekError>().is_some()` and
   that no row exists on that date afterward (mirrors
   `future_dated_start_is_rejected`'s assertion shape exactly).
2. **`delete_note_malformed_date_is_rejected`** — same shape with
   `"--date", "not-a-date"`, mirroring `note_malformed_date_is_rejected`.
3. **`quoting_round_trip_leading_flag_lookalike`** — seed a note via
   direct `storage::insert_note` (not through `note()`, to control the
   exact body precisely) with body `"--verbose logging bug"` on
   `today()`. Call `delete_note` with `id: Some(1)`. Capture the
   printed recreate line is not directly assertable (no stdout capture
   fixture exists in this file) — **instead**, call
   `note_recreate_line` directly with the row `delete_note` returned
   from `storage::delete_note` (or, to test the actual integration,
   call `note_recreate_line` on a `Note` built inline with that body
   and `today()`), extract the trailing quoted token from the returned
   string, and feed it through `Cli::try_parse_from(["mlm", "note",
   <token>])` (splitting the recreate line the same way a real shell
   would: everything after `mlm note ` as a single arg since it's
   already one shell-quoted token conceptually — in the test, since
   there is no real shell, construct the argv directly as `["mlm",
   "note", &quote_shell_single("--verbose logging bug")]` is wrong
   because that passes the *quoted* string as one literal argv
   element, not what a shell would hand back after unquoting; instead
   assert the **shell-unquoting inverse**: strip the leading/trailing
   `'` and un-escape `'"'"'` back to `'` in the test, and confirm that
   equals the original body — this is the correct round-trip check
   without needing an actual subprocess shell). Document this
   reasoning as a comment in the test itself. Assert the recovered
   body equals `"--verbose logging bug"` exactly. **Why leading,
   not mid-sentence:** the underlying bug is position-dependent —
   clap's `trailing_var_arg` only re-checks the first token of the
   captured free-text region against registered flags; a mid-sentence
   flag-lookalike would pass this test identically whether or not the
   quoting fix exists, proving nothing.
4. **`quoting_round_trip_embedded_quote`** — same shape as #3, body
   `"it's a 'quoted' fix"`, confirming the `'"'"'`-escaped output
   un-escapes back to the exact original body.
5. **`punch_recreate_line_uses_stop_not_end_verb`** — direct unit test
   of `punch_recreate_line` (no DB): construct a `Punch { kind:
   PunchKind::End, .. }` inline, assert the returned string contains
   `"mlm stop "` and does not contain `"mlm end"`. Pins the
   `as_str()`-divergence correction from §7.
6. **`legacy_newline_body_falls_back_to_plain_description`** — seed a
   note directly via `storage::insert_note` (bypassing `note()`'s
   normalization entirely, simulating a pre-Milestone-13 row) with
   body `"line one\nline two"` on `d(2026, 2, 10)`. Call `delete_note`
   with `id: Some(1)`. Assert (via a direct call to
   `note_recreate_line` on the returned deleted row, same pattern as
   #3) the output is exactly `"deleted note (2026-02-10): line
   one..."` — not a `mlm note ...` line, no embedded second line.
7. **`list_mode_empty_date_prints_nothing_to_delete`** (note and
   punch, two tests or one parameterized) — `test_db()`, no rows for
   `today()`, call `delete_note`/`delete_punch` with
   `delete_note_args(&[])`/`delete_punch_args(&[])`; assert `Ok(())`
   and that `notes(&conn)`/`punches(&conn)` are still empty
   (list mode is a pure read, asserted via a follow-up read showing
   zero mutation even on the empty path).
8. **`list_mode_never_mutates_storage_and_numbers_correctly`** (note
   and punch) — seed two punches (`09:00` start, `17:00` end) /two
   notes on `today()` via `start`/`stop`/`note` (so ordering is
   real storage order, not hand-constructed), call `delete_punch`/
   `delete_note` with no `id`, assert `punches(&conn).len() == 2`/
   `notes(&conn).len() == 2` afterward (no mutation) — this test
   should call the handler and then separately verify the **printer
   helpers' output ordering** by calling `print_punch_list`/
   `print_note_list` isn't directly assertable via stdout either, so
   instead assert on the fetched `rows` vector itself (`storage::
   punches_for_date`/`notes_for_date` called the same way the handler
   does) that `rows[0]` is the `09:00 start` and `rows[1]` is the
   `17:00 end` (matching storage order) — this is the concrete
   "numbering/ordering/formatting matches storage order" check the
   milestone plan requires, expressed against the data the printer
   consumes since stdout itself isn't captured in this file's existing
   test harness.
9. **`delete_recreate_line_omits_date_for_today_includes_for_other_date`**
   (note and punch) — two sub-cases in one test: (a) seed+delete a
   same-day entry, assert the recreate line (via direct
   `punch_recreate_line`/`note_recreate_line` call on the deleted row)
   does **not** contain `"--date"`; (b) seed+delete an entry on
   `d(2026, 2, 10)` (a plain non-DST-crossing past date, distinct from
   the DST-specific test below), assert the line **does** contain
   `"--date 2026-02-10"`.
10. **`note_recreate_line_date_precedes_body`** — direct unit test of
    `note_recreate_line` with a `Note { date: d(2026, 2, 10), body:
    "fixed migration runner bug".into(), .. }` and `today = today()`
    (a different date, so `--date` is included). Assert, via
    `str::find`, that the byte index of `"--date"` is **less than**
    the byte index of the quoted body's opening `'`. A distinct
    assertion from the quoting round-trip tests (#3/#4) — this is
    purely an ordering check on the printed string, per the
    backdated-punches footgun.
11. **`delete_note_valid_id_removes_correct_row`** /
    **`delete_punch_valid_id_removes_correct_row`** — seed two entries
    on `today()`, delete `id: Some(1)`, assert (via
    `notes_for`/`punches_for`) exactly one row remains and it is the
    *second* seeded one (verified by field, e.g. body text or `at_utc`
    — a follow-up read, per spec §8).
12. **`delete_id_zero_and_past_count_are_rejected`** (note and punch)
    — seed one entry on `today()`; call with `id: Some(0)`, assert
    `Err` and the row still present (`len() == 1`); call with `id:
    Some(2)` (`N+1` for `N=1`), assert `Err` and the row still
    present. Both cases in one test per kind, since they share setup
    and both must show zero mutation — but assert each `Err` path
    separately (two distinct calls, two distinct assertions), per the
    milestone plan's "both paths are real and both need their own
    coverage."
13. **`delete_scoping_never_touches_other_date_with_same_count`** (note
    and punch) — seed one entry on `d(2026, 2, 10)` and one on
    `d(2026, 2, 11)` (both count 1, the "overlapping counts" case the
    spec calls out), delete `id: Some(1)` scoped to `--date 2026-02-10`
    via `delete_note_args(&["--date", "2026-02-10"])`... (with `id`
    appended, e.g. `&["1", "--date", "2026-02-10"]"`), assert
    `2026-02-10` is now empty and `2026-02-11` is unaffected (`len()
    == 1`, same body/kind as seeded).
14. **`dst_crossing_punch_delete_echoes_correct_local_time`** — mirror
    the DST fixture convention `storage.rs`'s existing DST tests use
    (`TZ_WARSAW`-style, or simpler: rely on the host's `Local` for a
    known DST-transition date — check `storage.rs`'s existing DST test
    setup, e.g. line ~967's `d(2026, 3, 29)` Warsaw spring-forward
    case, for the exact date/timezone-handling convention already
    established, and reuse the same date rather than inventing a new
    one). Seed a punch at a specific local wall-clock time on the far
    side of that DST boundary from `fixed_now()`'s date (2026-02-12),
    delete it, assert the recreate line's `HH:MM` matches the punch's
    actual local wall-clock time at insertion, not a time shifted by
    reusing `fixed_now()`'s (pre-transition) UTC offset. This is a
    `commands.rs`-level integration test (through `start`/`delete_punch`),
    not a repeat of `storage.rs`'s own DST unit tests — it proves the
    *recreate line*, specifically, uses per-instant conversion.
15. **`earliest_data_date_shift_changes_later_status`** — mirror the
    existing `backdated_punch_retroactively_changes_a_later_closed_weeks_owed`
    test's structure (drives `status::resolve` directly against the
    same connection): seed the *only* entry on what is currently the
    earliest tracked date, confirm via `storage::earliest_data_date`
    it is that date, delete it via `delete_punch`/`delete_note`,
    assert `storage::earliest_data_date` now returns a later date and
    a subsequent `status::resolve(...)` call's week line differs from
    before deletion (same "before"/"after" `status::resolve` pattern
    the existing backdated test already uses).
16. **`bare_delete_command_has_no_clap_equivalent_here`** — not
    required (Task 2 already covers clap-level parsing); skip. (Listed
    here only to note explicitly that no duplicate parse-level test
    belongs in this file — `commands.rs` tests call the handlers
    directly with constructed `DeleteEntryArgs`, matching how
    `punch_args`/`note_args` are used for `start`/`stop`/`note`'s own
    tests.)

Full list, cross-checked against the milestone plan's Task 3
acceptance criteria and spec §8: future-date rejection (#1/#2), quoting
round-trip leading-flag-lookalike (#3), quoting round-trip embedded
quote (#4), legacy-newline fallback (#6), list-mode empty (#7),
list-mode non-empty ordering/formatting (#8), `--date` omitted/included
(#9), `--date`-precedes-body ordering (#10), valid-id delete both kinds
(#11), out-of-range id both `0` and `N+1` (#12), cross-date scoping
with overlapping counts (#13), DST-crossing punch delete (#14),
earliest-data-date shift (#15). Every bullet from both the milestone
plan and spec §8 is covered by name above; `punch_recreate_line`'s
`stop`-not-`end` correction (#5) is an implementation-detail test this
plan adds beyond the spec's explicit list, since it is a concrete
bug this plan's own drafting surfaced (§7).

## 10. Verification

- Targeted, this task's new tests only (all live under the new
  banner comment; no shared name prefix guarantees a single `--`
  filter catches exactly this set, so run the whole module target
  instead):
  ```
  cargo test --lib commands::
  ```
- Full workspace suite (required before calling the task done, per the
  milestone plan's "Full existing `commands.rs` test suite still
  passes"):
  ```
  cargo test
  ```
- Both of this project's clippy gates, run and verified **separately**
  (quoted verbatim from the milestone plan's Task 3 verification
  step — passing one does not imply the other):
  - CI's plain gate:
    ```
    cargo clippy --all-targets -- -D warnings
    ```
  - The pre-commit hook's additional cognitive-complexity pass:
    ```
    cargo clippy --all-targets --quiet --message-format=json -- -W clippy::cognitive_complexity
    ```
    checked against `cognitive-complexity-threshold = 15` in
    `clippy.toml`. If `delete_note`/`delete_punch` (or any new helper)
    trips this, delegate further rather than raising the threshold —
    e.g. split the `let Some(id) = args.id else { ... }` empty-vs-list
    branch inside the handler into its own
    `print_list_or_empty_message(rows, date_label)`-style helper if
    needed.

## 11. Summary of new/changed items in `src/commands.rs` (for the
implementer)

- Imports: add `DeleteEntryArgs` (from `cli`), `format_date` (from
  `date`), `Note`, `Punch`, `StorageError` (from `storage`).
- New private helpers: `quote_shell_single`, `validate_entry_id`,
  `print_punch_list`, `print_note_list`, `punch_recreate_line`,
  `note_recreate_line`.
- New public handlers: `delete_note`, `delete_punch` (both `conn:
  &Connection`, not `&mut Connection`).
- New test-module additions: `delete_note_args`/`delete_punch_args`
  extractor helpers, ~16 new tests under a new banner comment (§9).
- No changes to any existing function, struct, or test.
