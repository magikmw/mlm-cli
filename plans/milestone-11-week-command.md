# Milestone 11 — `week` command and rendering

**Status**: design plan, ready to hand to a TDD subagent.
**Spec sections**: SPEC.md §3.6, §7.2, §7.3, §4.2, §2.4, §6.2, §6.3, plus
flows F6, F8b, E13, E3.
**PLAN.md**: wave-4 leaf; depends on Milestones 2, 5, 6, 9 via the pinned
interface contracts 1-6, **not** against their actual code.

This document is written so that tests can be authored before any of
Milestones 2/5/6/9 exist, by stubbing their contract shapes.

---

## 0. Design principle that drives everything below

**Split the milestone into a pure renderer and a thin command wiring.**

```
[clap args] -> resolve -> build WeekView (pure data) -> render_week(&WeekView) -> String -> stdout
```

- `render_week` is a **pure function**: `WeekView` in, `String` out. No DB,
  no clock, no I/O, no `println!` inside it. Every §7.2/§7.3 layout
  assertion in §5 below is a unit test on this function. This is the only
  way to get byte-exact golden-output tests without solving the
  "injectable DB path + injectable now" test-infrastructure problem (which
  PLAN.md leaves unassigned — see §6, Risk R1).
- The command wiring (`resolve` + I/O) gets a small number of integration
  tests, only for things the pure renderer cannot prove (WEEK_ID parsing
  reaching the resolver, end-to-end carry across an idle gap, exit code).

Everything the renderer needs is in `WeekView`. It never recomputes
anything, never branches on "is this week current" itself, never formats a
duration itself, and never composes headline wording itself.

---

## 1. Clap shape for `week [WEEK_ID]`

### 1.1 The subcommand collision with Milestone 8

`mlm week [WEEK_ID]` (§3.6, this milestone) and `mlm week target [WEEK_ID]
DURATION` (§3.7, Milestone 8) share the `week` command word. The `Week`
variant therefore carries **both** an optional positional and an optional
nested subcommand:

```
Command::Week {
    // optional nested subcommand: currently only `target` (Milestone 8)
    subcommand: Option<WeekSubcommand>,   // #[command(subcommand)]
    // optional positional week id, used when no subcommand is given
    week_id: Option<String>,
}
```

with `#[command(args_conflicts_with_subcommands = true)]` on the `Week`
variant so clap does not try to satisfy the positional and the subcommand
at once.

Resolution behaviour required (assert these in clap-level tests):

| invocation | outcome |
|---|---|
| `mlm week` | `subcommand: None, week_id: None` -> current week |
| `mlm week 7` | `subcommand: None, week_id: Some("7")` |
| `mlm week 2026-07` | `subcommand: None, week_id: Some("2026-07")` |
| `mlm week 2026-7` | `subcommand: None, week_id: Some("2026-7")` |
| `mlm week target ...` | routes to Milestone 8, never to this milestone |
| `mlm week 7 8` | clap error (unexpected extra positional), nonzero exit |

`WEEK_ID` is taken as a **raw `String`** at the clap layer, not parsed by a
clap value parser. Parsing (and its hard-error message) is Milestone 2's
job and happens in the resolver, so that E3's error text is owned in one
place rather than by clap's value-parser wrapper.

**Coordination point**: Milestone 8 and Milestone 11 both edit the same
`Command::Week` variant in `cli.rs`. Milestone 11 owns the parent variant's
shape (positional + `args_conflicts_with_subcommands`); Milestone 8 owns
the `WeekSubcommand::Target { .. }` variant's contents. Whoever lands
second rebases onto the other's variant rather than replacing it. Note
also PLAN.md's closing risk: the scaffold's `Command::Log` is leftover and
must be deleted, not preserved.

### 1.2 Also delete the scaffold placeholders touched here

`src/cli.rs`'s `Log` variant and `main.rs`'s `Log` arm are not spec. This
milestone removes the `Log` arm only if Milestone 10 has not already; it
does not invent a replacement.

---

## 2. Resolution flow (command wiring)

Inputs: the raw optional `WEEK_ID` string, an injected `now` (contract 6 —
a supplied local-aware "current instant" value, never a hidden global clock
read), and a DB handle.

1. **Resolve the week id.**
   - If `WEEK_ID` is `None`: derive the current week id from `now`
     (Milestone 2's "date -> week id" helper applied to `now`'s *local*
     date).
   - If `Some(s)`: call Milestone 2's WEEK_ID parser with `s` and the
     current year derived from `now`'s local date (bare `WW` defaults the
     year to the current one, §3.6/F6; `2026-7` normalises to `2026-07`).
   - A parse failure is a §6.1 **hard error**: message on stderr, nonzero
     exit, nothing rendered, nothing written (E3).
2. **Compute the Mon-Sun span** for the resolved week id via Milestone 2's
   span helper -> an ordered array of exactly 7 local calendar dates,
   `[Mon, Tue, Wed, Thu, Fri, Sat, Sun]`. This array is the row order and
   is never re-sorted or filtered.
3. **Build the 7-date rollup** (contract 4) — see §3 below.
4. **Run Milestone 6's week accounting** for the resolved week id, with
   `now`. Returns contract 3: `target`, `carry_in`, `worked`,
   `fulfillment`, `owed`, `carry_out` (all signed integer minutes) and the
   explicit `is_current_week` boolean.
5. **Build the headline** by handing Milestone 9 the accounting result and
   `now`. Milestone 9 returns the **finished headline string** (contract
   5) — deadline-framed for the current week, plain `Total still
   owed:`/`Total ahead:` form otherwise. Milestone 11 prints it verbatim
   and makes **no** decision about wording, sign, or weekday.
6. **Assemble `WeekView`**, call `render_week`, print to stdout, exit `0`
   (§6.3 — `0` even when anomaly markers are present, §6.2).

A DB open/migration failure at any point is a §6.1 hard error (nonzero
exit, stderr message) — verified in the wave-5 cross-cutting pass, not
re-proven here.

### 2.1 `WeekView` — the renderer's only input

```
WeekView {
    week_id_display: String,     // "2026-07", already normalised (M2 formatter)
    span_start_display: String,  // "2026-02-09" (M2 date formatter)
    span_end_display: String,    // "2026-02-15"
    headline: String,            // verbatim from Milestone 9, no trailing punctuation added
    rows: [WeekRow; 7],          // Mon..Sun, exactly 7, never fewer
    carry_in_minutes: i64,
    worked_minutes: i64,
    fulfillment_minutes: i64,
    target_minutes: i64,
}

WeekRow {
    weekday_abbrev: &'static str, // "Mon".."Sun" — explicit table, see §4.3
    date_display: String,         // "2026-02-09"
    minutes: i64,                 // completed-stint minutes only, >= 0
    is_ongoing: bool,             // (ongoing) marker
    has_anomaly: bool,            // [!] marker
}
```

`owed` is deliberately **absent** from `WeekView`: §7.2 states "`still
owed` isn't repeated down here since the headline already states it
plainly", and the headline is already a finished string. Keeping `owed`
out of the renderer makes it structurally impossible for Milestone 11 to
accidentally re-render it.

---

## 3. Building the "all 7 dates, including empty ones" rollup (contract 4)

### 3.1 Recommendation: one week-wide range read, then per-date pairing

**Do not call Milestone 5 with a date-keyed DB read seven times.** Instead:

1. Issue **one** storage read for all punches whose local `date` column
   falls in `[span_start, span_end]` (a `WHERE date BETWEEN ? AND ?` over
   `punches`, ordered by `at_utc` then `id` — Milestone 4's pinned
   ordering contract). This is a natural extension of Milestone 4's
   existing single-date read, hits the existing `date` index, and is one
   round trip instead of seven.
2. **Bucket the returned punches by their `date` column**, in memory.
3. For each of the 7 dates in span order, call **Milestone 5's pairing
   function once**, on that date's bucket (an empty bucket included —
   Milestone 5's acceptance criteria already require a zero-punch date to
   yield zero stints and zero anomalies cleanly).
4. Fold each date's Milestone 5 result (contract 2) into a `WeekRow`.

Bucketing per `date` before pairing is **mandatory, not an optimisation**:
§4.3 pairs strictly per calendar date, and §1.2 makes cross-midnight
pairing an explicit non-goal (E15). Handing Milestone 5 a whole week's
punches in one call would silently implement the deferred feature and
break E15.

**Fallback if Milestone 4 does not expose a range read** by the time this
milestone starts: loop the 7 dates calling Milestone 4's single-date read.
Behaviour is identical; only the round-trip count differs. The rollup
builder's signature must therefore take *punches already fetched*, not a
DB handle, so this choice is swappable and the builder stays unit-testable
with fixtures. **This is the single decision to confirm with Milestone 4
before starting.**

### 3.2 Mapping Milestone 5's per-date result to a `WeekRow`

Given contract 2 (completed stints, optional open stint, multi-open flag,
list of orphaned-end anomalies):

- `minutes` = **sum of completed stints' durations only**. The open stint's
  live minutes are excluded (§2.4, NOTES 37) and orphaned ends contribute
  nothing (§2.4, NOTES 38). A zero-length stint (E14) contributes `0` and
  is still a completed stint.
  - Cross-check in the §7.2 example: Thu shows `07h 25m (ongoing)` while
    §7.1 shows the same day with an open `17:45-now (00h 15m)` stint — the
    15 live minutes are correctly absent from the row.
- `is_ongoing` = `there is an open stint on this date` **AND** `this date
  == now's local date`. See Risk R2 — the `date == today` clause is what
  keeps a stale dangling `start` on a past date from printing `(ongoing)`,
  matching §7.2's "only possible on today's row, and only when the
  requested week is the current one".
- `has_anomaly` = Milestone 9's **cheap per-row marker predicate**
  (contract 2's "has-any-anomaly signal", contract 5's marker form)
  applied to the date's Milestone 5 result. Milestone 11 must **not**
  compute `multi_open || !orphans.is_empty()` itself — that duplication is
  precisely what Milestone 9 exists to prevent.
- `weekday_abbrev` / `date_display` come from the span array, independent of
  whether any punches exist — this is what guarantees all 7 rows.

### 3.3 Invariant worth asserting

`rows.iter().map(|r| r.minutes).sum() == worked_minutes` for the requested
week. Both §7.2 examples satisfy it (1885 = 31h 25m; 2210 = 36h 50m). See
Risk R3 — this is the test that catches Milestone 6 and Milestone 11
computing "worked" by two divergent paths.

---

## 4. The rendered layout — exact template

### 4.1 Line-by-line template (identical for BOTH current and closed weeks)

```
Week {week_id} ({span_start} - {span_end})
<blank>
{headline}
<blank>
  {Abb} {YYYY-MM-DD}   {HHh MMm}[ (ongoing)][  [!]]
  ... x7, Mon..Sun ...
<blank>
Carry-in:      {HHh MMm}
Worked:        {HHh MMm}
Fulfillment:   {HHh MMm}
Target:        {HHh MMm}
```

Exactly **16 lines**, terminated by a single trailing newline. Nothing else
is ever emitted: no "still owed" line at the bottom, no empty-row
suppression, no section headers, no separators.

### 4.2 Character-exact rules (derived by counting SPEC.md §7.2/§7.3)

- **Header**: literal `Week `, week id, ` (`, start date, ` - ` (space,
  ASCII hyphen-minus `0x2D`, space), end date, `)`.
- **Headline line**: Milestone 9's string verbatim, no indent, no added
  punctuation.
- **Row**: `"  "` (2 spaces) + weekday abbrev (3 chars) + `" "` (1 space)
  + `YYYY-MM-DD` (10 chars) + `"   "` (3 spaces) + duration (7 chars,
  `HHh MMm`). So the duration column starts at byte offset 19.
  - If `is_ongoing`: append `" (ongoing)"` (1 space + literal).
  - If `has_anomaly`: append `"  [!]"` (**2** spaces + `[!]`), per §7.3's
    `  Wed 2026-02-11   08h 00m  [!]`.
  - When both apply: `07h 25m (ongoing)  [!]` — ongoing first, marker last,
    since §7.2 says the marker is "appended to its row". See Risk R4.
  - No trailing whitespace on any row (a row with neither suffix ends at
    the duration).
- **Trailing block**: label left-padded to a **15-character field**, then
  the formatted duration left-aligned (i.e. `format!("{:<15}{}", label,
  dur)`). Verified against all four spec lines: `Carry-in:` 9+6,
  `Worked:` 7+8, `Fulfillment:` 12+3, `Target:` 7+8 — all 15. Matches
  §7.1's `Day total:     ` (10+5=15) too, so the two commands' summary
  columns line up as §4.2 intends.
  - A negative carry-in simply extends the left-aligned value leftward-free:
    `Carry-in:      -02h 10m` (the sign is inside the value, the label
    field stays 15). Confirmed against §7.2's first example.
  - `worked` and `target` are non-negative by construction; `carry_in` and
    `fulfillment` may be negative and use Milestone 1's signed format.
- **Every** duration in the output goes through Milestone 1's single
  formatter (§4.2). Milestone 11 contains no `/ 60`, no `% 60`, no `{:02}`.

### 4.3 Weekday abbreviations

Use an explicit `["Mon","Tue","Wed","Thu","Fri","Sat","Sun"]` table indexed
by the date's ISO weekday, **not** a locale-sensitive format call. Output
must be byte-stable regardless of the runner's locale env.

### 4.4 Worked example A — current/ongoing week (§7.2, first example)

`now` = 2026-02-12 (Thursday) 18:00 local; requested week `2026-07`.

```
Week 2026-07 (2026-02-09 - 2026-02-15)

10h 45m left by end of Thursday

  Mon 2026-02-09   08h 10m
  Tue 2026-02-10   07h 50m
  Wed 2026-02-11   08h 00m
  Thu 2026-02-12   07h 25m (ongoing)
  Fri 2026-02-13   00h 00m
  Sat 2026-02-14   00h 00m
  Sun 2026-02-15   00h 00m

Carry-in:      -02h 10m
Worked:        31h 25m
Fulfillment:   29h 15m
Target:        40h 00m
```

Arithmetic the fixture must satisfy: worked 490+470+480+445 = 1885;
carry_in −130; fulfillment 1755; target 2400; owed 645 = `10h 45m`
(the headline's figure, produced by Milestone 9, not by the renderer).

### 4.5 Worked example B — past/closed week (§7.2, second example)

Same `now`; requested week `2026-06`. **Full table and full trailing
block** — this is the blocker NOTES.md decision 25 fixed; only the
headline wording differs.

```
Week 2026-06 (2026-02-02 - 2026-02-08)

Total still owed: 03h 10m

  Mon 2026-02-02   07h 30m
  Tue 2026-02-03   08h 00m
  Wed 2026-02-04   07h 45m
  Thu 2026-02-05   08h 10m
  Fri 2026-02-06   05h 25m
  Sat 2026-02-07   00h 00m
  Sun 2026-02-08   00h 00m

Carry-in:      00h 00m
Worked:        36h 50m
Fulfillment:   36h 50m
Target:        40h 00m
```

Arithmetic: 450+480+465+490+325 = 2210; carry_in 0; fulfillment 2210;
owed 190 = `03h 10m`. No `(ongoing)` on any row (no row is today).

A **future** week renders by the same closed-week path — `is_current_week`
is false, so Milestone 9 gives the plain form, all 7 rows are `00h 00m`
(unless a target override exists, which only changes `Target:`).

---

## 5. Test plan

Group A runs against `render_week` with hand-built `WeekView` fixtures
(zero I/O, byte-exact). Group B runs against the rollup builder with
fixture punch lists. Group C runs against the resolver/CLI.

### Group A — golden rendering (pure, no DB, no clock)

- **A1 — current-week golden.** `WeekView` for §7.2 example A. Assert the
  rendered string equals the 16-line block in §4.4 **byte for byte**,
  including the trailing newline. (PLAN.md acceptance criterion 1.)
- **A2 — `(ongoing)` on today's row only.** Same fixture: assert exactly
  one row contains `(ongoing)`, and it is the Thu row; assert the other six
  rows do not contain the substring.
- **A3 — past-week golden, full table.** `WeekView` for §7.2 example B.
  Byte-for-byte equality with §4.5. Additionally assert positively that the
  output contains all 7 date strings and all four trailing labels — i.e.
  an explicit regression test that a closed week is **not** a bare
  one-liner (NOTES.md decision 25 / PLAN.md acceptance criterion 2).
- **A4 — future week.** Same shape as A3 with all rows at zero and a
  future span. Asserts no `(ongoing)` anywhere and the full block present.
- **A5 — never-touched week, all-zero rows (E13, render half).** All 7
  rows `00h 00m`, `carry_in` some nonzero walked value, `worked` 0,
  `fulfillment == carry_in`, `target` 2400. Assert 7 rows are emitted and
  each reads `00h 00m` — no rows collapsed or omitted.
- **A6 — anomaly marker present/absent per row.** Fixture with
  `has_anomaly` true on exactly the Wed row. Assert the Wed row ends
  `08h 00m  [!]` (two spaces) and no other row contains `[!]`.
- **A7 — ongoing + anomaly on the same row.** Assert
  `07h 25m (ongoing)  [!]` ordering (locks in the R4 decision).
- **A8 — negative carry-in and negative fulfillment formatting.**
  `carry_in = -130` renders `Carry-in:      -02h 10m`; a fixture with
  `fulfillment = -50` renders `Fulfillment:   -00h 50m`. Asserts the
  15-char label field is not disturbed by the sign.
- **A9 — default 40h target display (F6, render half).** `target_minutes =
  2400` renders exactly `Target:        40h 00m`.
- **A10 — zero target (F7b interaction).** `target_minutes = 0` renders
  `Target:        00h 00m`, not `0h 0m` and not an omitted line.
- **A11 — plain-ASCII audit (§7).** Over the output of A1, A3, A6 and A7:
  assert every byte is in `0x20..=0x7E` or `\n`. Explicitly assert absence
  of U+2013/U+2014 en/em dashes and any U+2500-block box-drawing char.
- **A12 — no trailing whitespace / exact line count.** For each golden:
  16 lines, exactly one trailing `\n`, no line ends in a space, lines 2, 4
  and 12 are empty.
- **A13 — headline passthrough.** `WeekView` with a deliberately odd
  `headline` string ("XYZZY") renders it verbatim on line 3 with no
  indent, prefix, or added punctuation — proves the renderer makes no
  wording decision of its own.
- **A14 — row ordering is Mon..Sun regardless of input construction.**
  Feed rows whose dates are Mon..Sun and assert rendered order matches;
  a guard against accidental sorting by minutes or by anomaly.

### Group B — rollup construction (contract 4)

All with a fixed injected `now`.

- **B1 — 7 rows always.** Zero punches anywhere in the week -> 7 rows, all
  `minutes == 0`, `is_ongoing == false`, `has_anomaly == false`.
- **B2 — per-date bucketing does not pair across midnight (E15).** A
  `start 23:30` on Tue and an `end 00:45` on Wed -> Tue has an open stint
  contributing 0 minutes, Wed has an orphaned end contributing 0 minutes
  and `has_anomaly == true`; no 1h15m stint appears anywhere.
- **B3 — completed-stints-only totals (§2.4 / NOTES 37).** Today has two
  completed stints plus one open stint; row `minutes` equals only the two
  completed ones, and `is_ongoing == true`.
- **B4 — orphaned end contributes nothing (NOTES 38).** A date with one
  clean stint plus an orphaned end: `minutes` equals the clean stint only,
  `has_anomaly == true`.
- **B5 — `is_ongoing` false on a past date with a dangling start.** A
  past date in a past week holding one unmatched trailing start ->
  `is_ongoing == false`, `minutes == 0`. (Locks in Risk R2's resolution.)
- **B6 — multi-open flags the row.** Two dangling starts on today ->
  `has_anomaly == true`, `is_ongoing == true`, `minutes` = completed only
  (E7 at the week-row level).
- **B7 — zero-length stint (E14).** A start/end at the identical instant
  contributes `0` minutes and does **not** set `has_anomaly`.
- **B8 — rows sum to Milestone 6's `worked`.** Seed a week, build the
  rollup, run Milestone 6's accounting, assert
  `sum(row.minutes) == accounting.worked`. (Risk R3's guard.)

### Group C — resolution and end-to-end

Requires the injectable-DB-path + injectable-`now` harness (Risk R1). If
that harness is not yet available, C-tests target the resolver function
directly (taking `now`, a DB handle and the raw arg) rather than spawning
the binary; the assertions are unchanged.

- **C1 — no WEEK_ID defaults to the current week (§3.6).** With
  `now = 2026-02-12`, `mlm week` resolves `2026-07` and its header line
  reads `Week 2026-07 (2026-02-09 - 2026-02-15)`.
- **C2 — bare week number defaults to the current year (F6/§3.6).**
  `mlm week 7` with `now` in 2026 produces output identical to
  `mlm week 2026-07`.
- **C3 — unpadded full id.** `mlm week 2026-7` produces output identical
  to `mlm week 2026-07` (E3's accepted case).
- **C4 — malformed WEEK_ID is a hard error (E3).** `0`, `abcd`, and a
  week number exceeding its year's real ISO week count (`2027-53` if 2027
  has 52) each exit nonzero with a stderr message and print no table.
- **C5 — default 40h target end-to-end (F6).** A week with no
  `week_targets` row renders `Target:        40h 00m`.
- **C6 — target override end-to-end (F7, read half).** After
  `week target 2026-07 33h30m`, `week 2026-07` renders
  `Target:        33h 30m` and a fulfillment/owed consistent with it.
- **C7 — never-touched week (E13), end-to-end.** Seed data in week N,
  request week N+5 (no punches, no notes, no override). Renders 7 rows at
  `00h 00m`, `Target:        40h 00m`, and a `Carry-in:` equal to the value
  produced by walking every intervening week — not zero, and not week N's
  carry-out.
- **C8 — multi-week idle-gap carry, end-to-end (F8b).** Seed week N with
  data, week N+1 with **zero** punches, week N+2 with data. Assert week
  N+2's rendered `Carry-in:` equals the value Milestone 6 computes for the
  idle-gap case, and that it differs from what a naive skip-empty-weeks
  walk would produce (N+1's full deficit must be visible in it). Also
  render week N+1 itself: 7 zero rows, its own carry-in from N.
- **C9 — anomaly marker end-to-end + exit code 0 (§6.2/§6.3).** Seed an
  orphaned end mid-week; the affected row carries `[!]`, other rows do
  not, and the process exits `0`.
- **C10 — current vs. closed headline end-to-end.** With `now` inside week
  W: `mlm week` headline matches Milestone 9's deadline form and names
  **today's** weekday; `mlm week <W-1>` headline matches the plain form
  and contains no weekday name. (Milestone 11 asserts only that it prints
  Milestone 9's string in the right slot; the wording itself is Milestone
  9's test.)
- **C11 — clap shape.** `mlm week target 2026-07 20h` never reaches the
  week-view path; `mlm week 7 8` is a clap error.
- **C12 — plain-ASCII over real output.** Repeat A11's byte scan on C1's
  and C8's actual process stdout, catching anything the wiring (e.g. a
  headline from Milestone 9) might inject that fixtures would not.

---

## 6. Risks, ambiguities and disagreements

**R1 — Test harness for DB path and "now" is unassigned in PLAN.md (high).**
Milestone 11's acceptance criteria (E13, F8b, F6) are stated
end-to-end-through-the-CLI, but `db.rs` hardcodes `ProjectDirs` and PLAN.md
assigns no milestone the job of making the DB path overridable; PLAN.md's
contract 6 pins "now" as injectable at the *library* level but says nothing
about the binary. Recommendation, needed before Group C can run: an
`MLM_DB_PATH` env override (or a hidden `--db` global flag) and an
`MLM_NOW` RFC-3339 override read once in `main`, both test-only in spirit.
Mitigation baked into this plan: the pure `render_week` split means **every
layout assertion in §7.2/§7.3 is provable without the harness**; only carry
and parsing plumbing need it, and those degrade gracefully to
resolver-level tests.

**R2 — §7.2 asserts `(ongoing)` is "only possible on today's row", which is
not true of the data model (medium).** §4.3 makes a single unmatched
trailing `start` an open stint computed live against now, on *any* date —
E15 deliberately creates one on a past date. Taken literally, a past week
containing such a stale start could render `(ongoing)`. Resolution adopted
here: gate `(ongoing)` on `date == now's local date`, matching §7.2's
stated intent. **Consequence worth escalating**: a lone stale dangling
start on a past date then renders as a bare `00h 00m` row with *no*
`(ongoing)` and *no* `[!]` (because §4.3 says one trailing start is not an
anomaly) — E15's day-one punch becomes invisible in the week view. That is
what the spec says today; it is arguably a spec gap for the wave-5
cross-cutting pass, not something Milestone 11 should fix unilaterally.

**R3 — Two independent paths compute "worked" (medium).** Milestone 6
derives per-week worked minutes from its own source; Milestone 11 derives
per-date minutes from Milestone 5. Nothing in PLAN.md forces them to agree,
yet §7.2's `Worked:` line is by construction the sum of the 7 rows.
Recommendation: both should bottom out in one shared "completed-stint
minutes for a date" primitive. Minimum mitigation if that is not done: test
B8 asserts the invariant, so a divergence fails loudly instead of shipping
a table whose rows do not add up to its own total.

**R4 — Row suffix ordering when a row is both ongoing and anomalous is
unspecified.** SPEC.md never shows the combination. Decided here:
`(ongoing)` first, `  [!]` last (§7.2 calls the marker "appended to its
row"). Locked by test A7; cheap to flip if reviewed otherwise.

**R5 — Headline punctuation: SPEC.md and PLAN.md disagree.** SPEC.md's
rendered examples use a colon (`Total still owed: 03h 10m`, §7.2;
`Total still owed: 01h 40m`, §7.1); PLAN.md's Milestone 9 prose writes
`"Total still owed <owed>"` with none. The rendered spec examples are
authoritative. Milestone 11 is insulated either way — it prints Milestone
9's string verbatim — but Milestone 9 must be told to follow SPEC.md.

**R6 — `Total ahead`: sign handling unspecified.** §7.2's aside gives
`Total ahead: 00h 00m` for a week "that ended at/above target", i.e. `owed
<= 0`, while also calling it "same signed summary formatting". Rendering
`Total ahead: -00h 50m` would be a double negative. Recommended rule:
`owed > 0` -> `Total still owed: fmt(owed)`; `owed <= 0` -> `Total ahead:
fmt(-owed)` (so an exactly-met week reads `Total ahead: 00h 00m`,
matching the example). **This is Milestone 9's call, not Milestone 11's** —
flagging it so it is not silently decided twice. The same question exists
for the deadline form on an ahead-of-pace current week (§5 explicitly
allows a negative `owed` mid-week, but §7.2 shows no example); Milestone 9
owns that too.

**R7 — §3.6 and §7.2 disagree on the trailing block's contents (low).**
§3.6 lists "week total, carry-in, target, fulfillment, still owed"; §7.2
fixes the order as carry-in, worked, fulfillment, target and explicitly
drops "still owed" from the block. §7.2 (the layout section, and the later
NOTES.md decision) wins; §3.6 is a prose summary written before decision
25. No action beyond following §7.2.

**R8 — Contract 4's `is_ongoing` is per-row, but the decision needs `now`
and the current-week flag (low).** The rollup builder therefore needs
`now`'s local date as an input; it must not be inferred from the span.
Stated explicitly so a fixture-driven implementation does not omit it.

**R9 — Milestone 4 may not expose a date-range punch read (low, but confirm
early).** §3.1's fallback (7 single-date reads) is behaviourally identical;
keeping the rollup builder's input as "already-fetched punches" makes the
choice a one-line change rather than a rework.

**R10 — clap's positional-vs-subcommand resolution needs empirical
confirmation (low).** The `args_conflicts_with_subcommands` pattern in §1.1
is the standard clap v4 shape for this, but the exact interaction with an
`Option<String>` positional should be nailed down by test C11 first, before
the rest of the wiring is built on it.
