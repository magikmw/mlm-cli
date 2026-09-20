# Adversarial review, round 2 — boundary-context-cues-plan.md

Reviewed against: `docs/dev/plans/boundary-context-cues-plan.md` (post-rewrite),
`docs/dev/specs/2026-09-20-boundary-context-cues.md` (amended §4),
`src/status.rs`, `src/render.rs`, `src/stint.rs`, `src/week.rs`.
Prior round: `docs/dev/plans/reports/boundary-context-cues-plan-review.md`
(needs-rework, 4 findings + 1 undeclared gap).

## What the rewrite got right (verified, not just asserted)

- **Finding 1 (task-split compile break) is fully resolved.** `render::status_week_line`
  (`src/render.rs:76`) is a free function with 5 positional params, called from
  `src/status.rs:212` (`week_line()`) and `src/status.rs:1313`
  (`week_line_matches_status_week_line_field_for_field`). The plan now puts both files
  under one task (plan.md:169: "Files owned exclusively: `src/render.rs`, `src/status.rs`"),
  so the signature change and both call-site fixups land in the same commit. No
  intermediate non-compiling state is possible. Confirmed by inspection — no other
  caller of `status_week_line` exists outside these two files (`week_view.rs` uses only
  `week_headline`/`Anomalies`, not `status_week_line`).
- **Finding 2 (predecessor's-predecessor fetch) is correctly dropped and correctly
  justified for the *half* of the claim it makes.** `classify_at`'s (prev, day) branch
  (`src/stint.rs:301-303`) only ever calls `day.orphaned_ends.remove(0)`; it never
  touches `day.open`, and — independently re-verified here — the `prev` variable it
  uses is itself built via plain `classify(prev_punches, now)` (`src/stint.rs:293`),
  **not** a further-neighbor-aware `classify_at`. So `prev.open.len()` is invariant to
  what `prev`'s own predecessor's punches are; plan.md's Facts section (109-117) is
  accurate on this specific point.
- **§4/Day-total wording matches the amended spec exactly.** Plan.md:192-201 reproduces
  spec §4's three cases and the `(+ ongoing)` → `(+ unclosed)` Day-total change
  verbatim, matching spec.md:63-101 including the literal caption string
  `, ongoing - elapsed since now, not a running total` and the trigger conditions
  (today unchanged, yesterday unchanged Day total, 2+-days-back gets `(+ unclosed)`).
  The ASCII-literal constraint list (plan.md:135-138) also lists `(+ unclosed)` for the
  golden-test extension, matching spec's testing plan §6.

## Findings

### 1. [HIGH] §3's check, as described, is ambiguous about *which* classification of `punches` to test — and the natural, already-in-scope one silently defeats the whole feature

**Claim** (plan.md:76-84): "`resolve()` gains one additional read: `target_date`'s
predecessor's own punches, classified with `classify`... to check whether that
classification leaves an open stint that matches §4.3.1's splice condition into
`target_date`."

**Evidence**: `resolve()` (`src/status.rs:326-396`) already computes, at line 344:
`let day = stint::classify_at(&prev_punches, &punches, &next_punches, now_utc);` — this
is the *post-splice* `DayStints` for `target_date`. Inside `classify_at`
(`src/stint.rs:301-303`), the (prev, day) direction does `day.orphaned_ends.remove(0)`
**whenever the splice condition is true** — i.e., precisely in the case the plan's §3
header suffix is supposed to detect, `day`'s copy of `orphaned_ends` has *already* lost
the one orphan that would prove the condition. `splice_candidate`
(`src/stint.rs:334-342`) requires `later.orphaned_ends.len() == 1`; after
`classify_at`'s own removal, that count is 0 for exactly the true case.

So if an implementer follows the plan's plain reading — call `classify(prev_punches,
now)` for the prev side, then test the splice condition "into `target_date`" using the
`day` variable *already sitting in scope* from line 344 (the obvious, minimal-diff
thing to do, since `resolve()` doesn't otherwise need a second classification of
`punches`) — the header suffix will **never render**, in every case where it's supposed
to. The only way to get this right is a *second, independent* `classify(punches,
now_utc)` call (discarding `classify_at`'s splice mutation), which the plan never says
explicitly. The plan's own Facts section (109-117) carefully re-derives why `prev`'s
side is safe to compute plainly, but is silent on the day-side pitfall entirely — this
is not covered by anything already re-verified from round 1.

This is a correctness landmine, not a style nit: a plan this precise everywhere else
(explicit struct fields, explicit call sites, explicit "not `classify_at`" callouts)
leaves exactly the one place where reusing an in-scope variable produces the opposite
of the intended behavior, unstated.

Mitigating factor: the acceptance criteria (plan.md:187-188) does require "cover this
with a test, don't just assume it" for the splice-suffix case, and a positive-case test
(suffix present in the true-splice scenario) would fail loudly rather than silently —
so this would likely surface during implementation, not ship silently. Still a real
planning gap that costs an implementer a debugging cycle the plan should have
foreclosed.

### 2. [HIGH] The splice check the plan specifies is not implementable without either violating the plan's own "no new public API" constraint or duplicating private logic — neither option is named

**Claim** (plan.md:79-81, restated at Facts 109-117): resolve() should check "whether
that classification leaves an open stint that matches §4.3.1's splice condition into
`target_date`" — i.e., exactly `splice_candidate`'s three-part gate (arity 1 open,
exactly 1 orphan, orphan is the chronologically-first punch).

**Evidence**: `splice_candidate` (`src/stint.rs:334`) is declared `fn
splice_candidate(...)` with no `pub` — module-private, unreachable from `status.rs`.
Verified against `src/lib.rs:13` (`pub mod stint;`, so the module itself is public, but
its own item-level privacy still hides this one function) and a full grep of
`stint.rs`'s function signatures, which shows only `classify`/`classify_at` are `pub`;
`close_group` and `splice_candidate` are both bare `fn`.

To implement the plan's §3 check as described, `status.rs` needs this exact 3-part gate
(arity-1-open, exactly-1-orphan, orphan-is-first-punch) and has exactly two options:
(a) raise `splice_candidate`'s visibility (at least `pub(crate)`), or (b) reimplement
the same three conditions inline in `status.rs`, hand-duplicating logic that `stint.rs`
itself treats as the single authoritative definition (the module's own doc comments
elsewhere — e.g. `DayStints::compute_has_anomaly`, `src/stint.rs:71-79` — go out of
their way to avoid exactly this kind of duplication: "so the two can never drift
apart"). The plan never names either option.

Worse, option (a) directly contradicts the plan's own architecture prose (plan.md:
104-106): "No new public API is added to `stint.rs`, `week.rs`, or `week_view.rs`, and
none of their existing arithmetic changes." Widening `splice_candidate`'s visibility
*is* new public (or at least new crate-visible) API on `stint.rs`, even though it
doesn't touch `classify`/`classify_at`'s own signatures — the plan's global constraint
(plan.md:126-130) is scoped explicitly to "`classify`/`classify_at`... not their
signatures, not their internal logic," so a visibility bump to `splice_candidate` is
arguably compliant with the *narrower* constraint but flatly contradicts the *broader*
one two paragraphs earlier in the same document. That internal inconsistency is itself
worth flagging independent of which option the implementer picks.

### 3. [LOW] "resolve() gains one additional read" mischaracterizes what's actually new

**Claim** (plan.md:76-77): "`resolve()` gains one additional read: `target_date`'s
predecessor's own punches..."

**Evidence**: `resolve()` already fetches this exact data at `src/status.rs:340`:
`let prev_punches = storage::punches_for_date(conn, target_date.pred_opt().unwrap())?;`
— it's fetched today, for the existing `classify_at` call at line 344. No new DB read
is needed for §3 at all; the only new work is an additional `classify()` *call* over
data already in scope (and, per finding 1, a second `classify()` call over `punches`
too, which the plan doesn't budget for even as a call, let alone a read). Calling this
"one additional read" is imprecise in a plan that is otherwise careful to distinguish
reads from computation (see its own Architecture intro: "the `resolve` (I/O +
arithmetic) / `render` (pure) split"). Low severity because it doesn't change what
needs to be built, but it's the kind of imprecision that, combined with finding 1's
ambiguity, nudges an implementer toward reusing the wrong variable.

### 4. [LOW/MEDIUM] Day total's day-age delivery mechanism is the one spot in the plan still phrased as an open choice, not a decision

**Claim** (plan.md:92-97): "`StatusView` also gains a field (or reuses the same
per-line classification, read by `day_total_line()`) so the **Day total** line's `(+
ongoing)` suffix becomes `(+ unclosed)`..."

**Evidence**: `day_total_line()` (`src/status.rs:122-152`) already takes `&StatusView`
and could read either a new `StatusView`-level field or scan `view.stints` for a
`StintEnd::Now` entry's age field — both are mechanically workable, since
`day_total_line()` has the whole view in hand either way. This is not a correctness
gap like findings 1-2 (either implementation works), but it is the single remaining
place in an otherwise field-and-call-site-precise plan where the delivery mechanism is
left as an "or," which is exactly the category of gap round 1's finding 3 flagged and
this rewrite was supposed to close for every per-item mechanism. Every other new
signal in this plan (span-suffix bool, header-suffix string, per-stint day-age) gets an
unambiguous "X gains a field on Y" sentence; this one alone doesn't land on one.
Worth a one-line resolution before implementation, not a blocking issue given it's a
single-task/single-implementer plan.

Note for the implementer either way: since day-age is a fact about `target_date`
relative to `today`, not about any individual stint, every `StintEnd::Now` `StintLine`
in one view necessarily carries the *same* age value (there is exactly one
`target_date` per view). Putting the field on `StintLine` (as plan.md:98-101 specifies
for the per-stint caption) is therefore slightly redundant in the rare multi-open (E7)
case — harmless, but worth naming as "known and accepted" rather than leaving it to be
rediscovered.

## Other checks performed, no defect found

- Rechecked finding 4 and the undeclared-gap item from round 1: both fully addressed —
  Day total's `(+ unclosed)` fix is in the plan's acceptance criteria (plan.md:199-201)
  and matches the spec's amended §4 wording exactly (spec.md:94-96).
- Checked whether adding fields to `StintLine` (`src/status.rs:75-79`, no `Default`
  derive) breaks the ~15 existing struct-literal test fixtures in `status.rs`'s test
  module: yes, mechanically, but this is the ordinary and unavoidable cost of any
  additive struct-field change within a single task/file, not a cross-task compile-gate
  risk like round 1's finding 1. Not counted as a plan defect.
- Checked `week.rs` for `carry_in`: present (`src/week.rs:66`), plumbing claim in §5 is
  accurate, no arithmetic change implied or needed.
- No new scope creep found: the plan still confines itself to spec bullets 1/2/6/7 and
  doesn't touch `week_view.rs` or `classify`/`classify_at`'s public signatures.

## Verdict

**needs-rework** — findings 1 and 2 are both high because they attack the same
feature (§3's header suffix) from two angles that compound: the plan's described check
is (a) ambiguous in a way whose natural reading silently disables the feature, and
(b) not implementable as literally described without either contradicting the plan's
own "no new public API" constraint or hand-duplicating `stint.rs`'s private
splice-detection logic. Neither is named or resolved anywhere in the document. Finding
4 is a smaller residue of round 1's finding 3 that should be closed out at the same
time. Finding 3 is cosmetic. Recommend the plan add one concrete sentence for §3:
which classification of `punches` to test the splice condition against (a fresh
`classify(punches, now_utc)`, distinct from the already-spliced `day`), and how
`status.rs` gets access to the three-part splice gate (name the visibility change to
`splice_candidate` explicitly, or spell out the duplicated three-line condition and
accept the duplication in the same breath the plan already accepts "no new public API"
needs an exception here).
