# Milestone 5 — Stint pairing: independent adversarial review

**Reviewed**: `bc19fda` on branch `milestone-5` (worktree
`/home/magikmw/projects/mlm-wt-milestone-5`), against `SPEC.md` §4.3/§1.3,
`PLAN.md` contracts 2/6/8, and `plans/milestone-5-stint-pairing.md`.
Reviewer did not write the code. No files were modified; all probing was done
in a throwaway clone.

---

## Verification of the report's own claims (all re-run, not taken on faith)

- `cargo build` — passes. Exactly 3 warnings, all `dead_code` in
  `src/time.rs` (`parse_hm`, `between`, `format_duration`). **None** from
  `src/stint.rs`. Claim confirmed.
- `cargo test` — `21 passed; 0 failed; 0 ignored`. Claim confirmed; the 21
  test names match the report's table 1:1.
- `cargo clippy --all-targets -- -D warnings` — fails with exactly 3 errors,
  all three the `time.rs` dead-code findings. **Baseline claim verified
  independently**: checked out the untouched parent commit `638f2d6` in a
  separate clone (`src/` there contains no `stint.rs`) and clippy fails with
  the identical three errors. So Milestone 5 introduces zero new clippy
  findings.
- `cargo clippy --all-targets -- -D warnings -A dead_code` — clean.
- `cargo clippy --all-targets -- -A dead_code -W clippy::pedantic` — one
  finding, `doc_markdown` at `src/db.rs:1`, pre-existing and outside scope.
  Claim confirmed.
- `cargo fmt --check` — clean.

## Algorithmic properties independently probed (all correct)

- **Sort key priority** (`src/stint.rs:154`): `sort_by_key(|q| (q.at_utc,
  q.kind, q.id))`. Tuple `Ord` is lexicographic, `DateTime<Utc>: Ord` is
  chronological, and the `PunchKind` derive puts `Start` (declared first)
  below `End`. So the effective key is genuinely `(at_utc, kind, id)` in that
  priority — not transposed. Matches the **current** SPEC.md §4.3 step 1 text
  ("ties … broken first by kind (`start` before `end`), then by `id`")
  exactly. Stable sort as the plan requires.
- **LIFO beyond the worked examples**: probed with 400 punches (200
  alternating pairs) fed in a deterministically shuffled order — 200 completed
  stints, correct 5-minute duration on every one, nothing open, nothing
  orphaned. Probed with 100 nested starts followed by 100 ends — innermost
  start pairs with the earliest end, outermost with the last, `completed`
  emitted strictly in end-instant order. No off-by-one, no FIFO leak.
- **`now` injection**: no `Utc::now()` anywhere in the file (grep returns only
  two test *function names* containing "now"). `now` is a plain positional
  `DateTime<Utc>`, per contract 6.
- **`now` earlier than an open start**: `(now - start.at_utc).num_minutes()
  .max(0)` (`src/stint.rs:183`) — no panic, no negative value. chrono's
  `DateTime - DateTime` cannot overflow within `DateTime<Utc>`'s ±262k-year
  range, so no hidden panic path either.
- **Empty slice vs. the precondition assert** (`src/stint.rs:142`): the
  `punches[0]` index sits inside the closure passed to `Iterator::all`, which
  never invokes it for an empty iterator. Safe, and T9 covers it.
- **Ordering guarantees hold structurally**, not just in the fixtures:
  `completed` is ascending by end instant (pushed as each `End` is scanned),
  `open` is ascending by start instant (a subsequence of push order),
  `orphaned_ends` ascending by instant (construction order).

## Named §4.3 edge cases — all five correctly handled

| §4.3 case | verdict |
|---|---|
| One unmatched trailing `start` | correct — `open.len()==1`, `has_anomaly()` **false**, `is_ongoing()` true. The two signals are kept independent as §7.1/§7.3 require. |
| 2+ unmatched trailing `start`s (E7) | correct — one `OpenStint` each, each against the same `now`; `has_anomaly()` true; exactly **one** `MultipleOpenStints { count }`, not one per extra start. |
| `end` on an empty stack (E8) | correct — one `OrphanedEnd` per orphan, never coalesced, nothing added to `completed`, so zero contribution to any total. `orphaned_end_before_a_clean_pair` correctly shows the orphan does not consume a later start. |
| Same-instant `start`/`end` (E14) | correct in **both** entry orders — a legal zero-length stint, `has_anomaly()` false. `same_instant_pair_still_pairs_when_end_entered_first` is the load-bearing test and it genuinely exercises the kind-before-id tiebreak. |
| Nested entry | correct — nearest-match, `10:00` pairs with `11:00`, asserted explicitly rather than only via the span vector. Double-counting is spec-blessed (§4.3 + §2.4), not a bug. |

## Contract 8 — `Punch`/`PunchKind` stand-in vs. Milestone 4's owned shape

Compared `src/stint.rs:25-40` field-for-field against
`plans/milestone-4-punch-note-storage.md` §2.1/§2.2:

- `Punch { id: i64, at_utc: DateTime<Utc>, date: NaiveDate, kind: PunchKind }`
  — identical field names, identical types, identical order, all `pub`.
- `Punch` derives `Debug, Clone, Copy, PartialEq, Eq` — identical to M4's.
- `PunchKind { Start, End }` with `Start` declared first, deriving
  `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord` — `Copy + Ord` with
  `Start < End`, as required, and the declaration order is what makes the
  tiebreak work.
- Only divergence: M4's `PunchKind` additionally derives `Hash`, and adds
  `as_str`/`from_str` inherent methods. Both are purely **additive**, and
  nothing in `stint.rs` depends on their absence, so the swap-in is still a
  drop-in. Not a finding — recorded so nobody "fixes" the stand-in in the
  wrong direction.

Contract 2's result shape matches the plan §2 declaration with no renames:
`has_anomaly` is a private field with a single definition of the rule
(`src/stint.rs:187`), `DayStints` exposes no constructor, and
`has_anomaly_matches_anomalies_nonempty` pins the invariant the stored bool
depends on.

---

## Findings

### 1. `src/stint.rs:154` — the unconditional kind-before-id tiebreak silently loses worked minutes on a same-instant stop/restart boundary — **significant** (spec-inherited, not an implementation deviation)

SPEC.md §4.3 step 1 applies `start`-before-`end` at *every* identical instant,
and the implementation faithfully does so. The spec's own justification for the
rule only ever considers an **isolated two-punch pair** (E14). It breaks down
for the three-punch case where a same-instant `end`/`start` is a *boundary
between two stints* rather than a pair — which is an entirely ordinary thing to
type at minute granularity (`mlm stop 09:00` then `mlm start 09:00`).

Probed against this exact code (throwaway clone, punches given in natural entry
order):

- Input `Start 08:00, End 09:00, Start 09:00` (second stint still open).
  Sorted key puts `Start 09:00` *before* `End 09:00`, so the scan pushes
  08:00, pushes 09:00, then pops the **09:00** start into a zero-length stint —
  leaving the 08:00 start open. Result:
  `completed_minutes() == 0`, `open == [(08:00, 120 min elapsed)]`,
  `has_anomaly() == false`.
  The 60 genuinely worked minutes of 08:00–09:00 **vanish from the day total**,
  the open stint's live figure is measured from the wrong instant (120 instead
  of 60), and nothing flags it.
- Input `Start 08:00, End 09:00, Start 09:00, End 10:00` (both closed).
  Result: `completed == [zero-length @09:00, 08:00–10:00 (120)]`,
  `completed_minutes() == 120`. The total is coincidentally right, but the
  stint list Milestone 10 will render is wrong — one bogus zero-length stint
  plus one overlapping 2-hour stint, instead of two adjacent 1-hour stints.

Why it matters: the first variant is a silent, unflagged loss of real worked
time that propagates straight into §2.4 day/week/carry totals via
`completed_minutes()` — the exact class of error `has_anomaly()` exists to
prevent, and it produces no anomaly at all.

**Disposition**: this is a SPEC.md §4.3 defect, not a Milestone 5 coding error.
The code matches the current spec text precisely, so it should **not** be
patched here unilaterally (that would reintroduce the E14 regression the
tiebreak was added to fix). Escalate to the spec: the tiebreak likely needs to
be conditional (e.g. prefer `End` first when the stack is non-empty and the
pending `start` is strictly earlier, i.e. only hoist `Start` when there is no
unmatched start to close), or a new overlap/boundary anomaly is needed. This
should be settled **before** Milestones 6/10/11 start reporting totals built on
`completed_minutes()`.

### 2. `plans/milestone-5-stint-pairing.md` §3.1 (and the report's echo of it) — the tiebreak's blast radius is understated — **minor**

§3.1 asserts the kind tiebreak "changes nothing for any pair of punches at
distinct instants, and nothing for two punches of the same kind at the same
instant." Both clauses are individually true and together imply the change is
confined to an isolated same-instant pair — which finding 1 falsifies. The
three-punch boundary case is neither of the two excluded shapes and is affected
materially. The rationale as written is what let the case go unconsidered;
whoever owns the spec fix should correct this paragraph rather than leave a
documented "no collateral effect" claim standing.

### 3. `src/stint.rs:338-362` — no test covers a same-instant `end`/`start` boundary — **significant** (coverage gap)

The test suite covers same-instant pairs in both entry orders (T7/T7b), but
never a same-instant `end` and `start` in a sequence where the stack is
non-empty. That is precisely the shape in finding 1 and the only shape where
the tiebreak's effect is non-obvious. A regression test pinning whatever
behavior the spec decision in finding 1 lands on is required; today the
module's most fragile rule has its most damaging case untested.

### 4. `src/stint.rs:14-19` — the `TODO(integration)` overstates "no other change in this module" — **minor**

`use crate::storage::{Punch, PunchKind};` is a *private* import, so after the
swap `crate::stint::Punch` and `crate::stint::PunchKind` — valid paths today,
and explicitly listed in plan §5 as part of this module's public surface — stop
resolving. The module body compiles unchanged, but the module's exported
surface silently shrinks. Harmless right now (no consumers exist), but it makes
the "one-line swap" claim false the moment Milestones 9/10/11 are written
against `stint::Punch`. Either make it `pub use crate::storage::{Punch,
PunchKind};` or correct plan §5 to state that downstream imports `Punch` from
`storage`, not `stint`.

### 5. `src/stint.rs:10` — module-wide `#![allow(dead_code)]` — **minor** (self-flagged, but the scope is wider than needed)

The allow covers the entire module for the whole life of the file, so any
genuinely dead helper added to `stint.rs` between now and Milestone 10/11
landing will be invisible. Nothing is currently dead beyond the
consumers-don't-exist-yet surface (verified: every type, field and method is
exercised by the tests). Preferably narrow it to the item level, and in any
case it needs a tracked removal at the first real call site.

### 6. `src/stint.rs:62-67` + `108-121` — `OrphanedEnd` is a zero-value newtype and orphan data is stored twice — **minor / nit**

`OrphanedEnd { punch: Punch }` carries no invariant and no extra field over
`Punch`, so `Vec<OrphanedEnd>` buys nothing that `Vec<Punch>` wouldn't, and
`anomalies()` then re-wraps the same punch a second time into
`Anomaly::OrphanedEnd`. Plan §2 mandates this shape, so it is conformant rather
than wrong — noting it only because a wave-3 reviewer will ask, and because
collapsing it is a pure-mechanical simplification if Milestone 9 never needs
the intermediate type.

### 7. `Anomaly::MultipleOpenStints { count }` — redundant with `open.len()` — **nit** (already self-flagged)

Agreed with the report's own assessment: keep it if Milestone 9 renders from
the `Anomaly` value alone, drop it if Milestone 9 takes `&DayStints`. No action
in this milestone.

### 8. Forward-looking note, not a defect — §7.2's "stale open stint is silent"

`is_ongoing()` is date-blind by construction (`!open.is_empty()`), so
`cross_midnight_halves_are_two_separate_anomalies` correctly has day 1 report
ongoing. SPEC.md §7.2 requires `week`'s table to suppress `(ongoing)` unless the
open stint's date equals today. `open[0].start.date` is available, so the data
Milestone 11 needs is present — but Milestone 11 must do that comparison
itself, and nothing in this milestone's output signals it. Recording it so it
is not rediscovered as a bug in wave 4.

---

## Verdict

**APPROVE WITH NITS** — the implementation is faithful to the current SPEC.md
§4.3 (including its corrected kind-before-id tiebreak), correctly handles all
five named edge cases, injects `now` cleanly, holds up under long alternating
and deeply nested sequences well beyond the worked examples, matches contracts
2 and 8 field-for-field, and every verification claim in the report reproduces
exactly — including the out-of-scope `time.rs` clippy failure, which is
confirmed pre-existing on the parent commit. Finding 1 is a real and reachable
loss of worked minutes, but it is a spec defect this code faithfully inherits
rather than a Milestone 5 mistake, so it must be escalated to SPEC.md (with the
finding-3 regression test) before wave 2/3 builds totals on top of it — not
fixed inside this merge.
