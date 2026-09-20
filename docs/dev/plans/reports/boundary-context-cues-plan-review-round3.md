# Plan review — round 3

Plan: `docs/dev/plans/boundary-context-cues-plan.md`
Spec: `docs/dev/specs/2026-09-20-boundary-context-cues.md`
Prior review: `docs/dev/plans/reports/boundary-context-cues-plan-review-round2.md` (needs-rework, 4 findings)

Scope of this pass: verify round 2's four fixes hold, and hunt for anything
the rewrite introduced. Traced against `src/status.rs`, `src/stint.rs`,
`src/render.rs`, `src/week.rs` as they exist today (not as the plan
describes them).

## Round 2 fixes: status

1. **§3 classification ambiguity (was HIGH)** — fixed. The plan now
   explicitly requires a fresh `classify(punches, now_utc)` call,
   independent of the `day` variable `classify_at` already produced at
   `src/status.rs:344`, and spells out the exact reason (the (prev, day)
   splice direction removes the matching orphan from `day.orphaned_ends`
   before `day` is available). Verified against `stint.rs:293-303`: the
   (prev, day) branch's mutation is exactly as described — `day` really
   does lose the evidence by the time it's built. The fix is correct.

2. **`splice_candidate` visibility (was HIGH)** — fixed, and verified
   usable as described. `splice_candidate`'s real signature
   (`src/stint.rs:334`) is
   `fn splice_candidate(earlier_open_count: usize, later: &DayStints, later_punches: &[Punch]) -> bool`.
   Widened to `pub(crate)`, it is callable from `status.rs` exactly as the
   plan implies: `status.rs` already imports `stint::{self, DayStints}`
   (`src/status.rs:34`), so no new import is needed for the `&DayStints`
   argument, and `later_punches: &[Punch]` accepts `&punches` — the same
   `Vec<storage::Punch>` already in scope at `status.rs:338`, of the same
   `Punch` type `stint.rs` itself uses. No additional plumbing beyond what
   the plan states. Confirmed correct — see also new finding 2 below for a
   residual naming inconsistency in how the two `classify()` calls feeding
   this gate are described.

3. **"additional read" mischaracterization (was LOW)** — fixed. The plan
   now correctly says "no new read, two additional classify() calls,"
   matching the real cost (`prev_punches` and `punches`/`next_punches` are
   already fetched at lines 340-341; only the extra `classify` calls are
   new).

4. **Day total day-age delivery mechanism (was LOW/MEDIUM, "or" phrasing)**
   — the "or" is gone; the plan now pins one design: a single new
   `StatusView` field, copied onto `StintLine` at construction, read
   directly by `day_total_line()`. The decision itself is sound (matches
   how `day_total_line(view: &StatusView)` at `status.rs:122` already
   takes the whole view, while `stint_line(line: &StintLine)` at
   `status.rs:155` takes only one line and has no view access). But the
   *mechanism* description doesn't survive contact with the real code —
   see Finding 1 below. This is the round's central issue.

## Findings

### Finding 1 — HIGH — StintLine's day-age field cannot be "copied from the StatusView value at construction time"

**File**: `docs/dev/plans/boundary-context-cues-plan.md:114-118`, vs.
`src/status.rs:381` and `src/status.rs:384-395`.

**Claim**: "`StintLine` also gains a field carrying this same
classification, copied from the `StatusView` value at construction time,
so `stint_line()`'s `StintEnd::Now` branch can render one of the three
forms without taking the whole view as a parameter."

**Evidence**: In the real `resolve()`, `StintLine`s are built by
`build_stint_lines(&day)` at `status.rs:381` — a plain call producing
`Vec<StintLine>` that is bound to a local `stints` variable. The
`StatusView` struct literal is not constructed until `status.rs:384-395`,
three lines later, and `stints` (already fully built) is simply moved
into it. There is no `StatusView` value in existence at the point
`StintLine`s are constructed to copy a field *from*. Taken literally, the
plan describes an impossible order of operations.

This is functionally fixable — compute the today/yesterday/2+-days-back
classification once as an ordinary local (it only needs `today` and
`target_date`, both already bound by `status.rs:331-336`, well before
`build_stint_lines` runs), thread it as a new parameter into
`build_stint_lines` (whose signature the plan never says needs to change),
and use that same local for both the `StintLine` field and the
`StatusView` field. But that is not what "copied from the `StatusView`
value at construction time" says, and it is exactly the kind of
plausible-but-wrong reading round 2's finding 1 was about: an implementer
who takes the sentence at face value could reasonably try to build
`StatusView` first (moving the `daily_target`/`eod`/`weekday_name`/
`notes` computation at `status.rs:357-382` ahead of stint-line
construction, which the plan's own file-scope and ordering sections never
authorize) rather than the much simpler fix of hoisting one enum value
into a local variable. The plan should say: "computed once as a local
value before `build_stint_lines` is called, threaded into
`build_stint_lines` as a new parameter, and also stored on `StatusView`" —
not "copied from the `StatusView` value."

This is the direct answer to the round's specific question ("does it
introduce a new ordering/construction problem, e.g. StintLines built
before StatusView exists?") — yes, exactly that problem exists in the
plan's prose, even though the underlying design intent (one classification
value, shared) is sound and cheaply fixable.

### Finding 2 — MEDIUM — the plain-ASCII test-extension requirement is wrong for `t33` on 5 of 6 new literals

**File**: `docs/dev/plans/boundary-context-cues-plan.md:176-183`
("Global constraints"), also restated at lines 255-258.

**Claim**: "every new literal (`, spans to next day`, `continues previous
day's stint`, `ongoing - elapsed since now, not a running total`,
`(unclosed)`, `(+ unclosed)`, `carry-in`) must be checked against the
existing plain-ASCII golden-output tests (`t13_...` in `status.rs`,
`t33_every_produced_string_is_ascii` in `render.rs`) — extend those
tests' fixtures to cover the new strings."

**Evidence**: `t33_every_produced_string_is_ascii` (`src/render.rs:569`)
only calls `render.rs`'s own functions directly — `week_headline`,
`status_week_line`, and `Anomalies::detail_lines()`/`row_marker()` — and
asserts their return values are ASCII. It has no access to `status.rs` at
all. Checking where each new literal is actually produced:

- `, spans to next day` — `status.rs`'s `stint_line()` (new branch).
- `continues previous day's stint` — `status.rs`'s `render()` header
  assembly (new branch).
- `ongoing - elapsed since now, not a running total` — `status.rs`'s
  `stint_line()` (new branch).
- `(unclosed)` — `status.rs`'s `stint_line()` (new branch).
- `(+ unclosed)` — `status.rs`'s `day_total_line()` (new branch).
- `carry-in` — the one literal that *is* new output of
  `render.rs`'s own `status_week_line()`.

Five of the six new literals are produced exclusively by functions that
live in `status.rs`, never pass through anything in `render.rs`, and so
cannot appear in any string `t33` exercises no matter how its fixtures are
extended. Only `carry-in` is legitimately coverable by `t33`. The
constraint as written asks a future implementer to extend a test in a way
that is impossible for 5 of 6 literals — they will either waste time
trying, or silently drop the requirement without the plan having told
them which literals it actually applies to. `t13` (status.rs) can and
should cover all six; `t33` should only be asked to cover `carry-in`.

### Finding 3 — LOW — inconsistent instant-variable naming across §3's two `classify()` calls

**File**: `docs/dev/plans/boundary-context-cues-plan.md:81-94`.

**Claim**: item 1 is "`classify(prev_punches, now)`"; item 2 is
"`classify(punches, now_utc)`."

**Evidence**: In `resolve()` there is exactly one `DateTime<Utc>` instant
in scope — the local `now_utc` at `status.rs:343` — and one
`DateTime<Local>` — the function parameter `now` at `status.rs:328`.
`stint::classify` requires `DateTime<Utc>` (`src/stint.rs:124`). Read
literally, item 1's `now` argument names the wrong-typed parameter and
would not compile; both calls must in fact use the same `now_utc` local.
This reads as `stint.rs`'s own doc-comment habit of calling its parameter
"now" bleeding into the plan's prose rather than a genuine two-instant
design, and the Rust compiler would force the correction immediately (so
it's not a silent-defeat risk like round 2's finding 1), but it's a
loose edge in a plan otherwise held to very tight variable-naming
precision, and worth tightening to `now_utc` in both places before
implementation starts.

### Finding 4 — LOW — §5's printed "worked" figure isn't sourced

**File**: `docs/dev/plans/boundary-context-cues-plan.md:53-61, 124-128`.

**Claim**: `status_week_line` gains one new parameter (carry-in, singular:
"the existing `week_line()` wrapper is extended to pass `acct.carry_in`
through to `status_week_line`'s new parameter") and can then print
`fulfillment F = worked W + carry-in C / target T`.

**Evidence**: `status_week_line`'s current signature
(`src/render.rs:76-82`) takes `fulfillment_minutes` and `target_minutes`
but never `worked_minutes`. To print `worked W`, the function must either
derive it internally as `fulfillment_minutes - carry_in` (algebraically
identical to `WeekAccounting.worked` by construction —
`fulfillment = worked + carry_in` is enforced in `week_series`,
`src/week.rs:180`, so this is safe and never diverges) or receive
`worked_minutes` as a second new parameter. The plan states only one new
parameter is added and never says which of these two the implementer
should do. Both are correct and cheap, so this isn't a correctness risk,
but "this task only plumbs it into a rendered string" (line 175) is not
quite true if a subtraction has to happen inside `render.rs` to reconstruct
`worked` — worth one sentence in the plan naming the derivation so a
reviewer isn't left wondering whether a second parameter was silently
dropped.

## Verdict

needs-rework

Finding 1 is the blocking issue: the plan's stated mechanism for sharing
the day-age classification between `StatusView` and `StintLine` describes
an order of construction that does not exist in the real `resolve()`, and
doesn't authorize the (simple) fix — hoisting the classification into an
early local and threading it through `build_stint_lines`'s signature.
Finding 2 is a concrete, checkable factual error (an unsatisfiable test
requirement for 5 of 6 new literals) that should be corrected in the same
pass. Findings 3 and 4 are minor precision gaps, safe to fold in
alongside the other two rather than requiring a separate round on their
own.
