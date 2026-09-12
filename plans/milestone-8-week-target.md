# Milestone 8 — `mlm week target [WEEK_ID] DURATION`

Implementation plan, written to be handed straight to a TDD subagent.
Spec sources: SPEC.md §3.7 (command), §3.6 (WEEK_ID forms), §4.2
(DURATION input grammar), §2.3 (`week_targets` table), §6.1 (hard
errors), §6.3 (exit codes), §8's F7 / F7b / E3 / E4 / E9 / E10.
Plan source: PLAN.md "Milestone 8", wave 2, contracts 2/6/7.

**Dependencies (per PLAN.md): Milestone 2 (WEEK_ID parsing) and
Milestone 3 (schema) only.** Milestone 1 supplies the DURATION parser,
which PLAN.md's Milestone 8 scope line also names ("on top of
Milestones 1, 2, and 3"); the wave-2 bullet at the top of PLAN.md names
only 2 and 3 — see §7 Ambiguity A1. This milestone does **not** call
Milestone 6's accounting; it only writes a row.

---

## 1. Assumed upstream contracts

Design against these shapes, not against the other milestones' actual
code. If a real signature differs at integration time, only the thin
adapter at the top of `cmd_week_target` changes.

### 1.1 From Milestone 1 (`src/time.rs`)

```rust
/// Parses the DURATION input grammar (§4.2): `20h`, `33h30m`, `45m`, `0h`.
/// Returns whole minutes. Rejects `10` (no unit), `10x`, `-5h`, empty.
pub fn parse_duration(s: &str) -> anyhow::Result<i64>;
```

Milestone 8 assumes the returned value is **already guaranteed
non-negative** (Milestone 1's acceptance criteria: "a negative parsed
minute value is rejected even if the grammar otherwise matches").
Milestone 8 still re-asserts `>= 0` defensively — see §3.4.

### 1.2 From Milestone 2 (`src/date.rs` or wherever it lands)

```rust
/// Normalized ISO week identity. `Display`/`as_str` yields `YYYY-WW`,
/// zero-padded, no `W` — exactly the `week_targets.week_id` shape (§2.3).
pub struct WeekId { /* iso_year: i32, iso_week: u32 */ }

/// Parses `YYYY-WW`, `YYYY-W` (unpadded), or a bare `WW` / `W`.
/// `current_year` supplies the year for the bare form.
/// Rejects `0`, `abcd`, and a week number that is not a valid ISO week
/// for its year (`2027-53` when 2027 has 52).
pub fn parse_week_id(s: &str, current_year: i32) -> anyhow::Result<WeekId>;

/// The ISO week containing the given local date.
pub fn week_of(date: NaiveDate) -> WeekId;
```

If Milestone 2 instead exposes `parse_week_id(s: &str, now: NaiveDate)`
or a `WeekId::current(now)`, adapt at the call site only.

### 1.3 From Milestone 3 (`src/db.rs`)

```sql
CREATE TABLE week_targets (
    week_id        TEXT PRIMARY KEY,
    target_minutes INTEGER NOT NULL CHECK (target_minutes >= 0)
);
```

Plus a connection helper. **Milestone 8 needs a test-addressable
connect** — see §7 Risk R1.

### 1.4 Contract 6 — "now" is injected

`cmd_week_target` takes the current local date as a parameter
(`today: NaiveDate`), never reading a global clock itself. `main`
passes `Local::now().date_naive()`. This is what makes the
"omitted WEEK_ID defaults to current week" test deterministic.

### 1.5 Contract 7 — error shape — resolved: `anyhow`, no shared enum

**Superseded.** This section originally proposed a shared crate-wide
`AppError` enum. The project has since settled on `anyhow` (PLAN.md
contract 7, and `anyhow` is now a real dependency — the "do not add
`thiserror` unilaterally" concern below is moot, the dependency
decision was made at the project level, just with `anyhow` instead).

- Milestone 2's `WeekIdParseError` and Milestone 1's
  `DurationParseError` remain concrete, precise `std::error::Error`
  types — this milestone does not touch or wrap them.
- This milestone's own command handler returns `anyhow::Result<()>`.
  `?` on either parse error, or on a `rusqlite`/`DbError` failure,
  auto-converts via `anyhow`'s blanket `From` impl — no manual
  `From`/`Display` plumbing to write here.
- `main` prints the propagated `anyhow::Error`'s `Display` to
  **stderr** and exits **nonzero** (§6.3). Use exit code `1` for all
  §6.1 hard errors, consistent with Milestone 7.
- clap's own errors (missing/unknown argument) are emitted by clap and
  exit with clap's code `2`. Tests must assert **nonzero**, not a
  specific code, unless the project pins one (see Ambiguity A3).

---

## 2. Clap derive shape

### 2.1 The `week` / `week target` ambiguity

`mlm week [WEEK_ID]` and `mlm week target [WEEK_ID] DURATION` means the
`week` command carries **both** an optional positional and an optional
subcommand. Clap supports this: a first token matching a known
subcommand name is taken as the subcommand, otherwise it falls through
to the positional. No week id can ever be the literal string `target`
(week ids are digits and one dash), so there is no real collision.

Milestone 11 owns `mlm week`'s rendering; Milestone 8 owns only the
`target` arm. **Both touch the same `WeekArgs` struct — coordinate, or
land Milestone 8's struct first and let Milestone 11 fill in the
non-subcommand arm.**

### 2.2 Recommended shape

```rust
// src/cli.rs
use clap::{Args, Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    // ... Start / Stop / Note / Status (other milestones)
    /// Show a week's totals, or set its target.
    Week(WeekArgs),
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct WeekArgs {
    #[command(subcommand)]
    pub action: Option<WeekAction>,

    /// Week to show: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7).
    /// Defaults to the current week.
    #[arg(value_name = "WEEK_ID")]
    pub week_id: Option<String>,
}

#[derive(Subcommand)]
pub enum WeekAction {
    /// Set an absolute target override for a week.
    Target(WeekTargetArgs),
}
```

`args_conflicts_with_subcommands = true` makes `mlm week 2026-07 target`
a clap error rather than a silently-ignored mix.

### 2.3 The `[WEEK_ID] DURATION` positional problem — and the fix

The natural spelling

```rust
pub struct WeekTargetArgs {
    pub week_id: Option<String>,   // optional, index 1
    pub duration: String,          // REQUIRED, index 2   <-- illegal
}
```

**does not work.** Clap 4 asserts at command-build time that a required
positional may not follow an optional one (it would be unparseable:
given one token, clap cannot know which slot it fills). This would fail
loudly in debug builds, including under `cargo test`.

Use a single variadic positional and disambiguate by arity:

```rust
#[derive(Args)]
pub struct WeekTargetArgs {
    /// [WEEK_ID] DURATION  — e.g. `2026-07 33h30m`, or just `33h30m`
    /// for the current week. DURATION uses the human format (20h,
    /// 33h30m, 45m), never raw minutes.
    #[arg(
        value_name = "ARGS",
        num_args = 1..=2,
        required = true,
        allow_hyphen_values = true,
    )]
    pub args: Vec<String>,
}
```

- `required = true` + `num_args = 1..=2` ⇒ **zero args is a clap-level
  missing-argument error** (E10) and **three or more args is a
  clap-level unexpected-argument error**. Both exit nonzero before any
  code of ours runs, so nothing is written.
- `allow_hyphen_values = true` is **load-bearing for E9**: without it,
  `mlm week target -5h` is parsed by clap as an unknown *flag*
  `-5h`. With it, `-5h` reaches our DURATION parser and produces the
  domain error §6.1 actually describes. (Either way the exit is nonzero
  with no write, but the message should be the duration one.)

Splitting helper, kept next to the struct so it is unit-testable
without spawning a process:

```rust
impl WeekTargetArgs {
    /// (week_id_token, duration_token) — 1 arg means WEEK_ID omitted.
    pub fn split(&self) -> (Option<&str>, &str) {
        match self.args.as_slice() {
            [duration]            => (None, duration.as_str()),
            [week, duration]      => (Some(week.as_str()), duration.as_str()),
            _ => unreachable!("clap enforces num_args = 1..=2"),
        }
    }
}
```

DURATION is **always the last token** — that is the rule the help text
must state, because `mlm week target 2026-07` (duration forgotten) is
one token and therefore reads as a malformed DURATION, not a missing
one. See Ambiguity A2.

### 2.4 Rejected alternatives (record so review doesn't relitigate)

- **Two `Option<String>` positionals, required-ness checked by hand.**
  Legal clap, but E10 stops being a clap-level error, which PLAN.md's
  acceptance criteria and SPEC.md E10 both name explicitly. Rejected.
- **`--week` flag instead of a positional.** Contradicts §3.7's
  `week target [WEEK_ID] DURATION` surface. Rejected.
- **A top-level `week-target` command.** Contradicts §3.7. Rejected.

---

## 3. Command body

### 3.1 Location

`src/commands/week_target.rs` (or `src/cmd.rs::week_target` if the
project stays flat — match whatever Milestone 7 establishes for
`start`/`stop`/`note`; do not invent a second convention).

### 3.2 Signature

```rust
pub fn run(
    conn: &Connection,
    today: NaiveDate,          // injected "now" (contract 6)
    args: &WeekTargetArgs,
) -> anyhow::Result<()>;
```

Returning `Result` rather than printing/exiting inline keeps the whole
body unit-testable against an in-memory SQLite connection; `main` does
the stderr print + exit-code mapping once for every command.

### 3.3 Body, in order

1. `let (week_tok, dur_tok) = args.split();`
2. **Resolve the week id** (first, see §3.4):
   - `Some(tok)` ⇒ `parse_week_id(tok, today.iso_week().year())` — pass
     the **ISO** year of `today` for the bare-`WW` default, not the
     Gregorian calendar year, so a `mlm week target 2 …` run on
     2026-12-31 (ISO year 2027) resolves the way §3.6 intends
     (`NaiveDate::iso_week()` is the real `chrono` API for this — not
     `year_ce_iso()`, which doesn't exist). If Milestone 2's parser
     takes `today: NaiveDate` directly, hand it that and let Milestone
     2 own the rule.
   - `None` ⇒ `week_of(today)`.
   - `?` on error propagates as `anyhow::Error` (wrapping Milestone 2's
     `WeekIdParseError`), nothing written.
3. **Parse the duration**: `let minutes = parse_duration(dur_tok)?;`
   ⇒ propagates Milestone 1's `DurationParseError` on failure via
   `anyhow`, nothing written.
4. Defensive `debug_assert!(minutes >= 0)` plus a real guard returning
   `InvalidDuration` if a future Milestone-1 change ever lets a
   negative through — the schema `CHECK` is the last backstop, not the
   first line of defense, and a `CHECK` violation would surface as an
   opaque `Db` error rather than the §6.1 message the user needs.
5. `db::set_week_target(conn, &week, minutes)?;`
6. Print one short confirmation on **stdout** and return `Ok(())`.
   Suggested: `Set target for week 2026-07 to 33h 30m` — reusing
   Milestone 1's canonical formatter (§4.2) for the duration, so this
   line never grows its own padding logic. See Ambiguity A4: SPEC.md
   §7 does not specify this line; tests should assert the *exit code
   and the stored row*, and at most that stdout **contains** the week
   id, never the exact sentence.

### 3.4 Validation order — and why

**WEEK_ID is validated before DURATION.** Rationale: left-to-right in
command-line order, so the first thing the user sees flagged is the
leftmost thing that is wrong. This is a *choice*, not a spec mandate
(§6.1 lists both as hard errors without ordering them); pin it here so
two independent tests don't assume opposite orders. A test with **both**
tokens malformed (`mlm week target 2027-53 -5h`) asserts the week-id
message wins.

Both parses happen **before** the single `set_week_target` call, so a
failure at either step is structurally incapable of leaving a write
behind — there is exactly one statement, it is the last thing that
runs, and it is a single-row upsert (no multi-statement partial state,
hence no transaction needed; see §4.3).

Order relative to opening the DB: `main` currently opens the connection
before dispatching. Either order satisfies §6.1 (a DB failure is also a
hard error) — but prefer parsing the arguments **before** touching the
DB where `main`'s structure allows, so a typo'd duration does not
create the app-data directory on a first run. Not a blocker; note it
for the Milestone 7 / `main` wiring pass.

---

## 4. The upsert

### 4.1 SQL

```sql
INSERT INTO week_targets (week_id, target_minutes)
VALUES (?1, ?2)
ON CONFLICT(week_id) DO UPDATE SET target_minutes = excluded.target_minutes;
```

- `ON CONFLICT … DO UPDATE` (SQLite ≥ 3.24, far below the `bundled`
  version) is preferred over `INSERT OR REPLACE`, which is a
  delete-then-insert: it would churn the row and, once anything ever
  references `week_targets`, behave differently under foreign keys and
  triggers. `ON CONFLICT DO UPDATE` mutates in place.
- The conflict target is named explicitly (`ON CONFLICT(week_id)`)
  rather than bare `ON CONFLICT`, so the statement fails loudly if
  Milestone 3's primary key ever moves.
- **`?1` must be the normalized `YYYY-WW` string** — the zero-padded
  `Display`/`as_str()` of Milestone 2's `WeekId`, never the raw user
  token. This is what makes `mlm week target 2026-7 …` followed by
  `mlm week target 2026-07 …` update one row instead of creating two
  (the PK is `TEXT`, so `2026-7` and `2026-07` would be distinct keys).
  Called out because it is the single most likely silent bug in this
  milestone.

### 4.2 Rust wrappers (in `src/db.rs`, next to Milestone 3's schema)

```rust
/// Set (insert or replace) a week's absolute target override (§2.3, §3.7).
/// `target_minutes` must be >= 0; the schema CHECK is the backstop.
pub fn set_week_target(
    conn: &Connection,
    week_id: &WeekId,
    target_minutes: i64,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO week_targets (week_id, target_minutes)
         VALUES (?1, ?2)
         ON CONFLICT(week_id) DO UPDATE SET target_minutes = excluded.target_minutes",
        rusqlite::params![week_id.as_str(), target_minutes],
    )?;
    Ok(())
}

/// The stored override for a week, or `None` when the week has no row
/// (caller applies the 40h default — §2.3/§6.2). Milestone 6 will want
/// a bulk variant; this single-week read exists for Milestone 8's own
/// read-back tests and is the natural building block for it.
pub fn get_week_target(
    conn: &Connection,
    week_id: &WeekId,
) -> anyhow::Result<Option<i64>> {
    conn.query_row(
        "SELECT target_minutes FROM week_targets WHERE week_id = ?1",
        rusqlite::params![week_id.as_str()],
        |row| row.get(0),
    )
    .optional()          // rusqlite::OptionalExtension
    .map_err(anyhow::Error::from)
}
```

`i64` (not `u32`/`i32`) because the column is SQLite `INTEGER` and
every other minute value in this codebase is a signed i64 (§5's
carry/owed values go negative). Keeping one integer type across the
whole project avoids conversion noise at the Milestone 6 boundary.

### 4.3 No transaction

One statement, one row, atomic on its own. Adding an explicit
transaction here would be cargo-culting. (Milestone 7's
punch+note-in-one-command case is the one that genuinely needs one.)

---

## 5. Test plan

Two layers. Write the unit layer first (fast, no process spawn), then
the CLI layer for the arity/exit-code behavior that only clap can
produce.

### 5.1 Unit tests — `src/db.rs` (in-memory SQLite)

Each opens `Connection::open_in_memory()`, runs Milestone 3's
migrations against it, and exercises `set_week_target`/
`get_week_target`. (Requires Milestone 3 to expose its migration
application against an arbitrary `Connection` — Risk R1.)

| # | Name | Assertion |
|---|---|---|
| U1 | `set_week_target_inserts_new_row` | after setting `2026-07` to 2010, `get_week_target` returns `Some(2010)` (F7, storage half) |
| U2 | `set_week_target_zero_is_accepted` | setting `2026-07` to `0` succeeds; read-back is `Some(0)` (F7b) |
| U3 | `set_week_target_replaces_existing` | set `2026-07`→2400, then →2010; read-back is `Some(2010)` **and** `SELECT COUNT(*) FROM week_targets` is `1` (upsert, not a duplicate, not an error) |
| U4 | `set_week_target_absent_week_reads_none` | `get_week_target` for a never-set week returns `Ok(None)`, not an error (§6.2) |
| U5 | `set_week_target_negative_is_rejected_by_schema` | calling with `-1` returns `Err` downcasting to `DbError` and leaves zero rows — proves the CHECK backstop exists, distinct from the parser-level rejection in C4 |
| U6 | `set_week_target_keys_on_normalized_id` | build the `WeekId` from `"2026-7"` and from `"2026-07"`, set both, assert exactly **one** row keyed `"2026-07"` (guards §4.1's normalization bug) |

### 5.2 Unit tests — arg splitting (`src/cli.rs`)

Parse via `Cli::try_parse_from([...])`, no process spawn.

| # | Name | Assertion |
|---|---|---|
| P1 | `split_one_arg_is_duration` | `["mlm","week","target","33h30m"]` ⇒ `(None, "33h30m")` |
| P2 | `split_two_args_is_week_then_duration` | `["mlm","week","target","2026-07","33h30m"]` ⇒ `(Some("2026-07"), "33h30m")` |
| P3 | `no_args_is_clap_error` | `try_parse_from(["mlm","week","target"])` is `Err`, kind `MissingRequiredArgument` (E10) |
| P4 | `three_args_is_clap_error` | `["mlm","week","target","2026-07","33h30m","extra"]` is `Err` |
| P5 | `hyphen_duration_reaches_our_parser` | `["mlm","week","target","-5h"]` parses **successfully at the clap layer** to `(None, "-5h")` — proves `allow_hyphen_values` is wired; the rejection is C4's job |
| P6 | `week_positional_still_works` | `["mlm","week","2026-07"]` is the non-subcommand arm, `action: None`, `week_id: Some("2026-07")` — guards the §2.1 subcommand/positional ambiguity |

### 5.3 Command-body tests (`run`, in-memory conn, injected `today`)

| # | Name | Assertion |
|---|---|---|
| R1 | `explicit_week_id_writes_that_week` | `run` with `["2026-07","33h30m"]`, `today` far away ⇒ row `("2026-07", 2010)` (F7) |
| R2 | `omitted_week_id_defaults_to_current_week` | `today = 2026-02-12` (a Thursday in ISO week 2026-07), args `["33h30m"]` ⇒ the single row is keyed `"2026-07"` and `get_week_target(week_of(today))` is `Some(2010)`. Repeat with a **year-boundary** `today` (e.g. `2026-12-28`, ISO `2027-01`) to prove the ISO year, not the Gregorian one, is used |
| R3 | `zero_duration_accepted` | args `["2026-07","0h"]` ⇒ `Ok`, row `("2026-07", 0)` (F7b). Also `"0m"` |
| R4 | `negative_duration_rejected_no_write` | args `["2026-07","-5h"]` ⇒ `Err` downcasting to Milestone 1's `DurationParseError` and `SELECT COUNT(*) FROM week_targets` is `0` (E9) |
| R5 | `malformed_week_id_rejected_no_write` | for each of `"0"`, `"abcd"`, `"2027-53"` (a 52-week year — use whichever year Milestone 2's tests establish as 52-week, do not hardcode a guess) ⇒ `Err` downcasting to Milestone 2's `WeekIdParseError`, zero rows (E3) |
| R6 | `unpadded_week_id_normalized` | args `["2026-7","33h30m"]` ⇒ row keyed `"2026-07"` (E3's positive half) |
| R7 | `malformed_duration_rejected_no_write` | `"10"` (no unit), `"10x"`, `""` ⇒ `Err(InvalidDuration)`, zero rows (E4 applied here) |
| R8 | `week_id_validated_before_duration` | args `["2027-53","-5h"]` (both bad) ⇒ the error is `InvalidWeekId`, not `InvalidDuration` (pins §3.4's order) |
| R9 | `override_replaces_not_duplicates` | run twice for `2026-07` with `40h` then `33h30m` ⇒ one row, value 2010 (the end-to-end twin of U3) |
| R10 | `succeeds_for_a_week_with_no_data` | setting a target for a far-future, never-touched week is fine (§6.2 — no requirement that the week have punches) |

### 5.4 CLI integration tests (`tests/week_target.rs`)

Only what needs a real process: exit codes and stderr. Needs a
temp-dir DB (Risk R1) and a process runner (Risk R3).

| # | Name | Assertion |
|---|---|---|
| C1 | `success_exits_zero` | `mlm week target 2026-07 33h30m` ⇒ exit `0`, and a follow-up read of the temp DB shows the row (§6.3, F7) |
| C2 | `missing_duration_is_clap_error` | `mlm week target` ⇒ **nonzero** exit, stderr non-empty and mentions the argument; nothing written (E10) |
| C3 | `malformed_week_id_exits_nonzero` | `mlm week target abcd 33h30m` ⇒ nonzero exit, message on **stderr** (not stdout), zero rows (E3, §6.1, §6.3) |
| C4 | `negative_duration_exits_nonzero` | `mlm week target 2026-07 -5h` ⇒ nonzero exit, stderr message naming the duration, zero rows (E9) |
| C5 | `zero_duration_exits_zero` | `mlm week target 2026-07 0h` ⇒ exit `0`, row present with `0` (F7b end-to-end) |
| C6 | `omitted_week_id_end_to_end` | `mlm week target 20h` ⇒ exit `0`, exactly one row whose key equals the ISO week of the machine's today (computed in the test the same way, via Milestone 2's `week_of`) |

Deliberately **not** tested here: anything about how the stored target
later affects fulfillment/owed math — that is F7's *reading* half and
belongs to Milestones 6 and 11 (PLAN.md is explicit that Milestone 8
"never calls into the week-walk").

---

## 6. Files touched

| File | Change |
|---|---|
| `src/cli.rs` | add `Week(WeekArgs)`, `WeekArgs`, `WeekAction`, `WeekTargetArgs`, `split()`; delete the scaffold's `Log` variant only if Milestone 7/10/11 have not already (PLAN.md: `Log` is leftover scaffold, not spec) |
| `src/db.rs` | add `set_week_target`, `get_week_target` |
| `src/commands/week_target.rs` (new) | `run()` |
| `src/main.rs` | dispatch `Command::Week(a)` with `Some(WeekAction::Target(t))` ⇒ `week_target::run(&conn, Local::now().date_naive(), t)`; map `Err` ⇒ stderr + nonzero exit |
| `tests/week_target.rs` (new) | §5.4 |
| `Cargo.toml` | **possibly** `[dev-dependencies]` — see Risk R3, coordinate first |

---

## 7. Ambiguities, risks, disagreements

**A1 — PLAN.md contradicts itself on this milestone's dependencies.**
The wave-2 bullet says Milestone 8 "needs Milestone 2 (WEEK_ID parsing)
and Milestone 3 (schema) **only**", but the Milestone 8 body says
"on top of Milestones 1, 2, and 3". Milestone 1 is obviously required —
§3.7 mandates the §4.2 DURATION grammar, which Milestone 1 owns. Read
the "only" as excluding **Milestone 6**, which is the actual point
being made there. No action needed beyond knowing Milestone 1 must also
have landed; flagging so nobody starts this milestone believing
`parse_duration` is out of scope and re-implements it locally.

**A2 — `mlm week target 2026-07` (DURATION forgotten) is not a
missing-argument error.** With the variadic positional, one token is
always the duration, so this produces a *malformed DURATION* error
(`2026-07` is not a duration) rather than clap's "missing DURATION".
E10 is satisfied by the genuinely-zero-argument case (C2), which is
what SPEC.md E10 literally describes ("missing the `DURATION` argument
entirely"). Mitigation: the propagated `DurationParseError`'s message should
name the expected forms, e.g.
`invalid duration '2026-07': expected a duration like 20h, 33h30m or 45m`.
Accepting this is the price of §3.7's `[WEEK_ID] DURATION` ordering; the
alternative (required-before-optional) is illegal in clap.

**A3 — exit code for hard errors is unpinned.** §6.3 says only
"nonzero". Our errors will exit `1`; clap's exit `2`. So E10's exit code
differs from E9's. That is consistent with ordinary CLI convention and
with §6.3 as written — but it means **tests must assert `!= 0`, not a
literal**, unless a cross-cutting decision pins one. Worth resolving in
PLAN.md's wave-5 exit-code pass rather than here.

**A4 — no spec'd success output.** §7 specifies layouts for `status`
and `week` but says nothing about what `week target` prints on success.
Silence-on-success is also defensible (unix-y), but every other
mutating command in this tool is equally silent and the user gets no
confirmation that `33h30m` was read as 33h30m and not 33 minutes. Plan
proposes a one-line confirmation on stdout; tests assert exit code +
stored row, and at most a `contains(week_id)` on stdout, so the exact
wording stays free to change. **Escalate to the spec owner** — this is
a real §7 gap, not an implementation detail.

**R1 — the scaffold's `db::connect()` is untestable.** It hardcodes the
real platform app-data dir via `ProjectDirs` and `.expect()`s on
failure. Milestone 8's integration tests (§5.4) would write to the
developer's actual `~/.local/share/mlm/mlm.db`. Milestone 3 must expose
**either** `connect_at(path: &Path)` **or** an env override
(`MLM_DB_PATH`), plus a way to run migrations against an arbitrary
`Connection` for the in-memory unit tests. This is a hard blocker for
§5.1 and §5.4 and it belongs to **Milestone 3, not Milestone 8** — raise
it with whoever holds Milestone 3 before starting, and prefer the env
override (it also makes every other milestone's CLI tests possible, so
it should not be solved six times). The `.expect()`s in the scaffold
should become `Result` at the same time (§6.1: a DB failure is an error,
not a panic).

**R2 — `src/cli.rs` is a four-way merge point.** Milestones 7, 8, 10
and 11 all rewrite the `Command` enum, and 8 and 11 share `WeekArgs`
specifically. Either land the full `Command` enum shape once, up front,
as a shared prerequisite, or expect a conflict. Milestone 8 should add
`WeekArgs`/`WeekAction`/`WeekTargetArgs` as a self-contained block and
leave the `action: None` arm's behavior to Milestone 11.

**R3 — no `[dev-dependencies]` exist.** §5.4 wants a process runner and
a temp dir (conventionally `assert_cmd` + `predicates` + `tempfile`, or
hand-rolled `std::process::Command` + `env!("CARGO_BIN_EXE_mlm")` +
a manual temp dir). `Cargo.toml`/`Cargo.lock` are shared across every
parallel worktree, so an unilateral dependency add is a guaranteed
lockfile conflict. Decide the test-harness dependency set **once,
project-wide, before wave 2 opens**. `env!("CARGO_BIN_EXE_mlm")` with
plain `std::process::Command` needs zero new dependencies and is the
lowest-friction fallback if the answer is "no new deps".

**R4 — `allow_hyphen_values` is easy to lose.** If it is dropped in a
later refactor, E9's test still passes (clap rejects `-5h` as an unknown
flag ⇒ nonzero exit, no write) while the user-facing message silently
regresses to something about an unexpected argument. P5 exists
specifically to catch that; do not delete it as redundant.

**R5 — bare-`WW` year source at an ISO/Gregorian boundary.** §3.6 says
a bare week number "defaults the year to the current one" without
saying *which* year. On e.g. 2026-12-28 the Gregorian year is 2026 but
the ISO year is 2027, and `mlm week target 1 20h` would mean different
weeks under each reading. This plan uses the **ISO** year (consistent
with §1.3's "identified by an (ISO year, ISO week) tuple") and R2's
second case tests it. The decision properly belongs to **Milestone 2**;
if Milestone 2 lands the opposite rule, Milestone 8 defers to it and
R2's boundary case is updated — but the two must not disagree silently.

**Disagreement with PLAN.md: none substantive.** Milestone 8's scope,
wave placement, and acceptance criteria are sound and the
no-dependency-on-Milestone-6 call is correct — this command genuinely
only writes a row. The gaps found are A1's wording slip, A4's spec hole,
and R1/R3's missing test infrastructure, all of which are upstream of
this milestone rather than flaws in it.
