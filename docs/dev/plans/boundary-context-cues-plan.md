# Plan: boundary-context-cues

Spec: `docs/dev/specs/2026-09-20-boundary-context-cues.md` (locked).

## Ladder-altitude check

Revised after phase-6 review
(`docs/dev/plans/reports/boundary-context-cues-plan-review.md`,
needs-rework). The original plan split this into 2 tasks (Task 1
`render.rs`, Task 2 `status.rs`), reasoning that `status_week_line`'s
new carry-in parameter was a cleanly separable file scope. Review
found that split doesn't survive contact with Rust: `status.rs` calls
`status_week_line` with its current 5-argument signature at two call
sites (`week_line()` and a golden test), and Rust has no optional
positional arguments — so Task 1, merged alone under an exclusive
`render.rs`-only file boundary, would leave the crate not compiling
until Task 2 landed. That's not a sequencing inconvenience, it's a
broken merge gate.

**Corrected to a single task.** There was never a parallelism benefit
to the split (the dependency graph was already a strict chain, one
worktree, no concurrent work) — only a file-ownership argument that
turned out to be false once the call-site coupling is accounted for.
One task, owning both `src/render.rs` and `src/status.rs`, removes the
broken-intermediate-build risk entirely and matches the coordinator's
original collapsed-ladder estimate.

## Goal

Today, `status`'s output silently drops four pieces of context a user
needs to trust the numbers they're looking at: a stint that crosses
midnight looks like an ordinary same-day stint; a date that "received"
the tail end of the previous day's stint shows no trace of where that
time came from; an open stint left running from days ago reports an
implausible live duration with no warning that it's stale (and Day
total's own summary line doesn't say so either); and a week carrying a
deficit or surplus from a prior week shows a fulfillment number with
no explanation of how it was built. After this changeset, a user
reading `status` output can see, inline and without consulting other
commands: which stints span midnight, which dates "inherited" a punch
from the day before, when an open stint's elapsed time is no longer
meaningful (consistently, in both the stint line and the Day total
summary above it), and how a week's fulfillment figure decomposes into
worked time and carry-in. No accounting numbers change — only what
`status` prints about them.

## Architecture (prose)

One task, entirely inside the `resolve` (I/O + arithmetic) / `render`
(pure `&StatusView -> String`) split that `src/status.rs` already
documents at its top. No new module is introduced.

- **`src/render.rs`**: `status_week_line` gains one new parameter,
  `carry_in_minutes: i64`, and the ability to render carry-in as a
  third term in its parenthetical (`fulfillment F = worked W +
  carry-in C / target T`) whenever it's non-zero. `W` (`worked`) is
  derived inside `status_week_line` as `fulfillment_minutes -
  carry_in_minutes` — algebraically identical to `WeekAccounting.worked`
  by construction (`fulfillment = worked + carry_in` is enforced in
  `week_series`, `src/week.rs`), so no second new parameter is needed
  and no risk of the two ever diverging. Each of `F`/`W`/`C` is signed
  independently using the sign convention `format_minutes`
  (`src/time.rs`) already applies (`-` for negative, nothing for
  zero/positive — the same convention §4.2 uses everywhere else, so no
  new sign-handling code, only a new template branch). This
  parenthetical only ever appears for `WeekFraming::Current`, unchanged
  from today. `week_headline`, `WeekFraming`, `week_framing`, and
  `Anomalies` (also in `render.rs`, also used by `week_view.rs`) are
  untouched.
- **`src/status.rs`**: the same commit that changes
  `status_week_line`'s signature also updates its two call sites
  (`week_line()` and the golden test asserting field-for-field
  equivalence with it), so the crate compiles at every point along the
  way — no intermediate state where one file's signature and the
  other's call site disagree.
  - `resolve()`'s stint-building step gains a per-stint "spans to next
    day" signal, computed by comparing a completed `Stint`'s
    `start.date` and `end.date` (both already present on
    `storage::Punch` — no new data fetch). `StintLine` gains a field
    to carry it, and `stint_line()` gains the suffix branch, in the
    same duration-parenthetical slot the existing `, ongoing` suffix
    occupies.
  - No new data fetch: `target_date`'s predecessor's punches are
    already read at the existing `classify_at` call site (they feed
    `prev_punches`). Two additional `classify()` calls are needed,
    over data already in scope — **not** a reuse of any existing
    `classify_at` result, per the pitfall below:
    1. `classify(prev_punches, now_utc)` — `prev`'s own `open` count, to
       know whether the previous date had exactly one open stint to
       splice forward.
    2. `classify(punches, now_utc)` — a **fresh** classification of
       `target_date`'s own punches, independent of the `day` variable
       the existing `classify_at` call already produced at this call
       site. This is the pitfall: `classify_at`'s (prev, day) splice
       direction already removes the matching orphan from `day`'s own
       `orphaned_ends` whenever the splice condition is true — reusing
       that already-spliced `day` to test the condition would find
       zero orphans in exactly the case being detected, and the header
       suffix would never fire. The fresh, unspliced `classify(punches,
       now_utc)` result is what `splice_candidate`'s gate must run
       against.
    Test both results against the same three-part condition
    `splice_candidate` (`src/stint.rs`) already implements: exactly one
    open stint on the `prev` side, exactly one orphaned end on the
    `target_date` side, and that orphan is `target_date`'s
    chronologically-first punch. `splice_candidate` is currently
    module-private; widen it to `pub(crate)` rather than
    re-implementing the same three conditions in `status.rs` — the
    module's own convention elsewhere (`DayStints::compute_has_anomaly`)
    already avoids exactly this kind of duplication so the two
    definitions can't drift apart. This is the one named exception to
    "no new public API" below. `StatusView` gains a field carrying the
    (at most one) header suffix string; the header line's assembly
    grows a suffix append.
  - `resolve()`'s open-stint handling gains a three-way classification
    — today / yesterday / two-or-more days back — computed once, as an
    ordinary local, from `today` and `target_date` (both already bound
    early in `resolve()`, before stint-line construction runs). That
    local is threaded as a new parameter into `build_stint_lines`
    (whose signature changes accordingly) so every `StintLine` it
    builds can carry the same value in a new field — a deliberate,
    harmless duplication in the rare multi-open (E7) case, since every
    open `StintLine` in one view shares the same classification. The
    same local is also stored, unchanged, as a new field on the
    `StatusView` struct literal built afterward (it is not copied
    *from* `StatusView` — `StintLine`s are built first, before
    `StatusView` exists). `day_total_line()` reads the `StatusView`
    field directly (it isn't per-stint) so the **Day total** line's
    `(+ ongoing)` suffix becomes `(+ unclosed)` for the
    two-or-more-days-back case, matching the stint line's own wording
    — today's and yesterday's cases leave Day total unchanged.
  - The existing `week_line()` wrapper is extended to pass
    `acct.carry_in` through to `status_week_line`'s new parameter.
    `WeekAccounting` already has a `carry_in` field (`src/week.rs`) —
    no new arithmetic, just plumbing an already-computed value one
    call further.

No new public API is added to `week.rs` or `week_view.rs`, and no
existing arithmetic changes anywhere. The one exception is `stint.rs`:
`splice_candidate`'s visibility widens to `pub(crate)` (§3's check,
above) — its logic, signature, and behavior are otherwise untouched,
and `classify`/`classify_at` themselves gain no new callers' worth of
new behavior, only an additional call each with existing argument
shapes.

## Facts gathered (why the plan looks the way it does)

- `classify_at`'s (prev, day) splice direction only ever removes an
  element from `day.orphaned_ends`; it never touches `day.open`
  (`src/stint.rs`, the (prev,day) branch). The (day, next) direction is
  the one that inspects and mutates `day.open`. Since the header-suffix
  check only needs "did the previous date's own open stint get
  consumed" — a fact about `prev`'s `open`, never its `orphaned_ends`
  — a plain `classify(prev_punches, now_utc)` already gives the right
  answer. Passing a third, further-back neighbor into `classify_at` for
  `prev` buys nothing for this check and was dropped.
- Reusing the `day` variable the existing `classify_at` call already
  produces at this call site (`src/status.rs:344`) for the
  target-date side of the §3 check would silently defeat the feature:
  `classify_at`'s (prev, day) branch removes the matching orphan from
  `day.orphaned_ends` precisely when the splice condition holds, so by
  the time `day` is available, the evidence the check needs is already
  gone. The check must run a second, independent `classify(punches,
  now_utc)` instead.
- `StintLine` and `stint_line()` (`src/status.rs`) are the natural home
  for both new per-stint conditions (span suffix, day-age caption)
  because both are properties of one stint, known at the point each
  `StintLine` is constructed, and `stint_line()` is already the single
  place `StintEnd::Now`/duration formatting happens.

## Global constraints

- No change to `stint::classify` / `stint::classify_at` — not their
  signatures, not their internal logic, not their tests' expected
  values. This task calls `classify` twice more with existing
  arguments (§3, above); it does not add a new code path inside
  `stint.rs` itself. The one permitted `stint.rs` edit is widening
  `splice_candidate` from private to `pub(crate)` — no change to that
  function's own logic or signature, just who can call it.
- No change to week accounting math (`src/week.rs`'s
  `week_accounting`/`WeekLedger`/`WeekAccounting` fields or formulas).
  `carry_in` already exists and is already computed; this task only
  plumbs it into a rendered string.
- Plain-ASCII output, per SPEC.md §7: every new literal must be
  checked against the existing plain-ASCII golden-output tests, each
  extended only for the literals its own function can actually
  produce — `t13_plain_ascii_output_and_non_ascii_notes_pass_through`
  (`status.rs`) for the five literals produced inside `status.rs`
  (`, spans to next day` and `ongoing - elapsed since now, not a
  running total` and `(unclosed)` in `stint_line()`, `continues
  previous day's stint` in the header assembly, `(+ unclosed)` in
  `day_total_line()`); `t33_every_produced_string_is_ascii`
  (`render.rs`) for `carry-in` only, the one new literal
  `status_week_line` itself produces. No parallel ASCII check is
  added.
- Additive-only render changes: every new field on `StatusView`/
  `StintLine` must have a default/omitted rendering identical to
  today's output when the new condition doesn't apply (non-spanning
  stint, non-spliced date, today's own open stint, zero carry-in). No
  existing golden test's expected string may change unless that
  golden test's fixture itself falls into one of the four new
  conditions (verified against the current test module: none do).
- Stay inside the spec's §1 scope table: bullets 1, 2, 6, 7 only.
  Bullet 4 (`[!]` remedy pointers) and bullets 3/5 (splice-vs-flag
  gate, lone-unclosed-`start` distinction) are not touched, in code or
  in prose.
- `week`'s per-date row (`src/week_view.rs`) is not touched — the
  status/week asymmetry described in spec §3 is accepted, not fixed,
  for this changeset.

## Ordering and parallelization

Single task, single worktree. Nothing to parallelize; no interface
contract to pin in advance (there is only one implementer, and the
`status_week_line` signature and its call site land in the same
commit).

## Task 1 — `status`/`render`: span suffix, header suffix, backdated caption, Day total match, carry-in wiring

**Spec citations**: §2, §3, §4, §5.

**Files owned exclusively**: `src/render.rs`, `src/status.rs`.

**Acceptance criteria**:

- §2: a completed stint whose `start.date != end.date` gets
  `, spans to next day` appended inside the existing duration
  parenthetical (same slot as `, ongoing`), verbatim per spec's
  example (`(01h 15m, spans to next day)`). A same-date completed
  stint and any open stint (`StintEnd::Now`) are unaffected by this
  specifically (§4 governs open-stint rendering).
- §3: `resolve()` runs `splice_candidate` (widened to `pub(crate)`)
  against `classify(prev_punches, now_utc)` and a **fresh**
  `classify(punches, now_utc)` — not the `day` variable the existing
  `classify_at` call already produced at this call site, which has
  already had the matching orphan removed by the time it's available
  (see Facts). When the gate is true, the header line gets the suffix
  `  (HH:MM continues previous day's stint)` where `HH:MM` is that
  stint's end time on `target_date`; when false, no suffix, no bracket
  marker, no additional line. At most one suffix ever appears (§4.3.1
  splicing is 1:1 — cover this with a test asserting the positive case
  actually fires, not just that the negative case stays silent).
  Covers the case where the consumed punch was `target_date`'s *only*
  punch (stint-list section is entirely omitted for that date per
  SPEC.md §7.1, but the header suffix, Day total, and week line still
  render).
- §4: for `target_date == today`, open-stint rendering (both the stint
  line and Day total's `(+ ongoing)`) is byte-identical to today's
  current output. For `target_date == today - 1 day`, an open stint
  keeps its live duration figure and gains the caption
  `, ongoing - elapsed since now, not a running total`; Day total is
  unchanged (`(+ ongoing)`). For `target_date <= today - 2 days`, the
  stint's duration figure is dropped entirely and it renders as
  `(unclosed)` with no minutes number anywhere in that line, **and**
  Day total's suffix changes from `(+ ongoing)` to `(+ unclosed)` for
  that date.
- §5: `week_line()` passes `acct.carry_in` to `status_week_line`'s new
  parameter. `carry_in == 0` status output is byte-identical to
  today's. Verify at least the three spec worked examples (deficit,
  surplus, fulfillment-goes-negative) end-to-end through
  `resolve()`/`render()`.
- Regression: every existing golden test in `status.rs`/`render.rs`
  passes unchanged unless its fixture falls into one of the four new
  conditions above (spanning stint, spliced receiving date, non-today
  open stint, non-zero carry-in) — none of the existing fixtures do,
  per inspection of the current test modules.
- Plain-ASCII: extend
  `t13_plain_ascii_output_and_non_ascii_notes_pass_through` to cover
  at least one instance of each of the five `status.rs`-produced
  literals, including `(+ unclosed)`; extend
  `t33_every_produced_string_is_ascii` (owned by Task 1's `render.rs`
  half) to cover `carry-in`.

**Out of scope**:

- §1.2a bullet 4 and bullets 3/5 (explicitly deferred by the spec's
  §8).
- Any `week`/`week_view.rs` marker for the §3 receiving-date fact —
  the status/week asymmetry is accepted per spec §3/§8.
- Any change to `stint::classify`/`classify_at` themselves.
- The §4.3.1 residual (non-1:1) case.
- SPEC.md/NOTES.md prose updates (spec §7) — tracked separately from
  this code task.
