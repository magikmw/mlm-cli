# Milestone 10 — `status` command and rendering

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


Implementation plan. Detailed enough that a TDD subagent can write tests
first without further clarification.

**Spec basis**: SPEC.md §3.5 (`status` behavior), §7 (plain-ASCII rule),
§7.1 (status layout — the two literal worked examples are the golden
targets), §7.3 (anomaly rendering), §4.2 (duration format), §2.4
(daily-target derivation, completed-stints-only totals), §6.3 (exit
codes), §8 flows F1, F2, F3, F4, F9, F10, F11 and E7, E8, E11.

**Depends on** (design against PLAN.md's *pinned interface contracts*,
not against these milestones' actual code): M1 (duration formatter),
M2 (DATE parsing, week derivation), M4 (punch/note reads), M5 (pairing),
M6 (week accounting + daily-target helper), M9 (headline + anomaly
detail lines).

---

## 1. Module shape and testability

The single most important structural decision: **rendering must be a
pure function returning a `String`**, not a pile of `println!`s. Every
golden test in §5 below runs against that function with hand-built
inputs; only the exit-code test needs the actual binary.

Proposed split (three layers, each independently testable):

1. **`resolve`** — `(date_arg: Option<&str>, now: <injected now>, db) ->
   Result<StatusView, HardError>`. Does all I/O and all arithmetic.
2. **`render`** — `(&StatusView) -> String`. Pure, no clock, no DB, no
   `Option<&str>` parsing. This is the golden-test surface.
3. **`run`** — CLI entry point: calls `resolve`, on `Ok` prints the
   rendered string to stdout and exits `0`; on `Err` prints to stderr
   and exits nonzero.

`StatusView` is a plain owned struct — all values already computed and
already in their final numeric/textual form, so `render` makes no
decisions beyond layout and section omission. Suggested fields (shape,
not a type signature):

- `header_weekday_abbrev: String` + `date: NaiveDate` (or one
  pre-formatted header string)
- `day_total_minutes: i64` (completed stints only)
- `has_open_stint: bool`
- `daily_target: Option<DailyTargetHint>` — `None` whenever DATE is not
  today. When `Some`: `{ target_minutes: i64, gap_minutes: i64 }`
  where `gap_minutes = target_minutes - day_total_minutes` (signed).
- `eod: Option<EodState>` — `None` when the line is omitted entirely;
  otherwise `EodState::At(NaiveTime)` or `EodState::TargetAlreadyMet`
- `week_headline: String` — **produced verbatim by M9**, never assembled
  here (see §4)
- `week_id: String` (formatted `YYYY-WW`, from M2)
- `week_detail_suffix: Option<String>` — the
  `(fulfillment HHh MMm / target HHh MMm)` parenthetical; see §2.4 for
  when it is `Some`
- `anomaly_lines: Vec<String>` — **produced verbatim by M9's detail
  form**, already carrying the `[!] ` prefix
- `stints: Vec<StintLine>` — `{ start: NaiveTime, end: StintEnd,
  duration_minutes: i64 }` where `StintEnd` is `At(NaiveTime)` or `Now`
- `notes: Vec<String>` — bodies in insertion order

**"Now" is injected** (PLAN.md contract 6): `resolve` takes it as a
parameter; nothing below `run` reads a global clock. `run` is the only
place `Local::now()` appears.

---

## 2. Clap shape and resolution flow

### 2.1 Clap

Add to `cli.rs`'s `Command` enum:

```
/// Show a date's stints, notes and totals (defaults to today).
Status {
    /// Date to show, YYYY-MM-DD. Defaults to today.
    date: Option<String>,
},
```

- Positional, optional, no flags. Matches §3.5's `mlm status [DATE]`.
- Take it as `Option<String>` and parse with **M2's DATE parser inside
  the handler**, not via a clap `value_parser`. Rationale: §6.1's
  malformed-DATE hard error then flows through the same error
  type/message path as every other hard error (PLAN.md contract 7)
  instead of clap's own error formatting. (A `value_parser` would also
  work and would still exit nonzero — noted as an accepted alternative,
  but the handler-side parse is the recommendation.)
- **Also delete the scaffold's `Log` subcommand** (PLAN.md's last open
  risk: `Log` is leftover scaffold, not spec). `status` replaces it.
- `--short` (the README/AGENTS.md prompt-integration idea) is **not** in
  scope — it is a §1.2 non-goal.

### 2.2 Resolution flow (exact order)

1. `now` supplied by `run` as a timezone-aware local instant.
2. `today = now`'s local calendar date.
3. `target_date` = if `date` arg is `None` → `today`; else M2's DATE
   parse of the string. Parse failure → hard error, nonzero exit,
   nothing rendered (E2, though E2's own test lives in M2).
4. `is_today = (target_date == today)`. This single boolean gates the
   daily-target hint **and** the EOD line, and nothing else.
5. Read `target_date`'s punches via M4 (already ordered per contract 1:
   by instant, ties by insertion order) and `target_date`'s notes via
   M4 (insertion order).
6. Run **M5's pairing** over those punches with `now` supplied, giving
   the contract-2 classification result: completed stints, open stint(s),
   multi-open flag, orphaned-end list.
7. `day_total_minutes` = sum of **completed stints only** (§2.4 /
   NOTES 37, 38). Open stints and orphaned ends contribute nothing.
8. Derive the ISO week containing `target_date` via **M2**, then run
   **M6's accounting** for that week id. This yields target, carry_in,
   worked, fulfillment, owed, carry_out — no "is current" boolean;
   contract 3 was amended to drop it, since that comparison is M9's
   job alone (see step 9).
9. Call **M9's headline helper** with the M6 result + `today` (not the
   M6 result's week id compared against anything precomputed) — M9
   itself decides whether that week is current, from the week id and
   `today`, per contract 3/9 → the headline string (§4).
10. Call **M9's anomaly-detail helper** with M5's classification →
    `Vec<String>` of `[!] `-prefixed lines.
11. If `is_today`: compute the daily-target hint and EOD state (§3).
    Else both are `None`.
12. Build `StatusView`, hand to `render`.

Note step 8 uses **`target_date`'s** week, not today's — §3.5 /
NOTES decision 26. A `status 2026-01-05` run on 2026-02-12 accounts
`2026-02`, not `2026-07`.

---

## 3. Layout template (golden-test target)

Rendered as a single `String`, lines joined by `\n`, **terminated by
exactly one trailing `\n`, with no trailing blank line**.

### 3.1 Line-by-line algorithm

```
L1   "{weekday_abbrev} {YYYY-MM-DD}"
L2   ""                                       (always)
L3   day-total line                           (always)
L4   week line                                (always)
     if !anomaly_lines.is_empty():
L5     ""
L6+    one line per anomaly, verbatim from M9
     if !stints.is_empty():
       ""
       one line per stint
     if !notes.is_empty():
       ""
       "Notes:"
       one line per note
```

### 3.2 Label column

Both lead lines use a **left-aligned label field padded to 15
characters**, then the value. Verified against every labelled line in
§7.1 and §7.2:

| label | len | pad | total |
|---|---|---|---|
| `Day total:` | 10 | 5 | 15 |
| `Week 2026-07:` | 13 | 2 | 15 |
| `Carry-in:` (§7.2) | 9 | 6 | 15 |

Week ids are always 7 chars (`YYYY-WW`), so the week label is always 13
chars — but implement the generic pad-to-15, not a hardcoded two spaces.

### 3.3 Header (L1)

`"{%a} {%Y-%m-%d}"` — English three-letter weekday abbreviation, a
space, the ISO date. Examples: `Thu 2026-02-12`, `Mon 2026-01-05`.
Must be **locale-independent** (chrono's `%a` is; do not use any
locale-aware formatter).

### 3.4 Day-total line (L3)

Four segments; segments 2-4 are conditional and each is prefixed by
`", "` (segment 2's flag is prefixed by a single space instead — see
the literal example).

```
"Day total:" padded to 15
 + <duration(day_total_minutes)>                          [always]
 + " (+ ongoing)"             if has_open_stint           [any date]
 + ", {duration(gap)} left to {duration(daily_target)} daily target"
                              if is_today
 + ", est. EOD {HH:MM}"       if is_today && has_open_stint && gap > 0
   or ", target already met"  if is_today && has_open_stint && gap <= 0
   or nothing                 if !has_open_stint
```

Full-fat example (§7.1, first block) reproduced exactly:

```
Day total:     07h 25m (+ ongoing), 00h 35m left to 08h 00m daily target, est. EOD 18:35
```

Minimal example (§7.1, second block):

```
Day total:     06h 15m
```

Points to respect:

- Day total **never** folds ongoing time in (§7.1: "avoids the total
  silently changing mid-read"). `(+ ongoing)` is a flag, not a value.
- `(+ ongoing)` is driven by `has_open_stint` alone, and is **not**
  gated on `is_today` — see §6 ambiguity A4.
- `gap` is signed and uses the same `-HHh MMm` format (§7.1: "Negative
  once the day total already meets/exceeds it — shown the same signed
  way as any other summary value"). So a day over quota with no open
  stint renders e.g.
  `Day total:     08h 30m, -00h 30m left to 08h 00m daily target`.
- EOD clock time is `HH:MM`, 24h, zero-padded, **local time**.

### 3.5 Week line (L4)

```
"Week {week_id}:" padded to 15
 + <M9 headline string>
 + " (fulfillment {dur} / target {dur})"   if current-week form only
```

Current-week form (§7.1 first example):

```
Week 2026-07:  10h 45m left by end of Thursday (fulfillment 29h 15m / target 40h 00m)
```

Closed/future-week form (§7.1 second example):

```
Week 2026-02:  Total still owed: 01h 40m
```

The parenthetical suffix appears in the first example and **not** in
the second; pin it to the deadline-framed form only, so both literal
examples golden-match. Flagged as an inference — see §6 ambiguity A2.

`week_id` here is the week containing `target_date` (§3.5), formatted by
M2. Note the second example shows `2026-02` while "today" is in
`2026-07` — that difference is the whole point of NOTES decision 26 and
is a required assertion in test T10.

### 3.6 Anomaly block

Verbatim M9 output, **not indented** (§7.3's example shows column 0),
one line per anomaly, separated from the week line by one blank line and
from the stint list by one blank line:

```
[!] 2 open stints for this date (unmatched starts)
[!] orphaned end at 18:00 (no matching start)
```

Ordering: multi-open line first (at most one), then orphaned ends in
ascending timestamp order. §7.1 places the block "between the two lead
lines and the stint list". Both the surrounding blank lines and the
intra-block ordering are inferences — see §6 ambiguity A3.

### 3.7 Stint list

Section **omitted entirely** (including its leading blank line and with
no placeholder) when there are zero stints — §7.1, F4, E11. An orphaned
end produces **no** stint line (E8).

Each line: two-space indent, then a **range field padded to 11
characters**, then two spaces, then the parenthesised duration.

```
"  " + pad(range, 11) + "  (" + duration + [", ongoing"] + ")"
```

- Closed: `range = "{HH:MM}-{HH:MM}"` → exactly 11 chars.
- Open: `range = "{HH:MM}-now"` → 9 chars, padded to 11 (hence the four
  spaces before `(` in the literal example).

```
  09:00-13:00  (04h 00m)
  14:05-17:30  (03h 25m)
  17:45-now    (00h 15m, ongoing)
```

Ordering: by start time ascending (ties by insertion order, inherited
from M5's sort). Open stints sort into the same list by their start
time — with a single open stint that naturally puts it last, matching
the example; with the multi-open anomaly (E7), each open stint is its
own line in start order.

### 3.8 Notes

Section **omitted entirely** when the date has zero notes (§7.1, E11).

```
Notes:
  - fixed migration runner bug
  - started punch pairing tests
```

Header line `Notes:` at column 0, then `"  - " + body` per note, in M4's
insertion order. Bodies render **verbatim** — already trimmed at storage
(§2.3), no re-trimming, no wrapping, no escaping, no truncation in MVP.

---

## 4. Current-week determination (do not recompute)

**Rule: Milestone 10 never decides this itself.**

**Corrected from an earlier draft of this plan**: M6's accounting
result does **not** carry an `is_current_week` boolean — contract 3
was amended to drop it after cross-plan review. M9 computes "is this
the current week" itself, directly from the week id (M6's result
carries the week id) and `today` — it is never handed a precomputed
boolean from M6 or anywhere else. Milestone 10's job is unchanged
either way:

- M9 consumes the M6 result (week id + owed figures) plus `today` and
  returns the finished headline string — deadline-framed (`"{owed}
  left by end of {Weekday}"`, keyed to **today's** weekday) or plain
  (`"Total still owed: {owed}"` / `"Total ahead: {owed}"`).
- Milestone 10 stores that string in `week_headline` and prints it after
  the label. It must not contain a `Weekday` reference of its own, must
  not compare week ids, and must not branch on `is_today` for this.

The F11 case falls out for free: `status 2026-02-09` (Monday) run on
Thursday 2026-02-12 has `is_today == false` (so no daily-target/EOD),
but M9 independently determines that `2026-02-09`'s week (`2026-07`)
is the actual current week (comparing it against `today`, not against
`target_date`), so M9 returns `"... left by end of Thursday"` —
Thursday from `today`, not Monday from `target_date`. **Confirmed
against M9's landed plan**: the weekday is derived from `today`, and
the plain form's colon punctuation matches SPEC §7.1/§7.2 exactly
(`Total still owed: 01h 40m`) — both PLAN.md and M9's own plan were
corrected to match; no outstanding bug.

---

## 5. Test cases

All render tests call `render(&StatusView)` with hand-built fixtures and
compare against a full multi-line expected string (golden, byte-exact).
Resolution tests call `resolve` with a fixed injected `now`, an
in-memory/temp DB seeded via M4, and assert on the resulting
`StatusView` fields. Where a flow needs both, write both.

Baseline fixture for the "today" family: `now = 2026-02-12 18:00 local`
(Thursday), week `2026-07`, target 2400, daily target 480.

| # | Flow | What it asserts |
|---|---|---|
| **T1** | F1 | Fresh day, one `start` only, no `end`. Day total `00h 00m`; `(+ ongoing)` present; zero completed-stint lines; exactly one stint line ending `-now` with `, ongoing`; no anomaly lines; notes omitted. |
| **T2** | F2 | `start 09:00` / `stop 17:30`. Exactly one stint line `09:00-17:30  (08h 30m)`; day total `08h 30m`; no `(+ ongoing)`; no EOD line (no open stint); daily-target hint **present** (is_today, gap independent of EOD); no anomalies. |
| **T3** | F3 | Punches entered `start 09:00`, `start 14:00`, `stop 18:00`, `stop 13:00`. Renders exactly two lines, `09:00-13:00  (04h 00m)` then `14:00-18:00  (04h 00m)`, in that order; day total `08h 00m`; no anomalies. Guards that entry order never leaks into output. |
| **T4** | F4 | Note-only day: zero punches, two notes. Stint section **entirely absent** (assert the string contains no `(` duration line and no double blank run); `Notes:` block present with both bodies in insertion order; day total `00h 00m`. |
| **T5** | E11 | Zero punches **and** zero notes. Output is exactly four lines: header, blank, day-total, week line — plus the single trailing newline. Byte-exact assertion; this is the tightest omission test. |
| **T6a** | F9 state 1 | Open stint, gap > 0: `, est. EOD HH:MM` present, value `= now + gap`. Use the §7.1 numbers (total `07h 25m`, target `08h 00m`, now `18:00`) and assert the literal `18:35`. |
| **T6b** | F9 state 2 | Open stint, gap `<= 0`: segment reads `, target already met` and contains no `est. EOD`. Test **both** gap `== 0` and gap `< 0` (boundary: zero must take the met branch, per §7.1's "already zero or negative"). Also assert the daily-target segment still renders, with a signed negative gap in the `< 0` case. |
| **T6c** | F9 state 3 | No open stint: neither `est. EOD` nor `target already met` appears anywhere; `(+ ongoing)` absent; daily-target segment still present. |
| **T7** | F10 | `status 2026-01-05` with `now = 2026-02-12 18:00`. **Golden-match §7.1's second example verbatim**, all five lines. Asserts: no `daily target` substring, no `est. EOD`, no `target already met`, week label reads `Week 2026-02:` (DATE's week, not today's), headline is the plain `Total still owed: 01h 40m` form, no `(fulfillment ... / target ...)` suffix. |
| **T8** | F11 | `status 2026-02-09` (Monday) with `now = Thursday 2026-02-12`. Week line keeps the deadline framing and names **Thursday**; the string contains no `Monday`. Day-total line has **no** daily-target segment and no EOD segment (is_today false). Separately assert at the `resolve` level that `is_today == false` while M9's own current-week check (computed independently from the week id and `today`, not from any M6 field) returns `true` — the two are independent and must not be conflated. |
| **T9** | E7 | Three unmatched `start`s on one date. Three separate stint lines each ending `-now  (..., ongoing)` in start order; exactly one anomaly line `[!] 3 open stints for this date (unmatched starts)`; day total unaffected by the open time. |
| **T10** | E8 | Two orphaned `end`s (e.g. `18:00`, `19:30`) plus one clean pair. **Two** anomaly lines, one per orphan, naming each timestamp — never coalesced into a count; the orphans produce **no** stint lines; day total counts only the clean pair. |
| **T11** | golden, full | Reconstruct §7.1's **first** example exactly (day total `07h 25m`, ongoing, daily target `08h 00m`, EOD `18:35`, week `2026-07` owed `10h 45m`, fulfillment `29h 15m`, target `40h 00m`, three stints, two notes) and assert byte-equality with the spec block. The single highest-value test in this milestone. |
| **T12** | §4.2 consistency | Over the output of T11, T7, T9 and T6b: extract every duration-shaped substring and assert each matches `^-?\d{2,}h \d{2}m$`. Guards against any locally-formatted duration slipping in — every one must come from M1's formatter. |
| **T13** | §7 plain ASCII | Over the rendered output of T11, T7, T9, T10 (all with **ASCII-only note bodies**), assert every byte is `0x20..=0x7E` or `\n`. Explicitly no en/em dashes, no box drawing, no `…`. **Important**: §2.3 permits non-ASCII in note *bodies*, so this assertion must be run against ASCII-only fixtures, or scope itself to non-note lines. A companion test should confirm a non-ASCII note body passes through **unmangled** rather than being stripped or replaced. |
| **T14** | §6.3 exit code | Binary-level: seed a DB with an E7/E8 anomaly, run `mlm status`, assert exit status `0` and that the anomaly lines appear on **stdout** (anomalies are data, not diagnostics — stderr stays empty). |
| **T15** | §6.1 / E2 | Binary-level: `mlm status 2026-02-30` and `mlm status 13/02/2026` exit nonzero with a message on stderr and nothing on stdout. |
| **T16** | DST (cross-cutting) | `status` for a date adjacent to a real local-tz DST transition renders stint times in that date's own offset (§2.1, F12's display half). **Blocked on** PLAN.md's open risk "Local timezone source in tests" — coordinate with whatever M4 settles on; do not invent a second mechanism here. |

Test-infrastructure notes for the implementing agent:

- `render`'s golden tests need **no** DB and no `cargo` test harness
  beyond `#[test]` — build `StatusView` literals directly.
- T14/T15 need to invoke the binary. Per PLAN.md's consolidated
  dev-dependency decision, `assert_cmd`/`predicates` are **explicitly
  declined** — drive the binary via `std::process::Command` against
  `env!("CARGO_BIN_EXE_mlm")` instead, which needs no new deps. The DB
  path must be redirectable for tests via `MLM_DB_PATH` (Milestone 3's
  fix) — if `db.rs` still hardcodes `ProjectDirs` with no override,
  that is a blocker to raise against M3/M4 rather than
  work around here.
- Where a test depends on M9's exact headline wording, assert on the
  full rendered line anyway (not a substring) — if M9's wording differs
  from SPEC §7.1, the failure should surface here loudly.

---

## 6. Ambiguities, risks, disagreements

**A1 — PLAN.md drops a colon that SPEC.md has.** PLAN.md Milestone 9
writes the plain form as `"Total still owed <owed>"` / `"Total ahead
<owed>"`; SPEC.md §7.1 and §7.2 both show `Total still owed: 01h 40m`
and `Total ahead: 00h 00m`, **with** a colon. SPEC is the source of
truth — the colon stays. Raise with M9 so both milestones' golden tests
agree.

**A2 — the `(fulfillment X / target Y)` suffix is only shown once.**
§7.1's current-week example has it; its closed-week example does not.
Nothing in the prose says whether that is deliberate. This plan pins it
to the deadline-framed form only, because that is the only reading under
which both literal examples golden-match. If the intent was "always,"
the closed-week example in §7.1 is wrong and needs a SPEC fix — worth
confirming before T7 is written, since T7 golden-matches that example.

**A3 — anomaly block spacing and ordering are unspecified.** §7.1 says
only "between the two lead lines and the stint list"; §7.3 shows the
lines but not their surroundings, and no worked example combines
anomalies with a full page. This plan pins: blank line before, blank
line after, column-0 (unindented), multi-open line first, then orphaned
ends by ascending time. All four are inferences. If a reviewer disagrees
it is a one-line change in `render` plus the affected goldens.

**A4 — `(+ ongoing)` on a non-today date.** §4.3 classifies a trailing
unmatched `start` as an open stint regardless of date, and §1.3 says an
open stint's "duration is computed live against the current time when
displayed." Taken literally, `status` for a *past* date that has a
dangling `start` (exactly what E15's midnight-crossing case leaves
behind on day one) renders `23:30-now  (2153h 15m, ongoing)` and a
`(+ ongoing)` flag on a day that ended weeks ago. This plan implements
the literal reading — there is no spec text carving out past dates, and
inventing one here would diverge from M5's classification. **But it is
almost certainly not what a user wants**, and it interacts badly with
E15, which SPEC §1.2 explicitly accepts as a known limitation. Flagging
for a spec decision; a plausible fix (out of scope for this milestone)
is to render a past-date open stint as an unterminated anomaly rather
than a live one. Consequence to test either way: **M1's duration
formatter must handle 3+ digit hour values** without panicking or
clamping (`2153h 15m`), and the T12 regex allows `\d{2,}h` for exactly
this reason.

**A5 — the EOD estimate is a moving target (design concern, implement
as specified).** §7.1 defines `EOD = now + (daily target − day total)`,
and day total excludes the open stint's live minutes. So while a stint
is open, `now` advances but `day total` does not, and the estimate
slides forward minute for minute — it never arrives. Worked check
against the spec's own example: total `07h 25m`, target `08h 00m`, gap
`35m`, open stint started `17:45` with `00h 15m` elapsed so `now =
18:00`, and the spec prints `18:35`. The alternative reading (credit the
open stint: `18:00 + (35 − 15)`) would print `18:20`, so **the example
pins the literal formula** and this plan implements it. Worth raising
with the spec owner regardless: `stint_start + (daily target − day
total)` would give a stable figure that behaves like a real ETA. Do not
"fix" it inside this milestone — T6a golden-matches `18:35`.

**A6 — `target already met`: partial or whole replacement?** §7.1 says
the EOD is "replaced with `target already met`". This plan replaces the
**entire** `est. EOD HH:MM` segment, so the line ends `..., target
already met` (not `..., est. EOD target already met`). PLAN.md
Milestone 10's acceptance criterion says "the literal `target already
met`", consistent with this reading. Low risk, but it is an inference.

**A7 — daily-target source.** §2.4 and §7.1 both define it as "today's
week target ÷ 5, floored". Since the hint only renders when
`is_today`, DATE's week *is* the current week, so "today's week target"
and "the M6 result's target" are the same number — take it from **M6's
daily-target helper applied to the M6 result**, never recompute `÷ 5`
locally (PLAN.md cross-cutting: duration/derivation logic lives in one
place). Note the helper divides the **target**, which is unaffected by
carry (§1.3) — a week carrying a deficit does **not** get a raised daily
target. Also note `÷ 5` is hardcoded to a 5-day week even for a `0h`
target week (F7b), giving a `00h 00m` daily target and a gap that is
negative from the first minute worked; that is consistent with the spec
but produces `target already met` on a day-off week, which reads oddly.
Not a blocker.

**A8 — multi-line note bodies.** §2.3 places no charset or content
restriction on note bodies, so a body containing `\n` will break the
`  - ` list alignment. MVP renders verbatim (no escaping, no wrapping).
Noted, not handled.

**A9 — `resolve` reads notes even on an anomaly-heavy date.** No
interaction, but worth stating: notes and punches are fully independent
(§3.4), so the Notes block's presence never depends on stints and vice
versa. E11 is the conjunction of both omissions, not a separate rule.

**A10 — scaffold removal.** This milestone deletes `cli.rs`'s `Log`
variant and its `main.rs` arm. Confirmed as intended by PLAN.md's final
open-risk entry. If M7 (`start`/`stop`/`note`) lands first and has
already restructured the enum, merge onto its shape rather than
reverting it.
