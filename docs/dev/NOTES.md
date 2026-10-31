# Design notes

Running log of design and process decisions, numbered. Not a spec:
current behavior lives in `docs/dev/SPEC.md`, and decisions here can
change.

## Current state

Read this block at changeset start, not the log below. Update it at
each phase close. Find log entries with `grep -n '^[0-9]\+\. '` and
read only the one you need.

- **Next entry**: 162
- **Last release**: `v0.4.0`
- **`main`**: `3e9213f` (`v0.4.0` + five unreleased changesets)
- **Active changeset**: `doc-drift` (locked, entry 161). `relative-ids`, `panic-fixes`, `export-notes` and `week-notes` are on `main` and unreleased. `orphan-stop-warning` is on `main` and unreleased; the user wants more changesets in the next release
- **Possible issues**: SPEC.md §1.2a (the only list; not duplicated here)
- **Triage rules**: "Triaging review findings" below; apply at every fold

Phase 0 profile (`coordinating-development`), set in entry 65:

| Slot | Value |
|---|---|
| Verification | `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`; pre-commit hook adds cognitive complexity ≤ 15 and a coverage ratchet (`coverage-baseline.json`) |
| Doc root | `docs/dev/` — `SPEC.md`, `NOTES.md`, `specs/`, `plans/`, `plans/reports/` |
| Decision log | this file, numbered entries |
| Model tier | Sonnet everywhere; escalation needs in-the-moment approval |
| Release | `scripts/prep_release.sh` (reversible bump + changelog), then `scripts/release.sh` (irreversible tag + push); read both before running |
| Branches | one per changeset, one per task merged `--no-ff` into it, changeset to `main` at release |
| Changeset id | topic slug, not a number |
| Spec location | `docs/dev/specs/<date>-<slug>.md` |
| Plans | `docs/dev/plans/<slug>-plan.md`, `<slug>-task-N-<topic>.md`; reports in `plans/reports/` |
| Artifact | `cargo build --release` → `target/release/mlm` |
| User-visible surface | `README.md`, `mlm --help` and subcommand `--help`, `status`/`week`/write-command output |
| Context-starvation hook | absent |

## Workflow note (process, not product)

When we get to implementation: use subagents for TDD (write tests first,
implement to green) and a separate independent adversarial-review
subagent pass before merging — not the same agent that wrote the code.

At final spec review (before implementation starts): give §6 (error
handling & validation) particular scrutiny by mapping out concrete
user flows end-to-end — tests will be written directly against those
flows/assumptions, so gaps there become gaps in test coverage.

## Triaging review findings (process, not product)

mlm reports what is in the data. The user audits their own punches in
`status`. mlm does not guess intent or guard against mistakes the user
can see there. Reviewers are told to over-report, so the coordinator
filters at every fold. Set in entry 117, after `orphan-stop-warning`
grew three new mechanisms and 13 known issues from findings on its own
fixes.

- Drop a finding that restates the feature. New output appearing where
  there was none is the change, not a gap.
- Drop a finding that matches behavior the project already accepts
  elsewhere, such as `start` doing the same thing.
- Drop a finding whose harm needs the user to misread their own day,
  when `status` would show the result straight away.
- A finding whose fix needs a new mechanism (a threshold, a trigger, a
  hint rule) is the user's call. Default to dropping it or filing it as
  a known issue. Never fold one in as a coordinator decision.
- Findings about a fix's own edge cases mean the fix is too big. Cut
  the fix back. Do not patch its edges.
- A known issue is a rough edge a user hits in normal use and is
  misled by. Everything else is dropped, with one line in the decision
  log saying so.
- One spec review round. A second round is needed only when the fold
  added a mechanism, and that is itself a sign the fold went wrong.

## Current manual process (plain text file)

Each work day:

- Line per stint: wall-clock start time, then end time gets appended to
  the same line once known, plus that stint's summed duration (h+m).
  Sums usually happen incrementally, not all at EOD.
- A running work log above the day's time lines: short free-text notes
  on anything substantial done that day. Sometimes prefixed with a
  project name if specific enough (project tracking = stretch goal,
  not MVP).
- Below the stint lines: a day total (sum of stints).
- At EOD, a week-status block, updated daily:
  - hours worked this week so far
  - hours spilled over from last week (cap: 40h/week is the max that
    counts as "regular" — nothing above 40 becomes overtime, so only
    a *shortfall* spills forward, not surplus)
  - hours still owed by end of current day, given the above
  - a spillover/deficit note
- Dashed line delimits the start of a new week.

Pain point: summing time in your head is tedious, especially
incremental resumming through the day. Motivating the tool.

## MVP scope (from conversation)

- Internal time unit: minutes. Keep it simple.
- CLI to add start/end times for stints, **not necessarily
  chronologically** (need to handle out-of-order entry/edits).
- CLI to attach notes (day work log entries), separate from stints.
- Output (read-only, MVP):
  - today's status: all of today's stints, today's notes, day sum,
    week sum so far
  - a separate whole-week view
- Schema: normalized, designed to be easy to extend (migrations
  support from day one, since schema/data will evolve with features).

## Deferred / stretch

- Project tagging per note/stint.
- Terminal dashboard (ratatui, see README) — bar/sparkline views.
- Shell prompt integration (fast, side-effect-free status query).

## Decisions (from Q&A)

1. **Week boundary**: ISO week (Mon start) for MVP. Design should not
   hardcode this so deeply that other week-start conventions become
   impossible later — but supporting them is a stretch goal, not MVP.
2. **Week identity**: weeks are identified by an **(ISO year, ISO week
   number) tuple**, not by a start date. Robust across year boundaries
   (ISO week 1 of a year can include late-December dates and vice
   versa — `chrono` gives us `iso_week()` for this).
3. **Surplus/deficit carry**: the **target stays fixed at 40h by
   default** (or an explicit override, decision 7) — carry does not
   shift the target. Instead, carry-in is folded into the week's
   *fulfillment* sum, like an extra virtual day that itself worked a
   (possibly negative) number of minutes: `fulfillment = sum(this
   week's stints) + carry_in`. Owed = `target - fulfillment` (can
   already be negative, i.e. ahead of target from minute one of the
   week if carry-in is a surplus). Carry-out for the next week =
   `fulfillment - target` at week's end (signed: positive surplus
   *and* negative deficit both propagate). "No overtime" just means
   surplus is never paid out specially in the current week beyond
   being counted plainly toward fulfillment — it's not a bonus, just
   arithmetic.
4. **Stints from point-in-time punches, not stored ranges**: stints
   are *derived* like matched parentheses from a sequence of
   start/end time points, not stored as a single row with two
   columns. This is what makes non-chronological entry natural: you
   insert a point (start or end) at any time value, points get sorted
   by time, then paired sequentially (start, end, start, end, ...) to
   produce stints. Precomputing/caching paired stints in the DB is
   possible later but not worth it for MVP — compute at read time.
5. **Editing/deleting past entries**: deferred, not in MVP. Add-only.
   (Superseded for deleting: `mlm delete note|punch` shipped in 0.3.0,
   see SPEC.md §3.9 — editing
   is still deferred, correction path is delete-then-recreate.)
6. **Timezone**: no explicit open question raised as blocking; assume
   single-machine local wall-clock time unless it comes up again.

## More decisions (from Q&A round 2)

7. **Target-override mechanism**: absolute value only for MVP (no
   delta/relative adjustment). Command takes a week id and a value;
   week id typing defaults to current (year, week) when omitted, and
   the id is written without the ISO `W` prefix (e.g. `2026-07`, not
   `2026-W07`) — KISS on input format.
8. **Dangling/open punch display**: shown as ongoing, duration
   computed live against current time, updates on each view.
9. **Punch surrogate id**: yes — `id INTEGER PRIMARY KEY
   AUTOINCREMENT` on the punch table, independent of `(date, time)`.
   Rationale: a natural `(date, time)` key breaks the moment you edit
   a punch's time (the key itself would be what's changing) and can't
   disambiguate two punches at an identical timestamp. Free in
   SQLite, standard practice, avoids an awkward migration once editing
   lands.

## More decisions (from Q&A round 3)

11. **Time zone**: store UTC internally, convert to/from the user's
    local (system) timezone at input/output only. Local calendar date
    for a punch is therefore computed app-side, not derivable from the
    stored UTC instant by plain string slicing (see SPEC.md §2.1).
12. **Punch time input formats**: `HH:MM`, `HHMM`, `HH` (minute
    defaults to `:00`), 24h only for MVP. 12h AM/PM format is a
    stretch goal.
13. **Note-only entry**: `mlm note NOTE` — a separate command that
    attaches a work-log note to today without touching punches, for
    EOD/next-day notes with nothing to punch.
14. **Week id input shorthand**: accepts a full id (`YYYY-WW`) or a
    bare week number, defaulting the year to the current one.
15. **Relative day/week stretch**: `+N`/`-N` notation for `status`'s
    date argument and `week`'s week-id argument, relative to
    today/this-week. Stretch goal, not MVP.

## More decisions (from Q&A round 4, spec §7)

17. **Daily pace hint on `status`**: not a new independent target —
    a display-only "X left to daily target" figure, where daily
    target = the current week's target ÷ 5 (floor), always derived,
    never overridden on its own.
18. **Week owed framing**: `status`'s week line and `week`'s headline
    both lead with the owed figure. For the *current* week it's
    framed as "`X` left by end of `<weekday>`"; a closed or
    not-yet-started week (no "today" inside it to frame against) uses
    a plain "Total still owed `X`" / "Total ahead `X`" form instead —
    closed weeks are done, they just report their final number.
19. **Estimated EOD**: shown on `status` only when today has an open
    stint — `now + (daily target − day total)`, i.e. "the clock time
    you'd hit today's quota if you kept going from right now."
    Omitted with no open stint; replaced with "target already met"
    once the gap is ≤ 0.

## More decisions (from flow-mapping + adversarial spec review, round 5)

Flow-mapping SPEC.md's §6 against concrete user flows surfaced gaps
G1-G4 (now resolved below); a separate independent adversarial review
pass over the whole spec then found one self-contradiction (blocker)
and several real underspecified-behavior gaps. All folded directly
into SPEC.md; logged here for the record.

21. **G1 — empty-day rendering**: a date with zero stints omits the
    stint-list section entirely (same treatment as Notes with none).
22. **G2 — zero-target weeks**: `0h` is a legal target override (an
    explicit week off), distinct from silently letting a normal
    week's deficit accrue. Only negative durations are rejected.
23. **G3 — note trimming**: note `body` is trimmed of leading/trailing
    whitespace before storage (emptiness is still checked pre-trim).
24. **G4 — backdated entry**: confirmed intentional — `start`/`stop`/
    `note` only ever target today in MVP, no exceptions.
25. **Blocker fix — §7.2 closed-week example**: was contradicting its
    own prose (bare one-liner vs. claimed full breakdown). Resolved:
    a closed/future week gets the same full per-day table and
    carry-in/worked/fulfillment/target block as the current week —
    only the headline wording differs (plain total vs.
    weekday-deadline framing).
26. **`status DATE` week scope**: fixed to show the week *containing*
    `DATE`, not always "today's" week — the old behavior (a past
    date's stints shown next to today's unrelated week totals) read
    as a bug, not a feature. The deadline-vs-plain-total headline
    split from decision 25 applies here too, keyed off whether
    `DATE`'s week is the actual currently-ongoing one (using today's
    weekday for the deadline phrase even if `DATE` itself is a
    different day in that same week) or a past/future one. The
    daily-target and estimated-EOD lines only ever appear when `DATE`
    is literally today, since both depend on "day total so far" and
    "now."
27. **Overnight stints — accepted limitation**: a session crossing
    midnight splits into two anomalies (permanently-open `start` on
    day one, orphaned `end` on day two) since pairing is strictly
    per-`date`. Deliberately not fixed for MVP — pairing across date
    boundaries is real complexity for a rare case.
28. **DST-safe conversion**: every UTC↔local conversion must use the
    offset that applied *at that specific instant*, never a single
    "current" offset reused across punches — otherwise punches either
    side of a DST transition would display incorrectly.
29. **`WEEK_ID` validation**: unpadded week numbers (`2026-7`) are
    accepted and normalized, matching the lenient-input style used
    for `TIME`/`DURATION` elsewhere. The out-of-range check is a real
    per-year ISO-week-count check (52 vs. 53), not a flat `1..=53`.
30. **`TIME` boundary**: `24:00` is rejected outright, not treated as
    a next-day-midnight alias. Valid range is `00:00`-`23:59`.
31. **Zero-length stints**: a `start`/`end` pair at the identical
    instant is legal (`00h 00m`, not an anomaly) — matched-parentheses
    pairing doesn't care about same-instant ordering.
32. **Multiple orphaned `end`s**: each gets its own flagged line
    (§7.3), never coalesced into a single count.
33. **First run vs. DB failure**: a missing app-data directory/db file
    bootstraps silently (already how `src/db.rs`'s scaffold works);
    only a genuinely broken DB (bad permissions, corruption, a failed
    migration) is a hard error.
34. **Concurrency**: explicitly out of scope for MVP — single-user,
    single-machine tool, SQLite's default locking trusted to fail
    safely rather than corrupt data, no WAL/busy-timeout tuning
    planned.
35. **Note content policy**: no length cap, no charset restriction
    beyond embedded `\r`/`\n` being collapsed to a single space rather
    than preserved verbatim (so a stored body is always exactly one
    line, applies to notes written from that point forward only, no
    backfill of rows already stored) — §7's plain-ASCII commitment is
    about rendered layout characters, not what a user can type into a
    note body.

## More decisions (from implementation-plan review, round 6)

The tech-lead planning subagent (PLAN.md) flagged two accounting
questions it had to infer rather than found explicitly stated —
confirmed:

37. **Totals exclude open-stint live time**: any total feeding
    target/carry/fulfillment/owed math counts completed stints only.
    A currently-open stint's live minutes never enter that math — they
    only ever appear on the stint's own display line and the
    estimated-EOD figure (§2.4, §7.1).
38. **Orphaned `end`s contribute nothing to totals**: same treatment —
    an orphaned `end` has no duration to contribute (no paired
    `start`), so it never adds to a day/week sum, only ever surfacing
    as its own flagged anomaly line.

## More decisions (from 11-milestone detailed planning + cross-plan review, round 7)

Per the workflow note at the top of this file, PLAN.md's 11 milestones
each got a detailed dev-agent implementation plan (`plans/*.md`),
followed by an independent adversarial review across all 11 together.
That review surfaced real spec gaps (not just implementation
ambiguity) and cross-plan contract divergences, now fixed in both
SPEC.md and PLAN.md:

40. **§4.3 tie-break fix**: identical-instant punches now break ties
    by kind (`start` before `end`) before `id` — the old `id`-only
    rule contradicted E14's stated zero-length-stint outcome whenever
    `stop` was entered before a same-instant `start`.
41. **DST edge cases added**: a spring-forward gap (a typed local time
    with no real instant) is a hard error; a fall-back-ambiguous time
    resolves to its earlier occurrence. Previously unaddressed in
    SPEC.md despite §2.1's per-instant conversion rule implying they
    exist.
42. **Strict TIME/NOTE positional order**: `start`/`stop`'s `TIME`,
    when given, is always the first positional; a value there that
    fails `TIME`'s grammar is a hard error, never silently
    reinterpreted as `NOTE` text (no shape-sniffing — any rule
    permissive enough to accept free text there also swallows E1's
    required hard-error case).
43. **Write commands are silent on success**: `start`, `stop`, `note`,
    `week target` print nothing; exit `0` is the only success signal
    (§7.4). Previously unspecified, and two independent milestone
    plans had proposed two different confirmation-line wordings.
    `delete note`/`delete punch` are a deliberate, narrow exception
    added by the delete-punches/notes milestone: list mode and a
    successful delete both print to stdout (§7.4,
    `docs/dev/specs/2026-09-13-delete-punches-notes.md` §5) — the
    silence rule above still governs every other write command
    unchanged.
44. **`created_at_utc` is minute-granular, not real-seconds**: fixes a
    genuine §4.1/§2.3 tension ("no seconds precision anywhere" vs. a
    tiebreaker column that needs sub-minute precision to do its job).
    `id ASC` is the actual tiebreaker; the timestamp column orders
    coarsely.
45. **§7.2's `(ongoing)` wording corrected**: it's gated on "this
    date's open stint's date is today, and the week is current," not
    just "any open stint" — a cross-midnight session (E15) can leave
    a stale open start on a past date, which is not what the marker
    means. That stale stint renders silently (no marker, not an
    anomaly either).
46. **§3.6 field order fixed** to match §7.2's actual layout
    (carry-in, worked, fulfillment, target — "still owed" only in the
    headline).
47. **Interface contract cleanup** (PLAN.md): the first planning pass
    produced real divergence the pinning step was meant to prevent —
    four different completions of the error-shape contract, three
    different owners claimed for the same `Punch`/`PunchKind` type,
    two incompatible `WeekId` designs, and a `Minutes` newtype two of
    its three intended consumers had already independently rejected.
    All reconciled to one answer each in PLAN.md's contracts 7-11.
48. **`db::connect()` blocker fixed**: three independent milestone
    plans (4, 8, 10) hit the scaffold's hardcoded path + panic-on-
    failure before writing a single test. Milestone 3 now owns a
    testable `connect_at`, an `MLM_DB_PATH` override, and a
    path-free `apply_migrations` helper.

## More decisions (from wave-1 implementation + review, round 8)

50. **Same-instant end/start boundary defect — deferred, not fixed**:
    Milestone 5's implementation review found that §4.3's kind-before-
    id tiebreak (added for E14) mis-pairs an ordinary back-to-back
    boundary (`stop 09:00` then `start 09:00`, no gap between two real
    stints) — zero-pairs the boundary instead of closing the
    already-open stint, silently dropping its worked time with no
    anomaly. A correct fix needs per-instant-group handling (close an
    already-open start first, only then pair remaining tied end/start
    as an isolated E14 case) — a real algorithm change, not a tiebreak
    tweak. User decision: merge Milestone 5 as-is, fix later. Marked
    directly in SPEC.md §1.2 (non-goals) and §4.3 (inline note on the
    tiebreak step) so it's visible without digging through review
    files.

51. **Multi-week-format stretch**: deferred entirely. Not designing
    for it now; revisit only if it becomes a real ask.

## More decisions (round 9 — user bug report on real usage)

52. **Decision 17 reversed: daily pace hint is now carry-inclusive**.
    User reported the `status` pace hint didn't match their original
    manual process (NOTES.md "Current manual process": "hours still
    owed by end of current day, given the above [carry]"). Discovery
    traced it to decision 17 (round 4) and the §5 note it introduced
    ("no separate day-by-day pacing formula... the original process's
    'how much I should still put in today' and week-level `owed` are
    the same question") — that reasoning was wrong: the manual
    process's day figure was carry-inclusive but day-scoped, distinct
    from the week-level `owed`, and the two decisions had silently
    dropped the carry-in half of it. Renamed the concept
    **required-by-day** (SPEC.md §2.4, §7.1): `daily target ×
    min(ISO weekday, 5)` compared against the same `carry_in +
    worked` fulfillment the week's `owed` uses, instead of against
    plain same-day-only "day total." §5's explanatory note rewritten
    to state the two figures answer different questions and can
    diverge. Decisions 17/19 (round 4) and the day-by-day-pacing
    rejection in §5 are superseded by this entry.
53. **Adversarial review of decision 52 folded**: stale `est. EOD`
    sample number fixed (18:35 → 20:45, re-derived from the new
    `required − fulfillment` gap, not the old day-total one).
    Sat/Sun `required` reworded — it's `5 × daily target`
    (`5 × floor(week target / 5)`), not always exactly the week
    target, since a non-multiple-of-5 override loses up to 4 minutes
    to the floor same as any other day. F9b extended to cover a
    non-multiple-of-5 override on a Sat/Sun `status`. ISO weekday
    numbering cross-referenced to the existing convention already in
    use in the week-accounting code.
54. **Milestone-15 task plan decisions**: weekday-name string carried
    on `StatusView` (not nested inside `DailyTargetHint`) — simpler
    threading to the render site. New F9b edge cases (large carry-in,
    Sat/Sun cap, non-multiple-of-5 override) tested at the `resolve()`
    level against a real DB fixture, not at the render level, since
    that's the layer that actually exercises the formula.

## More decisions (round 10 — boundary stint pairing spec)

55. **Same-instant tie-break and cross-midnight splice designed as one
    changeset** (`docs/dev/specs/2026-09-19-boundary-stint-pairing.md`):
    both of SPEC.md §1.2's deferred stint-pairing defects trace to the
    same root — `stint::classify` only ever looks at a single calendar
    date and its tie-break was only ever patched for the isolated
    zero-length case. Fixed together, no global/cross-history scan:
    (a) within a tied instant group, an `end` now closes an
    already-open `start` from before that instant first, before
    pairing any leftover tied `start`/`end` zero-length — a strict
    generalization, every non-tied group (nearly all real data)
    behaves exactly as before; (b) a new `classify_at(prev, day, next,
    now)` sits on top of the unchanged `classify()` and splices a
    midnight-spanning stint only on an exact 1:1 match — `day`'s one
    trailing open, literal next date's one orphaned `end`, and that
    orphan must be the next date's chronologically first punch (not
    merely its only orphan — rejects an unrelated late-day stray
    orphan being mistaken for a carried-over stint). Minutes land on
    the day the stint started, not the day it crossed into. Anything
    short of that exact shape (2+ opens, 2+ orphans, a gap day with no
    punches in between) stays flagged exactly as today — deliberate
    "mechanical fix only" scope guardrail, not a heuristic that guesses
    among candidates.
56. **Adversarial review of decision 55 folded**
    (`docs/dev/plans/reports/boundary-stint-pairing-review.md`, 7
    findings, ship-with-followups): fixed a wrong NOTES.md citation
    backing the spec's own scope rationale (now cites SPEC.md §4.3
    directly), corrected "three" to "four" `classify()` call sites,
    added README.md's "Known limitations" section to the doc-update
    checklist, added a `debug_assert!` requirement to `classify_at`
    mirroring `classify()`'s own date-consistency check, named the
    residual case where a legitimate carried-over orphan sharing its
    date with one unrelated stray orphan still doesn't splice (1:1
    guardrail working as designed, just not previously called out),
    added a test case for the §3/§4 mechanisms interacting at the same
    boundary, and extended the `worked_minutes` call site's
    perf-justification note to match `build_ledger`'s.

57. **Round 2 adversarial review of decision 55/56 folded**
    (`docs/dev/plans/reports/boundary-stint-pairing-review-round2.md`,
    2 findings, needs-rework): round 1's fix for the missing
    `classify_at` debug_assert (decision 56) indexed `punches[0]`
    unguarded — a plain slice index, not compiled out in release like
    the rest of a `debug_assert!`, so it panicked on every idle day
    bordering a worked one, i.e. the ordinary case once `classify_at`
    replaces `classify()` everywhere. Fixed: gate on `punches.first()`,
    mirroring `classify()`'s own closure-based safety property, and a
    §5.2 test case pins the empty-`punches` shape down. Separately,
    `DbWeekData::worked_minutes`'s widened 9-day fetch would have kept
    its current `by_date.values().sum()` pattern, which — once the
    fetch widens past the week's own 7 dates — sums in adjacent weeks'
    padding-day buckets too, both leaking an unrelated stint into the
    wrong week's total and double-counting it into the neighboring
    week's own total. Spec §4.3 now requires summing over `week.dates()`
    explicitly (matching `build_rows`'s existing pattern), and §5.3
    gains a fixture with a real stint on a padding day specifically,
    since the existing `c7`/`c8` multi-week tests use fully-idle
    padding and wouldn't have caught this.

58. **Task plan judgment calls resolved (boundary stint pairing, Tasks
    1 & 2)**: Task 1's three findings were informational only (a
    clippy cognitive-complexity gate to re-check once code exists;
    confirmation that `DayStints::has_anomaly`'s private field needs no
    restructuring since `classify_at` lives in the same module; one
    extra positive-control test added beyond the spec's literal
    bullets to pin the tie-break/splice eligibility boundary) — no
    changes needed. Task 2's four: (1) rewriting
    `b2_bucketing_does_not_pair_across_midnight`
    (`src/week_view.rs:613-635`) in place, since its assertions encode
    the exact old per-date-only behavior this changeset removes —
    accepted, this is the spec's intended behavior change surfacing in
    an existing test, not scope creep; (2) `build_ledger`'s prev/next
    fetches placed inside the existing `if !day_punches.is_empty()`
    branch rather than unconditionally every iteration — accepted, an
    idle day's `punches` is empty so `classify_at` can't produce a
    splice for it regardless of what its neighbors hold, making the
    fetch-only-when-needed version equivalent in outcome to fetching
    always, not a deviation from the contract; (3) `build_rows`/
    `worked_minutes`'s neighbor-bucket lookups chain
    `pred_opt()`/`succ_opt()` straight into `unwrap_or_default()`
    instead of an explicit `.unwrap()` first — accepted, strictly safer
    (never panics even at chrono's theoretical date-range edge) and
    identical in every real-world case; (4) CI e2e smoke block appended
    at end of file rather than grouped after the backdate block —
    accepted per the plan's own recommendation.

59. **Final review (Opus-escalated, explicit approval) of the shipped
    boundary-stint-pairing changeset**
    (`docs/dev/plans/reports/final-review.md`, 6 findings,
    ship-with-followups): verified §3/§4 correct by mutation testing
    (four targeted breaks, each caught by exactly the intended test)
    and by hand-running the release binary against every spec worked
    example, including a three-week boundary/padding-day scenario —
    all matched. Findings: the SPEC.md/README §6 doc pass was still
    pending at review time (expected, done as part of this same
    round — see decision 60); a few more stale E15/"accepted
    limitation" references beyond §6's original checklist; a
    non-portable `\s` in the new CI grep (BSD/macOS greps read it
    literally, a silent false-negative-safe vacuous match, never a
    false failure); two cosmetic `stint.rs` nits (`splice_candidate`
    returning an `Option<usize>` that can only ever be `Some(0)`, and
    `has_anomaly`'s recomputation duplicating `classify()`'s
    expression instead of sharing one definition); and one
    informational note that the locked spec's own rationale sentence
    for the round-2 `debug_assert` fix is technically imprecise
    (`debug_assert!` *is* compiled out in release; the fix was still
    correct and necessary for debug/test builds) — not worth reopening
    the spec over, per the reviewer's own call.
60. **Fresh-eyes UX check surfaced findings a spec-compliance review
    structurally cannot catch**
    (`docs/dev/plans/reports/boundary-stint-pairing-ux-check.md`, 9
    findings): dispatched a separate agent with the spec, changeset
    plan, and this decision log explicitly withheld — told to read
    only what a real installed-binary user could (`README.md`,
    `--help`) and role-play scenarios, judged against "would a
    stranger be confused," not against any document. Found real gaps
    the final review's spec-conformance lens couldn't have: a spliced
    cross-midnight stint has no visual cue it crosses two dates; the
    date receiving the actual `stop` punch shows zero trace of it now
    (previously at least visible as a flagged orphan); and the
    sharpest one — whether an unclosed stint reaching into the next
    day silently merges or gets flagged-and-left-unmerged depends on
    an internal 1:1-ambiguity gate (§4.3.1) the user has no way to
    observe, undocumented anywhere in output or `--help`. These three
    trace directly to spec §7's deliberate "no new markers, additive
    later if wanted" call — a locked-spec-behavior question, so
    surfaced to the user rather than folded silently. Decision: keep
    §7's design as shipped for this changeset (accepted, deliberate
    KISS tradeoff); split SPEC.md §1.2 into **§1.2 Non-goals**
    (deliberate future scope) and a new **§1.2a Known issues to
    revisit** (shipped-but-rough behavior, not committed to fix) since
    conflating "haven't built this" with "this works but expect a
    rough edge" made triage harder and read worse to a user hitting a
    known issue. All 9 UX findings plus the changeset's own §4.3.1
    residual case now live in §1.2a; README gained a matching "Known
    issues" section pointing at it, replacing the old "Known
    limitations" section (which only ever listed the two now-fixed
    defects).
61. **Boundary & context disclosure cues changeset scoped, then cut
    down** (`docs/dev/specs/2026-09-20-boundary-context-cues.md`):
    started as a fix for six of §1.2a's seven known issues (bullets
    1,2,3,5,6,7), grouped because all six were the same root cause —
    `status`/`week` computing correct state and rendering it without
    disclosing context (a spliced stint's date span, which date a
    punch was typed against, the carry-in component of a negative
    figure) or without documenting an existing rule (the §4.3.1
    splice-vs-flag gate, the lone-unclosed-`start` distinction).
    Bullet 4 (anomaly remedy pointers) was excluded from the start —
    different mechanism (advice generation, not disclosure of an
    existing fact) and a different review failure mode. Bullets 3 and
    5 were then also cut mid-review: their only proposed fix was
    adding explanatory prose to README/`--help`, which doesn't help a
    user confused *at the moment* they hit the gate in `status`/`week`
    output, and a stronger in-band fix risked exposing internal gate
    mechanics or drifting into bullet-4's remedy-pointer territory —
    left undecided rather than shipped as a weak doc-only fix. Final
    scope: bullets 1, 2, 6, 7 only, all render-only. Early drafts also
    used single-character inline markers (`+1` concatenated onto a
    stint's end time, `[~]` as a standalone anomaly-slot line) —
    replaced with suffixes in the existing duration-parenthetical/
    date-header slots after review found they broke the stint list's
    fixed-width alignment and read as cryptic; the backdated
    open-stint caption was similarly narrowed from "always show live
    duration with a caption" to "show duration only for yesterday,
    otherwise just `(unclosed)`" since an untethered hundreds-of-hours
    figure is noise, not information.
    Adversarial review (`docs/dev/plans/reports/boundary-context-cues-review.md`,
    needs-rework, 6 findings) caught: an em dash in one example
    violating SPEC.md §7's plain-ASCII rule; a wrong arithmetic result
    in a second worked example (42h00m, not 52h00m); an over-broad
    claim that the carry-in expansion applies to closed-week framing
    too, when closed weeks show no fulfillment parenthetical at all
    today (narrowed to current-week only); and an undeclared
    user-visible gap — the receiving-date header suffix (§3) has no
    `week`-side equivalent, a new status/week asymmetry the spec
    didn't name (now declared explicitly, accepted as out of scope for
    this changeset). All six findings folded into the spec directly.
62. **boundary-context-cues changeset plan (phase 5/6) reworked after
    review** (`docs/dev/plans/boundary-context-cues-plan.md`,
    `docs/dev/plans/reports/boundary-context-cues-plan-review.md`,
    needs-rework, 4 findings + 1 undeclared gap): the plan's original
    2-task split (Task 1 `render.rs`, Task 2 `status.rs`) turned out
    to be a planning error, not a real seam — Task 1 merged alone
    under its own exclusive-file boundary would leave `status.rs`'s
    two existing `status_week_line` call sites failing to compile,
    since Rust has no optional positional arguments and Task 1 doesn't
    own the file that calls it. Collapsed to a single task owning both
    files. Also fixed: §3's planned extra "predecessor's predecessor"
    punch fetch was unneeded scope creep (proved via `stint.rs`'s
    splice-direction code that a plain `classify(prev_punches, now)`
    already gives the correct signal, since the (prev,day) splice
    direction never touches `day.open`); §4's day-age signal gained an
    explicit delivery mechanism (a field on `StintLine`, mirroring
    how §2's span flag is already delivered, instead of being left
    unspecified). The review's undeclared-gap finding — §4's stint-line
    caption change contradicts Day total's unconditional `(+ ongoing)`
    wording for a stale backdated stint (one line says "ongoing," the
    next says the duration is unreliable) — was put to the user rather
    than silently accepted or fixed: chose to fix it, extending §4 to
    give Day total a matching `(+ unclosed)` form for the
    two-or-more-days-back case (spec updated, see its Status line).
    Plan review went three more rounds after that (round 2, round 3,
    round 4 — same reports directory): round 2 caught that the §3
    splice check as planned would silently never fire (reusing the
    already-spliced `day` variable in scope at `status.rs:344` instead
    of a fresh `classify()` call) and that its `splice_candidate` gate
    is module-private in `stint.rs` with no plan-stated visibility
    fix; round 3 caught that the day-age value's plan-stated delivery
    ("copied from the `StatusView` value at construction time") is an
    impossible order of operations (`StintLine`s are built before
    `StatusView` exists) plus a wrong test-extension target (5 of 6
    new literals never pass through `render.rs`'s
    `t33_every_produced_string_is_ascii`) and two smaller precision
    gaps (`now`/`now_utc` naming, an unsourced `worked` figure); round
    4 verified all of round 3's fixes against real source and found
    nothing further — green. Net effect on the plan: collapsed from
    its original 2-task split (a planning error — Task 1 alone would
    leave `status.rs` not compiling) to 1 task; `splice_candidate`
    widened to `pub(crate)` as a named, deliberate exception to
    "no new public API"; the day-age value is one local hoisted before
    stint-line construction and threaded into `build_stint_lines`'s
    signature, stored separately on both `StintLine` and `StatusView`;
    `worked` for §5's printed line is derived by subtraction inside
    `status_week_line`, not passed as a second parameter. Round 4
    (`.../boundary-context-cues-plan-review-round4.md`): green, no
    further findings — plan locked.
63. **boundary-context-cues Task 1 low-level plan** written
    (`docs/dev/plans/boundary-context-cues-task-1-status-render.md`,
    exact signatures for `status_week_line`, `splice_candidate`'s
    `pub(crate)` widening, `StintLine`/`StatusView`'s new fields,
    `build_stint_lines`'s new parameter). Two judgment calls made and
    accepted rather than re-escalated: `open_stint_age` is
    `Option<OpenStintAge>` on `StintLine` (`None` for completed
    stints, since age is meaningless there — explicit over an
    arbitrary always-populated default); a future-dated open stint
    (`target_date > today`, never discussed by spec or the four review
    rounds) falls into `TwoOrMoreDaysBack` via the `else` arm of the
    today/yesterday/else chain — defensible (a future stint's "age" is
    nonsensical either way, and `(unclosed)` avoids printing a
    misleading duration) and cheap to change if a reviewer disagrees.
64. **boundary-context-cues merged (commit `c28977d` on branch
    `boundary-context-cues`); final review and fresh-eyes UX check
    run in parallel.** Final review
    (`docs/dev/plans/reports/boundary-context-cues-final-review.md`,
    ship-with-followups, 3 findings): code correct, every spec worked
    example reproduced by hand, security clean; the task plan's
    future-dated-open-stint ambiguity turned out moot (an existing
    input guard already rejects future `DATE`s at write time, so the
    case can't occur). Findings: F1 (medium) — the spec's own §7
    SPEC.md/NOTES.md doc updates never landed, since Task 1 was scoped
    code-only and nothing else in the changeset did them, leaving
    SPEC.md §4.3.1 asserting the literal opposite of what shipped; F2
    (low) — `stint_line()` computed and discarded a value, matching
    the same tuple twice; F3 (low) — one end-to-end test asserted by
    substring rather than exact string, so a transposed
    `WeekAccounting` field would've passed silently (values verified
    correct by hand regardless).
    Fresh-eyes UX check ("returning user" tier —
    `docs/dev/plans/reports/../fresh-eyes-report.md`, run from a
    sandbox outside the repo containing only the built binary and
    README, 9 findings): two are this changeset's own new copy — the
    three-tier open-stint caption/Day-total wording (by design, but
    flagged the "elapsed since now" phrase as reading backwards) and
    nothing else new. The other seven are pre-existing behavior this
    changeset didn't cause: two (README claiming the midnight-cue and
    inline-fulfillment-explanation gaps still exist, when this
    changeset just fixed them) were stale-doc symptoms, folded into
    the F1 fix below. Five are genuinely pre-existing UX gaps, newly
    written down: a multi-day-old forgotten `stop` is invisible
    everywhere except the exact date it started (including `week`'s
    per-day table) — called the strongest finding of the exercise; a
    past (closed) date/week's output uses an undocumented, differently-
    shaped sentence than the README's only examples; `est. EOD` can
    point at tomorrow with no date shown; the day-total and week-total
    lines can show an identical figure with redundant-looking phrasing
    on the week's last weekday; "Total still owed" reads punitive for
    what's otherwise neutral vocabulary.
    Decisions, put to the user and agreed: fix F1 (this entry — SPEC.md
    §1.2a/§4.3.1/§7.1/§7.2 updated, README "Known issues" updated), F2,
    and F3 now, as part of this changeset; reword the "elapsed since
    now" caption to "duration as of right now" now, since it's this
    changeset's own new copy; add the five genuinely-pre-existing
    fresh-eyes findings to SPEC.md §1.2a as new known issues rather
    than fixing them here — real UX gaps, but unrelated to what this
    changeset touched, left for a future changeset's own triage. F2/F3/
    caption-reword dispatched as a follow-up implementation task on the
    `boundary-context-cues` branch (not yet landed as of this entry).

65. **`status-wording-fixes` changeset started** — coordinator-subagent
    workflow (`coordinating-development` skill), phase 0 profile filled:
    verification set `cargo fmt`, `cargo build`, `cargo test`,
    `cargo clippy --all-targets -- -D warnings` (from `AGENTS.md`
    "Verifying changes"); doc root `docs/dev/` (`SPEC.md`, `NOTES.md`,
    `specs/`, `plans/`, `plans/reports/`); decision log is this file,
    appended as numbered entries; model tier default (Sonnet) everywhere,
    no escalation requested; release procedure `scripts/prep_release.sh`
    (reversible bump+changelog) then `scripts/release.sh` (irreversible
    tag+push), not read in full yet — will be before phase 17 if this
    changeset reaches release; integration branch scheme one branch per
    changeset (`boundary-context-cues` precedent); changeset spec location
    `docs/dev/specs/<date>-<slug>.md`; runnable artifact `cargo build
    --release` → `target/release/mlm`; user-visible surface `README.md`,
    `mlm --help`, `status`/`week` rendered output — all four fixes below
    touch it, so phase 14 (fresh-eyes check) runs, not skipped. Context-
    starvation hook: absent, not configured in `.claude/settings.json`.
    Changeset numbering: repo's existing plan files use a topic slug
    (`boundary-context-cues-plan.md`, `boundary-context-cues-task-1-...`),
    never a bare integer — deviated from the skill's numeric default to
    match precedent; this changeset's id is the slug
    `status-wording-fixes` everywhere the skill's templates say `<N>`.
    Scope: the four known issues newly filed in entry 61
    (SPEC.md §1.2a bullets on closed-period sentence shape, `est. EOD`
    with no date, last-weekday redundant wording, "Total still owed"
    tone) — user picked all four for one changeset, greenlit in chat
    2026-09-22/23. Two tasks: Task 1 (code — `render.rs`/`status.rs` and
    their in-file tests, covers "Total behind" rename + `est. EOD
    (tomorrow)` + last-weekday wording), Task 2 (docs only —
    `README.md`/`SPEC.md` closed-period example) — user asked for Task 2
    to get its own dispatch rather than being folded into Task 1's
    verification pass. 1–2 task changeset: phase 10 (cross-document
    review) skipped per the scaling table; phases 5-7 collapse into one
    short changeset plan, reviewed once.

66. **`status-wording-fixes` spec adversarially reviewed and locked**
    (`docs/dev/plans/reports/status-wording-fixes-spec-review.md`,
    needs-rework, 5 findings). Two were implementation-blocking: Fix
    C's condition (`weekday_number.min(5) == 5`) fires on Friday,
    Saturday, *and* Sunday, not "the one day" the draft repeatedly
    claimed — corrected in place, since the underlying redundancy
    genuinely spans all three (required_minutes is capped at the full
    week target on each), not a bug to route around, just a wrong
    premise in the prose; and Fix B's suggested `.date()` comparison
    is deprecated since chrono 0.4.23, returns the wrong type against
    `today: NaiveDate`, and would fail this repo's `clippy -D
    warnings` CI gate outright — corrected to `.date_naive()`. One
    would have misdirected Task 2: SPEC.md already has both
    closed-period worked examples (§7.1/§7.2), so Fix D's SPEC.md
    portion is a string update, not a new example — only README.md
    actually lacks one; scope narrowed accordingly. Also folded: a
    fuller `Total still owed` inventory (SPEC.md has 8 hits, not the
    2 the draft named — four are normative prose at lines 647/672/
    679/776) and an explicit weekend-day test-plan addition for Fix C
    (the fixture gap that would have caught the first finding).
    Undeclared-user-visible-gap check came back empty — all four
    fixes already record their visible consequences. Spec locked.

67. **`status-wording-fixes` changeset plan adversarially reviewed
    (collapsed form) and folded** — plan written
    (`docs/dev/plans/status-wording-fixes-plan.md`), reviewed
    (`docs/dev/plans/reports/status-wording-fixes-plan-review.md`,
    needs-rework, 5 findings). Two high-severity: Fix C's
    `weekday_number.min(5) == 5` condition has no data path into
    `day_total_line` (a pure `&StatusView -> String` function with no
    access to `today`/`weekday_number`) — both spec and plan named the
    condition without saying where it lives; fixed by threading a
    third field, `day_reaches_week_cap: bool`, onto `DailyTargetHint`,
    mirroring how Fix B's bool already threads through `EodState`. And
    both spec and plan claimed no existing test hits the capped
    weekday range — false:
    `resolve_f9b_sunday_pin_non_multiple_of_five_target_override`
    (`status.rs:1759-1780`) already asserts the old wording via
    `render()` on a Sunday and will break under Fix C; folded in as a
    required update, not left as silent fallout. One medium,
    filed as an undeclared user-visible gap: README's existing
    Saturday `status` example (`README.md:196-198`) goes stale the
    moment Fix C ships (Saturday triggers the same condition), and
    Task 2's scope hadn't named touching any *existing* example, only
    adding new ones — folded in, along with a pre-existing, unrelated
    defect noticed at the same line (an `est. EOD ... target already
    met` concatenation the renderer can't actually produce), fixed
    since Task 2 is already editing that exact line. One low: the
    `EodState::At` conversion-site enumeration wrongly included
    `base_view` (which has no such construction) — corrected to the
    actual five sites. Both spec and plan updated; spec's Status line
    now records both review rounds. Changeset plan locked, task
    dispatches (phase 8) next.

68. **`status-wording-fixes` Task 1 and Task 2 implemented and merged**
    — both dispatched via worktree, strict TDD, shipped exactly per
    their task plans (no completion reports needed). Task 1 (code,
    `src/render.rs`/`src/status.rs`, commit `c3da272`) added one extra
    test beyond its plan (`resolve_eod_target_already_met_with_open_stint`)
    to close a coverage gap the repo's coverage-regression hook
    flagged — not a behavior or signature deviation. Task 2 (docs,
    `README.md`/`docs/dev/SPEC.md`, commit `cbbeefa`) cross-checked its
    worked examples against Task 1's actual merged code before
    writing them, rather than trusting the plan's pre-implementation
    derivation. Both merged `--no-ff` into `status-wording-fixes`
    (merge commits for Task 1 then Task 2, sequential — Task 2 was
    dispatched only after Task 1 merged, since it needed real output
    strings). Coordinator re-verified independently after each merge:
    `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets
    -- -D warnings` all green both times; confirmed zero remaining
    `Total still owed` occurrences repo-wide after Task 2's merge.
    Merge gate closed. Final review (phase 13) and fresh-eyes check
    (phase 14) next — phase 13 needs the user's explicit approval
    before dispatch.

69. **Phase 13/14 dispatched** for `status-wording-fixes`. Phase 13
    (final review) approved by the user at default (Sonnet) tier,
    scoped to the diff `v0.3.5..status-wording-fixes` (v0.3.5 is
    tagged and is main's current tip). Phase 14 (fresh-eyes) tier:
    `returning` — this changeset changes wording an existing user
    already relies on in `status`/`week` output, not first-run
    onboarding or an opt-in flag, so the most-naive-applicable-tier
    rule lands on `returning`. Sandbox prepared at
    `/tmp/claude-659200001/mlm-fresh-eyes-sandbox/` (release binary
    built from `status-wording-fixes` tip + `README.md` only — the
    phase-0 user-visible-surface slot's other member, `--help`/
    rendered output, the persona generates itself by running the
    binary).

70. **`status-wording-fixes` phase 13/14 back; fresh-eyes findings
    triaged with the user.** Final review (phase 13, Sonnet tier,
    `docs/dev/plans/reports/status-wording-fixes-final-review.md`):
    green, no findings, verdict ship — built and ran the artifact by
    hand against the spec's worked examples, all four fixes confirmed
    correct. Fresh-eyes (phase 14, `returning` tier,
    `docs/dev/plans/reports/status-wording-fixes-fresh-eyes.md`): 5
    findings, all pre-existing behavior outside this changeset's
    scope (none are regressions it introduced). Triaged one by one
    with the user, decisions:
    - Finding 1 (day-total line mixes today's hours with a week-scoped
      pace clause) — **not a bug**. Confirmed by the user as original
      design intent: the day-total line is deliberately a combined
      day+week statusline, not a day-only figure. Not filed.
    - Finding 2 (`est. EOD (tomorrow)` is flatly wrong for gaps many
      days out, demonstrated up to ~49 days via repeated `week target`
      inflation) — **dismissed**. The user judged the scenario
      unrealistic (no real workflow produces a 49-day-open stint); the
      spec's original "accepted edge case, technically imprecise"
      language stands as written. Not filed, no doc change.
    - Finding 3 (`status` silently accepts future dates; week framing
      then names the real current weekday, not the queried date) —
      **confirmed as a real bug** by the user ("status has no business
      showing future dates"), but explicitly deferred to its own
      future changeset rather than folded into this one, since fixing
      it means new validation logic in already-reviewed-and-merged
      code, outside this changeset's presentation-only scope. Needs
      its own spec/plan/implementation cycle later — not yet started.
    - Finding 4 (the expanded `fulfillment = worked + carry-in`
      output format has no README/SPEC.md precedent, only the simple
      form is ever shown) — **confirmed, fixed now**. Small, bounded,
      docs-only fix pass dispatched (phase 16) rather than filed as a
      known issue, since it's cheap and the user said outright "needs
      documentation."
    - Finding 5 ("Total behind: Nh" reads identically for an
      untracked week and a worked-but-short one) — **not a bug**. The
      user's read: this is a target-configuration matter (set the
      week's target for weeks you don't mean to track), not a wording
      ambiguity in the tool, and the same phrasing existed under the
      old "Total still owed" wording too — this changeset's rename
      neither caused nor worsened it. Not filed.
    Net: only Finding 4 produces a change (phase 16 fix pass,
    docs-only). Finding 3 is logged here as a known future changeset,
    not yet scoped or specced.

71. **`status-wording-fixes` fix pass merged; Finding 3 filed as a
    known issue.** Fix pass (`docs/dev/plans/reports/status-wording-fixes-fixpass.md`)
    merged and independently re-verified green. Finding 3 (`status`
    silently accepting future dates, week framing anchored to the
    real "today") added as a new bullet to both `SPEC.md §1.2a` and
    README's "Known issues" — filing the issue itself, not its fix,
    which stays deferred to its own future changeset per entry 70.
    Changeset ready for phase 17 (release) pending user go-ahead.

72. **`status-wording-fixes` released as v0.3.6.** Merged to `main`
    (commit incorporating both tasks, the fix pass, and the finding-3
    known-issue filing), changelog curated by hand — `git-cliff`'s
    auto-generated entry listed internal process commits ("Task 1:
    status/week wording fixes", "Log merge gate...") rather than
    user-facing changes; rewritten to describe the actual fixes,
    trimmed twice more per the user's steer (terser bullets, dropped
    the known-issue line entirely — a filed-not-fixed issue doesn't
    belong in a changelog). `prep_release.sh 0.3.6` then
    `release.sh 0.3.6`: pushed `main`, CI green, tag `v0.3.6` pushed,
    triggering the release workflow (build/sign/publish). Changeset
    closed.

73. **`current-week-ahead-wording` changeset started, spec locked.**
    User report: once a week's target is met, the current-week
    headline on `status` and `week` reads `-04h 35m left by end of
    Saturday`. SPEC.md and README only showed that form in examples
    and never recorded it as a decision. Phase 0 profile reused from
    entry 65 unchanged. Wording picked by the user in chat
    2026-09-24: for `owed < 0` the headline reads `{abs(owed)} ahead
    of target` and drops the weekday. `owed >= 0` is unchanged,
    including `00h 00m left by end of <weekday>` at zero. This
    supersedes decision 18's current-week framing for `owed < 0`. The
    closed-week strings stay as shipped (`Total behind: X` /
    `Total ahead: X`, `render.rs:68,70`); decision 18's own `Total
    still owed` text is stale. Spec
    `docs/dev/specs/2026-09-24-current-week-ahead-wording.md`,
    reviewed (`docs/dev/plans/reports/current-week-ahead-wording-spec-review.md`,
    ship-with-followups, 3 findings), all folded. Finding 1 was a
    testing gap: no test drove `resolve()`/`build_week_view()` to a
    negative current-week `owed`. Task 1 now adds one pipeline test in
    each of `status.rs` and `week_view.rs`, so its scope grows to
    those files' test modules. Finding 2 concerned the stale decision
    18 strings, handled above. Finding 3 was an undeclared visible
    gap: on an ahead week, today's pace hint can name a weekday above
    a week line that names none. Accepted as intended. It is recorded
    in SPEC §7.1 and in the changeset spec's §1.2, not filed in
    §1.2a, because it follows directly from the user's wording choice.
    The zero-boundary mismatch (current `00h 00m left` vs closed
    `Total ahead: 00h 00m`) predates this changeset and goes into
    §1.2a as a known issue. Two tasks (code, docs), so phase 10 is
    skipped and phases 5-7 collapse into one short plan. Phase 14
    runs at tier `returning`, because the change alters output that
    existing users already read daily.

74. **`current-week-ahead-wording` plan reviewed and folded.** Plan
    `docs/dev/plans/current-week-ahead-wording-plan.md`, reviewed
    (`docs/dev/plans/reports/current-week-ahead-wording-plan-review.md`,
    ship-with-followups, 2 findings), both folded mechanically. SPEC
    §7.1's pace-hint bullet justifies `required today` on Fri-Sun by
    saying the week line above names the same weekday. That is false
    on an ahead week, so Task 2 now rewords it. Task 1's carry-in
    ahead test now pins worked `00h 00m`, the surplus-only shape spec
    §3 calls out. The plan's doc-string check pointed Task 2 at Task
    1's output, which a parallel task cannot see. It now points at the
    plan's pinned interface contract, and the merge gate checks the
    built binary. Phase 8 (task plans) skipped: spec §3 already pins
    the exact format strings and the change is one match arm, so a
    task plan would restate the spec. Phase 10 skipped per the 1-2
    task scaling rule.

75. **`current-week-ahead-wording` merged, reviewed, triaged.** Both
    tasks merged `--no-ff` and re-verified green at the merge gate. A
    hand-run of the release binary matched the spec for owed > 0,
    == 0 and < 0 on `status` and `week`. Final review ran at the
    default tier (Sonnet), which the user picked over escalating or
    skipping
    (`docs/dev/plans/reports/current-week-ahead-wording-final-review.md`,
    ship, no findings). Fresh-eyes check at tier `returning`
    (`docs/dev/plans/reports/current-week-ahead-wording-fresh-eyes.md`)
    found the new wording clear and raised 4 findings, all
    pre-existing. The user triaged them in chat 2026-09-25. F2 (README
    never says the pace hint measures the week's running fulfillment)
    gets a docs-only fix pass now. F1 (the pace hint sits on the `Day
    total:` line and reads like arithmetic on the day total) and F4
    (Mon-Thu the hint names today's own weekday where `required
    today` would fit every day) are filed in SPEC §1.2a and README
    known issues as the next changeset's scope. F3 (zero-boundary
    wording) confirms a known issue already filed by this changeset,
    so nothing changes.

76. **`open-punch-visibility` changeset started.** Scope is the four
    SPEC §1.2a known issues on unclosed and unpaired punches: the
    forgotten multi-day `stop`, missing remedies on `[!]` lines, the
    lone-vs-double open start asymmetry, and the unobservable §4.3.1
    join gate. The user picked all four plus a `mlm start` stderr
    warning in chat 2026-09-26, with a 14-day lookback window. Phase 0
    profile reused from entry 65 unchanged. Baseline `v0.3.6` (`main`
    at `267060b`); branch `open-punch-visibility`. Spec
    `docs/dev/specs/2026-09-26-open-punch-visibility.md`. A stale open
    start uses the existing `(unclosed)` age threshold of 2 days, so a
    lone open start dated yesterday stays silent for overnight
    sessions. Phase 14 runs, because the change alters `status`,
    `week` and `start` output.

77. **`open-punch-visibility` spec reviewed and locked**
    (`docs/dev/plans/reports/open-punch-visibility-spec-review.md`,
    9 findings, all folded). `status -0` is a hard error in
    `resolve_date`, so the spec's `-0` alias is dropped. `delete punch
    --date D` only lists, so each delete hint says to rerun with the
    number. `stop HH:MM` pairs by time, so a too-early time makes a new
    orphan and E7 closes the latest start; hints now name the start
    time the stop must follow, and E7 leads with delete. The `start`
    warning's written-date skip hid a self-inflicted E7; the written
    date now warns when it has two or more open starts. Orphaned ends
    outside today's page and the 15-date read cost on every `status`
    are accepted and listed in the spec's §1.2. Tests added for fix
    commands resolving their anomaly. Phase 14 runs at tier
    `returning`, since existing users read `status` daily. Provisional
    task count is 3+ (detection, rendering, `start` warning, docs), so
    the full ladder runs, phase 10 included.

78. **`open-punch-visibility` plan reviewed and folded.** Plan
    `docs/dev/plans/open-punch-visibility-plan.md`, six tasks: stint
    predicates, rendering, `status` wiring plus the shared lookback,
    `start` warning, `week` markers, docs. Order is (1 ∥ 2) → (3 ∥ 5)
    → 4 → 6. Review (`docs/dev/plans/reports/open-punch-visibility-plan-review.md`,
    ship-with-followups, 5 findings) all folded. `tests/status_cli.rs`
    pins the old E7 wording and had no owner; Task 2 now owns it.
    Contract B now pins the refused-join predicate to raw
    single-date classifications, so `status` and `week` cannot wire it
    differently. Task 1's boundary tests gain age 0. Task 4's
    read-failure path is tested only if a seam exists, since adding
    one would be production plumbing for a test. Task 4's test list
    had also contradicted the spec on a self-inflicted E7; corrected
    to warn. A lookback line not mentioning its refused join is
    accepted and added to the spec's §1.2.

79. **`open-punch-visibility` task plans written (phase 8).** Six
    plans, `docs/dev/plans/open-punch-visibility-task-{1..6}-*.md`.
    Task 2's plan replaced the existing render anomaly type outright,
    which leaves the build red between batch 1 and batch 2 and would
    fail the pre-commit test gate on Task 2's own commit. Decided:
    Task 2 is additive and keeps the old type working; Tasks 3 and 5
    switch to the new type; Task 4 deletes the old type once both have
    merged (sequential ownership of `render.rs`, never parallel).
    `tests/status_cli.rs` moves from Task 2 to Task 3, since new E7
    wording reaches the binary only when `status` switches. Task 2's
    per-date fact gains the latest open start's time, which the spec's
    E7 hint needs and Contract A had omitted. Task 1's refused-join
    predicate stays a plain bool; the caller's argument order already
    says which date is which. Task 6 drops a changelog-style clause
    from the spec's §1.2 bullet when copying it into SPEC.md, per
    AGENTS.md's self-containment rule. Phase 9 re-aligns Tasks 2-6 to
    the pinned signatures in Tasks 1 and 2.

80. **`open-punch-visibility` cross-document review (phase 10)**
    (`docs/dev/plans/reports/open-punch-visibility-consistency.md`,
    2 findings, both stale prose with no effect on code). Task 5's plan
    still says Task 2 replaces `render::Anomalies` outright; Task 2 is
    additive and Task 4 deletes the old type. Task 4's plan still warns
    about Task 3 names that Task 3's re-align already fixed. Neither
    changes what gets built, so each is passed to the implementer as a
    correction note in its dispatch instead of another re-align round.
    Signatures, ownership, batch order and output strings otherwise
    agree across all six plans.

81. **`open-punch-visibility` Tasks 1-5 merged.** Each merged
    `--no-ff` into `open-punch-visibility` and re-verified at the merge
    gate (fmt, build, test, clippy), plus hand-runs of the built binary
    for `status`, `week` and `start`. Parallel tasks each ratchet
    `coverage-baseline.json`, so every second merge in a batch
    conflicts on it; resolved by keeping the lower value and letting
    the merge commit's hook ratchet it. Task 4's deletion of the
    superseded `render::Anomalies` removed about 300 fully covered
    lines, which dropped coverage from 99.114% to 99.104% because
    pre-existing uncovered test-fixture panic arms in `commands.rs`
    now weigh more. The user chose to lower the baseline by hand to the
    measured value, which stays above the changeset's starting 99.031%.
    The harness blocked the subagent's commit of that change as a CI
    bypass, so the user committed it (`23d5378`). Four doc comments in
    `status.rs` still name the deleted type and a removed
    `to_anomalies`; queued for the fix pass.

82. **`open-punch-visibility` phase 13/14 back and triaged.** Final
    review ran on Opus at the user's choice
    (`docs/dev/plans/reports/open-punch-visibility-final-review.md`,
    ship-with-followups, 7 findings). Fresh-eyes check at tier
    `returning` (`docs/dev/plans/reports/open-punch-visibility-fresh-eyes.md`,
    4 findings). User triage in chat 2026-09-30:
    - Review F2 (the stale hint can lead to nested, double-counted
      stints) exposes a pre-existing pairing flaw in §4.3, not a
      regression. User ruling for the next changeset: pairing matches
      the outer stint, inner starts/stops are ignored and flagged, and
      time never double-counts — "you can't work double time". Filed
      as a known issue now; hint wording unchanged here.
    - Fresh-eyes F1 (`HH:MM` placeholder looks literal): dropped.
    - Review F3: fixed. The `start` warning skips the written date only
      when its single open start is the punch just inserted. User also
      asked for a delete alternative on the warning, so the single
      form becomes three lines laid out like `status`'s (spec §3.6
      amended).
    - Review F1, F6, F4, F5, F7 and fresh-eyes F4: fixed in one fix pass
      (docs, tests, comments, README carry-in note).
    - Fresh-eyes F2 (yesterday's open session invisible on today's
      page) and F3 (a past week's anomaly skews carry-in unflagged):
      filed as known issues.

83. **`open-punch-visibility` fix pass merged; changeset complete on
    its branch.** Fix pass (`docs/dev/plans/reports/open-punch-visibility-fixpass.md`)
    closed review F1, F3-F7 and fresh-eyes F4, and filed the three new
    known issues in SPEC §1.2a and README. Merge gate green, and the
    F3 case hand-checked on the merged binary. Review F5's
    binary-level `start` stderr test was not added. The fix pass
    judged that the unit tests through `warn_stale_starts` with a real
    connection cover the same logic; `start()` itself still writes to
    real stderr untested. Pre-existing plan narration in older comments
    (`week_view.rs` "Landmine", `cli.rs`/`commands.rs` "Task N" from
    milestone 14) is outside this changeset and left alone.
    Stale-doc sweep of README, AGENTS.md and SPEC found no remaining
    claims this changeset made false. Release not prepared, at the
    user's request; `open-punch-visibility` is not yet merged to
    `main`.

84. **`live-eod` changeset started.** User report from real use
    2026-10-03: `est. EOD` drifts later while a stint is open, so it
    can never be reached without `stop`. Root cause is §7.1's
    `now + (required − fulfillment)` leaving out the open stint's
    live minutes, which §2.4 already says feed `est. EOD`. User chose
    to subtract live minutes from the pace hint's `gap` too, so both
    figures count down together. User also chose to omit EOD for a
    stale open stint, but today's page only ever pairs today-dated
    punches, so that case cannot arise and the spec states it as a
    guarantee. Multiple open starts today use the most recent one's
    live minutes, which avoids double counting. Phase 0 profile reused
    from entry 65 unchanged. Baseline `main` at `c9e980b`; branch
    `live-eod`. Spec `docs/dev/specs/2026-10-03-live-eod.md`. Phase 14
    runs, because `status` output changes.

85. **`live-eod` spec reviewed and locked**
    (`docs/dev/plans/reports/live-eod-spec-review.md`,
    ship-with-followups, 12 findings, all folded). User chose to file
    the overnight-session gap (F3) as a known issue next to the
    existing yesterday's-open-session one. Coordinator calls:
    `live` reads `ctx.day.open.last()`; `est. EOD` is defined as
    `open start + (required − fulfillment)`, which drops the false
    one-minute-drift claim and makes a start typed for later today
    project from its own time; `DailyTargetHint.gap_minutes` holds the
    post-live value; the hint/week-line divergence, minute-by-minute
    output, backdated-punch jumps and two-open-start crediting go in
    the spec's known issues. This changeset reverses SPEC §7.1's
    explicit "fulfillment here still excludes the open stint's live
    minutes" for the pace hint and `est. EOD`; entries 19 and 37 stay
    as history. Phase 14 runs at tier `returning`, because `status`
    output changes for a daily user. Provisional size two tasks
    (code in `status.rs`, docs in SPEC/README), so phases 5-7
    collapse into one short plan and phase 10 is skipped.

86. **`live-eod` plan reviewed and folded**
    (`docs/dev/plans/reports/live-eod-plan-review.md`,
    ship-with-followups, 12 findings). Two parallel tasks confirmed:
    Task 1 owns `src/status.rs`, Task 2 owns SPEC.md and README.md;
    the reviewer found every `DailyTargetHint`/`gap_minutes`/`EodState`
    consumer in `status.rs`. F1-F7 folded as a "Review folds" section
    at the end of the plan. U1-U5 are visible consequences of locked
    behavior, so they went into spec §1.2 as known issues without
    changing behavior. The real-clock binary cannot reproduce the
    pinned sample lines, so Task 1 pins the §7.1 sample in a
    resolve-level test instead. With one task per file set and a
    detailed plan, phase 8 task plans are skipped; each implementer
    plans inside its own dispatch.

87. **`live-eod` Tasks 1 and 2 merged** `--no-ff` into `live-eod`
    (`139143c` docs, `7a673fb` code). Both shipped to plan; Task 2
    also fixed three stale old-formula passages in SPEC the plan had
    not listed. Merge gate first failed on a full disk: 39 leftover
    agent worktrees held about 60 GB of `target/` caches. At the
    user's choice the caches were deleted and the 33 worktrees that
    were merged and clean were removed with their branches; four
    unmerged and two dirty worktrees were left for the user. Gate then
    green (fmt, build, 511 lib + integration tests, clippy). Hand run
    of the binary: two `status` reads 65 s apart printed the same
    `est. EOD` while `left to` dropped by a minute.

88. **`live-eod` phase 13/14 back and triaged.** Final review ran on
    Opus at the user's choice
    (`docs/dev/plans/reports/live-eod-final-review.md`,
    ship-with-followups, 2 low doc findings, no code findings; tests
    fail under every regression tried). Fresh-eyes check at tier
    `returning` (`docs/dev/plans/reports/live-eod-fresh-eyes.md`,
    11 findings). User triage in chat 2026-10-03, coordinator's
    recommendations accepted:
    - Review F1 (SPEC line wrap) and F2 (§2.4 `required` bullet implies
      `owed` subtracts live): fixed.
    - Fresh-eyes F1 (`(tomorrow)` on estimates days away, now common):
      fixed. Spec amended with §3.4: weekday abbreviation 2–6 days out,
      weekday plus date 7+ days out. This reverses SPEC §7.1's
      bare-`(tomorrow)` rule.
    - F7 (which of two open starts counts) and F11 (README `status`
      section silent on far-off EOD): README doc fixes. F7's narrow
      `fix:` text filed as a possible issue.
    - F2 (Saturday treats the whole week as due), F6 (future start shows
      as `-now`), F8 (future week reads `Total behind`), F9 (raw UTC
      migration log line on first run): filed as possible issues.
    - F3, F4, F5, F10: already documented, no action.
    User also asked to rename "Known issues" to "Possible issues", since
    the list keeps growing and holds nothing users need to worry about.
    Applied to the README heading and its cross-references and to SPEC
    §1.2a's heading; earlier NOTES entries keep their wording.

89. **`live-eod` fix pass merged; changeset complete on its branch.**
    Fix pass (`docs/dev/plans/reports/live-eod-fixpass.md`) closed both
    final-review findings and fresh-eyes F1, F2, F6-F9 and F11, plus the
    "Possible issues" rename. `est. EOD` now names the landing day past
    tomorrow (weekday, or weekday plus date 7+ days out). The new
    Saturday possible issue says "from Friday on", because `required`
    plateaus at the full target from Friday. Merge gate green (fmt,
    build, 514 lib + integration tests, clippy); hand run printed
    `est. EOD 01:00 (Mon)` for a Saturday 09:00 start. Fix-pass worktree
    and branch removed after the gate. Stale-doc grep of README, SPEC
    and source found no leftover "Known issues" or bare-`(tomorrow)`
    rule.

90. **`point-in-time-week` changeset started.** User report from real
    use (Thursday 2026-10-01): the week line read `left by end of
    Thursday` against the full 40h target while the pace hint said 32h
    was due by end of Thursday. Code matched SPEC; the design paired the
    whole-week `owed` with today's deadline wording. User chose to make
    the current week's headline point-in-time (`required − fulfillment`)
    in both `status` and `mlm week`, with "left to / over" wording
    ("by end of <Weekday>" Mon-Thu, "<target> target" Fri-Sun). User
    also chose that the running stint's live minutes count everywhere
    for the current week (day total, worked, fulfillment, week line,
    `mlm week` rows and block), with `(incl. ongoing)` replacing
    `(+ ongoing)`/`(ongoing)`, and that `est. EOD` stays on the Day
    total line while the pace clause is removed. Coordinator call:
    Fri-Sun `required` becomes the exact target instead of
    `5 x floor(target / 5)`, so Friday's figure matches the target the
    line names. Phase 0 profile reused from entry 65. Baseline `main`
    at `efd97e4`; branch `point-in-time-week`. Spec
    `docs/dev/specs/2026-10-03-point-in-time-week.md`.

91. **`point-in-time-week` spec reviewed and locked**
    (`docs/dev/plans/reports/point-in-time-week-spec-review.md`,
    needs-rework, 9 findings + 10 undeclared gaps, all folded without
    changing user-chosen behavior). Coordinator calls:
    - F1: live enters the current week's worked minutes before carry is
      walked, so a future week's `Carry-in` matches the current week's
      live-inclusive `fulfillment - target` (follows the user's "live
      counts everywhere").
    - F2: `live` always comes from today's punches and `required` from
      today's weekday, for every `status` of the current week, so
      `status DATE` classifies today separately.
    - F3/F9: Fri-Sun `required = target` kept; its up-to-4-minute
      `est. EOD` effect for non-multiple-of-5 targets is declared.
    - F4-F8: section reference fixed to SPEC §1.2a, doc and code-comment
      sweep listed, testing section added, zero-target rendering
      specified, motivation wording fixed.
    - U1-U10 all declared in spec §6.
    This decision supersedes entry 37's "live minutes never enter that
    math" for displayed fulfillment and worked; entry 37 stays as
    history. Phase 14 runs at tier `returning`, because `status` and
    `week` output change for a daily user. Provisional size two tasks
    (code across `src/`, docs in SPEC/README), so phases 5-7 collapse
    into one short plan and phase 10 is skipped unless the plan splits
    the code.

92. **`point-in-time-week` plan reviewed and folded**
    (`docs/dev/plans/reports/point-in-time-week-plan-review.md`,
    ship-with-followups, 9 findings + 4 undeclared gaps). Two parallel
    tasks confirmed: Task 1 owns all of `src/` (code stays unsplit
    because `status` and `mlm week` share the live-inclusive accounting
    and headline), Task 2 owns SPEC.md and README.md. F1-F9 folded as a
    "Review folds" section at the end of the plan: extra SPEC §8.1/§4.2
    sweep, complexity watch moved to `build_ledger` and
    `stint::classify`, full doc-comment sweep, inverted tests rewritten
    rather than deleted, `mlm week` given today's date with rows and
    `Worked:` from one source, splice-aware classification for `live`.
    U1-U4 added to spec §6 as possible issues. Phase 8 task plans are
    skipped, as in entry 86: one task per file set and a detailed plan,
    so each implementer plans inside its own dispatch. Phase 10 skipped
    (two tasks).

93. **`point-in-time-week` Tasks 1 and 2 merged** `--no-ff` into
    `point-in-time-week` (`adc5592` docs, `de604a1` code). Both shipped
    to plan with no completion report. Both worktrees started on `main`
    and rebased onto the plan commit `19d3393` before work. Task 1's
    unpinned choices: `render::week_headline`/`status_week_line` take the
    week accounting and today; `StatusView::weekday_name` and
    `DailyTargetHint::day_reaches_week_cap` removed with the pace clause;
    `DayStints::live_minutes` added; `DbWeekData` gains `today`. Merge
    gate green after each merge (fmt, build, 550 lib + integration
    tests, clippy); coverage ratcheted 99.15% to 99.19%. Hand run on a
    seeded scratch database (Saturday): `status`, `status 2026-10-01`
    and `week` print the same `05h 16m over 40h 00m target`, `week`
    rows sum to `Worked:`, and the closed week 39 still reads
    `Total behind`. Both worktrees and branches removed after the gate.

94. **`point-in-time-week` phase 13/14 back and triaged.** Final review
    ran on Opus at the user's choice
    (`docs/dev/plans/reports/point-in-time-week-final-review.md`,
    ship-with-followups, 3 findings; 10 mutations all caught except one
    with no visible effect). Fresh-eyes at tier `returning`
    (`docs/dev/plans/reports/point-in-time-week-fresh-eyes.md`,
    14 findings; the weekend clock hid the Mon-Thu form, which the
    fixed-clock tests cover). User accepted the coordinator's triage in
    chat 2026-10-03:
    - R1 (`est. EOD` formula docs read live-inclusive fulfillment),
      R2 (duplicate gap computation, unrendered `daily_target`, dead
      shim), R3 (SPEC §4.2 minus-sign sentence): fix.
    - F3: one README line saying `est. EOD` assumes no breaks.
    - F2 (`target already met` reads as a today-only target): kept as
      is; spec §6 already records it. The user gave no option, so the
      coordinator took the no-change one.
    - F13 (debt week reads more left than the target), F8 (`week target`
      silent), F9 (`week target` usage on wrong argument order), F11
      (`--help` defines no terms): filed as possible issues.
    - F1, F4-F7, F10, F12, F14: already declared, pre-existing, or
      covered by tests; no action.

95. **`point-in-time-week` fix pass merged; changeset merged to
    `main`.** Fix pass (`docs/dev/plans/reports/point-in-time-week-fixpass.md`)
    closed R1-R3 and F3, and filed F8, F9, F11 and F13 as possible
    issues. R2 added `week::required_and_gap` as the one gap computation
    and removed `DailyTargetHint`, `StatusView::daily_target` and the
    `_use_datelike` shim; earlier NOTES entries keep the old names as
    history. Merge gate green (fmt, build, 552 lib + integration tests,
    clippy); coverage 99.22%. Fix-pass worktree and branch removed.
    Stale-doc grep of README, SPEC, AGENTS.md and `src/` (including
    `--help` text in `src/cli.rs`) found no leftover old-rule claim.
    Release not prepared; awaiting the user's call.

96. **`possible-issues-triage` changeset started.** User asked for a
    PO-style triage of SPEC §1.2a (32 items) by impact and agreed with
    the result. Tier 1 (fix; wrong data or silent failure): (a) a
    session left open from yesterday gives no signal on today's
    `status`/`mlm start` and drops out at midnight (two items, one
    root), fixed by a hint line rather than an anomaly so the 2-day
    stale rule stays; (b) nested entry double-counts overlap with no
    `[!]`, fixed by an overlap anomaly with LIFO pairing unchanged;
    (c) `mlm week` shows a storage read failure as no data
    (`unwrap_or_default` in `src/week_view.rs`), fixed by propagating
    the error; (d) `mlm week target` prints nothing on success, fixed
    by echoing the stored target. Tier 2 (cheap polish): default log
    level `Warn` so the migration line needs `--verbose`; a short term
    glossary in `status --help`; a targeted error for `week WEEK_ID
    target DURATION`; reword `target already met` on the Day total
    line; reject a future `status DATE` like `start`/`stop`, which also
    retires the future-date weekday and future-week `Total behind`
    items. Tier 3 (accept): the live-model items (minute-by-minute
    figures, backdate jumps, `est. EOD` semantics, framing wording,
    `incl./+ ongoing`, debt-week left, two-open-starts credit, 14-day
    window, earlier-date week line) move out of §1.2a into the
    relevant spec sections as plain behavior statements. Deferred, kept
    in §1.2a: a past week's anomaly skewing later carry-in with no
    pointer. README's list trims to the items still open. User asked
    that every item be re-validated against current code first, since
    many were filed in recent changesets and never revisited. Phase 0
    profile reused from entry 65. Baseline `main` at `be85617`; branch
    `possible-issues-triage`.

97. **`possible-issues-triage` spec reviewed and locked**
    (`docs/dev/specs/2026-10-03-possible-issues-triage.md`). Validation
    (`plans/reports/possible-issues-triage-validation-review.md`) found
    34 items, not 32; 31 valid, I4/I5/I28 partly valid. Changes to
    entry 96: I1 and I4 are fixed together by a carry-in `[!]` line
    (user's call; I1 skews carry-in silently past the 14-day window);
    I28 is hardening of three reads, not the main punch read; future
    `mlm week` keeps `Total behind`, accepted as behavior. User approved
    the draft's visible choices (`[i]` hint, overlap line, carry line,
    `Week W target: D` echo, `done for today`, glossary in long help).
    Adversarial review (`plans/reports/possible-issues-triage-spec-review.md`,
    needs-rework, 11 findings + U1-U10) folded: F1 inverted the stop-hint
    rule, so hints now name `between S and N` (first punch after the
    target start) when one exists; F2 glossary now says surplus carries,
    matching `week.rs` and SPEC §1.3; F3 overlap covers today's open
    stint; F4 exit codes pinned to 1 (runtime) and 2 (usage), since
    §6.3 has no named codes; F5 the age-1 hint reads today's punches;
    F6/F7/F8 mechanisms left to the plan with constraints pinned; F9
    adds a SPEC section map and a test list; F10 defines overlap
    exactly; F11 suppresses the hint beside a refused join and records
    `mlm start` and `23:59` as non-goals. U1-U10 recorded in spec §6.
    Phase 14 runs, tier `first-run` (default log level and `--help`
    change first-run output). Provisional task count 4-5: full ladder.

98. **`possible-issues-triage` plan reviewed and folded**
    (`docs/dev/plans/possible-issues-triage-plan.md`, six tasks: T1
    foundation seams + §3.4/§4.4/§4.5; T2 overlap; T3 yesterday hint;
    T4 carry scan; T5 CLI surface; T6 docs). Plan decisions: prefetched
    ledger via one shared loader (traits stay infallible); token-shape
    interception after a fallible clap parse; one per-date anomaly
    module feeds rows, block and carry marker. Plan review
    (`plans/reports/possible-issues-triage-plan-review.md`, 11 findings
    + U1-U15) folded: F1 clap's usage override leaves `<ARGS>` in the
    Arguments section and E10, so T5 hides the positional, writes the
    Arguments section by hand and intercepts E10; F2 two future-`status`
    tests named; F3 T1 gains `src/storage.rs` for one ranged override
    read; F4 discriminator is token shape, help/version pass through;
    F5 coordinator ruling: the clean-pair window includes N's minute
    (a same-instant stop pops S first, SPEC §4.3 step 2), so it always
    holds a legal minute; recorded in spec §3.2.2 as clarification, not
    a behavior change; F6 phrases live in `src/render.rs`, frozen at T1
    merge; F7 T1 may fix `src/commands.rs` import paths; F8 ranged-read
    failure covered by a loader unit test; F9 T1 owns `src/date.rs`
    comments, T6 owns AGENTS.md Layout; F10 loader keeps earliest-week
    anchoring and live data-week marking; F11 T2 splits the grown
    builder. U1-U3, U6, U7, U9, U10 added to spec §6; U4/U5 closed by
    F5; U14 T6 wording clarified; the rest already declared or
    timing-only.

99. **`possible-issues-triage` task plans written, cross-checked and
    folded.** Six task plans under `docs/dev/plans/`
    (`possible-issues-triage-task-{1..6}-*.md`). T1, T5, T6 planned
    first; T2-T4 against T1's published seams. Coordinator rulings:
    `mlm week` carry marker sits in the footer slot above the Carry-in
    block (contract 6); T5 handles `week target` errors by rewriting
    clap's error context (covers E10 and too-many-values); short `-h`
    may gain clap's `(see more with '--help')` suffix instead of a
    replacement help flag (spec §4.2/§6 updated); T2 prints one `fix:`
    per overlapping pair and puts the longer stint first on equal
    starts; `mlm start` gets no refused-join suppression (a page rule).
    Consistency review (`plans/reports/possible-issues-triage-consistency.md`,
    8 findings) folded: F1 T2 labels spliced stints with real times;
    F2 T6 checks literals by shape; F3/F4/F7 changeset plan wording
    (T3 entry wrapper, loader derives live, `-h` suffix); F5 the merge
    gate checks hint-then-marker order end to end; F6 T2-T4 may re-pin
    an inverted assertion in a T1-owned file and must list it; F8 T6
    rewrites SPEC §7.1's future-date `status` text. T2 and T6 plans
    re-aligned by dispatch. T1 and T5 implementation dispatched.

100. **`possible-issues-triage` all tasks merged; phase 13/14 out.**
    Merged `--no-ff` into `possible-issues-triage`: T5 (`8d823a2`), T6
    (`872d915`), T1 (`f8789ee`), T2, T3 (`fc44f4d`), T4 (`0897272`), then
    an integration fix (`83e3506`) that re-pins T3's order test to end
    with T4's carry marker (the F5 hint-then-marker check) and adds T4's
    deferred overlap-only marker tests. T4 re-pinned one assertion in
    T1-owned `src/status.rs` under the plan's exception; accepted. T2
    dropped the receiving-side splice span (unreachable: the spliced end
    is that date's first punch); accepted. Coverage-baseline conflicts
    resolved to the higher value. Merge gate green: fmt, build, 804
    tests, clippy, smoke runs; T6's doc literals checked against the
    binary. Timing: `mlm week 9000-01` 0.3 s (was 59 s); 150-week seeded
    DB `status`/`week` ~0.01 s release. Incident: a T4 smoke run without
    `MLM_DB_PATH` wrote a punch and a note to the user's real DB; the
    agent deleted both; user told. Discovered: `.githooks/pre-commit`
    passes commits with failing tests, because a failed `cargo llvm-cov`
    yields empty JSON and the coverage check prints SKIP; the T4 merge
    went in red that way. User chose to fold the hook fix into the fix
    pass. Phase 14 ran (`first-run`, 10 findings; the agent's report
    file was not written, so the findings from its reply are kept here):
    FE1 a 3-week-old unclosed start shows `160h left`, carry-in -120h,
    empty weeks count -40h each, `[!]` names only the anomalous week;
    FE2 carry-in over empty weeks differs between a fresh DB and one
    with punches, future weeks say `Total behind`; FE3 glossary omits
    Total behind/ahead, Day total, overlapping, orphaned end, `N days
    old`, and nothing points from `-h` or top-level help to it; FE4 old
    unclosed stints print `09:00-now (unclosed)` at 3 or 21 days; FE5
    overlap `fix:` does not say which punch, today's page does not
    mention an older overlap (the carry marker does, from the next
    week); FE6 `stop` accepts times outside the advised window silently;
    FE7 the `or: mlm stop` line vanishes once today has a start, with no
    word that a bare `stop` would close the new stint; FE8 `est. EOD
    17:08 (tomorrow)` on a first Sunday run surprises; FE9 the future
    `status` error does not point to `mlm week`; FE10 the `week target`
    echo has no date range, and README's Possible issues mentions E7/E8
    codes users never see. User approved Opus for phase 13.

101. **`possible-issues-triage` phase 13 back and triaged.** Opus
    final review (`plans/reports/possible-issues-triage-final-review.md`):
    ship-with-followups, 5 findings, all gates re-run green in a clone,
    25 deliberate mutations all caught by tests. User approved the
    triage plus FE4. Fix pass: F1 the yesterday hint's `or: mlm stop`
    prints only when no punch follows the open start on yesterday and
    today is empty (otherwise following it nests a later stint); F2 the
    argument-order interception fires only when the tokens parse as a
    week id and a duration (stops echoing arbitrary text into a
    copyable command); F3 `week target --help` stray blank lines; F4
    replace the tautological history test with concrete figures; hook
    fails on a nonzero `cargo llvm-cov`; FE3 glossary adds `Total
    behind`/`Total ahead`, overlapping stints, orphaned end; FE9 future
    `status DATE` error points to `mlm week WEEK_ID`; FE10b README
    Possible issues drops E7/E8 codes; FE4 old unclosed stint lines
    print `09:00-?` (spec §4.6, coordinator wording). Accepted as-is:
    FE1, FE2, FE5, FE6, FE7, FE8, FE10a. F5 (README period, archive
    banners) at close.

102. **`possible-issues-triage` fix pass merged; changeset merged to
    `main`.** Fix pass (`docs/dev/plans/reports/possible-issues-triage-fixpass.md`)
    closed F1-F4, the hook, FE3, FE9, FE10b and FE4. The pre-commit hook
    now fails the coverage step when `cargo llvm-cov` exits nonzero.
    Merge gate green: fmt, build, 808 tests, clippy, smoke runs; F1, F2,
    FE4 and FE9 checked by hand. Close-out: archive banners on the spec,
    changeset plan and six task plans; README period (F5); stale-doc
    grep of README, SPEC, AGENTS.md and `src/` found one stale test
    comment (lookback window ages), fixed. All task worktrees and
    branches removed. Release not prepared; awaiting the user's call.

103. **`overnight-visibility` changeset started.** Last SPEC §1.2a
    item: a stint open across midnight gets no live credit and, beyond
    the `[i]` hint, no visibility. Options laid out: A live credit while
    the stint would splice, B show it without counting, C accept. User
    chose B (they do not work overnight; it happened once waiting on a
    migration), and asked that `mlm week` mark yesterday's row too,
    after seeing mockups. On Mondays the current week's view also
    names Sunday's open stint in a footer line (user's call, from a
    side-agent note). Reverses `possible-issues-triage`'s rule that
    `[i]` never marks a row. If overnight users appear, revisit A.
    Phase 0 profile reused from entry 65. Baseline `main` at `6edd67b`;
    branch `overnight-visibility`. Spec
    `docs/dev/specs/2026-10-04-overnight-visibility.md`.

104. **`overnight-visibility` spec reviewed and locked.** Adversarial
    review (`plans/reports/overnight-visibility-spec-review.md`, 9
    findings + U1-U8) folded: F1 footers point to `mlm status` (today's
    page carries the fix lines; `status DATE` for a past date has none);
    F2/U5 a row with an anomaly and the overnight stint shows `[!] [i]`
    and both footers, so `[!]` never hides it; F3 the line is rendered
    apart from the date-sorted stint list, and `status -1` keeps its
    wording; F4 stale SPEC passages and the `lookback.rs` test that
    asserts the old rule are listed for update; F5 data model left to
    the plan (reviewer confirmed `mlm week` can decide the trigger from
    loaded buckets, no new reads); F6 §1.2a and README both read "None
    currently."; F7 moved behavior adds no `est. EOD` and the
    Sunday-midnight `Worked` gap; F8 spec drops option narration; F9
    tests added. U1-U4, U7, U8 recorded in spec §7. Phase 14 runs, tier
    `returning`. Provisional task count 2 (code, docs): collapsed
    phases 5-7, phase 10 skipped.

105. **`overnight-visibility` plan reviewed and folded**
    (`docs/dev/plans/overnight-visibility-plan.md`, collapsed form, two
    parallel tasks: T1 code, T2 docs). Plan decisions: the overnight
    line is its own optional field on the status view, rendered first;
    week rows gain a flag and the view one optional date for the Monday
    footer; one pure trigger decision in `src/lookback.rs`. Plan review
    (`plans/reports/overnight-visibility-plan-review.md`, 9 findings +
    G1-G8) folded: F1 NOTES removed from the docs task; F2 glossary
    sentence pinned as a literal so the tasks run in parallel; F3 the
    trigger derives refused-join itself and the hint calls it; F4 week
    row criteria for joined/refused/two-open/age-2; F5 three
    old-rule tests named for rewrite; F6 fixture sites named; F7 spec
    §8 precedence wording fixed; F8 Monday needs Sunday from the
    buckets; F9 ASCII sweep. G1-G7 added to spec §7; G8 already there.

106. **`overnight-visibility` task plans written; implementation
    dispatched.** T1 (`plans/overnight-visibility-task-1-code.md`) and
    T2 (`plans/overnight-visibility-task-2-docs.md`). Coordinator calls:
    T1 may make the mechanical edits to `src/lookback.rs` tests that the
    dropped refused-join parameter forces; the suppression lives in the
    shared `overnight_stint` decision and only `today_page_lines` filters
    the hint with it, so `mlm start` keeps warning about yesterday's
    open start; clock-dependent cases are unit tests with injected
    `now`. T2 covers four SPEC passages the spec missed and adds edge
    case E26 for `week`. Phase 10 skipped (two tasks).

107. **`overnight-visibility` tasks merged.** T2 docs (`78c172f`), T1
    code (merge after it). Merge gate green: fmt, build, 841 tests,
    clippy; hand repro (22:00 start yesterday plus today's own open
    stint) shows the hint, the overnight line ahead of today's stint,
    the Saturday `[i]` row and footer, unchanged figures. Doc literals
    match the binary. T1 notes: some new tests were red only as compile
    errors; one CLI test written after the code. Worktrees and branches
    removed.

108. **`overnight-visibility` phase 13/14 back and triaged.** Sonnet
    final review (`plans/reports/overnight-visibility-final-review.md`):
    ship; gates re-run, three mutations caught; Monday covered by unit
    tests only (real clock was Sunday). Phase 14 (`returning`, 8
    findings; report file again not written by the `cd-fresh-eyes`
    agent, findings kept here): FE1 splice suffixes `spans to next day`
    / `continues previous day's stint` unexplained; FE2 footer `run mlm
    status to stop it` overpromises; FE3 `start` says `warning:` where
    `status` says `[i]`; FE4 a 2-day-old start shows `[!]` with no
    stint line and a generic footer; FE5 bare `stop` on it leaves an
    orphaned end silently; FE6 `[i]` footer does not name its row; FE7
    `est. EOD 00:00 (Tue)` reads oddly; FE8 glossary lacks the
    overnight line. User approved: fix FE2+FE6 (both `[i]` footers
    become `[i] open stint on <Ddd YYYY-MM-DD>[ (last week)]: mlm status
    shows how to stop it`) and FE1+FE8 (glossary entries for the
    overnight line and the splice suffixes). FE3, FE4, FE5, FE7
    accepted.

109. **`overnight-visibility` fix pass merged; changeset merged to
    `main`.** Fix pass (`plans/reports/overnight-visibility-fixpass.md`)
    closed FE2+FE6 (`[i]` footers name the stint's date) and FE1+FE8
    (glossary entries for the overnight line and splice suffixes). Merge
    gate green: fmt, build, 841 tests, clippy; footer and glossary
    checked by hand. Close-out: archive banners on the spec, plan and
    task plans; stale-doc grep of README, SPEC, AGENTS.md, `src/` and
    `tests/` clean. SPEC §1.2a and README Possible issues read "None
    currently." Release not prepared; awaiting the user's call.

110. **`v0.4.0` released.** Ships `open-punch-visibility`, `live-eod`,
    `point-in-time-week`, `possible-issues-triage` and
    `overnight-visibility`. `prep_release.sh 0.4.0`; changelog
    hand-curated (Features / Changed / Bug Fixes; `Changed` is not a
    `cliff.toml` group). Release smoke run on the release build green.
    `release.sh` pushed `acaf24e` but CI failed on Windows only: the
    `short_help_goldens` test compared exact `-h` output and Windows
    prints `mlm.exe` in usage lines. Fix `962ac9f` normalizes `mlm.exe`
    in that test; CI green on all five targets; tag `v0.4.0` placed on
    `962ac9f` and pushed by hand per the script's recovery note. Release
    workflow green: artifacts, GitHub Release, crates.io publish,
    signing.

111. **`orphan-stop-warning` changeset started.** Takes FE5 from entry
    108: a bare `mlm stop` against a start 2+ days old writes an
    orphaned end on today and prints nothing. Brainstorm options were
    A (`stop` warns on stderr, still writes), B (`stop` refuses the
    write) and C (`stop` retargets the stale start's date). User chose
    A. B reverses the no-validation rule (SPEC §1.2, §4.3) and blocks
    typing the stop before the start. C would credit a multi-day
    stint. The user also chose to warn on any orphaned end, not only
    when a stale start exists, so a forgotten `start` or a mistyped
    stop time warns too. The cost is one needless warning when
    backfilling stop-then-start. Stale-start lines from the lookback
    follow the orphan warning only when one fires.

112. **`orphan-stop-warning` spec review folded.** Review
    (`plans/reports/orphan-stop-warning-spec-review.md`, needs-rework,
    13 findings). User chose to fix F7 and U3 in this changeset rather
    than file them: the E8 `fix:` hint gains a `(a time between P and
    T)` window in `status` and `stop`, and `stop` warns when its end
    splices into a stint of 12h or more. The 12h threshold is the
    coordinator's pick; a genuine 12h+ overnight session warns too
    (known issue). Coordinator folds: F1 silent cases restated over the
    date's state after the write; F2 `stop` also checks `D+1`, since a
    backdated stop can break the next date's splice; U2 `stop`'s
    lookback uses today's `status` filter, so a refused join never
    yields a yesterday fix that loses overnight time; F5 all reads
    first, then print; F3 testing section; F4 full doc list; F6 FE5
    flow is E27 in §8.2. U1, U5, U6 and the repeat rule are known
    issues; U4 is a non-goal. The spec grew two triggers and a window
    rule, so it goes back for one more review round before locking.

113. **`orphan-stop-warning` spec round 2 folded; spec locked.**
    Review (`plans/reports/orphan-stop-warning-spec-round-2-review.md`,
    needs-rework, 13 findings; the reviewer's write hook refused the
    `-review-2.md` name, so the file is `-round-2-review.md`). All
    folded. Coordinator design calls, within the user's "fix both
    along the way": an orphan sharing its minute with another end gets
    a delete-only E8 block, because no start pairs cleanly with a
    duplicate (F1); the long-splice `fix:` stops the earlier date's
    start first, with the stop-hint window, so the user's own end
    survives as an orphan with its own fixes (F2, F9); the 12h check
    covers any splice into `D` or `D+1`, so a backdated stop that
    clears an E7 is caught (F5); the yesterday hint drops its `or: mlm
    stop` line once the start is 12h old, so mlm never suggests a
    command it then warns about (F6); `stop` prints refused-join lines
    for `D` and `D+1`, so a broken splice says so (F7). F3, F4, F13
    tighten the E8 window rule; F8, F10 and the empty window are known
    issues; F11, F12 complete the doc and test lists. Phase 14 skipped
    at the user's call (small, wording-level surface); phase 13 runs on
    Opus, approved by the user up front. Provisional task count: 2
    (`stop` warnings; E8 hint + yesterday hint), so phases 5-7
    collapse and phase 10 is skipped unless phase 5 finds 3+ tasks.

114. **`orphan-stop-warning` changeset plan written**
    (`plans/orphan-stop-warning-plan.md`). Two parallel tasks: Task 1
    all code and tests (new `src/stop_warnings.rs`; `src/anomalies.rs`
    owns the one E8 hint-line and refused-join renderer), Task 2 docs
    (SPEC.md, README.md, AGENTS.md). This replaces the coordinator's
    feature split, because `stop` reuses the E8 renderer and splitting
    by feature would only serialize. Planner findings folded: a refused
    join between `D` and `D+1` prints once, as `D`'s line (spec §3.1);
    "word for word" covers the E8 hint lines, not the header (spec
    §4.1); a real-clock test of the yesterday `or:` line moves to an
    injected clock (Task 1). Two tasks, so phase 10 is skipped.

115. **`orphan-stop-warning` plan review folded**
    (`plans/reports/orphan-stop-warning-plan-review.md`,
    ship-with-followups, 11 findings + U1-U6). All folded into the plan
    without changing locked behavior. F1: when `D` is today, `D`'s
    trio still reads tomorrow, as `status` does, and only `D+1`'s own
    checks are skipped. F2, F3: Task 2 also updates the standalone
    yesterday `or:` condition sentences, the first-run-silence claims
    (SPEC §3, F13, README) and SPEC §4.3's insert-time lead-in. F4-F7:
    tests for the once-only `D`/`D+1` refused join, a partial read
    failure, no carry marker, the 719/720 boundary, and named breaking
    tests. F8: the minute-inclusion flag dropped from the facts; tested
    as behaviour. F9-F11: status line and citations, retargeted
    section refs, `src/main.rs` to Task 1. U1-U6 accepted as known
    issues in spec §1.2 (12h+ genuine session re-warns with nothing to
    fix; the 11h 59m boundary race; past 12h no non-warning way to end
    a running session; FE5 against a 15+ day start names nothing;
    duplicate rows in `delete punch`; the `--help` glossary), plus the
    first-run stderr change.

116. **`orphan-stop-warning` task plans written**
    (`plans/orphan-stop-warning-task-1-code.md`, findings(3);
    `plans/orphan-stop-warning-task-2-docs.md`, green). Task 1 finding
    1: a start at the minute of `D`'s spliced first end E14-pairs with
    it and breaks the splice, so spec §5 now excludes that minute too
    (wording, no printed-text change). Finding 2: spec §8's "`D+1` not
    read" bullet corrected to match the plan. Finding 3: unit tests use
    Mon 2026-02-09 to dodge a southern-hemisphere DST shift; accepted.
    Task 2 plan's §7.3 minute-rule text predates finding 1, so its
    implementer gets the one-sentence correction as a dispatch note
    instead of a re-align round. Task 2 deviations accepted: new SPEC
    §7.4.1 for `stop`'s rules, no "FE5" label in SPEC, the yesterday
    warning's form restated for `stop`, and the README `18:02` example
    noting its under-12h assumption. Phase 10 skipped (two tasks).

117. **`orphan-stop-warning` rescoped.** The user cut the changeset
    back to its first idea: `stop` warns on stderr for each orphaned
    end on the date it wrote to, with `status`'s E8 hint lines. The
    principle is that mlm reports the data and the user audits it, so
    mlm does not guard against mistakes `status` already shows. Removed
    from scope: the long-splice warning and its 12h threshold, the
    yesterday hint's 12h rule, the E8 fix window and duplicate-minute
    form, the `D+1` check, refused-join lines, and the lookback lines
    after the warning. Each was a coordinator fold of a review finding,
    and together they grew 13 known issues. Of those, the user called
    the repeat-on-every-`stop` a feature, and five others restated the
    feature. One known issue remains, the stop-then-start backfill
    warning. The entries 112-116 plans and the task plans are
    superseded; the next planning pass overwrites them. New process
    section "Triaging review findings" records the rules this exposed.

118. **`orphan-stop-warning` rescoped spec reviewed and locked.**
    Review (`plans/reports/orphan-stop-warning-rescope-review.md`,
    ship-with-followups, F1-F6 + U1-U7), triaged under the new rules.
    Folded: F1, F2 wording on what `start` and `stop` report; F3 any
    read failure suppresses the whole warning, so a failed neighbor
    read can't fake an orphan; F4 a malformed-row injection for the
    read-failure test; F6 SPEC §6 lead-in and E8 on the doc list; U5
    one non-goal line; U6 known issue widened to any stop-before-start.
    Dropped: F5 (tests pinning non-features), U1 (the `start` hint is
    right when today's start was forgotten, and `delete` covers FE5),
    U2-U4 (the long-splice, `D+1` and refused-join mechanisms the user
    cut in entry 117), U7 (no age phrase restates the design). Phase 14
    skipped at the user's call; phase 13 on Opus, pre-approved.
    Expected tasks: 1-2 (code, docs), so phases 5-7 collapse and 10 is
    skipped.

119. **`orphan-stop-warning` rescoped plan reviewed and folded**
    (`plans/orphan-stop-warning-plan.md`; review
    `plans/reports/orphan-stop-warning-rescope-plan-review.md`,
    ship-with-followups, F1-F6 + U1-U5). Folded: F1 AGENTS.md's
    `anomalies.rs` caller list goes to Task 2; F2 Task 2 cites spec
    §3-§5; F3 stale note on the deleted task plans; F4 the
    no-lookback criterion dropped (a non-feature); F5 out-of-order
    inserts, pinned pre-existing-orphan times, ASCII coverage; F6 a
    failed stderr write is ignored. Dropped under the triage rules:
    U1-U3 (long splice, `D+1`, refused join, all cut in entry 117), U4
    (dropped in entry 118), U5 (a malformed `D+1` row suppressing the
    warning is corrupt-data territory and follows from reading like
    `status`).

120. **`orphan-stop-warning` task plans written**
    (`plans/orphan-stop-warning-task-1-code.md`, green;
    `plans/orphan-stop-warning-task-2-docs.md`, findings(4)). Task 2
    notes settled as dispatch corrections: E27 goes at the end of
    §8.2 to keep numeric order, not beside E19; rewording SPEC §3.7
    and §6.2 so they are true for `start` too is accepted; Task 1
    builds `stop`'s view with `history::PunchBuckets`, so Task 2 also
    updates AGENTS.md's `src/history.rs` bullet; docs-vs-code drift is
    checked at the merge gate against the built binary. Phase 10
    skipped (two tasks).

121. **`orphan-stop-warning` Tasks 1-2 merged.** Task 2 (docs,
    `0878e0e`) and Task 1 (code, `fd593cd`) merged `--no-ff`. Both
    implementers found their worktree off the wrong base and rebased
    onto `8a529ea` first. Task 1 swapped the plan's test writer for a
    zero-length slice, because the plan's never-run `flush` dropped
    coverage; no signature changed, so no report. Merge gate green:
    fmt, build, 858 tests, clippy. Hand-runs of the built binary: FE5,
    displaced end and backdated stop print the block, which matches
    README's example apart from the dates; clean pair and splice are
    silent; stdout empty and exit 0 throughout. Task worktrees and
    branches removed.

122. **`orphan-stop-warning` final review triaged.** Ran on Opus,
    approved by the user up front
    (`plans/reports/orphan-stop-warning-final-review.md`,
    ship-with-followups, 3 findings). Clone verified green, spec
    example byte-for-byte, every behavior-changing mutation caught
    except dropping the `D+1` read, which cannot change orphans
    (accepted in entry 119). All three fixed in one fix pass, as
    mechanical: F1 SPEC §4.3's "the one insert-time signal" sentence
    names both warnings and re-introduces the edge-case list; F2
    re-wrap README's stdout/stderr paragraph; F3 a CLI test for a bare
    `mlm stop`. Phase 14 skipped at the user's call. Scratch clone
    removed.

123. **`orphan-stop-warning` fix pass merged; changeset closed out.**
    Fix pass (`plans/reports/orphan-stop-warning-fixpass.md`, green)
    closed F1-F3. F3's bare-`stop` test was seen failing with the
    warning routed to a sink. Noted, not fixed: README's shell-alias
    line predates this changeset and runs long. Merge gate green: fmt,
    build, 859 tests, clippy. Close-out: archive banners on the spec,
    plan and task plans; stale-pointer grep of README, SPEC, AGENTS.md,
    `src/` and `tests/` clean. Fix-pass worktree and branch removed.
    Not merged to `main`; release awaits the user's call.

124. **`orphan-stop-warning` merged to `main`, unreleased.** The user
    held the release to gather more changesets into it, then had the
    branch merged to `main` (`--no-ff`). Gate on `main` green: fmt,
    build, 859 tests, clippy. Next changesets branch from `main`.

125. **Brainstorm: `week-notes` and `export-notes`.** Two changesets,
    in this order, each its own release. Phase 0 profile reused from
    entry 65. Baseline `main` at `3bcff42`; branch `week-notes`.
    `week-notes`: `mlm week --notes`/`-n` prints each date's notes
    indented under its row (layout A of three mockups). Row markers
    stay on the row, so a marked row reads marker first, notes below.
    `export-notes`: `mlm export notes` (aliases `ex`, `n`), range by
    one of `--date DATE` (accepts `-N`), `--week WEEK_ID`, `--month
    YYYY-MM`, default the current week. Plain lines, one note each, as
    `YYYY-MM-DD  body`. Stdout only, no `--output`. Empty range prints
    nothing, exit `0`. JSON waits for a future `export all`.

126. **`week-notes` spec locked.** The user approved the draft; one
    review (`plans/reports/week-notes-spec-review.md`, 11 findings)
    folded. F1 fixed a false claim: after `target`, `--notes` is a
    token in target's own argument list, so it fails as target's own
    errors and writes nothing; it is not a clap usage error. F2 the
    example is now the current-week form. F3 the error test renames
    `notes.body` so it fails only the new read. F4/F5 add the SPEC
    §6.1/E20, `history.rs` and AGENTS.md doc targets and the missing
    tests. U1/U2 (`-n` before `target`), U4 (long-body wrap) and U5
    (footers pushed down) recorded as known issues. U3 wording now
    says the layout differs from `status`. U6 (`mlm w n`) dropped,
    since `n` was never the flag. Phase 14 runs at tier `returning`,
    because an opt-in flag on an existing command changes nothing a
    first-run user meets. Expected 2 tasks (code, docs), so phases
    5-7 collapse and phase 10 is skipped.

127. **`week-notes` plan folded.** Plan
    (`plans/week-notes-plan.md`, 2 tasks: code+tests, docs) reviewed
    once (`plans/reports/week-notes-plan-review.md`, 10 findings), all
    folded. F1 pins the note read in the command entry point, so the
    view builder's signature and its test callers in `status.rs` and
    `lookback.rs` stay untouched. F2 the order test makes `id` and
    `created_at_utc` order disagree. F3 clock-dependent cases move to
    renderer unit tests. F4 allows mechanical pattern/fixture updates.
    F5 adds the `week_view.rs` doc passages. F6 pins the read's return
    as a flat list of the existing note type. F7 ASCII check moves to
    the new test file. F8 pins README spelling `-n/--notes`. U1 (one
    bad note row hides the `--notes` view) added to spec §1.2. U2 adds
    a help-glossary entry for note lines, a wording-level change to
    spec §8. Two tasks confirmed, so phase 10 stays skipped.

128. **`week-notes` known issues dropped.** The user had all four
    removed: long-body wrap, footers pushed down, `-n` before
    `target`, one bad note row hiding the `--notes` view. Reviewer
    over-reports, each either expected or consistent with existing
    hard-error policy. Spec §1.2 is `None.`; SPEC §1.2a and README's
    possible-issues list stay unchanged.

129. **`week-notes` task plans written.** Task 1
    (`plans/week-notes-task-1-code.md`) pins `notes_in_range`, the
    six-space note prefix, and a shared glossary entry placed third
    before `[!], [i]`; it also shows in `status --help`, worded for
    both. Task 1 gains `tests/week_target_cli.rs`, whose `mlm week -h`
    golden the flag line changes, a mechanical update. Task 2
    (`plans/week-notes-task-2-docs.md`) adds §3.5 to its targets and
    uses spec §5's example in full plus a README Features line. Task
    2 realigned against Task 1 for the glossary text.

130. **`week-notes` tasks merged.** Task 2 (docs, `f22a179`, no
    report) and Task 1 (code, `54079d4`,
    `plans/reports/week-notes-task-1-report.md`) merged `--no-ff`.
    Task 1 deviations: the `week --help` long form puts the flag's
    description on its own line (golden fixed to real output); test
    helpers merged to satisfy the coverage ratchet. SPEC §3.6 says
    `--help` "lists it as" the text, true in both layouts, so no doc
    change. Merge gate green on `week-notes`: fmt, build, 891 tests,
    clippy; smoke run of `week -n` on a seeded database matches the
    spec layout. Task worktrees and branches removed.

131. **`week-notes` reviewed and triaged; closed out.** Phase 14
    (`returning`, `plans/reports/week-notes-fresh-eyes.md`, 10
    findings) and phase 13 on Sonnet at the user's choice
    (`plans/reports/week-notes-final-review.md`, ship-with-followups,
    3 findings). All dropped with the user's agreement: fresh-eyes
    hits are the dropped known issues, existing note normalization,
    existing exit codes and headline wording. Final F1 (long-help
    layout) needs no fix. F2 (raw control characters in note bodies,
    `status` too) would be a new mechanism; the user skipped it. F3
    (`WeekArgs` doc comment past 80 columns) folds into
    `export-notes`, which edits `src/cli.rs`. No fix pass. Archive
    banners on the spec, plan and task plans. Scratch clone and
    sandbox removed.

132. **`week-notes` merged to `main`, unreleased.** The user had it
    merged `--no-ff` and moved on to `export-notes`, which branches
    from `main`.

133. **`export-notes` spec locked.** The user approved the draft
    (decisions beyond entry 125: `-d`/`-w`/`-m`, bare month number,
    future range prints nothing, the `MONTH` messages, the `WeekArgs`
    comment re-wrap). One review
    (`plans/reports/export-notes-spec-review.md`, 19 findings). Folded:
    F1 `--date` shares input forms, not the future check; F2 all three
    flags take hyphen values, repeated/missing-value rows added; F4
    `YYYY-M` accepted, four-digit year; F5 the "exactly one line"
    claim cut, since pre-v0.3.0 rows can hold newlines; F6/U7 help
    value names and default-week wording pinned; F7/F8 tests and doc
    targets added. Dropped: F3 broken pipe, since a month of notes
    fits in the pipe buffer and `week` behaves the same; F9 the user
    already took the re-wrap; U1-U6, U8-U10 per entry 128's
    known-issue default. Phase 14 runs at `returning`: a new opt-in
    command touches no first-run path. Expected 2 tasks (code, docs).

134. **`export-notes` plan folded.** Plan
    (`plans/export-notes-plan.md`, 2 tasks: code+tests, docs) reviewed
    once (`plans/reports/export-notes-plan-review.md`, 13 findings).
    Folded: F1 a flag followed by another flag takes it as its value
    and fails as an exit-1 parse error (spec §6 row added); F2 a
    resolver test pins calendar year for bare `M`; F3 MONTH needs its
    own out-of-range arm; F4 any four-digit MONTH year; F5 AGENTS.md
    Current state and storage entry, SPEC §1.2 and §6.2 added to Task
    2; F6 clock tests use `Utc::now()`; F7 Task 1 owns
    `coverage-baseline.json`; U4 `--date` help keeps `status`'s
    backticks. Dropped U1-U3, U5, U6 per entry 128's default; U3
    matches WEEK_ID's own `expected YYYY-WW or WW` wording.

135. **`export-notes` task plans written.** Task 1
    (`plans/export-notes-task-1-code.md`, green) pins `ArgKind::Month`,
    `parse_month`, `src/export.rs` and the full `export notes --help`
    golden; clap behavior checked on a prototype. Task 2
    (`plans/export-notes-task-2-docs.md`) adds SPEC §3.8 and §7.5 and
    fixes three sentences the new command makes false (README's alias
    rule, README's and SPEC §7.4's list of output commands). Both
    quote the contract strings verbatim, so no realign.

136. **`export-notes` tasks merged.** Task 2 (docs, `6ebd825`, no
    report) and Task 1 (code, `b28a05c`,
    `plans/reports/export-notes-task-1-report.md`) merged `--no-ff`.
    Task 1 deviation: `parse_month` split into a private helper so no
    unreachable `map_err` closure costs coverage; behavior unchanged.
    Merge gate green on `export-notes`: fmt, build, 932 tests, clippy.
    Smoke run on a seeded database matches spec §5 and §6; help and
    `MONTH` strings match SPEC §3.8 and §6.1. Task worktrees and
    branches removed.

137. **Next changeset: `relative-ids`.** The `export-notes` spec gave
    `-N` to `--date` only. The user meant `-N` for every range. The
    coordinator asked about `--date` alone and did not flag the gap.
    Decision: `-N` (N before the current one, `-0` invalid) for
    `WEEK_ID` in `mlm week` and `export notes --week`, and for `MONTH`
    in `export notes --month`. `week target` unchanged. This drops the
    SPEC §1.2 non-goal on relative week notation. Starts on its own
    branch from `main` after `export-notes` merges. No release yet.
    Final review of `export-notes` running on Opus (user's choice).

138. **`export-notes` reviewed, triaged and closed out.** Fresh-eyes
    (`plans/reports/export-notes-fresh-eyes.md`, 12 findings) all
    dropped; the one real gap (`-w -1`) is `relative-ids` (entry 137).
    Final review on Opus (`plans/reports/export-notes-final-review.md`)
    returned ship-with-followups: no export findings; F1 is an existing
    panic in `src/main.rs` (`with_second(0).unwrap()`) during the
    repeated autumn DST hour, reproduced live 2026-10-25 02:22 CEST.
    The user chose no fix pass here. F1 becomes its own changeset,
    `dst-now-panic`, after an audit of every `unwrap` the user asked
    for. Order: `dst-now-panic`, then `relative-ids`. Spec, plan and
    task plans archived. Merged to `main` `--no-ff`, unreleased.

139. **`panic-fixes` brainstorm.** Renames `dst-now-panic` (entry 138)
    and widens it to the unwrap audit's user-reachable panics
    (`plans/reports/unwrap-audit-review.md`). F1 DST `now`: fix. F2/F4
    huge target: cap `week target` at `168h`, the hours in a week; a
    future user config may raise it, with the risk stated. F3 stdout
    write failure: a broken pipe exits quietly with a nonzero code; any
    other write failure prints `error: cannot write output: <reason>`
    and exits `1`. The user chose quiet over a message because `| head`
    would print it on every run. F5-F7 and the two non-panic bugs
    (`parse_date` years 0000-0999, week ranges near 9999) dropped.
    Coordinator picks: exit `141` for the broken pipe (the shell's
    `SIGPIPE` code), the cap message text, the help line. Baseline
    `main` at `dd91952`; branch `panic-fixes`.

140. **`panic-fixes` spec review folded.** One review
    (`plans/reports/panic-fixes-spec-review.md`, 21 findings). F1: an
    untyped `start`/`stop` passes today's clock time to the storage
    re-resolve (`commands.rs`, `storage.rs`), which takes the earlier
    pass, so fixing `now` alone cannot store a second-pass instant. The
    spec shrinks to match: `now` stops panicking, storage is unchanged,
    a second-pass punch stores the first-pass instant as a typed one
    does. That keeps typed and untyped consistent and avoids the
    backwards-stint and orphan cases F17-F19 describe. F2 §1 and §1.1
    now say stored oversized targets can still panic. F3 write failure
    moves to §6.3 as a runtime error. F4 every stderr site ignores
    failure; help and version stay clap's. F5-F7 tests gated Unix or
    Linux, the write-keeps test uses `week target`. F8, F9, F10
    wording. F11-F16, F20, F21 dropped by default.

141. **`panic-fixes` spec locked.** The user chose option 1 for the
    repeated hour: fix only the crash, storage unchanged, a second-pass
    punch stores the first-pass instant like a typed one. Estimate two
    tasks (code, docs), so the plan is collapsed and phase 10 skipped.
    Phase 14 runs, `returning` tier: the cap error, the help line and
    exit `141` are user-visible.

142. **`panic-fixes` plan folded.** Plan
    `plans/panic-fixes-plan.md`, two tasks (code, docs), no shared
    file. Its three spec points went into the spec: `<reason>` is std's
    I/O error text with the `(os error N)` suffix, the swapped-target
    reminder comes before the cap, and SPEC §1.2a lists stored oversized
    targets. One review (`plans/reports/panic-fixes-plan-review.md`, 16
    findings). F1 §1.2a item now in Task 2. F2 risk removed. F3 the
    `rust-version = "1.85"` claim reworded; the gate is clippy's
    `incompatible_msrv` lint. F4 the export pipe test seeds a note. F5
    clock-dependent cases assert clock-free lines. F6 the write error
    is recognised at the stdout write path, never by a chain search. F7
    Task 2 rewords SPEC §6.3's exit-`1` sentence. F8 a first-pass test,
    and the `TZ` test skips across date or year boundaries. U1 and U2
    follow from option 1 (entry 141): second-pass untyped punches store
    an hour early, and a stint crossing the passes can store an end
    before its start. Entry 140's claim that option 1 avoids the
    backwards case is wrong for that crossing. U3-U8 dropped by
    default.

143. **`panic-fixes` task plans.** Task 1
    (`plans/panic-fixes-task-1-code.md`) pins the mechanisms: a helper
    child holding a pipe's read end for the closed-pipe tests (no
    `std::io::pipe`, no `libc`), a computed POSIX `TZ` rule for both
    passes, and an `OutputError` type in `src/lib.rs` that `main`
    checks at the top level only. Accepted deviations: R1 printing
    functions take a writer, so 40 existing test calls gain a sink
    argument with no assertion change; R2 the flush sits after each
    write, not once in `main`, so no unreachable branch costs coverage;
    R8 two pipe tests use a fixed past date. Task 2
    (`plans/panic-fixes-task-2-docs.md`) keeps S10 (README Possible
    issues mirrors SPEC §1.2a) and S8 (SPEC §7 "plain line output").
    U2 stays out of the docs. README's stale "1.85+" (the crate needs
    1.88) is out of scope. No realign: Task 2 rests on the contracts
    only, and Task 1 changed no pinned string.

144. **`panic-fixes` tasks merged.** Task 2 (docs, `40a44fa`, no
    report) and Task 1 (code, `54dc3c1`,
    `plans/reports/panic-fixes-task-1-report.md`) merged `--no-ff`.
    Merge gate green on `panic-fixes`: fmt, build, 969 tests, clippy.
    Smoke run matches: cap error exit 1, `168h` accepted, the swapped
    reminder before the cap, `/dev/full` gives the write error exit 1,
    `| true` exits 141 silent, help line per contract 3. Task 1 found
    an older bug: chrono's unix `Local` lists `Ambiguous` with the
    smaller offset first, so `storage.rs` keeps the second pass for a
    repeated-hour time, against SPEC §2.1. Confirmed by hand: `TZ=
    Europe/Warsaw mlm start 02:30` on 2026-10-25 stores `01:30Z`, not
    `00:30Z`. The AC3 test accepts either pass until the user decides.
    Worktrees and branches removed.

145. **Repeated-hour storage fixed in `panic-fixes`.** The user chose
    to fix the bug in entry 144 here. Storage picks the earlier of the
    two instants by comparing them, not by their order, so code matches
    SPEC §2.1 and the docs already merged. The AC3 test goes back to
    asserting the first pass. `src/storage.rs` joins the changeset's
    scope.

146. **Repeated-hour fix merged.** `4c3b9c1` merged `--no-ff`, no
    report. Storage compares the two instants and keeps the earlier.
    Gate green: fmt, build, 971 tests, clippy. By hand, `TZ=
    Europe/Warsaw mlm start 02:30 --date 2026-10-25` stores `00:30Z`.

147. **`panic-fixes` fresh-eyes.** `returning` tier,
    `plans/reports/panic-fixes-fresh-eyes.md`, 13 findings. F1 is a
    regression from `4c3b9c1`: on 2026-10-25 under Europe/Warsaw a
    typed `03:00` stores `01:00Z` (shown as 02:00) instead of `02:00Z`.
    chrono's unix `Local` reports the wall time at the hour's end as
    ambiguous, with one candidate that does not exist as a wall time,
    and `earlier_instant` picks it. F2 and F4 follow from F1. Fix
    dispatched: a candidate counts only if it reads back as the typed
    wall time, with tests at every boundary minute in autumn and
    spring. F3 (no way to enter the second pass) is SPEC §2.1 as
    designed. F5-F13 dropped by default: accepted decisions (entries
    139-141) or behavior older than this changeset.

148. **DST boundary fix merged.** `daa2392` merged `--no-ff`
    (`plans/reports/panic-fixes-fix-2-report.md`). Storage keeps only
    candidates that read back as the typed wall time, then the
    earliest; none left is the nonexistent-time error. The cause also
    hit spring: unix `Local` returned `Single` for the gap's first
    minute, so `start 02:00` on a spring-forward day stored `01:00Z`
    instead of failing. Both fixed, with tests at every boundary
    minute. Gate green: fmt, build, 978 tests, clippy. By hand under
    Europe/Warsaw: 2026-10-25 02:30 → `00:30Z`, 03:00 → `02:00Z`;
    2026-03-29 02:00 → gap error, 03:00 → `01:00Z`. The fix agent
    briefly wrote two punches to the UAT database and removed them.

149. **`chrono-tz` switch deferred.** chrono's unix `Local` mishandles
    DST edge minutes, while `chrono-tz` gets them right. Converting
    through `chrono-tz` plus `iana-time-zone` would remove the cause,
    at the cost of a bigger binary and tz updates that ship with
    releases. The user keeps the read-back check as is. The switch is
    a possible later changeset. Final review of `panic-fixes` running
    on Opus (user's choice).

150. **`panic-fixes` reviewed and closed out.** Final review on Opus
    (`plans/reports/panic-fixes-final-review.md`): ship-with-followups.
    A probe of every local minute within 4h of every transition
    2025-2028 in 14 zones found no storage mismatch; recreate-line
    replays stored identical rows; mutations were caught. F1 the
    repeated-hour tests skipped near midnight and on 1 January; the
    `TZ` rule now puts the local clock near midday, so they never skip.
    F2 `mlm::exit_code` is the one exit mapping, 141 included, and
    `main` calls it. Fix pass `3b60694` merged; gate green: fmt, build,
    983 tests, clippy. Spec, plan and task plans archived. Merged to
    `main` `--no-ff`, unreleased. Next: `relative-ids` (entry 137).

151. **`relative-ids` brainstorm.** Scope from entry 137, plus the
    user's call to include `week target`, so every `WEEK_ID` takes `-N`
    and the parser needs no special case. `MONTH` takes `-N` in
    `export notes --month`. `-0` invalid, grammar as `DATE`'s. No `+N`.
    Coordinator picks: shape errors `expected YYYY-WW, WW or -N (N >=
    1)` and the `MONTH` twin, offset errors `week offset is out of
    range` and `month offset is out of range`, help lines in the `DATE`
    style. Baseline `main` at `77fb19f`; branch `relative-ids`.

152. **`relative-ids` spec review folded**
    (`plans/reports/relative-ids-spec-review.md`, 11 findings). Every
    §3 shape parses under clap 4.6.6. F1: `mlm week` takes `-N` the way
    `status` takes `DATE`'s, so `mlm week -1x` and `-x` stay clap
    errors, exit 2; `week target` and `export notes` already pass
    hyphen values, so `-1x` there is exit 1. Parity with `DATE` is the
    user's stated rule. F2: `-N` is out of range when it lands on an id
    the absolute form rejects (`WEEK_ID` below `1000-01`, `MONTH` below
    `0000-01`), and `u64` overflow counts too. F3, F4: flipped tests and
    every stale SPEC/README line listed. U1-U7 go to the user as
    known-issue candidates before lock.

153. **`relative-ids` locked.** User triage of U1-U7: U6 fixed, so a
    `+` plus digits `WEEK_ID` or `MONTH` gets the shape message plus
    `; a future week takes YYYY-WW or WW` (month twin). U4 fixed, so a
    lone `week target` token that is a valid `WEEK_ID` and not a valid
    `DURATION` gives clap's missing-`DURATION` usage error, exit 2, as
    bare `week target` does. U1-U3, U5, U7 are non-issues and not
    filed. Phase 14 runs (user-visible surface), tier `returning`.
    Expected two tasks (code, docs), so phases 5-7 collapse and 10 is
    skipped.

154. **`relative-ids` plan folded**
    (`plans/relative-ids-plan.md`, review
    `plans/reports/relative-ids-plan-review.md`, 13 items). Two tasks,
    code and docs, disjoint files; phase 10 skipped. F1: clap's
    negative-number test also passes float shapes, so `mlm week -1.5`
    reaches the parser and gets the shape error, exit 1, as
    `mlm status -1.5` does today; spec §3 wording and contract 6
    corrected. F2: the `+` hint renders from the input in the shape
    arms, cause stays `Shape`. F3-F7: count fixed, tests added (clap
    leading-zero parse, lone-token aliases and `-v`, `DATE` `+1`
    message guard), Task 2 states the lone-token boundary. U1-U6 not
    filed: U1, U4 follow `DATE` parity, U3 (`+0` hint), U5, U6 are the
    non-issue class the user ruled on in entry 153. U2 (`week target
    40` now a usage error, exit 2, not the format hint) follows the
    user's own lone-token rule.

155. **`relative-ids` task plans.** Task 1 (`plans/relative-ids-task-1-code.md`):
    new `OffsetOutOfRange` cause, `+` hint at render time, `-N` branch
    first in both parsers with no week-by-week walk; the week floor
    checks the ISO year (1000-01 starts 0999-12-30). Lone-`WEEK_ID`
    `week target` re-parses argv without the token for the exact bare
    error. D1, D2 are count corrections, no action. Task 2
    (`plans/relative-ids-task-2-docs.md`): SPEC F16-F17, E35-E38, E10,
    E24, E29 rewritten; R2 (help-row padding quoted in §3.7) checked at
    merge. Both go to implementation in parallel.

156. **`relative-ids` tasks merged.** Task 2 `6a87f0f` (SPEC, README;
    E36 week floor given as numbers, report
    `plans/reports/relative-ids-task-2-report.md`), Task 1 `24d1799`
    (code, tests; shipped as planned, no report). Gate on the
    integration branch green: fmt, build, 1018 tests, clippy; coverage
    99.517%. E36's `-53540`/`-53541` match the shipped test. Binary
    spot checks match the contracts. Worktrees removed. Phase 14 next
    (`returning`); phase 13 awaits the user's model choice.

157. **`relative-ids` fresh-eyes triaged; final review on Opus.**
    Fresh-eyes (`plans/reports/relative-ids-fresh-eyes.md`, 13
    findings, `returning` tier): user said fix none, save none. Each was
    an already-decided rule or behavior that existed before the
    changeset. Final review dispatched on Opus (user's choice), clone
    `/tmp/cd-scratch/relative-ids-final` at `0580e6f`.

158. **`relative-ids` reviewed and closed out.** Final review on Opus
    (`plans/reports/relative-ids-final-review.md`): ship-with-followups.
    Manual runs matched every contract; a sweep of 42,732 date and
    offset pairs found no panic and no week below `1000-01`; 11 of 12
    mutations caught, the survivor equivalent. Reminder echo is safe:
    only parser-accepted tokens reach it. F1 README said every bare
    number is a week id under `week target`; now limited to valid week
    numbers, as SPEC §3.7 says. Applied by the coordinator (owned doc,
    wording only). Spec, plan and task plans archived. Merged to `main`
    `--no-ff`, unreleased.

159. **`doc-drift` brainstorm.** The user asked to fix the declared
    MSRV and sweep the live docs for drift. MSRV is 1.95, set by
    `rusqlite_migration` 2.6.0 (the code itself needs 1.88 for let
    chains); the user keeps current deps rather than pinning older
    ones. Merged as `9bc2266` on branch `doc-drift` (renamed from
    `msrv-fix`). Sweep (`plans/reports/doc-drift-sweep-review.md`):
    21 findings; user approved F1-F20, F21 dropped. Coordinator picks:
    banner every archived file including reports (F6), add the note
    to the `week --notes` example rather than dropping it from export
    (F13), new SPEC §3.9 for `delete`. Coordinator applied NOTES' own
    F15 header and the F4 pointer. Spec and the two-task split are in
    one draft (`specs/2026-10-31-doc-drift.md`), since the sweep
    already reviewed the ground; one review covers both. SAST scanning
    (cargo-deny pause in `prep_release.sh`) waits until this lands in
    `main`.

160. **`doc-drift` spec review folded**
    (`plans/reports/doc-drift-spec-review.md`, 17 findings). F4 now
    takes `src/commands.rs` as the source of truth over the archived
    delete spec (R1, R2), with §8.2 entries (R3). F20 is a rule plus
    grep, not a site list (R4). F2's dev command sets `MLM_DB_PATH`
    (R7). F8 keeps the rejection rule (R12); F9 changes the first
    sentence only (R11). F6 skips this changeset's own docs; F7 puts
    reports in the close-out step (R9, R10). `Cargo.toml` and
    `sign-artifacts.yml` join Task 1 for the `SIGNING.md` path, and
    the `exclude` list moves to `docs/dev/`, since
    `cargo package --list` showed the published crate ships every
    archived spec (R13). R5, R6, R8, R14-R17 wording folded. R1's
    doubled prefix (`deleted. to recreate: deleted note (...)...`)
    goes to the user: documenting it is in scope, fixing it is a
    behavior change.

161. **`doc-drift` locked.** User: fix R1. A deleted note whose body
    holds a backslash prints `deleted note (<date>): <body>` alone,
    date always shown, no recreate prefix, no `...`. That is the
    changeset's one behavior change besides the F8/F9 help text.
    Phase 14 runs (README and `--help` change), tier `returning`.
    Phases 5-8 collapse into the spec's §6: two tasks with disjoint
    files, every fix already stated at text level by the sweep, the
    review and the spec, so no separate task plans. Phase 10 skipped.

## Open questions (still need answers)

None currently — all resolved.
