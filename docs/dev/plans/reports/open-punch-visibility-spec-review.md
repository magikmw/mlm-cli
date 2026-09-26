# Adversarial review — open-punch-visibility spec draft

Reviewed: `docs/dev/specs/2026-09-26-open-punch-visibility.md` against
baseline `v0.3.6` (`main` at `267060b`), the actual `267060b` commit
being the tip of `main` per the conversation's git status — confirmed
correct.

## Findings

### 1. Factual error — `-0` is not a valid `status` DATE; spec assumes it is

**Severity: high.**

Spec, `docs/dev/specs/2026-09-26-open-punch-visibility.md:83`:

> When `status` shows today (`DATE` omitted, `-0`, or today's date), it
> also classifies every date in the lookback window.

This treats `-0` as one of three equivalent spellings of "today" for
`status`. It is not. `src/date.rs:126-156` (`resolve_date`), the exact
function `status::resolve` calls at `src/status.rs:434`, explicitly
rejects `-0`:

```rust
if n == 0 {
    return Err(err(Cause::Shape));
}
```

and this is pinned by a dedicated test,
`src/date.rs:1075-1086` (`resolve_date_rejects_minus_zero_as_shape`),
which asserts `resolve_date("-0", today)` returns
`Cause::Shape` ("expected YYYY-MM-DD or -N (N >= 1)").

So `mlm status -0` today is a hard error, not "today's status." The
spec's §3.3 behavior description (today's status runs the 14-day
lookback) is otherwise fine when `DATE` is omitted or equals today's
literal date, but the `-0` branch as written describes behavior the
codebase does not have and never will unless this spec also silently
proposes changing `resolve_date`, which it does not mention anywhere
(§1.1's non-goals list doesn't touch DATE parsing at all). This needs
either removal of the `-0` clause or an explicit new-behavior line
making `-0` valid, with its own test-plan entry — neither exists.

### 2. Undeclared user-visible gap — the `delete punch --date D` "fix" line does not delete anything

**Severity: high — this is the "undeclared user-visible gap" category.**

Every `fix:`/`or:` block in §3.4 pairs a direct-acting `fix:` command
with an `or:` alternative, e.g. (spec lines 110-113):

```
[!] 2 open stints for this date (only one can be open at a time)
    fix: mlm stop HH:MM --date D
     or: mlm delete punch --date D
```

`mlm delete punch --date D` with no `ID` is list mode
(`src/commands.rs:204-229`, `delete_punch`): when `args.id` is `None`
it only prints the date's punches with 1-based numbers
(`print_punch_list`, `src/commands.rs:82-92`) and returns — it never
calls `storage::delete_punch`. Pinned directly by
`src/commands.rs:1324-1330`
(`list_mode_empty_date_prints_nothing_to_delete_punch`) and by the
more general `list_mode_never_mutates_storage_and_numbers_correctly_punch`
test (`src/commands.rs:1332-1345`).

So a user who copies the literal `or:` line and runs it gets a listing,
not a fix — the anomaly is still there afterward, and they must read
the listing, pick an `ID`, and run a second, different command
(`mlm delete punch <ID> --date D`) to actually resolve anything. The
spec never states this two-step nature anywhere — every other `fix:`
line in the document (the `stop` line, the `start` line for E8) is a
complete, copy-pasteable remedy in one shot. This is exactly the
category the review brief calls out: a deliberate, correct choice (you
genuinely can't guess the `ID` without listing first) whose visible
consequence — the printed `or:` line looks actionable but is only
step one of two — is nowhere written down. It is also not tested:
the spec's own test plan (§ "Test plan") never asserts what running
the literal fix/or commands actually produces.

### 3. Missing edge case — `mlm stop HH:MM --date D` can silently fail to close the intended stint, or close the wrong one

**Severity: high.**

The spec presents `fix: mlm stop HH:MM --date D` as *the* remedy for
both the stale-open-start and the E7 multi-open anomaly, with `HH:MM`
as "a literal placeholder the user replaces with the real time"
(spec line 102-103). Two real pairing hazards are not mentioned:

- **No chronology check.** `src/stint.rs`'s own comment
  (`open_stint_in_the_future_clamps_to_zero_minutes`, lines 596-604)
  and SPEC.md's own §3.2 documented "no-chronology acceptance" mean an
  `end` earlier than the stale `start`'s own timestamp does not raise
  an error. Per `classify`'s LIFO/nearest-match algorithm
  (`src/stint.rs:118-184`), punches are sorted by `at_utc` before
  pairing; an `end` whose instant sorts *before* the stale `start`'s
  instant never sees that `start` on the stack (nothing has been
  pushed yet at that point in the scan) and becomes a **new orphaned
  end** instead of closing anything. Running the literal fix command
  with a wrong `HH:MM` therefore does not fix the stale start — it
  adds a second, brand-new anomaly (E8) on the same date, and the
  original stale start is still open. Nothing in §3.4's fix text warns
  the replacement time must be later than the open start's own time.
- **Wrong stint closes under E7.** For the "2 open stints" anomaly,
  `mlm stop HH:MM --date D` closes whichever open start is *nearest in
  time* to the new end (LIFO — `close_group`, `src/stint.rs:197-236`;
  demonstrated by `nested_entry_pairs_nearest_not_outermost`,
  `src/stint.rs:752-779`), which is not necessarily the one the user
  means to close, and running it once only ever resolves one of the
  two-plus open starts — it must be run multiple times for `count >
  2`. The spec's E7 fix text doesn't mention either fact.

Neither hazard is in the test plan (§ "Test plan" only checks that
`fix:`/`or:` lines "carry the real date for every anomaly kind" — never
that following them actually clears the anomaly).

### 4. Undeclared user-visible gap — skipping the just-written date in `start`'s lookback also hides a freshly self-inflicted E7

**Severity: medium.**

§3.6 (lines 171-188): after a `start` write, the lookback runs "relative
to today. It skips the date it just wrote to," justified only as
"keeps a backdated `start --date D` from warning about the punch it
just wrote" (line 187-188).

That justification covers the intended case (don't nag about the stint
you just opened). It does not cover the unintended one: if the user
runs `mlm start --date -5 09:00` on a date that *already* has an
unclosed start, the write creates a brand-new **E7 anomaly** (two open
stints, `src/stint.rs:606-628`) on exactly the date being skipped. By
design, `start`'s own warning mechanism will say nothing about it —
the user gets zero feedback that their backdated punch collided with
an existing one, until they separately run `status -5` or wait up to
14 days for it to fall inside a future lookback window on a date
*other* than the one just written (moot, since it's still the same
date, permanently excluded). This is a deliberate, correctly-reasoned
design choice (suppress noise about the punch you just made) with an
unstated side effect (it also suppresses a brand-new, unrelated-in-kind
anomaly the same write just created). Not in the test plan.

### 5. Missing edge case / scope note — lookback window only checks open starts, never orphaned ends

**Severity: medium.**

§2's **lookback window** and §3.3's mechanism are keyed entirely on
"open starts" ("Each window date with one or more open starts adds one
line," line 85). `render::Anomalies`/`DayStints` also track orphaned
ends (E8, `src/stint.rs:36-41`, always an anomaly regardless of age,
per `Anomalies::has_any()` at `src/render.rs:139-141`), but nothing in
§3.3's lookback, §3.4's lookback line shapes, or §3.6's `start` warning
ever surfaces an orphaned end sitting in the 2-14-day window. A stray
`end` punch with no matching `start`, more than a day old, remains
exactly as invisible from today's `status`/`start` as before this
changeset — only the "forgotten `stop`" (dangling `start`) half of the
problem gets the new lookback treatment. This may be intentional given
the spec's stated motivation (§1: "the commonest one, a forgotten
`stop`"), but it is never stated as a boundary of the fix, and it is
not listed in §1.2's "Known issues left after this changeset," which
otherwise itemizes exactly this shape of leftover gap for the >14-day
case. Worth a `known issues` bullet either way, so a future reader
doesn't mistake "the visibility feature" for "all anomalies now have
window visibility."

### 6. Undeclared user-visible gap — mandatory 14-date lookback on every plain `status` call, no opt-out, conflicts with a documented future goal

**Severity: medium.**

§3.3 makes the lookback unconditional for any `status` call resolving
to today: "it also classifies every date in the lookback window" —
always, not just when an anomaly is later found to exist. Per
`src/status.rs`'s `resolve()` shape, each of those 14 extra dates needs
its own `punches_for_date`/neighbor reads and a `classify_at` call —
real, unconditional extra I/O on every ordinary `mlm status`, even a
perfectly clean one with zero anomalies.

`AGENTS.md`'s own "Current state" section lists as a stretch goal:

> shell prompt integration — needs a fast, side-effect-free "status"
> query (e.g. `mlm status --short`) cheap enough for PS1/starship

This spec's design moves the default `status` command in the opposite
direction (heavier, not lighter) with no flag to skip the lookback and
no mention anywhere in §1 ("Summary"), §1.1 ("Non-goals") or §3.3 of
this tradeoff. A reader of just this document has no way to know the
plain, anomaly-free `mlm status` got slower, or that it now conflicts
with a goal recorded elsewhere in the repo's own project notes.

### 7. Internal inconsistency risk (not a bug, but underspecified) — refused-join can co-fire with E7 for a reason unrelated to the "exactly one" story the anomaly text tells

**Severity: low.**

§2's **refused join** definition ("adjacent dates `A`, `A+1` where,
after `classify_at`, `A` has at least one open start and `A+1` has at
least one orphaned end") is written as "at least one," but the
worked example text in §3.4 (line 136) narrates the gate as needing
"exactly one open start ... and exactly one unmatched end." Verified
against `splice_candidate` (`src/stint.rs:334-346`): the *splice* gate
is indeed the strict 1-and-1 shape, but the *refused-join anomaly*
as defined in §2 fires more broadly (`>= 1` on each side), so it also
fires when `A` already has an E7 (two-or-more opens) — a case whose
real cause is "too many opens," not "an ambiguous single pairing." The
rendered text (line 136's parenthetical, reused verbatim on both
dates) states the splice's strict two-single-punch condition as if it
were the reason nothing joined, which is technically still true (the
gate failed because the shape wasn't 1-and-1) but reads oddly next to
an already-present `[!] 2 open stints` line on the same date, since
the join failed for a reason the E7 line already fully explains. Not
wrong, but worth a one-line acknowledgment; verified there's no actual
double-counting or contradiction in the underlying data (see next
finding, which confirms no false positive exists mechanically).

### 8. Verified correct — no false positive in refused-join detection from post-splice leftovers

Checked directly against `classify_at` (`src/stint.rs:256-323`): a
successful splice always fully removes the consumed item —
`day.orphaned_ends.remove(0)` for a prev-direction splice, or
`day.open.remove(0)` (plus adding to `completed`) for a next-direction
splice — and the splice gate on each side requires the *raw*,
pre-splice count to be exactly right (`prev.open.len() == 1`,
`later.orphaned_ends.len() == 1`, per `splice_candidate`). There is no
code path that consumes one of two same-typed items and leaves the
other behind as a "leftover" that could masquerade as an unrelated
refused-join anomaly. Every "stays unspliced" test in `src/stint.rs`
(`splice_prev_two_opens_stays_unspliced`,
`splice_day_two_orphans_stays_unspliced`,
`splice_orphan_not_first_punch_of_day_stays_unspliced`) confirms the
unspliced state is the *original*, complete, un-consumed state on both
sides — exactly what §2's refused-join definition needs to read
correctly. This category, as asked, is empty beyond the "at least
one" vs. "exactly one" wording note above (finding 7), which is a
documentation clarity issue, not a mechanical false positive.

### 9. Testing gap

The spec's own "Test plan" section never tests:
- that the `fix:`/`or:` commands, when actually run, resolve the
  anomaly they're attached to (findings 2 and 3 above would have been
  caught by this);
- the `-0` DATE claim (finding 1);
- a backdated `start` that creates a fresh E7 on the skipped date
  (finding 4);
- an orphaned end sitting inside the lookback window with no
  accompanying open start (finding 5).

## Verdict

**needs-rework.** Findings 1-3 are user-facing correctness/consistency
problems in the spec's own worked examples and behavioral claims (a
DATE spelling that errors instead of working, and two of the
document's four "fix" commands that either don't act as a complete
remedy or can actively make things worse under plausible input) — not
edge-case nitpicks. These should be resolved before this draft is
treated as ready to implement against.
