# Design notes

Running log of background/story before formal spec exists. Not a spec —
decisions here can (will) change. Formal spec comes later as its own doc.

## Workflow note (process, not product)

When we get to implementation: use subagents for TDD (write tests first,
implement to green) and a separate independent adversarial-review
subagent pass before merging — not the same agent that wrote the code.

At final spec review (before implementation starts): give §6 (error
handling & validation) particular scrutiny by mapping out concrete
user flows end-to-end — tests will be written directly against those
flows/assumptions, so gaps there become gaps in test coverage.

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
   see `docs/dev/specs/2026-09-13-delete-punches-notes.md` — editing
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

## Open questions (still need answers)

None currently — all resolved.
