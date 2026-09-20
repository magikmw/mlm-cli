# Adversarial review — boundary-context-cues-plan.md

Reviewed against: `docs/dev/plans/boundary-context-cues-plan.md`,
`docs/dev/specs/2026-09-20-boundary-context-cues.md`, `src/status.rs`,
`src/render.rs`, `src/stint.rs`, `src/week.rs`, `src/week_view.rs`.

## Findings

### 1. [HIGH] Task 1 cannot land as an independently-compiling commit — the "no dependency in that direction" claim is understated to the point of being wrong

**Claim** (plan.md:161-169, plan.md:171-189): "Task 1 (`src/render.rs`) must land before Task 2 (`src/status.rs`) can compile against it" and "Do not open a second worktree for this changeset; run Task 1 to completion and merged, then run Task 2 against the merged result." The interface-contract section (plan.md:178-189) has Task 1 "decide the exact parameter shape (e.g., an added `carry_in_minutes: i64` argument...)" for `status_week_line`.

**Evidence**: `src/status.rs:211-213` (`week_line()`) calls `render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today)` with the current 5-argument signature, and `src/status.rs:1309-1315` (`week_line_matches_status_week_line_field_for_field`) calls `render::status_week_line(acct.week, acct.owed, acct.fulfillment, acct.target, today())` directly, also with 5 positional arguments. Rust has no default/optional positional arguments. If Task 1 adds a required new parameter to `status_week_line` (the plan's own suggested shape), both of these call sites in `status.rs` — a file Task 1 does **not** own — fail to compile the moment Task 1's change lands, before Task 2 touches anything.

The plan frames the dependency as one-directional ("Task 2 ... cannot compile ... until Task 1's exact new signature exists") and describes Task 1 as mergeable to completion on its own. In fact Task 1, merged alone under its own stated file-ownership boundary (`src/render.rs` only), leaves the crate in a non-compiling state — `cargo build`/`cargo test` breaks on `src/status.rs` — until Task 2 lands. This contradicts both "Files owned exclusively: `src/render.rs`" (Task 1) and the instruction to run Task 1 "to completion and merged" before starting Task 2, since "complete" for a Rust crate normally implies "the crate builds."

The plan never states this consequence or offers a way around it (e.g., landing Task 1 as an added `status_week_line_v2`/overload that leaves the old signature intact until Task 2 switches the call site, or explicitly accepting a red build between the two tasks). As written, a coordinator following "run Task 1 to completion and merged" literally will merge a broken build.

### 2. [MEDIUM] §3's extra "predecessor's predecessor" punch fetch is unnecessary — the check it enables doesn't need it

**Claim** (plan.md:86-96): "`resolve()` gains a small additional lookup — one more day's punches (`target_date`'s predecessor's predecessor) so it can classify the *previous* date with its own correct neighbors and check whether that classification produced a completed stint ending on `target_date`."

**Evidence**: In `src/stint.rs:296-322` (`classify_at`), the two splice directions are independent: the (prev, day) direction (lines 301-303) only ever removes an element from `day.orphaned_ends`; it never touches `day.open`. The (day, next) direction (lines 307-319) is the one that inspects `day.open.len()` (via `splice_candidate`, `stint.rs:334-342`) and, if it fires, removes from `day.open` and pushes to `day.completed`. Since `day.open.len()` is only ever read (never mutated) by the (prev, day) direction, `prev_date`'s own `open.len()` — the only fact needed to answer "did `target_date`'s first punch get consumed" — is identical whether it's computed via plain `classify(prev_punches, now)` (as `stint.rs`'s own existing call at `status.rs:344` already does internally, via its `let prev = classify(prev_punches, now);`) or via a "correctly neighbored" `classify_at(prev_prev_punches, prev_punches, punches, now)`. Passing `&[]` in place of real predecessor's-predecessor punches produces the exact same `day.open.len()` for this purpose, because a splice from the (prev_prev, prev) direction only ever shrinks `prev.orphaned_ends`, which this check never reads.

So the extra DB fetch (`target_date`'s predecessor's predecessor) that the plan adds to `resolve()` buys nothing for the stated purpose and is scope creep relative to the plan's own minimalism framing ("this task only *reads* that existing result, it does not add a new code path to `stint.rs`" — plan.md:92-94). It also somewhat undersells what actually changes: this is not "reading an existing result" from the existing `classify_at` call at `status.rs:344` (that call's `prev` variable and its consequences are discarded, never exposed) — it is a **second, new call site** into `classify_at` with different arguments, which is a bigger footprint than the plan's prose implies, independent of whether the extra fetch is dropped.

### 3. [MEDIUM] §4's day-age signal has no stated delivery mechanism into `stint_line()`

**Claim** (plan.md:98-104): "`resolve()`'s open-stint handling gains a three-way classification... `StintEnd::Now`'s rendering in `stint_line()` gains the two new forms."

**Evidence**: `stint_line()` (`src/status.rs:155-167`) takes only `&StintLine`, and `StintLine` (`src/status.rs:74-79`) carries no date or day-age field — nor does `render()`'s call site (`src/status.rs:187`, `.map(stint_line)`) pass any per-line or view-level context. Unlike §2, where the plan explicitly says "`StintLine` gains a field to carry it" (the spans-to-next-day bool), the plan gives no equivalent instruction for how the today/yesterday/2+-days classification reaches `stint_line()` for the `StintEnd::Now` branch — via a new `StintLine` field (redundant, since every open stint in one view shares the same target-date age), a changed `stint_line()` signature taking extra context, or something else. This is left to the implementer to invent, unlike every other sub-item in Task 2, which the plan pins down to the exact struct/field/call site. Low risk since both changes fall inside a single task with one owner, but it is a real gap relative to the plan's otherwise line-level precision.

### 4. [LOW] "no existing fixtures fall into one of the four new conditions" is correct but stated without the check that makes it true

**Claim** (plan.md:285-290): "none of the existing fixtures do, per inspection of the current test module, so no existing assertion should need to change."

**Evidence**: Verified true — `resolve_midnight_splice_earlier_date_shows_completed_stint_no_anomaly` (`status.rs:1142-1156`) and its siblings assert only on `StintLine`'s structured fields (`start`, `end`), never on `render()`'s string output, so the new `, spans to next day` suffix cannot break them; none of the `render()` golden tests (`t5`..`t13`, `t11`) construct a `StintLine` with distinct start/end dates or a non-today open stint. The claim holds up, but it is asserted rather than demonstrated in the plan — a reviewer has to redo this same inspection to confirm it, as done here. Not a defect, just worth recording that the review actually re-verified this rather than taking the plan's word for it.

## Undeclared user-visible gap

**Finding**: real, one item. §4's caption/`(unclosed)` change to the *stint line* creates a new inconsistency with the **day-total line**, which the plan does not touch and does not mention.

`day_total_line()` (`src/status.rs:122-152`) appends `" (+ ongoing)"` whenever `view.has_open_stint` is true (line 129-131), and `has_open_stint` is set unconditionally from `day.is_ongoing()` (`status.rs:347`) — not gated on `is_today`. Nothing in this plan's Task 2 scope touches `day_total_line()`. So for a `status` view of a date two or more days back with a stale open stint, after this changeset lands, the output will read:

```
Day total:     00h 00m (+ ongoing)
  09:00-now    (unclosed)
```

— the day-total line still says "ongoing" (implying a live, trustworthy in-progress figure) for the exact same stint that the per-stint line, one section down, now explicitly flags as stale enough that its duration figure has been withheld. That juxtaposition is a direct, visible consequence of this plan's own §4 decision (to selectively suppress trust in the stint line but nowhere else), and neither the plan nor the spec it implements records it. It doesn't fail any stated acceptance criterion (day_total_line is correctly "out of scope"/untouched), but a user reading the two adjacent lines gets contradictory signals about the same fact, introduced by this changeset. Worth a one-line callout in the plan (even just "day_total_line's `(+ ongoing)` wording is intentionally left as-is and will read oddly alongside a `(unclosed)` stint line for backdated dates") so it reads as an accepted asymmetry rather than an oversight — matching how the plan/spec already do this explicitly for the §3 status/week asymmetry.

## Verdict

**needs-rework** — Finding 1 is not a nitpick: as written, the plan's own ordering instructions ("run Task 1 to completion and merged, then run Task 2") produce a non-compiling intermediate state, because Task 1's file-ownership boundary (`render.rs` only) is incompatible with changing `status_week_line`'s signature while `status.rs` still calls it with the old arity. The plan needs to either accept and document a red-build window, or change Task 1's contract (e.g., an additively-named new function, or Task 2 folding in the one-line call-site fixup as an explicit, narrow exception to "Task 1 owns render.rs exclusively"). Findings 2-4 and the undeclared-gap item are real but lower-severity and could ship as noted follow-ups if the coordinator disagrees on priority — Finding 1 alone is why this isn't ship-with-followups.
