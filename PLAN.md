# mlm — implementation plan

Scope of work only: no code, no pseudocode, no concrete function/type/SQL
names. Each milestone below is meant to be handed to a TDD subagent
(tests first, implement to green) followed by an independent adversarial
review pass, so each one is written to stand alone as a reviewable unit:
a bounded piece of behavior, a spec citation, and acceptance criteria
concrete enough to write tests from without asking clarifying questions.

## Ordering & parallelization strategy

The current scaffold (`cli.rs`, `db.rs`, `time.rs`, `main.rs`) has the
right module shape but placeholder content only (a single flat
`entries` table, naive time parsing with no UTC/local split, a CLI
surface that doesn't match §3). The plan below replaces the scaffold's
contents milestone by milestone.

This revision (post independent review) is written for **parallel
worktrees**, not a strict linear sequence: milestones are numbered for
readability, not execution order. Two of the "trickiest pure-logic"
milestones (5, 6) are deliberately fixture-driven rather than
DB-dependent specifically so they don't have to wait on storage's real
implementation — only on an agreed data-shape contract, pinned in
advance (see "Interface contracts to pin down before parallel work"
below). Skipping that pinning step and letting each worktree guess its
own shapes is the single biggest risk to this working smoothly; do not
open parallel worktrees before those contracts are agreed.

**Wave 1 — start immediately, no dependencies beyond the current
scaffold and the pinned contracts:**
- Milestone 1 (time/duration parsing+formatting)
- Milestone 2 (date/week-id parsing+formatting)
- Milestone 3 (schema/migrations)
- Milestone 5 (stint pairing) — against the pinned punch-shape and
  stint-classification-result contracts, not Milestone 4's actual code
- Milestone 6 (week accounting) — against the pinned week-id and
  worked-minutes/accounting-result contracts, not Milestone 4/5's
  actual code

**Wave 2 — needs specific wave-1 artifacts to land:**
- Milestone 4 (punch/note storage) — needs Milestone 1 (TIME parsing)
  and Milestone 3 (schema)
- Milestone 8 (`week target` command) — needs Milestone 2 (WEEK_ID
  parsing) and Milestone 3 (schema) **only**; it does not actually
  exercise Milestone 6's accounting logic (its acceptance criteria
  never calls into the week-walk), so it belongs in wave 2 rather than
  after week accounting as earlier drafts of this plan implied

**Wave 3 — integration + the shared rendering surface:**
- Swap Milestone 5/6's fixture inputs for Milestone 4's real storage
  reads (no new behavior, just wiring real data through already-tested
  logic)
- Milestone 7 (`start`/`stop`/`note` commands) — needs Milestone 1 and
  Milestone 4
- **Milestone 9 (shared rendering helpers, new in this revision)** —
  needs Milestones 1, 2, 5, 6; exists specifically to stop Milestones
  10 and 11 from each independently reinventing the same
  current-week/headline-framing logic and anomaly-summary shape (see
  "Recommended plan changes" context below and the milestone itself)

**Wave 4 — leaves, need everything above:**
- Milestone 10 (`status` command + rendering)
- Milestone 11 (`week` command + rendering)

**Wave 5 — final verification, no new behavior:**
- Cross-cutting polish pass (error-tier consistency, exit codes, DST
  spot-check, plain-ASCII audit) — re-reads across everything already
  built rather than adding anything new

**Critical path** (the longest genuinely-sequential chain, which sets
the minimum wall-clock time regardless of worker count): Milestone 3
→ Milestone 4 → (fixture-to-real-data integration for 5/6) → Milestone
9 → Milestones 10/11 → cross-cutting. That's five sequential stages.
Skipping the fixture strategy (making 5/6 wait for 4's real
implementation instead) would add a stage and remove the wave-1
parallelism that makes 5 and 6 available on day one.

Non-goals from §1.2 (editing/deleting entries, project tagging, the
ratatui dashboard, shell-prompt integration, non-ISO weeks, 12h time
input, `+N`/`-N` relative dates, backdated punches, cross-midnight
stint pairing) are excluded from every milestone below; none of them
gets a milestone.

## Interface contracts to pin down before opening parallel worktrees

These are handoff shapes between milestones — described in plain
terms, not code. Agree all of these across whoever's picking up wave-1
work *before* any worktree opens; guessing independently here is what
causes painful merge-time rework, not the milestone boundaries
themselves.

1. **Punch value shape** (Milestone 4 → Milestone 5, and what
   Milestone 5's fixtures stand in for pre-integration): an ordered
   instant (UTC + local calendar date), a start/end kind, and an
   id/insertion-order tiebreaker — matching Milestone 4's existing
   "sorted by instant, ties broken by insertion order" contract. Pin
   down the actual *shape* handed across (a full row vs. a lighter
   intermediate value), not just the sort behavior.
2. **Stint classification result shape** (Milestone 5 → Milestones 9,
   10, 11): a list of completed stints (start, end, duration), the
   open stint if any, a multi-open flag, and a list of orphaned-end
   anomalies (each with its own timestamp). Must separate
   *renderable anomaly detail* (full sentences, needed by Milestone
   10's status output) from a cheap *has-any-anomaly* signal (needed
   by Milestone 11's per-row marker) — this split is exactly what
   prevents Milestones 10 and 11 from each building their own adapter
   over Milestone 5's raw output.
3. **Week accounting result shape** (Milestone 6 → Milestones 9, 10,
   11): target, carry_in, worked, fulfillment, owed, carry_out, all
   signed integer minutes, plus an explicit "is this the actual
   currently-ongoing week" boolean rather than each renderer
   recomputing that comparison independently.
4. **Per-day rollup shape for `week`'s 7-row table** (Milestones 5+6 →
   Milestone 11): a per-date list, all 7 calendar dates of the week
   included (empty ones at zero), each with total completed-stint
   minutes, a has-anomaly boolean, and an is-ongoing boolean — a
   distinct contract from Milestone 5's single-date stint list, since
   only Milestone 11 needs the week-wide roll-up.
5. **Shared rendering helpers** (Milestone 9's actual deliverable): the
   current-vs-past/future week decision plus its headline wording
   (deadline-framed vs. plain-total), and the duration formatter
   (already centralized in Milestone 1) — Milestones 10 and 11 both
   call the same implementation, never reimplement either.
6. **"Now" injection convention**: every milestone from 5 onward that
   needs "current time" (open-stint duration, EOD estimate, "is this
   week current," the `(ongoing)` marker) takes it as a supplied
   value, never a hidden global clock read. Agree the shape (a plain
   parameter vs. some other convention) once, up front.
7. **Error type/shape for hard errors**: every parsing/storage
   function that can hard-error (Milestones 1, 2, 4) needs an agreed
   error shape so Milestones 7 and 8's CLI wiring — plausibly built in
   parallel against a stub — doesn't need rework once the real error
   type lands.

---

## Milestone 1 — Time-of-day and duration parsing/formatting

**Spec sections**: §3.1 (TIME input), §4.1 (parsing/conversion to
minute granularity), §4.2 (duration display and DURATION input
grammar), §6.1 (TIME/DURATION hard-error cases).

**Scope**: Replace `time.rs`'s placeholder parsing with the real
`TIME` grammar (`HH:MM`, `HHMM`, `HH`, minute defaults to 0, valid
range `00:00`-`23:59`, `24:00` explicitly rejected) and the real
`DURATION` input grammar (`Hh`, `HhMMm`, `MMm`, non-padded, converting
to a plain integer minute count). Also implement the one canonical
duration *display* formatter (`HHh MMm`, zero-padded, hour part never
dropped, signed with `-` prefix for negative values, used identically
by both `status` and `week` output later). No DB, no CLI, no calendar
dates yet — this is pure string-in/value-out logic.

**Acceptance criteria**:
- Valid `TIME` forms (`9:05`, `17:30`, `0905`, `1730`, `9`, `17`) parse
  to the correct hour/minute.
- `24:00`, `25:00`, `9:75`, and non-matching strings (`abc`) are
  rejected as errors, not accepted or silently clamped.
- Valid `DURATION` forms (`20h`, `33h30m`, `45m`, `0h`) parse to the
  correct minute count; `0h` parses successfully to zero.
- Malformed `DURATION` (`10` with no unit, `-5h`, `10x`) is rejected;
  a negative parsed minute value is rejected even if the grammar
  otherwise matches.
- The duration formatter renders `07h 45m`, `00h 20m` (zero minutes
  never drops the hour part), and negative values as `-00h 50m`,
  `-03h 20m` (sign in front, same padding).
- Covers E1 and E4 at the parsing-unit level (final hard-error
  plumbing through the CLI is exercised again in later milestones that
  wire these parsers into commands).

---

## Milestone 2 — Calendar date and week-id parsing/formatting

**Spec sections**: §2.3 (`week_targets.week_id` shape), §3.5 (`DATE`
argument), §3.6 (`WEEK_ID` argument forms), §6.1 (DATE/WEEK_ID
hard-error cases), §1.3 (week identified by ISO year + week tuple).

**Scope**: Implement `DATE` parsing (`YYYY-MM-DD`, rejecting malformed
shape and invalid calendar dates like `2026-02-30`) and `WEEK_ID`
parsing (`YYYY-WW` or a bare `WW` defaulting year to current,
unpadded numbers normalized, and a genuine per-year ISO-week-count
validity check rather than a flat `1..=53` range). Also implement
week-id formatting (`YYYY-WW`, zero-padded) and deriving a week's
Mon-Sun date span from its id, both needed by `week` output later.

**Acceptance criteria**:
- Valid dates parse correctly; `2026-02-30` and `13/02/2026` are
  rejected (E2).
- `2026-07`, `7` (with an externally-supplied "current year"), and
  `2026-7` (unpadded) all parse to the same normalized week id (E3).
- `0`, `abcd`, and a week number exceeding its year's actual ISO week
  count (e.g. `2027-53` when 2027 has only 52) are rejected (E3) —
  verified against at least one real 53-week year and one real
  52-week year, not a hardcoded assumption.
- Given a week id, the Mon-Sun date span it covers is computed
  correctly, including a case where the ISO week crosses a
  Dec/Jan year boundary.

---

## Milestone 3 — Schema and migrations

**Spec sections**: §2.2 (migrations), §2.3 (`punches`, `notes`,
`week_targets` tables), §6.1 (DB open/migration failure as hard error;
first-run bootstrap is not an error).

**Scope**: Replace `db.rs`'s placeholder `entries` table with the real
schema (`punches`, `notes`, `week_targets`, plus whatever
`rusqlite_migration` needs to track applied versions) as an ordered,
embedded migration set applied on connect. No punch-pairing or
accounting logic here — this milestone only proves the schema exists,
applies cleanly from empty, and enforces its own constraints (the
`kind IN ('start','end')` check, `target_minutes >= 0` check, indexing
intent on `date`/`at_utc`).

**Acceptance criteria**:
- Connecting against a fresh/missing app-data directory creates the
  directory and database and leaves all three tables present with the
  documented columns — no error (distinct from a genuine DB failure).
- Re-connecting against an already-migrated database is a no-op (no
  duplicate migration application, no data loss).
- Inserting a punch with `kind` outside `start`/`end` is rejected by
  the schema itself.
- Inserting a `week_targets` row with a negative `target_minutes` is
  rejected by the schema itself; a zero value is accepted.
- A connection failure against a path that cannot be opened/created
  (e.g. a file where a directory is expected, or a permissions
  failure) surfaces as an error rather than panicking silently or
  succeeding (feeds E6, fully wired to command behavior in a later
  milestone).

---

## Milestone 4 — Punch and note storage

**Spec sections**: §2.1 (UTC storage, local-to-UTC conversion at
write), §2.3 (`punches`/`notes` column semantics, note trimming),
§6.1 (empty/whitespace-only note rejection, checked pre-trim).

**Scope**: Implement inserting a punch (given a `kind` and a local
wall-clock time already parsed by Milestone 1, converted to UTC and to
a local calendar `date` at write time per §2.1) and inserting a note
(given free text, rejecting empty/whitespace-only before trimming,
storing it trimmed with an insertion-order timestamp). Implement the
corresponding reads: all punches for a date, all notes for a date in
insertion order. No stint pairing, no CLI wiring, no accounting yet —
just correct persistence and read-back.

**Acceptance criteria**:
- A `start` or `end` punch inserted with a given local time is
  readable back with the correct UTC instant and the correct local
  calendar `date` column.
- A note inserted with leading/trailing whitespace is stored trimmed;
  reading it back returns the trimmed text.
- Inserting a note whose body is empty or whitespace-only is rejected
  and nothing is written (E5); a body that is only *padded* (not
  empty after trimming) is accepted (E5's negative case).
- Notes for a date are returned in insertion order (using the
  insertion-order tiebreaker column, not just `id` incidentally).
- Punches for a date are returned in a stable, deterministic order
  suitable for feeding directly into Milestone 5's sort step (sorted
  by instant, ties broken by insertion order) — either the read
  itself sorts this way, or the milestone documents that the caller
  must, but the contract is pinned down here rather than left
  implicit.
- A conversion made just before and just after a DST transition (two
  separate inserts, two separate instants) each records the correct
  local `date`/instant for its own moment — first concrete check
  toward F12/§2.1's per-instant conversion rule, at the storage layer.

---

## Milestone 5 — Stint pairing

**Spec sections**: §4.3 (nearest-match/LIFO pairing algorithm and all
its named edge cases), §1.3 (stint, open stint definitions).

**Scope**: Implement the pairing algorithm over a date's punches
(already sorted per Milestone 4's contract): sort by instant with
insertion-order tie-break, LIFO-match starts to ends, and classify the
result into completed stints, at most one legitimate open stint, a
multi-open anomaly when more than one trailing start remains, and one
flagged anomaly per orphaned end. Operates over plain in-memory punch
data (real rows from Milestone 4 or hand-built fixtures) — no
rendering, no "now" formatting beyond exposing that a stint is open
and computing its live duration against a supplied current-time value.

**Acceptance criteria**:
- F3's worked example (starts at `09:00`/`14:00`, ends at
  `18:00`/`13:00`, entered in that order) pairs into `09:00-13:00` and
  `14:00-18:00`, regardless of entry order — only time order matters.
- A single trailing unmatched start pairs into exactly one open stint,
  not flagged as an anomaly (F1).
- Two or more trailing unmatched starts each become their own open
  stint, and the result is flagged as a multi-open anomaly (E7).
- An end with no unmatched start on the stack becomes its own flagged
  orphaned-end anomaly, one line per orphan, never coalesced when
  there are two or more on the same date (E8).
- A start and its paired end at the identical instant produce a
  legal zero-length (`00h 00m`) stint, not an anomaly (E14).
- Legitimate nested entry (two starts before either end) pairs the
  second start with the nearest subsequent end, not the first
  (matches §4.3's nested-entry note).
- A date with zero punches produces zero stints and zero anomalies,
  cleanly (feeds F4/E11 at this layer).
- Open-stint duration is computed against a supplied "now" value
  rather than hidden global clock access, so tests can pin it exactly.

---

## Milestone 6 — Week accounting (target, carry, fulfillment)

**Spec sections**: §1.1 (carry as a signed adjustment to fulfillment,
not target), §1.3 (target, fulfillment terms), §2.3 (`week_targets`
sparse-override semantics, default 40h), §2.4 (fulfillment/carry
computed at read time by walking every week from the earliest data
week forward, including idle gap weeks; daily-target derivation), §5
(worked example and formulas).

**Scope**: Implement the full week-walk: given a target week id, a
source of per-week worked-minute totals (fed by whatever queries
Milestone 4's storage supports — total minutes per date summed per
week), and a source of target overrides, compute that week's target,
carry-in, fulfillment, owed, and carry-out by walking every ISO week
in sequence from the earliest week with any data through the requested
week — including weeks with zero data in between. Also implement the
daily-target derivation (`week target ÷ 5`, floored). No CLI, no
rendering.

**Acceptance criteria**:
- §5's worked table reproduces exactly: given the four weeks' worked
  minutes and `2026-03`'s override, the computed target/carry_in/
  fulfillment/owed/carry_out for each week match the spec's table
  values.
- The first tracked week (no prior week to walk from) has
  `carry_in = 0` (E12).
- A week with no `week_targets` row uses the default 2400-minute
  target, not an error (§6.2).
- A three-week sequence with data in week N and N+2 but *zero*
  punches/notes in week N+1 produces the same carry-in for N+2 as if
  N+1 had been walked explicitly with zero worked minutes — i.e. the
  deficit/surplus chain is not skipped across the idle week (F8/F8b).
- Requesting a week that has never been touched at all (no punches,
  no notes, no override, anywhere in its own week) still returns a
  full computed result (all-zero worked, default target, carry walked
  from the earliest data week) rather than an error (E13).
- A week whose target override is `0` computes fulfillment/owed with
  no deficit possible against that zero target, and any worked time
  puts it ahead (F7b).
- The daily-target helper returns `week target ÷ 5`, floored to the
  minute, for a representative non-round target value.
- Owed/carry values are allowed to go negative or exceed target in
  either direction with no clamping anywhere in the computation.

---

## Milestone 7 — `start`, `stop`, `note` commands

**Spec sections**: §3.2 (`start`), §3.3 (`stop`), §3.4 (`note`), §1.2
(today-only, no backdating), §6.1 (relevant hard errors), §6.3 (exit
codes).

**Scope**: Wire the CLI surface for the three write commands on top of
Milestones 1 and 4: parse `TIME`/`NOTE` arguments, always target
today's local date, insert the punch (and, for `start`/`stop`, an
accompanying note row when `NOTE` is given), or insert a standalone
note for `note`. Surface hard errors (malformed `TIME`, empty/
whitespace note) as a nonzero exit with a stderr message and no write.
No output rendering beyond whatever minimal confirmation is needed —
`status`/`week` rendering is Milestones 10/11.

**Acceptance criteria**:
- `start` with no arguments inserts a start punch at "now"; with a
  `TIME` inserts it at that time; with a trailing `NOTE` also inserts
  a note row in the same invocation (F5).
- `stop` mirrors `start`'s shape and inserts an `end` punch.
- `note` with only free text inserts a standalone note, unaffected by
  punches (F4).
- A malformed `TIME` on `start`/`stop` exits nonzero, writes nothing
  to either table, and prints a message on stderr (E1).
- An empty or whitespace-only `NOTE` on any of the three commands
  exits nonzero and writes nothing — including the punch itself when
  `NOTE` was attached to `start`/`stop` (E5) — i.e. a rejected note
  does not leave an orphaned punch behind.
- `start`/`stop` accept a `TIME` with no chronology requirement
  relative to existing punches that day (no ordering validation at
  insert time).
- Successful commands exit `0` (§6.3).

---

## Milestone 8 — `week target` command

**Spec sections**: §3.7 (`week target`), §6.1 (malformed
`WEEK_ID`/`DURATION`, missing `DURATION`), §2.3 (`week_targets` upsert
semantics).

**Scope**: Wire the CLI surface on top of Milestones 1, 2, and 3 only
— parse an optional `WEEK_ID` (defaulting to the current week) and a
required `DURATION`, and set/replace that week's target override. This
milestone does **not** depend on Milestone 6's accounting logic (it
only writes a row; Milestone 6 reads it back later) — it belongs in
wave 2, parallel with Milestone 4, not gated behind week accounting.

**Acceptance criteria**:
- `week target 2026-07 33h30m` sets that week's override to the
  correct minute count; a later read of `week_targets` for `2026-07`
  reflects it (F7, storage half).
- Omitting `WEEK_ID` targets the current week.
- `week target ... 0h` is accepted and stores a zero target (F7b).
- A negative-minute `DURATION` is rejected, nonzero exit, no write
  (E9).
- A malformed `WEEK_ID` (per Milestone 2's rules) is rejected (E3,
  applied here).
- Omitting `DURATION` entirely is a clap-level missing-argument error,
  nonzero exit (E10).
- Setting an override for a week that already has one replaces it
  (upsert, not a duplicate row or an error).

---

## Milestone 9 — Shared rendering helpers

**New in this revision**, extracted per independent review of the
original plan, specifically to make Milestones 10 and 11 safely
parallelizable — without it, both would need to independently
implement the identical "is this the actual current week" decision and
its headline wording, risking two subtly different implementations
rather than just duplicated code.

**Spec sections**: §7.1 and §7.2 (both define the same
deadline-framed-vs-plain-total headline split), §7.3 (anomaly
rendering conventions shared by both commands), NOTES.md decisions 18,
25, 26 (the underlying week-framing rule these both implement).

**Scope**: Implement, as a small standalone unit consumed by both
Milestones 10 and 11: (a) given a week (from Milestone 6's result) and
"now," decide whether that week is the actual currently-ongoing one,
and produce the correct headline wording either way — `"<owed> left by
end of <weekday>"` (today's weekday, always, even when the thing being
displayed is a different date within that same current week) or the
plain `"Total still owed <owed>"` / `"Total ahead <owed>"` form; (b)
the anomaly-rendering conventions from §7.3 in both their forms — a
full detail line (`status`'s "one line per anomaly") and a bare
per-row marker (`week`'s inline `[!] `) — built over Milestone 5's
anomaly output so Milestones 10/11 each call one implementation
instead of building their own adapter. No CLI wiring, no full-page
layout — just these two decision/formatting units.

**Acceptance criteria**:
- Given a week that is the actual current one, the headline decision
  returns the deadline-framed wording keyed to today's weekday — even
  when invoked for a non-today date that happens to fall in the same
  week (F11's exact scenario).
- Given a week that is not the current one (past or future), the
  headline decision returns the plain `Total still owed`/`Total ahead`
  wording, with no weekday reference (F10, and §7.2's second worked
  example).
- The full anomaly-detail form renders one line per anomaly, using the
  exact wording from §7.3's examples (`[!] N open stints...`, `[!]
  orphaned end at HH:MM...`), never coalescing two or more orphaned
  ends on the same date into a single line (E8).
- The per-row marker form returns a boolean/marker suitable for
  appending to a `week` table row, correctly true only when the date
  has at least one anomaly, without needing the full detail text.
- Both forms are driven by the same underlying anomaly data (Milestone
  5's output) — no divergent logic between what counts as "has an
  anomaly" for the two forms.

---

## Milestone 10 — `status` command and rendering

**Spec sections**: §3.5 (`status` behavior), §7.1 (status output
layout), §7.3 (anomaly rendering in status), §4.2 (duration format
reuse), all of §2.4's daily-target/EOD derivations as consumed here.

**Scope**: Wire `status [DATE]` on top of everything above: resolve
the target date (today if omitted), pull that date's punches/notes,
run Milestone 5's pairing, run Milestone 6's week accounting for the
week *containing* that date, and render the full layout — header,
day-total line, week line (calling Milestone 9's headline decision
rather than reimplementing the current-vs-not check), anomaly lines
(via Milestone 9's detail-line form), stint list (omitted when empty),
notes list (omitted when empty), and — only when `DATE` is literally
today — the daily-target pace hint and estimated-EOD line in all three
of its states.

**Acceptance criteria**:
- Today with one open stint and no completed ones: day total
  `00h 00m`, no completed-stint lines, one open-stint line (F1).
- An ordinary complete day: correct single stint line, no anomalies
  (F2).
- F3's out-of-order entry renders both resulting stints correctly.
- A note-only day (no punches at all): stint-list section fully
  omitted, notes section present (F4); a day with neither punches nor
  notes omits both sections (E11).
- Estimated-EOD renders in all three states: a clock time when an
  open stint exists and quota remains, the literal `target already
  met` when the gap is already zero or negative, and is omitted
  entirely when there is no open stint (F9).
- `status` for a past date in an already-closed week: no daily-target/
  EOD lines, and the week line uses the plain `Total still owed`/
  `Total ahead` form for *that* week (F10, matches §7.1's second
  worked example exactly).
- `status` for a different day within the current, still-open week:
  the week line still uses the weekday-deadline framing, keyed to
  today's actual weekday, not `DATE`'s (F11).
- Multiple dangling starts on a date: each renders as its own open
  stint line, plus one multi-open anomaly line (E7).
- An orphaned end: produces no stint line of its own, only an anomaly
  line naming its timestamp; two orphaned ends on one date produce two
  separate anomaly lines (E8).
- All duration values in the output use the exact `HHh MMm`
  (zero-padded, signed when negative) format from Milestone 1.
- Output uses plain ASCII only — the `[!] ` anomaly prefix, no
  box-drawing or unicode dashes anywhere in the rendered page (§7,
  cross-checked against every line produced by this milestone).
- Exit code is `0` even when anomalies are present (§6.3).

---

## Milestone 11 — `week` command and rendering

**Spec sections**: §3.6 (`week` behavior), §7.2 (week output layout),
§7.3 (anomaly rendering in week), §4.2 (duration format reuse).

**Scope**: Wire `week [WEEK_ID]` on top of Milestones 2, 5, 6, and 9:
resolve the target week (current if omitted), compute all 7 days'
per-date totals plus that week's target/carry-in/worked/fulfillment/
owed, and render the header (week id + Mon-Sun span), the headline
(calling Milestone 9's headline decision, not reimplementing it), all
7 date rows (always all 7, `00h 00m` for empty ones, `(ongoing)`
marker only on today's row when applicable), the anomaly marker
appended per-row (via Milestone 9's marker form), and the trailing
carry-in/worked/fulfillment/target block.

**Acceptance criteria**:
- The current, ongoing week renders exactly per §7.2's first worked
  example: correct per-day totals, `(ongoing)` on today's row only,
  weekday-deadline-framed headline, correct trailing block values.
- A past, closed week renders the same full 7-row table and trailing
  block, but with the plain `Total still owed`/`Total ahead` headline
  instead (matches §7.2's second worked example, resolving the
  blocker noted in NOTES.md decision 25 — a closed week is not a
  bare one-liner).
- A never-touched week (no data anywhere in it, past or future)
  renders all 7 dates at `00h 00m`, default target, and carry computed
  by walking the full sequence from the earliest data week (E13).
- Multi-week carry across a real gap: seeding week N with data, week
  N+1 with zero punches, and week N+2 with data reproduces the exact
  carry-in on N+2's rendered output that Milestone 6 computed for the
  idle-gap case (F8b, now visible end-to-end through the CLI).
- A date within the week that has a pairing anomaly gets the `[!]`
  marker appended to that row; a date with no anomaly does not.
- `WEEK_ID` accepts both a full id and a bare week number defaulting
  to the current year (F6/§3.6), consistent with Milestone 2's parser.
- A week with no override uses and displays the default 40h target
  (F6).
- All duration values use the exact `HHh MMm` format; plain ASCII
  throughout, no unicode/box-drawing.

---

## Cross-cutting concerns

These apply across multiple milestones rather than belonging to any
one of them. Call them out explicitly in each milestone's test plan
rather than re-deriving them per milestone:

- **DST-safe per-instant conversion (§2.1)**: applies to Milestone 4
  (storage/write path) and Milestone 10 (display path, since a
  `status` for a date spanning or adjacent to a transition must show
  correct local times). F12 is the direct test; it should be exercised
  at least once at the storage layer and once end-to-end through
  `status`, using a real transition date for the system/test
  timezone rather than a synthetic offset.
- **Two-tier error handling (§6)**: hard errors (§6.1, reject/no-write/
  nonzero exit) vs. anomalies (§6.2, accepted/stored/surfaced later)
  is a distinction every write-command milestone (7, 8) and every
  read-command milestone (10, 11) needs to preserve consistently — a
  hard error must never partially write, and an anomaly must never
  block a write. Worth a final consistency check in the last milestone
  rather than trusting each milestone's local tests to add up. This
  pass must also concretely re-test **E6 end-to-end** (a DB open/
  migration failure surfacing as a nonzero exit with a stderr message
  and no write) through at least one write command (e.g. `start`) and
  one read command (e.g. `status`) — Milestone 3 only proves this at
  the schema layer; nothing else currently re-asserts it through an
  actual command invocation, and it should not ship unverified at that
  level.
- **Exit codes (§6.3)**: `0` on success including anomaly-bearing
  output, nonzero only for §6.1 hard errors — check across all six
  commands (start, stop, note, week target, status, week) in one pass
  rather than per-milestone only.
- **Plain-ASCII output (§7)**: applies to Milestones 10 and 11's
  rendering; worth one dedicated scan over all rendered output (byte
  range check or explicit char-set assertion) rather than trusting
  visual inspection of each example.
- **Duration formatting (§4.2)**: implemented once in Milestone 1,
  consumed by Milestones 6, 9, 10, 11 — later milestones should reuse
  the same formatter/tests rather than re-implementing padding/sign
  logic, and review should flag any milestone that doesn't.
- **"Now" as an injectable value**: `status`'s day total, EOD estimate,
  open-stint duration, and `week`'s `(ongoing)` marker all depend on
  "current time," as does Milestone 9's current-week decision. Every
  milestone from 5 onward that touches this should treat "now" as a
  supplied/injectable value rather than a hidden global read, or its
  tests cannot be made deterministic (see contract 6 above).

## Open risks / ambiguities to resolve before the relevant milestone

- ~~Per-week worked-minutes query shape~~ — **resolved**: NOTES.md
  decisions 37/38 confirm totals feeding Milestone 6 are
  completed-stints-only, and an orphaned `end` contributes nothing to
  any total. Already folded into SPEC.md §2.4 and into Milestones 5/6/
  9's acceptance criteria above — a wave-1 worktree should treat this
  as settled, not re-litigate it.
- **Local timezone source in tests**: §2.1 uses the system's local
  timezone throughout, sourced how `chrono`/the OS exposes it. Milestone
  4's and 10's DST tests need a way to pin or select a timezone with a
  known transition date deterministically in a test environment (CI
  runner's system tz is not guaranteed to have a nearby, easily-dated
  transition) — a test-infrastructure decision to make before Milestone
  4 starts, not a spec gap, but it will block writing F12 concretely if
  left unresolved.
- ~~`week`'s per-day row totals vs. anomalies interaction~~ —
  **resolved**: same NOTES.md decisions 37/38 as above cover this too
  (orphans contribute nothing to any total); folded into interface
  contract 2 and Milestone 9's scope.
- **Milestone 5/6 fixture format**: now a first-class pre-work step,
  not left to be discovered inside Milestone 5's worktree — see
  interface contracts 1-4 above. Agree the concrete fixture/contract
  shapes before opening the wave-1 worktrees for Milestones 5 and 6,
  so Milestone 4's eventual real implementation doesn't turn out to
  mismatch what 5/6 assumed.
- **Scaffold's existing `Log` command and `entries` table**: the
  current stub CLI (`cli.rs`) defines a `Log` subcommand and `db.rs`
  defines a placeholder `entries` table, neither of which appears in
  SPEC.md (the spec's commands are `start`/`stop`/`note`/`status`/
  `week`/`week target`; the schema is `punches`/`notes`/
  `week_targets`). Milestone 3 removes the placeholder table and
  Milestones 7/10 replace the CLI enum's shape entirely; noting this
  explicitly so the milestone that touches `cli.rs` doesn't try to
  preserve `Log` by mistake under an assumption it's a real
  requirement — it is leftover scaffold, not spec.
