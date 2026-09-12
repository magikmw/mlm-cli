# Milestone 9 — Shared rendering helpers (implementation plan)

**Status**: plan only, no code written. Ready to hand to a TDD subagent.

**Spec basis**: SPEC.md §7.1, §7.2, §7.3, §4.2, §5; NOTES.md decisions
18, 25, 26, 32, 37, 38; PLAN.md "Milestone 9" + interface contracts 2,
3, 5, 6.

**Why this milestone exists**: Milestones 10 (`status`) and 11 (`week`)
are built in parallel worktrees and both need (a) the
current-vs-past/future week headline decision and (b) the §7.3 anomaly
rendering in two different shapes. Without a single shared
implementation they will each grow their own — and diverge. Everything
below is written so that neither 10 nor 11 ever has to *decide*
anything about wording, sign handling, or "what counts as an anomaly";
they only ever call.

**Deliverable**: one new module, `src/render.rs`, containing pure
functions and small value types. No DB access, no CLI wiring, no
`chrono::Local::now()` anywhere — "now" is always a parameter
(PLAN.md contract 6).

---

## 0. Dependencies this milestone assumes (and how to unblock TDD today)

| From | What Milestone 9 needs | If it hasn't landed yet |
|---|---|---|
| Milestone 1 | `format_minutes(i64) -> String` — the one canonical `HHh MMm` formatter (§4.2), signed with a leading `-` | Write tests against a locally-defined stub with the exact §4.2 behavior; swap the import at integration. Do **not** reimplement padding/sign logic permanently. |
| Milestone 2 | A week-id type exposing ISO year + ISO week number | See §3 below — Milestone 9 only needs two accessors, specified there. |
| Milestone 5 | Stint classification result | Milestone 9 defines its own `Anomalies` value type (§2) that M5's result converts into; M9's tests construct it directly. |
| Milestone 6 | Week accounting result | Milestone 9's primitive functions take plain `i64` minute values, not M6's struct (§1). A thin adapter is added at integration. |

This is deliberate: **every function specified below is testable right
now with nothing but `chrono` and a stub formatter.** That is the point
of extracting the milestone.

### 0.1 Shared layout constant

Both §7.1's lead lines and §7.2's trailing block left-align a label into
a fixed-width field. Count the spec examples:

```
Day total:     07h 25m      -> "Day total:"    = 10 chars + 5 spaces = 15
Week 2026-07:  10h 45m ...  -> "Week 2026-07:" = 13 chars + 2 spaces = 15
Carry-in:      -02h 10m     -> "Carry-in:"     =  9 chars + 6 spaces = 15
Worked:        31h 25m      -> "Worked:"       =  7 chars + 8 spaces = 15
Fulfillment:   29h 15m      -> "Fulfillment:"  = 12 chars + 3 spaces = 15
Target:        40h 00m      -> "Target:"       =  7 chars + 8 spaces = 15
```

So:

```rust
/// Width of the left-hand label column shared by §7.1's lead lines and
/// §7.2's trailing block. Labels are left-aligned and padded to this
/// width; the value starts at column 16 (1-indexed).
pub const LABEL_WIDTH: usize = 15;
```

Exported from `render.rs` so Milestones 10 and 11 pad identically
(`format!("{:<w$}{}", label, value, w = LABEL_WIDTH)`). A week id is
always exactly 7 characters (`YYYY-WW`), so `"Week <id>:"` is always 13
and the padding never degrades.

---

## 1. The week headline decision

### 1.1 Exact string templates (character-for-character from the spec)

Two forms, and only two. Extracted verbatim:

| Form | Template | Spec source |
|---|---|---|
| Current week | `{owed} left by end of {weekday}` | §7.1 line 430, §7.2 line 499 |
| Past/future, behind | `Total still owed: {owed}` | §7.1 line 486, §7.2 line 522 |
| Past/future, at-or-ahead | `Total ahead: {owed}` | §7.2 line 539 (prose) |

Worked examples the tests must reproduce byte-for-byte:

```
10h 45m left by end of Thursday
Total still owed: 01h 40m
Total still owed: 03h 10m
Total ahead: 00h 00m
```

Notes on the templates, each of which is a place an independent
implementation would get it wrong:

- `{weekday}` is the **full English weekday name** (`Thursday`), i.e.
  `chrono`'s `%A`, **not** the 3-letter abbreviation used in the
  §7.1 header (`Thu 2026-02-12`) and §7.2 row labels (`Mon 2026-02-09`).
  Both forms appear in the same output; do not share a formatter
  between them.
- The plain forms **have a colon** after the phrase
  (`Total still owed: 03h 10m`). PLAN.md's prose writes them without
  one (`"Total still owed <owed>"`). **The spec's worked examples win**
  — see Risk R1 in §5.
- The current-week form has **no** colon and **no** trailing period.
- `{owed}` is `format_minutes(owed_minutes)` from Milestone 1 — the
  same `HHh MMm` formatter as everywhere else (§4.2), never a bespoke
  one.
- The headline returned here is the **bare headline only**. §7.2 uses
  it as a standalone line; §7.1 uses it as the tail of a labelled line.
  Neither the `Week 2026-07:  ` prefix nor §7.1's
  `(fulfillment ... / target ...)` parenthetical is part of it —
  see §1.4.

### 1.2 Sign handling (the sharp edge)

Three distinct rules, kept separate on purpose:

1. **Current-week form**: render `owed_minutes` through
   `format_minutes` **verbatim, sign included**. A current week that is
   already ahead renders `-02h 30m left by end of Thursday`. §4.2
   mandates one display rule everywhere and nothing in §7 authorizes a
   special case here. See Risk R2 — this is the wording decision most
   worth a human confirming.
2. **Choosing between the two plain forms**: `Total still owed` when
   `owed_minutes > 0`; `Total ahead` when `owed_minutes <= 0`. §7.2's
   prose says "for a week that ended at/above target", and at-target
   means `fulfillment == target` means `owed == 0` — so zero belongs to
   `Total ahead`, which is exactly what the spec's `Total ahead: 00h 00m`
   example shows.
3. **`Total ahead`'s value is a magnitude**: render
   `format_minutes(-owed_minutes)` (equivalently `carry_out_minutes`),
   which is `>= 0` in this branch and so never carries a sign. Rationale:
   `Total ahead: -00h 50m` is a literal double negative that reads as
   "behind". The spec's only `Total ahead` example is the zero case,
   which is identical under either rule, so this is a judgment call —
   see Risk R3.
   - This makes `format_minutes` negative-zero behavior load-bearing:
     `-0` must render `00h 00m`, never `-00h 00m`. Assert it here even
     though it is Milestone 1's responsibility.

### 1.3 Signatures

```rust
use chrono::{Datelike, NaiveDate};

/// Which framing §7.1/§7.2's headline uses for a week.
///
/// NOTES.md decisions 18/25/26: the deadline framing exists only for
/// the week that "today" is actually inside; every other week reports
/// a plain final total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekFraming {
    /// `today` falls inside this week.
    Current,
    /// This week is already over, or has not started yet.
    Closed,
}

/// True iff `today` falls inside the ISO week identified by `week`.
///
/// Compares (ISO year, ISO week) tuples — never calendar year, never a
/// date-range containment check against a Mon-Sun span, both of which
/// go wrong across a Dec/Jan boundary (NOTES.md decision 2).
pub fn week_framing(week: WeekId, today: NaiveDate) -> WeekFraming;

/// The §7.1/§7.2 headline for a week, in its bare form (no `Week NN:`
/// label, no fulfillment/target parenthetical).
///
/// `owed_minutes` is Milestone 6's signed `owed` for that week.
/// `today` is the injected "now" date (PLAN.md contract 6) — the
/// weekday in the deadline phrase is always *today's*, never that of
/// whatever date the caller happens to be displaying (§7.1, F11).
pub fn week_headline(week: WeekId, owed_minutes: i64, today: NaiveDate) -> String;
```

**The F11 nuance is enforced structurally, not by discipline**:
`week_headline` takes no "date being displayed" parameter at all. There
is no argument a caller could pass that would make the weekday come out
as anything other than today's. Milestone 10 passes the `DATE` it is
rendering only to *choose which week* to account for (that is
Milestone 6's input, per decision 26); it never reaches the headline.
This is the single most important design property of this milestone and
a reviewer should check it survives.

Implementation sketch (three lines, no branching on dates beyond the
tuple compare):

```
framing = week_framing(week, today)
Current => format!("{} left by end of {}", fmt(owed), today.format("%A"))
Closed if owed > 0 => format!("Total still owed: {}", fmt(owed))
Closed            => format!("Total ahead: {}", fmt(-owed))
```

### 1.4 The §7.1-specific composition (recommended addition to scope)

§7.1's week line is *not* just the headline:

```
Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)
Week 2026-02:  Total still owed: 01h 40m
```

The current-week example carries a `(fulfillment X / target Y)`
parenthetical; the past-week example does not. If Milestone 9 returns
only the bare headline, Milestone 10 has to decide *when* the
parenthetical applies — which is exactly the kind of independent
decision this milestone exists to eliminate. **Recommend extending
scope by one function:**

```rust
/// The complete §7.1 week line, label and all, e.g.
/// `Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)`
pub fn status_week_line(
    week: WeekId,
    owed_minutes: i64,
    fulfillment_minutes: i64,
    target_minutes: i64,
    today: NaiveDate,
) -> String;
```

Rule (see Risk R4): the parenthetical is emitted for the `Current`
framing and omitted for `Closed`, tied to the *same* `week_framing`
call that picks the wording — so the two can never disagree.
Parenthetical template, verbatim from §7.1:
` (fulfillment {fulfillment} / target {target})` — leading space, lower-case
words, spaced slash, both values through `format_minutes`.

Milestone 11 needs no equivalent wrapper: §7.2's headline is the bare
string on its own line.

### 1.5 Adapter over Milestone 6's result (integration-time, not now)

Once Milestone 6's accounting struct exists, add thin forwarding
wrappers so callers pass one value instead of unpacking minutes:

```rust
pub fn week_headline_for(acct: &WeekAccounting, today: NaiveDate) -> String;
pub fn status_week_line_for(acct: &WeekAccounting, today: NaiveDate) -> String;
```

These must contain **no logic** beyond field extraction and delegation
to the primitives above. The primitives stay the tested surface; a
reviewer should reject any behavior that lives only in the adapter.

---

## 2. The anomaly-rendering split

### 2.1 The problem being solved

`status` needs full sentences (§7.3):

```
[!] 2 open stints for this date (unmatched starts)
[!] orphaned end at 18:00 (no matching start)
```

`week` needs only a marker appended to a row (§7.3):

```
  Wed 2026-02-11   08h 00m  [!]
```

PLAN.md contract 2 requires both to be driven by the same data with no
divergent "what counts as an anomaly" logic. The mechanism below makes
divergence structurally impossible: `detail_lines()` is non-empty
**iff** `has_any()` is true, and both read the same two fields.

### 2.2 The value type

```rust
use chrono::NaiveTime;

/// The §4.3 pairing anomalies for one date, in the minimal shape the
/// §7.3 renderers need. Milestone 5's classification result converts
/// into this; nothing here re-derives pairing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Anomalies {
    /// How many unmatched trailing `start`s the date has (i.e. how many
    /// open stints). 0 or 1 is normal and is NOT an anomaly (§4.3,
    /// §1.3, F1); 2 or more is the multi-open anomaly (E7).
    pub open_stint_count: usize,
    /// Local wall-clock time of each orphaned `end` (§4.3's empty-stack
    /// case), in the date's punch order (chronological, insertion-order
    /// tie-break). One entry per orphan — never deduplicated, never
    /// coalesced, even when two share a timestamp (E8, NOTES.md 32).
    pub orphaned_end_times: Vec<NaiveTime>,
}
```

Design notes:

- `open_stint_count`, not a pre-computed `multi_open: bool`. The detail
  line needs the number (`2 open stints...`), and deriving the boolean
  from the count keeps one source of truth. A bool plus a count would
  let them disagree.
- `orphaned_end_times` is a `Vec`, not a count. E8 / NOTES.md decision
  32 require one line *per orphan naming its own timestamp*, so the
  count alone is insufficient and a `HashSet` would wrongly merge
  same-instant duplicates (legal — punches carry a surrogate id, §2.3).
- Times are **local wall-clock** `NaiveTime`, already converted per
  §2.1's per-instant rule by the time they reach rendering. Milestone 9
  does no timezone work whatsoever.
- `Default` gives the "clean date" case (`0` opens, no orphans) for free
  — used for the 6 empty rows of a typical `week` table.

### 2.3 The three methods

```rust
impl Anomalies {
    /// The single definition of "this date has an anomaly", shared by
    /// both rendering forms. §4.3: exactly one trailing unmatched start
    /// is the ordinary open stint, not an anomaly.
    pub fn has_any(&self) -> bool {
        self.open_stint_count > 1 || !self.orphaned_end_times.is_empty()
    }

    /// `status`'s form (§7.3): one full line per anomaly, already
    /// prefixed with `[!] `, no trailing newline on any element.
    /// Empty vec when `has_any()` is false.
    pub fn detail_lines(&self) -> Vec<String>;

    /// `week`'s form (§7.3): the suffix to append to a date's row.
    /// `"  [!]"` when `has_any()`, `""` otherwise.
    pub fn row_marker(&self) -> &'static str;
}
```

`detail_lines()` construction, in this fixed order:

1. If `open_stint_count > 1`, exactly one line:
   `format!("[!] {} open stints for this date (unmatched starts)", open_stint_count)`
   - Always plural (`stints`) — the branch is only reachable at `n >= 2`,
     so no singular form is ever needed or should be written.
   - Verbatim from §7.3's `[!] 2 open stints for this date (unmatched starts)`.
2. Then one line per entry of `orphaned_end_times`, **in vector order**
   (no re-sorting — Milestone 5 already supplies punch order):
   `format!("[!] orphaned end at {} (no matching start)", t.format("%H:%M"))`
   - Verbatim from §7.3's `[!] orphaned end at 18:00 (no matching start)`.
   - `%H:%M`, zero-padded 24h, matching the stint-line time format in
     §7.1. Never `%R`-with-seconds, never 12h.

`row_marker()` exact spacing: §7.3's example is
`  Wed 2026-02-11   08h 00m  [!]` — **two** spaces between the duration
and `[!]`. So the returned suffix is `"  [!]"` (2 spaces + marker, no
trailing space), appended by Milestone 11 to an otherwise-finished row.
PLAN.md describes it as a `` `[!] ` `` suffix (trailing space); the spec
example has no trailing space. Spec wins — Risk R5.

Interaction with `(ongoing)`: §7.2 shows
`Thu 2026-02-12   07h 25m (ongoing)` (one space before `(ongoing)`) and
§7.3 shows the marker after the duration, but no example combines them.
Proposed order — **duration, then `(ongoing)`, then the marker**:
`  Thu 2026-02-12   07h 25m (ongoing)  [!]`. Risk R6. Milestone 11 owns
the row assembly; Milestone 9 only supplies the suffix string, so this
ordering should be written into Milestone 11's brief, not enforced here.

### 2.4 Conversion from Milestone 5 (integration-time)

Milestone 5's result (PLAN.md contract 2) carries completed stints, the
open stint(s), and the orphaned ends. Add, at integration:

```rust
impl From<&StintClassification> for Anomalies { ... }
```

mapping open-stint count and orphaned-end local times across. It must
contain **no filtering or thresholding** — the `> 1` rule lives in
`has_any()` alone. Milestone 11's per-day rollup (PLAN.md contract 4)
sets its `has_anomaly` boolean by calling `Anomalies::has_any()` on
that date's conversion, never by writing its own condition. A reviewer
should grep Milestones 10 and 11 for any comparison against
`open_stint_count` or `orphaned_end_times` and reject it — those two
fields are read in exactly one place, `has_any()`, plus
`detail_lines()`'s formatting.

---

## 3. Interface contract 3 — the "is this the current week" boolean

**Answer: Milestone 9 computes it itself, from the week id plus the
injected `today`. It does not consume a boolean from Milestone 6.**

Reasoning:

- The decision is a pure two-integer comparison
  (`(iso_year, iso_week)` of the week vs. of `today`). It carries no
  accounting state and belongs to the rendering layer that acts on it.
- Milestone 6's job (§2.4, §5) is the carry walk. It has no reason to
  need "now" at all — its inputs are a target week id, per-week worked
  totals, and overrides. Forcing a `now`-dependent field into its result
  makes an otherwise purely arithmetic, trivially-testable unit
  clock-dependent, and gives the project **two** places that can decide
  what "current week" means.
- Milestone 9 is on the critical path *after* Milestone 6, so taking the
  dependency the other way (M6 calling M9's predicate) is not possible
  in the wave ordering either.

**What Milestone 9 needs from Milestone 2** (not Milestone 6) — the
week-id type must expose ISO year and ISO week number as plain integers:

```rust
impl WeekId {
    pub fn iso_year(&self) -> i32;
    pub fn iso_week(&self) -> u32;
}
```

That is the *entire* dependency. Milestone 9 needs no Mon-Sun span, no
date arithmetic, no `Ord`. If Milestone 2 names these differently
(e.g. `year()` / `week()`), only `week_framing`'s body changes.

**What to do with PLAN.md contract 3's boolean field**, flagged for
cross-check once Milestone 6's plan lands:

- *Preferred*: Milestone 6 **drops** the field from its result. Its
  acceptance criteria (PLAN.md lines 330-353) never mention or test it —
  it is pure contract residue.
- *If Milestone 6 keeps it* (e.g. because its plan already landed with
  it): Milestone 9 still computes its own, and the integration step adds
  a test asserting the two agree across the §4 table cases plus the
  year-boundary cases. Under no circumstances should `week_headline`
  branch on a caller-supplied boolean — that reopens exactly the
  divergence this milestone closes, and would let a caller force the
  deadline framing onto a closed week.

**Action item**: whoever reviews Milestone 6's plan should confirm one
of those two outcomes explicitly before Milestone 9's worktree opens.

---

## 4. Test plan

All tests live in `src/render.rs`'s `#[cfg(test)]` module. All are pure
— no DB, no filesystem, no clock. Reference date throughout:
**`today = 2026-02-12`, a Thursday, ISO week `2026-07`** (matching
§7.1/§7.2's worked examples; `2026-01-01` is a Thursday, so ISO week 1
of 2026 is Mon 2025-12-29 – Sun 2026-01-04 and week 7 is
Mon 2026-02-09 – Sun 2026-02-15, exactly as §7.2's header states).

### 4.1 Headline — current week (§7.1/§7.2 worked example)

| # | Input | Expected |
|---|---|---|
| T1 | `week_headline(2026-07, 645, 2026-02-12)` | `"10h 45m left by end of Thursday"` |

645 minutes = `10h 45m`, cross-checked against §7.2's block:
fulfillment 29h15m = 1755, target 40h = 2400, owed = 645. ✓

### 4.2 Headline — past week (§7.2's second worked example, and §7.1's)

| # | Input | Expected |
|---|---|---|
| T2 | `week_headline(2026-06, 190, 2026-02-12)` | `"Total still owed: 03h 10m"` |
| T3 | `week_headline(2026-02, 100, 2026-02-12)` | `"Total still owed: 01h 40m"` |

T2 cross-check against §7.2's second block: worked 36h50m = 2210,
carry-in 0, fulfillment 2210, target 2400, owed 190 = `03h 10m`. ✓
T3 is §7.1's second example (`Week 2026-02:  Total still owed: 01h 40m`;
week 2026-02 is Mon 2026-01-05 – Sun 2026-01-11, containing the
`Mon 2026-01-05` that example renders). ✓

### 4.3 F11 — a different day within the current week uses TODAY's weekday

| # | Scenario | Expected |
|---|---|---|
| T4 | Rendering Monday `2026-02-09`'s status on Thursday `2026-02-12`: `week_headline(2026-07, 645, 2026-02-12)` | `"10h 45m left by end of Thursday"` — **not** `Monday` |
| T5 | Same, via `status_week_line(2026-07, 645, 1755, 2400, 2026-02-12)` | `"Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)"` |

Include an explicit comment in T4 that the displayed date is
`2026-02-09` and that **the API deliberately provides nowhere to put
it** — the test documents the structural guarantee. A regression that
added a `date` parameter would have to change T4's call to break it,
which is the signal we want.

### 4.4 F10 — a past date shows the plain total

| # | Input | Expected |
|---|---|---|
| T6 | `status_week_line(2026-02, 100, ?, ?, 2026-02-12)` | `"Week 2026-02:  Total still owed: 01h 40m"` — no weekday, no parenthetical |

Assert the output contains none of `Monday`..`Sunday`, no
`left by end of`, and (per §1.4's rule) no `fulfillment`.

### 4.5 Future week

| # | Input | Expected |
|---|---|---|
| T7 | `week_headline(2026-10, 2400, 2026-02-12)` | `"Total still owed: 40h 00m"` |

A not-yet-started week gets the same plain form as a closed one
(§7.2 prose: "A **past or future** week has no 'today'…").

### 4.6 `Total ahead` and sign handling

| # | Input | Expected | Note |
|---|---|---|---|
| T8 | `week_headline(2026-06, 0, 2026-02-12)` | `"Total ahead: 00h 00m"` | §7.2's literal example; zero belongs to `ahead`, not `still owed` |
| T9 | `week_headline(2026-06, -50, 2026-02-12)` | `"Total ahead: 00h 50m"` | §5's `2026-04` owed of −50; magnitude rule (§1.2 rule 3, Risk R3) |
| T10 | `week_headline(2026-06, 1, 2026-02-12)` | `"Total still owed: 00h 01m"` | the `> 0` boundary, one minute the other side of T8 |
| T11 | `week_headline(2026-07, -150, 2026-02-12)` | `"-02h 30m left by end of Thursday"` | current week, already ahead; sign passes through verbatim (§1.2 rule 1, Risk R2) |
| T12 | `format_minutes(-0i64)` / `week_headline(.., 0, ..)` | never contains `-00h 00m` | negative-zero guard |

### 4.7 ISO year-boundary framing (the tuple-compare trap)

| # | Input | Expected | Why |
|---|---|---|---|
| T13 | `week_framing(2026-01, today = 2025-12-29)` | `Current` | Mon 2025-12-29 is in ISO week 1 of **2026**; a calendar-year comparison returns `Closed` and fails |
| T14 | `week_framing(2026-53, today = 2027-01-01)` | `Current` | Fri 2027-01-01 is in ISO week **53 of 2026** (2026 starts on a Thursday, so it has 53 ISO weeks); a calendar-year comparison fails |
| T15 | `week_framing(2025-01, today = 2025-12-29)` | `Closed` | same calendar year as T13's `today`, different ISO week — the inverse trap |

### 4.8 Anomaly detail lines (§7.3, E7, E8)

| # | `Anomalies` | Expected `detail_lines()` |
|---|---|---|
| T16 | `{ open: 2, orphans: [18:00] }` | `["[!] 2 open stints for this date (unmatched starts)", "[!] orphaned end at 18:00 (no matching start)"]` — §7.3's example block, in that order |
| T17 | `{ open: 3, orphans: [] }` | `["[!] 3 open stints for this date (unmatched starts)"]` |
| T18 | `{ open: 0, orphans: [09:15, 13:00, 18:00] }` | **three** separate lines, one per orphan, in that order — never coalesced (E8, NOTES.md 32) |
| T19 | `{ open: 0, orphans: [13:00, 13:00] }` | **two** identical lines — same-timestamp orphans are distinct punches and are not deduplicated |
| T20 | `{ open: 1, orphans: [] }` | `[]` — one open stint is the ordinary ongoing stint (§4.3, F1), not an anomaly |
| T21 | `Anomalies::default()` | `[]` |
| T22 | `{ open: 0, orphans: [7:05] }` | line reads `orphaned end at 07:05` — zero-padded hour |

Also assert every produced line starts with the literal `"[!] "` and
contains no trailing newline and no trailing whitespace.

### 4.9 Has-anomaly boolean and the row marker

| # | `Anomalies` | `has_any()` | `row_marker()` |
|---|---|---|---|
| T23 | `default()` (0 open, no orphans) | `false` | `""` |
| T24 | `{ open: 1, orphans: [] }` | `false` | `""` |
| T25 | `{ open: 2, orphans: [] }` | `true` | `"  [!]"` |
| T26 | `{ open: 0, orphans: [18:00] }` | `true` | `"  [!]"` |
| T27 | `{ open: 1, orphans: [18:00] }` | `true` | `"  [!]"` |
| T28 | `{ open: 5, orphans: [1:00, 2:00] }` | `true` | `"  [!]"` |

### 4.10 The no-divergence invariant (the point of the milestone)

| # | Assertion |
|---|---|
| T29 | For every fixture in T16-T28: `a.has_any() == !a.detail_lines().is_empty()` |
| T30 | For every fixture: `a.row_marker().is_empty() == !a.has_any()` |
| T31 | Exhaustive sweep: for `open_stint_count` in `0..=4` × orphan-list lengths `0..=3` (20 combinations), assert T29 and T30 both hold |

T31 is cheap, needs no proptest dependency, and is the regression test
that catches anyone later adding a condition to one form and not the
other.

### 4.11 Row-composition sanity (spec-example reproduction)

| # | Assertion |
|---|---|
| T32 | `format!("  Wed 2026-02-11   08h 00m{}", a.row_marker())` with an anomalous `a` equals §7.3's `"  Wed 2026-02-11   08h 00m  [!]"` byte-for-byte |

This keeps the marker's exact leading spacing pinned to the spec
example rather than to Milestone 11's eventual row builder.

### 4.12 Plain-ASCII audit (§7)

| # | Assertion |
|---|---|
| T33 | Every string produced by any function in this module across all fixtures above is `is_ascii()` — no unicode dashes, no box-drawing |

Note this only proves it for values that are themselves ASCII;
`format_minutes` and the weekday names are ASCII by construction.

---

## 5. Risks, ambiguities, and disagreements with PLAN.md / SPEC.md

**R1 — PLAN.md drops the colon from the plain headline forms.** PLAN.md
line 442 writes `"Total still owed <owed>"` / `"Total ahead <owed>"`;
SPEC.md §7.1 line 486 and §7.2 line 522/539 all show a colon
(`Total still owed: 01h 40m`, `Total ahead: 00h 00m`). This plan follows
the spec's worked examples. *Impact if wrong:* every past-week headline
is off by one character; caught immediately by T2/T3. **Resolution:
spec wins; PLAN.md's prose is a paraphrase, not a template.**

**R2 — A current week that is already ahead has no specified wording.**
§5 explicitly allows a negative `owed` for "the *current, still-open*
week" (meaning ahead of pace), and §7.1 says the pace hint is "shown the
same signed way as any other summary value", but neither §7.1 nor §7.2
shows the deadline form with a negative value. This plan renders it
literally: `-02h 30m left by end of Thursday`. That is arithmetically
consistent and §4.2-conformant but reads awkwardly. The alternatives —
switching to `Total ahead:` mid-week, or a third phrasing like
`02h 30m ahead with Thursday to go` — both contradict NOTES.md decision
18, which ties the wording split to current-vs-closed and nothing else.
**Needs a human decision before Milestone 10/11 ship; the plan's choice
is the conservative, spec-literal one.** Whatever is chosen, it lives
here and only here.

**R3 — `Total ahead`'s value: signed or magnitude?** §7.2's only
example is `Total ahead: 00h 00m`, where the two rules coincide. This
plan renders the magnitude (`Total ahead: 00h 50m` for `owed = -50`),
since `Total ahead: -00h 50m` inverts the meaning of the sentence.
Counter-argument: §4.2 says the signed format is used "everywhere",
including for a "negative `owed`". **Flag for confirmation**; T9 is the
one test that changes if the ruling goes the other way, and it changes
in one place.

**R4 — §7.1's fulfillment/target parenthetical has no stated rule.**
The current-week example has it; the past-week example does not. The two
examples differ along *two* axes at once (current-vs-past week, and
`DATE`-is-today vs. not), so the spec does not determine which axis
gates the parenthetical. This plan ties it to the week framing (same
`week_framing` call as the wording) because §7.1's bullet on the week
line discusses only the current-vs-past split, and because the
`DATE`-is-today axis is already explicitly and exhaustively enumerated
in §7.1/§3.5 as gating exactly two things — the daily-target hint and
est.-EOD — with the parenthetical never mentioned among them.
**Flag for confirmation.** The contrary reading (parenthetical whenever
`DATE` is today) would require `status_week_line` to take an
`is_today: bool`, which weakens the F11 structural guarantee in §1.3 —
a reason to prefer the framing-based rule beyond the textual one.

**R5 — Marker spacing.** PLAN.md line 445 writes the week marker as
`` `[!] ` `` (trailing space); SPEC.md §7.3's example line is
`  Wed 2026-02-11   08h 00m  [!]` — two *leading* spaces, no trailing
space. Following the spec; `row_marker()` returns `"  [!]"`.

**R6 — `(ongoing)` + `[!]` on the same week row is unexampled.** §7.2
shows `(ongoing)` and §7.3 shows `[!]`, never together, yet both are
possible on today's row (an open stint plus an orphaned end is E15's
cross-midnight case exactly). This plan proposes
`07h 25m (ongoing)  [!]`. Milestone 9 only supplies the `"  [!]"`
suffix, so the ordering decision must be written into **Milestone 11's**
brief explicitly rather than left for it to invent.

**R7 — RESOLVED.** Contract 3 has been amended: `WeekAccounting`
carries no `is_current_week` field. Milestone 6's plan has been
updated accordingly. This milestone's `week_framing`/`week_headline`
computing "is current" themselves from `(WeekId, today)` — never
consuming a boolean from elsewhere — is exactly the design that
prevailed; no further action needed.

**R8 — Weekday name locale.** `chrono`'s `%A` is English-only by
default (no `unstable-locales` feature enabled in Cargo.toml), which
matches the spec's `Thursday`. If localization is ever added, `%A` would
silently start producing non-ASCII and break the §7 plain-ASCII
commitment (T33). Not a problem today; worth a one-line comment at the
call site so a future feature flag does not sneak past.

**R9 — Milestone 9 must not be handed rendering that belongs to 10/11.**
Scope boundary, stated so the TDD agent does not drift: this milestone
owns the headline string, the §7.1 week line, the anomaly detail lines,
the anomaly row marker, and `LABEL_WIDTH`. It does **not** own the
`status` header, day-total line, daily-target hint, est.-EOD, stint
lines, notes block, the `week` header, the 7 date rows, or the trailing
carry-in/worked/fulfillment/target block. Those stay in 10 and 11.

**R10 — `format_minutes`'s exact name/signature is assumed.** Milestone
1's plan (PLAN.md lines 148-152) specifies the formatter's *behavior*
precisely but not its name or whether it takes minutes, a `Duration`, or
a newtype. Milestone 9 assumes `fn(i64) -> String` over signed minutes.
If Milestone 1 lands a `Duration`-based signature, only the call sites
in `render.rs` change — but agree it before both worktrees open, since
§4.2's formatter is consumed by Milestones 6, 9, 10 and 11 (PLAN.md's
own cross-cutting note).
