# Round 4 review — boundary-context-cues-plan.md

Re-verified against current source (`src/status.rs`, `src/render.rs`,
`src/stint.rs`, `src/week.rs`) and the locked spec
(`docs/dev/specs/2026-09-20-boundary-context-cues.md`), re-deriving
each of round 3's four findings from the code rather than trusting the
plan's prose, then looking fresh for anything new.

## Round 3 fixes re-verified

1. **HIGH (day-age field "copied from StatusView" — impossible
   ordering).** Fixed correctly. The plan now says (lines 115–131) the
   three-way today/yesterday/2+-days classification is "computed once,
   as an ordinary local, from `today` and `target_date` (both already
   bound early in `resolve()`, before stint-line construction runs)"
   and is "threaded as a new parameter into `build_stint_lines`" and
   "also stored, unchanged, as a new field on the `StatusView` struct
   literal built afterward (it is not copied *from* `StatusView`)."
   This matches the actual control flow: `today`/`target_date` are
   bound at `src/status.rs:331-336`, `build_stint_lines(&day)` runs at
   `src/status.rs:381`, and the `StatusView { .. }` literal is built at
   `src/status.rs:384-395` — strictly after. The described ordering is
   now possible as written.

2. **MEDIUM (ASCII-test literal assignment).** Fixed correctly. The
   plan names exactly five literals for
   `t13_plain_ascii_output_and_non_ascii_notes_pass_through`
   (`src/status.rs`): `, spans to next day`, `ongoing - elapsed since
   now, not a running total`, `(unclosed)` (all three in
   `stint_line()`, `src/status.rs:155-167`), `continues previous day's
   stint` (header assembly), and `(+ unclosed)` (`day_total_line()`,
   `src/status.rs:122-152`) — all five are produced by functions
   defined in `status.rs`, confirmed by reading those functions
   directly; none of them route through `render.rs`. It names exactly
   one literal, `carry-in`, for `t33_every_produced_string_is_ascii`
   (`src/render.rs`) — the one new literal `status_week_line`
   (`src/render.rs:76-100`) itself would produce. The split is correct
   against the actual call graph.

3. **LOW (`now` vs `now_utc` inconsistency).** Fixed. Every `classify()`
   call description in §3 (plan lines 88, 92, 98-101, 154, 163, 235,
   238-241) now says `now_utc` uniformly, matching the actual bound
   name in `resolve()` (`src/status.rs:343`: `let now_utc =
   now.with_timezone(&Utc);`) and its use at the existing
   `classify_at` call (`src/status.rs:344`). No stray `now` remains in
   the classify-call prose.

4. **LOW (`worked` derived vs. second parameter).** Fixed and verified
   numerically. Plan §5 (lines 56-60) states `worked` is derived inside
   `status_week_line` as `fulfillment_minutes - carry_in_minutes`,
   "algebraically identical to `WeekAccounting.worked` by construction
   (`fulfillment = worked + carry_in` is enforced in `week_series`)."
   Confirmed against `src/week.rs:180` (`let fulfillment = worked +
   carry_in;`), and checked against all three spec worked examples:
   - deficit: `1755 - (-130) = 1885` = `31h 25m` ✓ (spec's `worked 31h
     25m`, carry-in `-02h 10m`, fulfillment `29h 15m`).
   - fulfillment-goes-negative: `180 - (-300) = wait` — recomputed
     directly: `worked = fulfillment - carry_in = -120 - (-300) = 180`
     = `03h 00m` ✓ (spec's `worked 03h 00m`).
   - surplus: `2350 - 50 = 2300`? recomputed: `worked = fulfillment -
     carry_in = 2400 - 50 = 2350` = `39h 10m` ✓ (spec's `worked 39h
     10m`).
   All three check out; the derivation is sound and matches spec
   exactly, no drift risk since it's a single enforced invariant, not a
   duplicated computation.

## Findings

None. I looked specifically for (a) new correctness/implementability
breaks introduced by this round's rewrite, and (b) undeclared
user-visible consequences, and didn't find anything worth reporting:

- The day-age classification is a per-`target_date` property (not
  per-stint), so every open `StintLine` in one view legitimately
  shares one value — the plan's "harmless duplication" framing for the
  rare multi-open (E7) case is accurate, not a bug.
- The `, spans to next day` (§2, donor date) and `(HH:MM continues
  previous day's stint)` (§3, receiving date) suffixes fire on two
  different dates' independent `resolve()` calls for the same boundary
  event (traced through `classify_at`'s (day,next) vs. (prev,day)
  branches, `src/stint.rs:296-319`), matching the spec's two separate
  worked examples — no overlap, no double-disclosure, no gap.
  - donor date (`target_date == A`): `classify_at`'s (day,next) branch
    adds the completed stint to `day.completed`, so `A`'s own stint
    list shows it with `, spans to next day` (§2).
  - receiving date (`target_date == A+1`): the (prev,day) branch only
    removes the orphan from `day.orphaned_ends` — `A+1`'s stint list
    stays empty for that punch, confirmed against the existing test
    `resolve_midnight_splice_later_date_shows_no_orphan_anomaly`
    (`src/status.rs:1159-1177`) — and only `A+1`'s header gets the §3
    suffix.
- The two-or-more-days-back Day total change (`(+ unclosed)`) is
  correctly gated on `has_open_stint`; a closed day's rendering is
  untouched regardless of the new day-age field's value, so this stays
  additive-only as claimed.
- Checked two very minor prose imprecisions and decided neither rises
  to a reportable finding: (1) the "two call sites" language in the
  Ladder-altitude section undercounts — `render.rs`'s own test module
  has four more direct `status_week_line(...)` calls
  (`src/render.rs:220,229,576,577`) that also need the new parameter —
  but since Task 1 owns `render.rs` in full and the Rust compiler
  forces every one of them to be fixed before the crate builds, this
  is self-correcting and changes no scope boundary. (2) the plan
  doesn't pin the new `carry_in_minutes` parameter's position in
  `status_week_line`'s signature, but the single-call-site wrapper
  pattern (`week_line()`, named `WeekAccounting` fields) that already
  mitigates this function's documented adjacent-`i64` transposition
  risk (`src/status.rs:16-20`) extends unchanged to the new parameter
  regardless of where it's inserted. Neither blocks implementation.

## Verdict

ship
