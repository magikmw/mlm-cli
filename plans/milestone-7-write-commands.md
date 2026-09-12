# Milestone 7 — `start`, `stop`, `note` commands

Implementation plan for the three write commands. Written to be handed
straight to a TDD subagent: every decision that would otherwise need a
clarifying question is made here, and the ones that genuinely need a
human sign-off are isolated in §8 and marked **DECISION**.

Spec basis: SPEC.md §3.1, §3.2, §3.3, §3.4, §4.1, §6.1, §6.3, §1.2,
§2.1, §2.3; flows F4, F5, E1, E5. PLAN.md Milestone 7, interface
contracts 6 and 7, cross-cutting "two-tier error handling", "exit
codes", "now as an injectable value".

Depends on **Milestone 1** (TIME parsing) and **Milestone 4**
(punch/note storage). Both are being built in parallel; everything
below is written against PLAN.md's stated contracts for them, not
against their code. §7 lists exactly what this milestone *requires*
those two milestones to expose — that list is the handoff and should be
reconciled with M1/M4's authors before implementation starts.

---

## 1. CLI surface (replaces the scaffold's `Command` enum)

### 1.1 What must be deleted

`src/cli.rs` currently defines:

- `Command::Start { note: Option<String> }` — wrong shape, no `TIME`.
- `Command::Stop { note: Option<String> }` — same.
- `Command::Log { date: Option<String> }` — **not in the spec at all.**
  SPEC.md §3 defines exactly six commands: `start`, `stop`, `note`,
  `status`, `week`, `week target`. There is no `log`. PLAN.md's "Open
  risks" section calls this out explicitly: `Log` is leftover scaffold,
  not a requirement.

**`Command::Log` must be removed outright in this milestone.** Do not
rename it to `status`, do not keep it as an alias, do not leave it
behind a `#[command(hide = true)]`. `status` is Milestone 10's
deliverable with a different argument (`[DATE]`) and a completely
different implementation; a surviving `Log` arm would be dead code that
a later milestone has to delete anyway.

`src/main.rs`'s `match` on `Command::Log` goes with it. Likewise
`AGENTS.md`'s "Verifying changes" block references `cargo run -- log`
— stale, but **out of scope for this milestone** (flagged in §8.9).

### 1.2 Proposed shapes

```rust
//! src/cli.rs

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mlm", version, about = "Quick time tracking from the CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Record a start punch for today.
    Start(PunchArgs),

    /// Record an end punch for today.
    Stop(PunchArgs),

    /// Record a work-log note for today.
    Note(NoteArgs),

    // Milestone 8 adds `Week`, Milestone 10 adds `Status`.
    // Nothing else exists yet; `Log` is gone.
}

/// Shared argument shape for `start` and `stop` (SPEC §3.2/§3.3 —
/// "same shape as start").
#[derive(Debug, clap::Args)]
pub struct PunchArgs {
    /// Time of day (HH:MM, HHMM or HH, 24h). Defaults to now.
    #[arg(value_name = "TIME")]
    pub time: Option<String>,

    /// Optional work-log note recorded for today alongside the punch.
    #[arg(value_name = "NOTE", trailing_var_arg = true, allow_hyphen_values = true)]
    pub note: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct NoteArgs {
    /// Work-log note text for today.
    #[arg(value_name = "NOTE", required = true, num_args = 1.., trailing_var_arg = true, allow_hyphen_values = true)]
    pub body: Vec<String>,
}
```

Notes on the shape:

- **`start` and `stop` share one `Args` struct** rather than two
  duplicated inline variants. §3.3 is literally "same shape as
  `start`"; one struct makes divergence impossible and lets both
  commands share a single handler parameterised by punch kind.
- **`TIME` stays a `String` at the clap layer**, not a
  `value_parser`-converted type. Reason: the parse failure must surface
  through *our* `AppError`/exit-code path with our own stderr wording
  (§3), not through clap's `ErrorKind::ValueValidation` formatting and
  clap's exit code 2. A `value_parser` would work but would make E1's
  message and exit code clap's business rather than ours, and would
  make the "no write happened" assertion untestable at the handler
  level. Keep clap dumb; validate in the handler.
- **`NOTE` is a `Vec<String>` joined with single spaces**, so both
  `mlm note "fixed the migration bug"` and
  `mlm note fixed the migration bug` work. `trailing_var_arg` +
  `allow_hyphen_values` let a note start with `-` or contain flag-like
  tokens without needing `--`. An empty `Vec` means "no note given" for
  `start`/`stop`; `required = true, num_args = 1..` makes it mandatory
  for `note` (§3.4 — `NOTE` is not optional there), so `mlm note` with
  no argument is a clap-level missing-argument error, same class as
  E10.
- Joining with a single space is lossy for multi-token input
  (`a    b` becomes `a b`); a note whose exact internal whitespace
  matters must be quoted. Documented deliberate behavior, see §8.6.

### 1.3 `TIME` vs `NOTE` positional disambiguation — the sharp edge

With two positionals, clap fills `TIME` first. So:

| invocation | `time` | `note` |
|---|---|---|
| `mlm start` | `None` | `[]` |
| `mlm start 9:05` | `Some("9:05")` | `[]` |
| `mlm start 9:05 fixed the bug` | `Some("9:05")` | `["fixed","the","bug"]` |
| `mlm start "fixed the bug"` | `Some("fixed the bug")` | `[]` → **hard error** |

The last row is the problem: SPEC §3.2 says both `TIME` and `NOTE` are
optional, which reads as though `mlm start "note text"` should be
accepted with `TIME` defaulting to now. It cannot be, not without
guessing.

**Decision taken here: strict positional order. The first positional is
always `TIME`.** `mlm start "fixed the bug"` is a malformed-`TIME`
hard error (E1), with a stderr message that names the cause explicitly
(see §3.4). A note-only start is expressed as two commands
(`mlm start` then `mlm note "..."`), or by giving the time
(`mlm start 9:05 "..."`).

Why not shape-sniff the first token (treat it as `TIME` only if it
looks like `\d{1,2}(:\d{1,2})?` or `\d{3,4}`, else as note text)?
Because it directly contradicts E1: `mlm start abc` must hard-error,
and under shape-sniffing `abc` would silently become a note. Any
sniffing rule that satisfies E1's `abc` case is a rule that also
swallows real notes. Strict order satisfies every one of E1's four
listed inputs (`25:00`, `24:00`, `9:75`, `abc`) with no special cases.
The cost is the `mlm start "note"` ergonomic gap — raised as
**DECISION** in §8.1.

---

## 2. Resolving "now" and "today"

Two separate resolutions, deliberately not routed through the same
code path.

### 2.1 "Now" is injected, never read inside a handler

Per PLAN contract 6, no handler reads the clock. `main` reads it once
and passes it down:

```
main:   let now: DateTime<Local> = Local::now().with_second(0).unwrap().with_nanosecond(0).unwrap();
```

Seconds and nanoseconds are **truncated to zero at the point of
capture** (§4.1: "There is no seconds precision anywhere — everything
is minute-granular"). Truncate once, in `main`, so every downstream
consumer (punch instant, note `created_at_utc`, today's date) sees the
same minute-granular instant. Truncation is toward the past (plain
zeroing, never rounding to the nearest minute) — a punch at 09:05:59 is
`09:05`, matching how a user reading their own clock would describe it.

Handler signatures therefore take `now` as a parameter (§4.1 of this
document). Tests pass a fixed `DateTime<Local>` and get fully
deterministic `at_utc` / `date` / `created_at_utc` values.

### 2.2 Today's local date

`today = now.date_naive()` — the local calendar date, computed directly
from the injected `now`. It is **never** derived from a UTC instant by
string slicing (§2.1 forbids exactly that), and it is never taken from
a `TIME` argument: §1.2 and NOTES.md decision 24 are unambiguous that
`start`/`stop`/`note` only ever target today, no backdating, no
exceptions. There is no code path in this milestone where `today` comes
from user input.

### 2.3 What goes to Milestone 1's parser vs. what is computed directly

| case | path |
|---|---|
| `TIME` **given** | the raw string goes to M1's `TIME` parser → a `NaiveTime` (hour, minute, seconds 0). Combined with `today` → `NaiveDateTime`. |
| `TIME` **omitted** | M1's parser is **not called at all**. `now.time()` is used directly (already second-truncated per §2.1). |

The omitted case deliberately does **not** format `now` back into a
`"HH:MM"` string and re-parse it. Round-tripping through the parser
would be pointless work and would create a second, subtly different
definition of "now" (e.g. a formatting/parsing disagreement at
midnight). `now` is already a typed value; use it.

Both cases converge on the same downstream shape:

```
local_wall: NaiveDateTime = today.and_time(time_of_day)
```

which is what gets handed to Milestone 4's punch insert. **Milestone 4
owns the local→UTC conversion and the `date` column computation**
(PLAN M4 scope: "given a `kind` and a local wall-clock time already
parsed by Milestone 1, converted to UTC and to a local calendar `date`
at write time per §2.1"). Milestone 7 does not do timezone math. It
hands over a local wall-clock `NaiveDateTime` plus the already-known
local `date`, and M4 does the rest.

DST edge: `today.and_time(t)` can be ambiguous (fold) or nonexistent
(gap) in the local zone. That resolution belongs to M4's
`Local.from_local_datetime` handling — see §8.5 for what M7 needs from
it.

---

## 3. Error surfacing: hard error → nonzero exit + stderr + no write

### 3.1 Shared error type (PLAN interface contract 7)

A single crate-wide error enum lives in a new `src/error.rs`, owned by
whichever milestone lands first and shared verbatim by M1, M2, M4, M7,
M8. Proposed shape:

```rust
//! src/error.rs

#[derive(Debug)]
pub enum AppError {
    /// §6.1 — malformed TIME: bad shape, out-of-range, or 24:00.
    InvalidTime { input: String, reason: TimeErrorKind },
    /// §6.1 — malformed DATE (Milestone 2/10).
    InvalidDate { input: String },
    /// §6.1 — malformed WEEK_ID (Milestone 2/8).
    InvalidWeekId { input: String },
    /// §6.1 — malformed or negative DURATION (Milestone 1/8).
    InvalidDuration { input: String },
    /// §6.1 — empty/whitespace-only NOTE body.
    EmptyNote,
    /// §6.1 — DB open/migration failure, or any query failure (E6).
    Db(rusqlite::Error),
    /// §6.1 — app-data dir could not be determined/created (E6).
    Storage { context: String, source: std::io::Error },
}

impl std::fmt::Display for AppError { /* one-line, plain ASCII, no trailing newline */ }
impl std::error::Error for AppError { /* source() forwards Db/Storage */ }
impl From<rusqlite::Error> for AppError { /* -> AppError::Db */ }
```

Requirements on this type that Milestone 7 depends on:

- `Display` produces **one plain-ASCII line, no trailing newline, no
  `error:` prefix** (the prefix is added once at the top level, §3.3),
  so the same value can be embedded in other contexts later.
- It is a plain enum, not a boxed `dyn Error` — handler tests need to
  match on variants (`assert!(matches!(err, AppError::EmptyNote))`)
  without string-matching.
- It carries the offending input where there is one, so messages can
  quote it back.

If M1/M4 land a different error type first, Milestone 7 adapts via
`From` impls rather than re-litigating; the only hard requirement is
that M7 can distinguish "invalid time" from "empty note" from "db
failure" by pattern match.

### 3.2 Handler signatures

Each command handler is a plain function returning `Result<(), AppError>`
— no printing of errors inside, no `process::exit` inside, no panics.

```rust
//! src/commands.rs  (new module)

pub fn start(conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> Result<(), AppError>;
pub fn stop (conn: &mut Connection, now: DateTime<Local>, args: &PunchArgs) -> Result<(), AppError>;
pub fn note (conn: &mut Connection, now: DateTime<Local>, args: &NoteArgs)  -> Result<(), AppError>;
```

`start`/`stop` are one-line wrappers over a shared

```rust
fn punch(conn: &mut Connection, now: DateTime<Local>, kind: PunchKind, args: &PunchArgs) -> Result<(), AppError>;
```

`&mut Connection` (not `&Connection`) because the punch+note pair runs
in a `rusqlite::Transaction`, which requires a mutable borrow of the
connection. See §4 and §7.2.

### 3.3 Top-level control flow in `main.rs`

```rust
fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    // clap handles --help/--version/missing-arg itself and exits(2) on error.
    let cli = Cli::parse();

    let now = Local::now().with_second(0).unwrap().with_nanosecond(0).unwrap();

    match dispatch(&cli, now) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn dispatch(cli: &Cli, now: DateTime<Local>) -> Result<(), AppError> {
    let mut conn = db::connect()?;          // E6: returns Err, never panics
    match &cli.command {
        Command::Start(a) => commands::start(&mut conn, now, a),
        Command::Stop(a)  => commands::stop(&mut conn, now, a),
        Command::Note(a)  => commands::note(&mut conn, now, a),
    }
}
```

Points that are load-bearing and must not be "simplified" away:

- **`fn main()` returns `()`, not `Result`.** A `Result`-returning main
  prints the error with `Debug` formatting (`InvalidTime { input:
  "25:00", .. }`) and exits 1. We want `Display`, on stderr, prefixed
  once. Hence the explicit `run() -> i32` + `process::exit`.
- **Exit code is 1 for every §6.1 hard error.** §6.3 only requires
  "nonzero", and a single code keeps the surface small. Note that
  clap's own errors (missing `NOTE` on `mlm note`, unknown subcommand,
  bad flag) exit **2**, clap's default — also nonzero, also correct per
  §6.3, and consistent with how E10 describes a "clap-level
  missing-argument error". Tests assert `!= 0`, not `== 1`, wherever
  the error might come from clap.
- **`db::connect()` must not panic.** The current scaffold does:
  `data_dir()` uses `.expect("could not determine application data
  directory")` and `connect()` uses `.expect("could not create
  application data directory")`, and `main` uses `.expect("failed to
  open database")`. All three are E6 violations — a permissions failure
  today aborts with a panic message and exit code 101 plus a backtrace
  hint, not a clean stderr line. Milestone 3 owns fixing `db.rs`;
  **Milestone 7 owns removing `main.rs`'s `.expect`** and must not
  reintroduce a panic path. If M3 has not landed the `Result`-returning
  `connect()` yet, M7 wraps the existing one but leaves a `TODO`
  pointing at M3 rather than papering over it.
- `dispatch` connects to the DB *before* dispatching, so E6 is uniform
  across all three commands. This does mean a malformed `TIME` still
  opens the DB first; that is harmless (opening is not writing) and
  keeps the connect/dispatch split simple. Tests for E1 assert "no
  rows", not "DB never opened".

### 3.4 Message wording (stderr, plain ASCII, §7)

Concrete strings, so tests can assert on them:

| condition | stderr line |
|---|---|
| malformed `TIME` shape | `error: invalid TIME 'abc': expected HH:MM, HHMM or HH (24-hour). TIME must come before NOTE.` |
| out-of-range `TIME` | `error: invalid TIME '25:00': hour must be 00-23 and minute 00-59.` |
| `24:00` specifically | same out-of-range wording (§6.1: `24:00` is rejected, not a midnight alias). |
| empty/whitespace `NOTE` | `error: NOTE is empty: a note must contain at least one non-whitespace character.` |
| DB failure | `error: database error: <source>` |

The "TIME must come before NOTE" tail on the shape-mismatch case is the
mitigation for §1.3's ergonomic gap: a user who typed
`mlm start "fixed the bug"` gets told why. Tests should assert on a
stable substring (e.g. `invalid TIME`, `NOTE is empty`) rather than the
full sentence, so wording can be tuned without breaking the suite.

---

## 4. Composing punch + note atomically

### 4.1 Two independent guarantees, both implemented

E5's negative case is explicit: an invalid `NOTE` attached to
`start`/`stop` must leave **no punch behind**. Two mechanisms, applied
together — validate-first is what actually makes the tests pass, the
transaction is what keeps it true when something unexpected fails.

**(a) Validate everything before writing anything.** Both hard errors
in this milestone (§6.1: malformed `TIME`, empty `NOTE`) are decidable
purely from the input, with no DB access. So the handler is strictly
ordered:

```
1. resolve kind (start|end)                     -- infallible
2. resolve time_of_day:
     Some(s) => m1::parse_time(s)?              -- E1 exits here, nothing written
     None    => now.time()
3. local_wall = today.and_time(time_of_day)
4. note_text = join(args.note, " ")
   if !note_text.is_empty():
       validate_note_body(&note_text)?          -- E5 exits here, nothing written
5. ---- first write happens only past this line ----
```

Steps 1-4 touch no storage. By construction, a rejected note cannot
leave an orphaned punch, because the punch insert has not been reached.
**`TIME` is validated before `NOTE`** — if both are bad, the user sees
the `TIME` error. (Arbitrary but fixed, so tests are deterministic; see
test T13.)

**(b) Wrap the pair in a real transaction anyway.** Validation covers
*invalid input*; it does not cover the note insert failing for a reason
we did not predict (disk full, a constraint we add later, a busy lock
from a concurrent invocation, a future validation rule added inside
M4). §6 is categorical: a hard error writes nothing. So:

```
let tx = conn.transaction()?;              // rusqlite::Transaction
storage::insert_punch(&tx, kind, local_wall, today)?;   // `?` -> early return, tx drops, ROLLBACK
if let Some(body) = note_text {
    storage::insert_note(&tx, today, &body, now)?;      // `?` -> ROLLBACK, punch undone
}
tx.commit()?;
```

`rusqlite::Transaction`'s `Drop` rolls back by default, so an early
return via `?` undoes the punch with no explicit rollback call. This is
the "actual DB transaction" half of the question, and it is cheap
enough that there is no reason to pick one mechanism over the other.

A single-row `note` command also runs inside a transaction, for
uniformity — no behavioral difference, but it keeps all three handlers
on one shape.

### 4.2 Consequence for Milestone 4's API

For (b) to be possible, **M4's insert functions must borrow a
connection-like handle rather than owning a connection or opening their
own**. `rusqlite::Transaction` derefs to `&Connection`, so a signature
of `fn insert_punch(conn: &Connection, ...)` composes correctly inside
a transaction and `&tx` passes directly. A design where M4 exposes
`struct Storage { conn: Connection }` with `fn insert_punch(&self, ...)`
would make the two inserts impossible to put in one transaction without
rework. **This is the single most important thing to confirm with M4
before implementing** — see §7.2.

### 4.3 Note body validation ownership

§6.1: "whitespace-only text is rejected rather than stored as a blank
log line (checked *before* the trim in §2.3)". Concretely:
`body.trim().is_empty()` → `Err(AppError::EmptyNote)`; otherwise store
`body.trim()`.

M4 already owns this rule (its acceptance criteria include rejecting an
empty/whitespace body). M7 needs the check *before* any write, so M4
must expose the predicate separately from the insert:

```rust
pub fn validate_note_body(body: &str) -> Result<&str /* trimmed */, AppError>;
```

`insert_note` calls it internally too (idempotent, defense in depth).
If M4 does not expose it, M7 implements the identical two-line check
locally and a follow-up consolidates — but duplicating the rule is the
worse outcome and should be raised with M4's author, not silently
accepted.

### 4.4 Shared timestamp

The punch's instant and the note's `created_at_utc` both derive from
the same injected `now`:

- punch `at_utc` = UTC of `today.and_time(time_of_day)` — which may be
  hours away from `now` if a `TIME` was given.
- note `created_at_utc` = UTC of `now` — **insertion order, not the
  punch time** (§2.3: "insertion-order tiebreaker for same-day notes").
  A `mlm start 09:00 "morning standup"` typed at 14:00 records a 09:00
  punch and a note created at 14:00, which sorts correctly against
  other notes typed that afternoon. Do not pass the punch time here.
- note `date` = `today` — same date as the punch, by §2.3 (a note is
  attached to a day).

---

## 5. Test plan

### 5.1 Test harness shape

**Primary: in-process handler tests.** Call `commands::start/stop/note`
directly with (a) a `Connection` opened against a temp file or
`:memory:` with migrations applied, and (b) a fixed
`DateTime<Local>`. Assert on the returned `Result` and on rows read
back through M4's read functions (or raw SQL counts, if M4's reads are
not ready). This needs no new dependencies, gives exact control of
"now", and makes "nothing was written" directly assertable.

Argument-shape assertions use `Cli::try_parse_from([...])` — no process
spawn needed to prove `start 9:05 fixed the bug` splits correctly.

Exit-code assertions use a thin, separately-testable mapping function
rather than spawning the binary:

```rust
pub fn exit_code(result: &Result<(), AppError>) -> i32 { if result.is_ok() { 0 } else { 1 } }
```

`run()` calls it; tests call it too.

**Secondary (optional, 2-3 smoke tests only): binary-level tests.**
These require (i) dev-dependencies `assert_cmd`, `predicates`,
`tempfile`, and (ii) a DB-path override so tests do not write to the
real app-data dir — propose an `MLM_DB_PATH` environment variable
honoured by `db::connect()`. Both are project-level additions that also
unblock the cross-cutting E6 end-to-end check PLAN.md demands in wave
5, so they are worth adding — but they are **not** this milestone's
critical path, and "now" cannot be injected across a process boundary,
so the deterministic cases must live in the in-process suite regardless.
See §8.7.

Where the tests live: `tests/write_commands.rs` (integration style,
exercising the public handler API) plus a small `#[cfg(test)]` module in
`cli.rs` for the pure parse-shape cases.

Fixed clock used throughout: `2026-02-12T14:23:00` local (a Thursday —
matches the spec's worked examples).

### 5.2 Enumerated cases

Punch/note row counts below mean "rows in `punches` / rows in `notes`
for today".

**Happy paths**

| # | name | invocation | assert |
|---|---|---|---|
| T1 | `start_no_args_uses_now` | `start` | Ok; 1 punch, kind `start`, `at_utc` == UTC of the fixed now (seconds zero), `date` == `2026-02-12`; 0 notes. |
| T2 | `start_with_time` | `start 9:05` | Ok; 1 punch at local 09:05 today, kind `start`; 0 notes. |
| T3 | `start_with_time_forms` | `start 0905`, `start 9`, `start 17:30` (separate DBs) | Ok; punches at 09:05, 09:00, 17:30 respectively. Proves M1's grammar is reached unmodified. |
| T4 | **F5** `start_with_time_and_note` | `start 9:05 "kicked off migration work"` | Ok; **1 punch AND 1 note**; punch `at_utc` = 09:05 local→UTC; note `body` == the text, note `date` == today, note `created_at_utc` == UTC of now (14:23), **not** 09:05. |
| T5 | `start_with_unquoted_multiword_note` | `start 9:05 kicked off migration work` | Ok; note body == `kicked off migration work` (joined with single spaces). |
| T6 | **stop mirrors start** `stop_all_shapes` | `stop`, `stop 17:30`, `stop 17:30 "wrapped up"` | Ok; punches have kind `end` in every case; the note case writes 1 punch + 1 note. Assert `stop`'s behavior is identical to `start`'s except `kind` — ideally a parameterised test over both. |
| T7 | **F4** `note_only_standalone` | `note "fixed migration runner bug"` | Ok; **0 punches**, 1 note with today's date and the trimmed body. |
| T8 | `note_padded_body_is_trimmed` (E5 negative case) | `note "  did a thing  "` | Ok; 1 note, stored body == `did a thing`. |
| T9 | **§6.3** `success_exits_zero` | each of T1, T4, T6, T7 | `exit_code(&result) == 0`. Include one case that produces a §4.3 anomaly (T12) to confirm anomalies never affect the exit code. |

**No-chronology acceptance (§3.2)**

| # | name | invocation | assert |
|---|---|---|---|
| T10 | `start_before_existing_start_accepted` | `start 14:00` then `start 09:00` | both Ok; 2 punches stored, both kind `start`. No ordering validation, no error, no warning. |
| T11 | `stop_before_its_start_accepted` | `start 14:00` then `stop 09:00` | both Ok; 2 punches stored. Pairing weirdness is Milestone 5's problem to *surface*, never this milestone's to *reject* (§6.2). |
| T12 | `f3_entry_order_accepted_verbatim` | `start 09:00`, `start 14:00`, `stop 18:00`, `stop 13:00` | all four Ok, exit 0; exactly 4 punch rows with the four expected instants and kinds, in insertion order by `id`. (This is F3's *input* half; the pairing assertion belongs to M5.) |

**Hard errors — E1, malformed TIME**

| # | name | invocation | assert |
|---|---|---|---|
| T13 | `malformed_time_rejected` — table-driven over `25:00`, `24:00`, `9:75`, `abc`, `9:5:5`, `""` | `start <X>` and `stop <X>` | `Err(AppError::InvalidTime { .. })`; **0 punches and 0 notes**; `exit_code != 0`; stderr message (via `Display`) contains `invalid TIME`. |
| T14 | `malformed_time_with_note_writes_nothing` | `start 25:00 "some note"` | Err; **0 punches AND 0 notes** — the note must not sneak in either. |
| T15 | `time_error_precedes_note_error` | `start 25:00 "   "` | Err is `InvalidTime`, not `EmptyNote` (documents §4.1's fixed validation order); 0 punches, 0 notes. |
| T16 | `note_positional_after_time_is_never_reparsed_as_time` | `start 9:05 25:00` | Ok; punch at 09:05, note body `25:00`. The second positional is note text, full stop. |

**Hard errors — E5, empty/whitespace NOTE**

| # | name | invocation | assert |
|---|---|---|---|
| T17 | `standalone_empty_note_rejected` — table over `""`, `"   "`, `"\t"`, `"\n"`, `" \t \n "` | `note <X>` | `Err(AppError::EmptyNote)`; 0 notes; exit != 0; message contains `NOTE is empty`. |
| T18 | **E5 core** `start_with_empty_note_leaves_no_punch` — same table | `start 9:05 <X>` | Err(`EmptyNote`); **0 notes AND 0 punches**. This is the orphaned-punch guard; it is the single most important assertion in this milestone. |
| T19 | `stop_with_empty_note_leaves_no_punch` | `stop 17:30 "   "` | same as T18 with kind `end`. |
| T20 | `empty_note_does_not_disturb_existing_rows` | seed `start 09:00` (Ok), then `start 10:00 "   "` (Err) | after the failure the DB still holds exactly the 1 seeded punch and 0 notes — the rollback undoes only the failed command's write, and does not touch prior data. |
| T21 | `bare_note_command_is_clap_error` | `Cli::try_parse_from(["mlm","note"])` | `Err`, `ErrorKind::MissingRequiredArgument`; nothing reaches a handler (E10's class, applied to §3.4). |

**Transactional guarantee (mechanism, not just outcome)**

| # | name | setup | assert |
|---|---|---|---|
| T22 | `note_insert_failure_rolls_back_punch` | force the note insert to fail at the DB level *after* validation — e.g. run against a connection where `notes` has been dropped/renamed, or inject a failing storage seam | Err (`Db`); **0 punches** — proves (b) in §4.1 independently of (a). If no clean seam exists, this may be marked `#[ignore]` with a comment rather than dropped; the guarantee should still be exercised at least once. |

**Parse-shape unit tests (`cli.rs`)**

| # | name | assert |
|---|---|---|
| T23 | `parse_start_variants` | `["mlm","start"]` → time `None`, note `[]`; `["mlm","start","9:05"]` → time `Some("9:05")`, note `[]`; `["mlm","start","9:05","a","b"]` → note `["a","b"]`. |
| T24 | `parse_note_joins_tokens` | `["mlm","note","a","b"]` → body `["a","b"]`. |
| T25 | `note_can_start_with_hyphen` | `["mlm","note","-ish","progress"]` parses as note text, not as a flag (proves `allow_hyphen_values`). |
| T26 | `log_subcommand_is_gone` | `Cli::try_parse_from(["mlm","log"])` is `Err` — a regression guard so the scaffold command cannot creep back. |

---

## 6. Implementation checklist (suggested order)

1. `src/error.rs` — `AppError` (or adopt M1/M4's, per §7).
2. `src/cli.rs` — replace `Command` wholesale; delete `Log`.
3. `src/commands.rs` — new module: `start`, `stop`, `note`, shared `punch`.
4. `src/main.rs` — `run() -> i32` + `process::exit`, remove all
   `.expect`, remove the `Log` arm, add `mod commands; mod error;`.
5. Tests per §5.

Files touched: `src/cli.rs`, `src/main.rs`, new `src/commands.rs`, new
`src/error.rs` (if not already landed), new `tests/write_commands.rs`.
`src/db.rs` and `src/time.rs` are **not** this milestone's to rewrite
(M3 and M1 own them) — except that `main.rs` must stop calling
`db::connect().expect(...)`.

---

## 7. What this milestone needs from M1 and M4

### 7.1 From Milestone 1

- A `TIME` parser taking `&str` and returning `Result<NaiveTime,
  AppError>` (or a type convertible into `AppError::InvalidTime`),
  implementing the full §3.1 grammar including the `24:00` rejection.
  M7 calls it once and does nothing else with time strings.
- That the returned value has seconds == 0.

### 7.2 From Milestone 4 — **confirm before implementing**

- `insert_punch` and `insert_note` take a **borrowed** connection
  (`&Connection`) so both can run inside one `rusqlite::Transaction`
  (§4.2). A `Storage`-struct-owns-`Connection` design breaks this.
- `insert_punch` accepts the local wall-clock `NaiveDateTime` (+ the
  local `date`) and owns the UTC conversion and `date` column.
- `insert_note` accepts `(date, body, created_at)` with `created_at`
  supplied by the caller, not read from a global clock (contract 6).
- `validate_note_body` (or equivalent) is exposed separately from
  `insert_note` (§4.3).
- Errors surface as the shared `AppError`, distinguishable between
  "empty note" and "db failure" (contract 7).

---

## 8. Ambiguities, risks, disagreements

**8.1 DECISION — `mlm start "note"` with no TIME is not expressible.**
SPEC §3.2 declares both `TIME` and `NOTE` optional, which a reader
naturally takes to mean "note without time works". With plain
positionals it cannot, and every disambiguation heuristic that makes it
work breaks E1's `abc` case (§1.3). This plan takes strict positional
order and turns `mlm start "note"` into a clear hard error. Options if
that is unacceptable: (a) accept the literal token `now` as a `TIME`
value meaning current time, so `mlm start now "note"` works — small,
unambiguous, but an extension to §3.1's grammar needing a spec edit;
(b) move `TIME` to a flag (`mlm start -t 9:05 "note"`) — unambiguous
but contradicts §3.2's positional signature; (c) accept the heuristic
and weaken E1 to only the numeric-shaped inputs. **Recommendation: ship
strict order now (it satisfies 100% of the written acceptance
criteria), and raise (a) as a separate spec amendment.** Do not silently
implement (a) or (c) inside this milestone.

**8.2 E1's `abc` is only a hard error because of 8.1's choice.** Worth
recording: under any note-sniffing scheme, `mlm start abc` becomes a
one-word note and E1's fourth example stops being reachable from the
CLI at all (it would only be testable at M1's unit level). The two
decisions are the same decision.

**8.3 Success output is unspecified.** SPEC.md never says what
`start`/`stop`/`note` print on success; PLAN M7 says "no output
rendering beyond whatever minimal confirmation is needed". Proposal,
for determinism: one plain-ASCII line on **stdout**, using the §4.2
conventions where a duration would appear (none appear here):
`start 09:05` / `stop 17:30` / `note added`, and for a punch with an
inline note, one line: `start 09:05 (note added)`. Tests should assert
exit code and DB state, and at most that stdout is non-empty — not the
exact phrasing — so Milestone 10/11 can harmonise wording later. If the
preference is total silence on success (unix-y), that is equally valid;
**pick one before writing tests**.

**8.4 Seconds truncation is an inference, not an explicit rule for the
"now" path.** §4.1 says "no seconds precision anywhere", which this
plan implements by zeroing seconds at capture (§2.1). The alternative —
storing the true instant with seconds and only truncating at display —
would make `at_utc` values non-minute-aligned and would quietly
contradict §4.1. Flagging because it is a storage-visible decision that
M4's tests may also assume; M4 and M7 must agree. Recommend: truncate
at capture (here), and have M4 additionally normalise defensively.

**8.5 DST gap/fold on today's wall clock.** `mlm start 2:30` on a
spring-forward day names a local time that does not exist; on a
fall-back day it names one that occurs twice. `Local.from_local_datetime`
returns `LocalResult::None` / `::Ambiguous`. §2.1 and §6.1 say nothing
about this. Proposal: `Ambiguous` → take the **earlier** offset (the
first occurrence, matching "what the user's clock showed first");
`None` → treat as a §6.1 hard error with a message naming the gap
(`error: 02:30 does not exist on 2026-03-29 in your timezone (clocks
skipped forward)`). This is M4's code but M7's error surface, so it
needs deciding jointly. It is a once-a-year edge case and should not
block the milestone, but it must not be an `unwrap()` — a panic here
violates §6.

**8.6 Multi-token note whitespace normalisation.** `mlm note a    b`
stores `a b`. Consistent with every shell tool that joins argv, and
`§2.3`'s "no length cap or charset restriction" does not speak to
interior whitespace. Documented, low risk. If exactness matters the
user quotes.

**8.7 Test-harness / DB-path risk.** `db::connect()` currently resolves
the real platform app-data dir unconditionally. Any binary-level test
would write to the developer's actual `~/.local/share/mlm/mlm.db`. An
`MLM_DB_PATH` override (or an explicit-path `connect_at()`) is needed
before binary-level tests exist — and PLAN.md's wave-5 cross-cutting
pass already requires binary-level E6 testing, so this is a project-wide
gap, not a M7-only one. M7 works around it with in-process tests and
should file the override as M3's or the cross-cutting pass's work.

**8.8 Two different nonzero exit codes for "no note".** `mlm note`
exits 2 (clap, missing argument); `mlm note "  "` exits 1 (our
`EmptyNote`). Both satisfy §6.3 and both are correct in their own
layer, but it is a visible inconsistency worth knowing about. Not worth
forcing clap to exit 1.

**8.9 Stale `AGENTS.md`.** Its "Verifying changes" block documents
`cargo run -- start "note"` (the old note-only-positional shape, which
§1.3 now makes an error) and `cargo run -- log` (a command being
deleted). Both become wrong the moment this milestone lands. Out of
scope here — flagging so whoever does the docs pass catches it. Same
for `AGENTS.md`'s "entries table" description, which Milestone 3
invalidates.

**8.10 `main.rs`'s `.expect("failed to open database")` is a live E6
violation** and is the one non-`cli.rs`/`commands.rs` change this
milestone must make (§3.3). If Milestone 3 has not yet landed a
`Result`-returning `connect()` that also handles the `ProjectDirs` and
`create_dir_all` failures, M7's fix is partial by necessity — note the
remaining panic paths in `db.rs` rather than claiming E6 is covered.
