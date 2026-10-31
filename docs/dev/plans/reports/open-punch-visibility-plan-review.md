# Adversarial review — `open-punch-visibility-plan.md`

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Reviewed against the locked spec
(`docs/dev/specs/2026-09-26-open-punch-visibility.md`) and the current
source (`src/stint.rs`, `src/render.rs`, `src/status.rs`,
`src/week_view.rs`, `src/commands.rs`, `src/storage.rs`, `src/main.rs`,
`tests/*.rs`) on branch `open-punch-visibility`.

## Findings

### 1. `tests/status_cli.rs` asserts the old E7 wording and is owned by no task — HIGH

**Claim**: Task 2's acceptance criteria says "no test elsewhere is left
asserting the old E7/E8 text" (plan, Task 2 acceptance criteria,
bullet 5), implying the plan accounts for every place the old wording
is pinned. Every task's "Files owned exclusively" list only names
files under `src/` and two files under the repo root
(`docs/dev/SPEC.md`, `README.md`).

**Evidence**: `tests/status_cli.rs:83` (function
`t14_status_with_anomalies_exits_zero_and_prints_to_stdout`) runs the
real compiled binary and asserts:

```rust
assert!(
    out.contains("[!] 2 open stints for this date (unmatched starts)"),
    ...
);
```

Task 2's own spec citation (§3.4) requires this line become:

```
[!] 2 open stints for this date (only one can be open at a time)
```

`tests/` is a top-level integration-test directory, never mentioned in
any task's "Files owned exclusively" list (Task 1: `src/stint.rs`;
Task 2: `src/render.rs`; Task 3: `src/status.rs`; Task 4:
`src/commands.rs`; Task 5: `src/week_view.rs`; Task 6:
`docs/dev/SPEC.md`, `README.md`). Nobody owns this file, so either a
task edits a file it doesn't own (breaking the parallel-worktree
ownership model the plan is built on) or this integration test breaks
after merge and nobody notices until CI does. `tests/ascii_sweep.rs`
was checked too and is safe (it asserts ASCII-ness and the generic
presence of `[!]`, not literal old wording); only `status_cli.rs:83`
is affected.

### 2. Refused-join predicate's input shape (`classify()` vs `classify_at()`) is not pinned — MEDIUM

**Claim**: Contract B says the refused-join predicate "takes two
adjacent dates' classifications" and mirrors `splice_candidate`
"exactly," implying the two inputs are unambiguous.

**Evidence**: `splice_candidate` is called inside `classify_at`
(`src/stint.rs:301,307`) against **raw** `classify()` output for both
sides — `prev = classify(prev_punches, now)` (`src/stint.rs:293`) for
the earlier side, and the not-yet-spliced `day = classify(punches,
now)` (`src/stint.rs:294`) for the later side — never against either
side's own `classify_at()`-resolved result. This matters: if date `A`
successfully splices with `A-1`, `A`'s `classify_at()` open count for
the pair `(A-1, A)` becomes irrelevant to whether `(A, A+1)` refuses —
what matters is `A`'s *raw* open count before any splice resolution.
A new refused-join predicate that is fed `classify_at()` results
instead (which is what `status.rs`'s `day` variable and
`week_view.rs`'s per-row `day` already are, per `src/status.rs:451`
and `src/week_view.rs:148`) would silently diverge from
`splice_candidate`'s real semantics in a three-consecutive-date chain
(e.g. `A-1→A` already spliced, changing what "A's open count" means
depending which classification vintage is passed).

Contract B pins "the plain-language behavior" of the predicate but
never states which classification vintage (`classify()` output, per
`splice_candidate`'s actual contract, vs. `classify_at()` output,
which is what both consuming call sites already have lying around for
other purposes) the predicate expects as input. `status.rs` already
has precedent for fetching the *raw* classification separately for
splice-adjacent purposes (`src/status.rs:453-460`,
`prev_classified`/`fresh_target_classified`), and Task 3's acceptance
criteria alludes to reusing that pattern ("fetched the same way the
existing splice-check neighbor data already is"). `week_view.rs` has
no such precedent — `build_rows` (`src/week_view.rs:126-169`) only
ever calls `classify_at`, discarding the internal raw `prev`/`next`
computations, so Task 5 must additionally call raw `classify()` on
each neighbor bucket itself. Since Task 3 and Task 5 run in separate
worktrees against Contract B with no shared code review between them,
this is exactly the kind of ambiguity that produces two different
`[!]` outcomes for the same underlying data between `status` and
`week` — silent, not a compile error.

### 3. Task 1's boundary-age test list omits age 0 — LOW (internal inconsistency)

**Claim**: Task 1 acceptance criteria: "A date with exactly one open
start and age 0 or 1 is not stale."

**Evidence**: The same task's own explicit unit-test requirement two
bullets later only lists "the boundary ages (1/2/14/15)" — age 0 is
asserted as a behavior requirement but dropped from the enumerated
test-boundary list. Spec §5 test plan explicitly wants age 0 and age 1
both asserted ("lone open start at age 0 and 1 is not an anomaly").
Trivial to add, but as written the acceptance criteria doesn't require
it, which is exactly the kind of coverage gap the "every branch needs
a test" global constraint is meant to catch.

### 4. Task 4 has no explicit test for the best-effort failure path it requires — LOW (testing gap)

**Claim**: Task 4 acceptance criteria: "If the lookback read fails
after the write already committed, `start` still exits `0` and prints
nothing extra (the check is best-effort...)."

**Evidence**: The same task's "Test coverage" bullet enumerates five
scenarios (warning present/ordering, stdout+exit invariant, no warning
for the just-written date under both sub-cases, no warning when clean,
backdated-into-stale-date behavior) and none of them is "the lookback
read itself fails." This best-effort behavior is also absent from the
spec's own §5 test plan, so the gap originates upstream, but the plan
had the chance to close it for its own task and didn't. Since
`commands::start`'s return value is propagated directly to
`main::exit_code` (`src/main.rs:45,70-78`) with no separate try/catch
visible anywhere else in the dispatch path, this behavior can only be
implemented and verified inside Task 4's own code — if it's untested,
a future refactor that lets a `?` leak from the lookback call would
silently turn a successful `start` into an exit-1 failure, and nothing
in the suite would catch it.

### 5. Undeclared user-visible gap — LOW/MEDIUM, genuine hit

The brief asks specifically whether the document records every
user-visible consequence of its own choices. It's mostly good about
this — §1.2's four "known issues" bullets are thorough and specific.
One real gap is not listed there.

**The choice**: §3.3 defines a lookback finding's *rendering* as
strictly one of exactly two shapes — "one open start" or "N open
stints, see status for that date" (Contract A, bullet 2: "nothing
else" beyond window date/age/single-or-multiple). A lookback finding
never carries or renders refused-join information, even when the
window date's lone open start is, in fact, part of a refused join with
its own neighbor (a shape the spec explicitly designs elsewhere, §3.2,
to fire "at every age").

**The consequence a user will see**: today's `status` (or a backdated
`start`'s stderr) reports a window date's forgotten stop as an
ordinary, unremarkable "unclosed start on Mon D at 09:00 (5 days ago)"
— identical in every way to a genuinely orphaned forgotten stop — even
in cases where that same open start is one half of a refused join
whose other half (the orphaned end on `D+1`) is sitting right there in
the database. The user has no reason from the lookback line alone to
suspect a refused join is involved; they'd have to separately think to
run `mlm status D` to discover the "not joined to ... across midnight"
line and its explanation.

**Why this isn't just the already-declared Known Issue #2**: §1.2's
second bullet ("The lookback ... counts open starts only. An orphaned
end older than yesterday stays visible only on its own date...")
covers the *end*-side of this scenario — that a lone orphaned end
inside the window is invisible from today's status. It does not cover
the *start*-side consequence identified here: that the visible
lookback line for the open start, when it does render, actively omits
the refused-join fact even though the predicate to detect it (Task 1's
own new refused-join predicate) exists and is already computed
elsewhere on the very same date. This is a distinct, additional
information loss the plan doesn't record anywhere — not in §1.2, not
in Task 3's out-of-scope note, not in the global constraints.

This is a legitimate finding for the category, not a request to widen
scope — Contract A's "nothing else" is a fine, deliberate scope
choice. It just isn't written down as a consequence anywhere a reader
of the plan or spec would find it.

## Verdict

**ship-with-followups.** The architecture, task split, and file
ownership are sound everywhere except the one concrete break (Finding
1), which must be fixed before merge — either fold `tests/` into an
owning task (Task 2 is the natural owner, since it already commits to
updating "every place the old wording is pinned") or add it explicitly
to the plan's file-ownership list. Finding 2 should be resolved by
pinning the classification vintage in Contract B text before Task 3
and Task 5's worktrees open, to avoid a silent status/week divergence.
Findings 3 and 4 are one-line acceptance-criteria additions. Finding 5
is worth a line in SPEC.md's §1.2 (Task 6) alongside the existing four
bullets, since it's the same flavor of "known, accepted" residual gap
as the other three.
