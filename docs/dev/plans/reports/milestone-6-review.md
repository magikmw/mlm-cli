# Milestone 6 — independent adversarial review

Reviewer: fresh read of `src/week.rs` on branch `milestone-6` (commit
`25adbfb`), against SPEC.md §1.1/§1.3/§2.3/§2.4/§5, NOTES.md decisions
3/37/38, PLAN.md contracts 3/6/9/10/11, and
`plans/milestone-6-week-accounting.md`. Base for comparison: `638f2d6`.

No code was modified by this review.

---

## Verification performed (not taken on faith)

**SPEC §5's table, recomputed by hand** from the three formulas
(`fulfillment = worked + carry_in`, `owed = target - fulfillment`,
`carry_out = fulfillment - target`, first tracked week `carry_in = 0`),
with the `2026-03` override of 2000:

| week | target | carry_in | worked | fulfillment | owed | carry_out |
|---|---|---|---|---|---|---|
| `2026-01` | 2400 | 0 | 2200 | 2200 | 200 | -200 |
| `2026-02` | 2400 | -200 | 2500 | 2300 | 100 | -100 |
| `2026-03` | 2000 | -100 | 1950 | 1850 | 150 | -150 |
| `2026-04` | 2400 | -150 | 2600 | 2450 | -50 | 50 |

This matches SPEC.md §5 cell-for-cell and matches the literal
`expected` vec in `reproduces_spec_section_5_table_exactly`
(`src/week.rs:391-431`) cell-for-cell, all seven fields on all four
rows. The override lands on `2026-03` only; `2026-04` returns to 2400.
The negative-`owed` / positive-`carry_out` row is present and unclamped.
**Correct.**

**Idle gap weeks (F8/F8b, §2.4).** The fold at `src/week.rs:229-254` has
no `continue`, no `filter`, and iterates `current = current.next()` over
*weeks*, never over `worked.keys()` or `data_weeks`. A gap week goes
through the identical arithmetic with `worked = 0` and therefore accrues
`-target`. Pinned three ways: whole-row assertion on the gap row
(`:543-567`), full-series equality between an absent week and an
explicit zero+data-week week (`:569-583`), and a two-gap chain reaching
`carry_in == -4800` (`:585-594`). **Correct, and the F8b equivalence is
asserted as series equality rather than inferred.**

**Contract 9 API match.** `WeekId` (`src/week.rs:22-70`) has private
fields, a checked `new`, and exactly `iso_year()` / `week()` / `start()`
/ `from_date()` / `next()` — not `year()` / `iso_week()` / `monday()`.
Field order is `iso_year` then `week` with derived `Ord`, so
`BTreeSet::iter().next()` in `earliest_data_week` (`:179`) is genuinely
chronological, asserted at `:367-371`. `next()` is
`from_date(start() + Duration::days(7))` (`:67-69`) — date-based, never
`week + 1`. **Correct.**

**53-week boundary, traced by hand.** 2026-01-01 is a Thursday, so ISO
2026 has 53 weeks; the test reads this off chrono (Dec 28) rather than
assuming it (`:294-299`, `:328-332`). `2026-53`'s Monday is
2026-12-28; `+7 days` = 2027-01-04, whose `iso_week()` is `2027-01`.
A naive `week + 1` with a 52 wrap would have produced `2027-01` from
`2026-52`, skipping `2026-53` entirely. Covered at unit level
(`:334-343`), at construction level (`2025-53` rejected, `:345-350`),
at `from_date` level (2027-01-01 → `2026-53`, `:360-365`), and at
walk level (`2026-52 → 2027-02` yields exactly 4 rows with an unbroken
carry chain, `:671-682`). **Correct and well covered.**

**No `is_current_week`, no orphaned `today` plumbing.** PLAN.md:109-117
(contract 3, current wording) reads: "target, carry_in, worked,
fulfillment, owed, carry_out, all signed integer minutes, **plus the
week id itself** ... Deliberately **no** 'is this the current week'
boolean here — that comparison is Milestone 9's sole job ... Milestone 6
has no other reason to know 'now'." `WeekAccounting`
(`src/week.rs:113-132`) has exactly the seven contract fields and no
boolean. `grep` over `src/week.rs` for `is_current_week`, `today`,
`Local`, `now()` returns one hit only: the word "today" inside a prose
comment at `:278`. Both `week_series` and `week_accounting` take
`(through|week, data, targets)` with no date parameter. **The
implementer's call was right, and the removal is complete — no dead
parameter, no unused import, no vestigial field.**

**No clamping.** `grep` for `clamp`, `.abs()`, `.max(`, `saturating`
finds no call sites in code (only the prose comments asserting their
absence). The single `.min(` is §5.1's start selection at `:221`. The
single `unwrap_or(0)` is `worked_minutes`' sparse-map default (`:183`),
which is the correct semantic ("absent week worked zero") and is
nowhere near target resolution — `resolve_target` (`:195-199`) uses
`unwrap_or(DEFAULT_WEEK_TARGET_MINUTES)`, and the `Some(0)` vs `None`
divergence is asserted side by side at `:520-539`. **Correct.**

**Zero-target override (F7b) genuinely produces no deficit.** With
`target = 0` and `worked = 0`, `owed = 0 - 0 = 0` and `carry_out = 0`
— asserted at `:508-518`. With worked time, it goes uncapped ahead
(`owed == -600`, `:494-506`). **Correct.**

**E12 first-tracked-week.** `carry_in` is initialized to 0 before the
loop (`:226`) with no special-casing inside it, and the series for the
earliest data week has length exactly 1 (`:447-467`), proving the walk
does not invent prior weeks. **Correct, no off-by-one.**

**Toolchain, run directly:**

- `cargo build` — succeeds (3 pre-existing `dead_code` warnings from
  `src/time.rs`, none from `src/week.rs`).
- `cargo test` — **28 passed, 0 failed, 0 ignored.** The report's
  "28/28" claim is accurate; all 28 are `week::tests::*`.
- `cargo clippy --all-targets --message-format short` — **zero
  diagnostics mentioning `src/week.rs`.**
- `cargo clippy --all-targets -- -D warnings` — fails with exactly three
  errors: `parse_hm`, `between`, `format_duration` never used, all in
  `src/time.rs`. **Independently confirmed pre-existing**: a clean clone
  checked out at base commit `638f2d6` produces the identical three
  errors. This branch introduces no new clippy diagnostic. The report's
  characterization is honest.

**Diff scope**, `git diff --stat 638f2d6 HEAD`: `src/main.rs` +1 line
(the `mod week;` declaration), new `src/week.rs`, new report. Nothing
else touched — `cli.rs`, `db.rs`, `time.rs` untouched as the plan's hard
boundary required.

---

## Findings

1. **`plans/milestone-6-week-accounting.md:186-194, 204-229, 340-346,
   506-523, 634-645` — the plan file is stale on `is_current_week` /
   `today` and now contradicts PLAN.md contract 3.** §2.3 still
   specifies a `today: NaiveDate` parameter, §3 still lists the
   `is_current_week` field in the result struct, §5.2's pseudocode still
   sets it in the fold, §7.7 is four test cases for it, and §8.3
   rationalizes it. PLAN.md:109-117 explicitly forbids it. *Why it
   matters*: the next reader of this plan will file a correct
   implementation as incomplete, or worse, "fix" it by adding the field
   back and creating exactly the two-milestones-disagree-about-"current"
   hazard contract 3 was written to prevent. The implementation is right
   and the plan is wrong; the plan should be corrected. Severity:
   **minor** (documentation, zero code impact — but correct it before
   Milestone 9 starts).

2. **`plans/milestone-6-week-accounting.md:491` — §7.6a's "the
   accumulated deficit of the 19 preceding weeks" is arithmetically
   wrong; 18 is right.** With `2026-01` worked 2400 against a 2400
   target, week 01's `carry_out` is 0, so `2026-20`'s `carry_in` is the
   sum of `carry_out` over weeks 02..=19 — 18 weeks at `-2400` =
   `-43200`. There are indeed 19 *preceding* weeks (01..=19), but only
   18 of them contribute a deficit. The implementation's test
   (`src/week.rs:609`) asserts `-2400 * 18` with the comment "2026-01 is
   on target; weeks 02..=19 are 18 empty weeks" — **the test suite
   encodes the correct number**, and §5's own formulas agree with the
   test, not the plan prose. Severity: **minor** (plan prose only; fix
   the wording so a future reader doesn't "correct" a correct test).

3. **`src/week.rs:32-35` vs `/home/magikmw/projects/mlm-wt-milestone-2/src/date.rs:181`
   — the stand-in's constructor signature will not swap in cleanly.**
   Milestone 2 (already landed on its own branch) ships
   `WeekId::new(iso_year: i32, week: u32, input: &str) -> Result<Self,
   DateWeekError>` with `week: u8` internally; the stand-in is
   `new(iso_year: i32, week: u32) -> Option<Self>`. *Why it matters*:
   the report's item 2 frames integration as "delete the stand-in and
   re-point the `use`", but the third parameter and `Result`-vs-`Option`
   mean the `wk()` test helper (`:289-291`) and every `WeekId::new`
   assertion (`:345-350`) need editing too. The *consumed* surface is
   what matters and it matches exactly — `iso_year()`, `week()`,
   `start()`, `from_date()`, `next()`, chronological derived `Ord` with
   `iso_year` first — so no accounting logic changes. Worth recording as
   known, bounded integration debt rather than a surprise at wave 3.
   Severity: **minor**.

4. **`/home/magikmw/projects/mlm-wt-milestone-2/src/date.rs:274-286` —
   Milestone 2's `next()` is `week + 1`, but correctly guarded.** Noting
   this so nobody flags it at integration as the bug the plan warns
   about: it is `if week < iso_weeks_in_year(iso_year) { week + 1 } else
   { year + 1, week 1 }`, i.e. it consults the real per-year week count
   rather than a hardcoded 52, so it is behaviorally identical to the
   stand-in's date-based version across 53-week years. The stand-in's
   date-based form remains the safer formulation. **Keep this module's
   53-week `next()` tests after the swap** (the plan and the report both
   already say so) — they are the only thing that would catch a future
   regression to a hardcoded 52 in Milestone 2. Severity: **minor**
   (informational; no defect).

5. **`src/week.rs:260-268` — `week_accounting` materializes the entire
   carry chain to keep one row, and the walk has no upper bound.**
   Milestone 2 accepts week ids up to year 9999 (`MAX_YEAR = 9999`,
   `date.rs:156`), so once wave 3 wires a parsed `WEEK_ID` into this
   function, `mlm week 9999-01` against data in 2026 walks ~415,000
   iterations and allocates a `Vec` of ~415,000 `WeekAccounting` (56
   bytes each, ~23 MB) only to `pop()` one element and drop the rest.
   Not a crash and not a hang — sub-second integer arithmetic, and §2.4
   explicitly accepts the O(n-weeks) cost — but the *allocation* is
   gratuitous and reachable from user input. A `fold`-style private
   helper that yields the last row without collecting (with
   `week_series` keeping the collecting path for Milestone 11's
   debugging use) removes it in a few lines. Severity: **minor**.

6. **`src/week.rs:156-160` — `WeekLedger::with_worked` overwrites rather
   than accumulates, which is a live trap for the planned wave-3
   adapter.** `self.worked.insert(week, minutes)` replaces any existing
   value. Plan §8.1 recommends a `WeekLedger::from_daily_totals(impl
   IntoIterator<Item = (NaiveDate, i64)>)` constructor built on
   Milestone 4's `punches_in_range` (contract 10) — i.e. feeding
   *per-date* rows in, five to seven of which map to the same ISO week.
   Built naively on `with_worked`, that silently keeps only the last
   day of each week and every carry figure downstream is wrong, with no
   test in this module able to catch it (same blast radius as the
   open-stint leak the trait doc rightly warns about at `:76-83`). The
   builder is correct as specified for its current use; what's missing is
   either a doc line saying "replaces, does not add — pre-aggregate per
   week before calling" or a companion `add_worked` that sums. Severity:
   **minor** now, **would be a blocker** if a `from_daily_totals` is
   layered on it without either.

7. **`src/week.rs:11` — module-wide `#![allow(dead_code)]` will keep
   suppressing after it stops being needed.** The `TODO(integration)`
   comment above it is clear and the justification (nothing in the
   binary references the module until Milestones 9/10/11) is legitimate,
   especially given the `-D warnings` situation. But it is a blanket
   module-scope allow, so once wave 3 consumes *part* of the module,
   genuinely unreachable items (e.g. `WeekLedger` itself, if the real
   adapter supersedes it) stay invisible. Removing it should be on wave
   3's checklist, not just on the honor system. Severity: **minor**.

8. **`src/week.rs` test module — no test pins plan §8.1's open reading
   that a `week_targets`-only week does not anchor the walk.** The
   implementation guarantees it structurally (`targets` is a separate
   `BTreeMap` from `data_weeks`, and `earliest_data_week` reads only
   `data_weeks`), so behavior is correct today. But §8.1 flags this as a
   genuine reading choice pending confirmation, and a three-line test
   (`WeekLedger::new().with_target(wk(2026,1), 2000).with_worked(wk(2026,10),
   2400)` → series length 1, starting at `2026-10`) would lock the
   decision in place and make a future flip a visible, deliberate change
   rather than a silent one. Severity: **minor** (coverage gap on a
   decision, not on a formula).

9. **`src/week.rs:293-299` — the `iso_weeks_in_year` test helper
   duplicates Milestone 2's public `iso_weeks_in_year`
   (`date.rs:162`).** Correct and well-motivated here (reading the year
   length off chrono rather than trusting the plan's claim is exactly
   right, and this module cannot import Milestone 2 yet). Fold it into
   the real one at integration. Severity: **minor / nit**.

### Checked and found clean — no finding filed

Sign conventions in all three formulas (each computed independently from
its own §5 formula at `:235-237`, so a sign error in one cannot be
masked by deriving it from another); `carry_out == -owed` held as an
invariant rather than an optimization; carry threaded directly from
`carry_out` to the next iteration's `carry_in` with no separate
accumulator (`:252`); loop termination in all three §5.1 cases including
the `min` branch that makes a pre-data `through` terminate; the E12
first-week zero; off-by-one at both ends of the walk (the length-20 and
length-1 and length-3 and length-4 assertions pin both ends); `i64`
throughout with no overflow reachable at any plausible scale; the
`div_euclid(5)` floor in `daily_target_minutes` with its
don't-simplify-this comment (`:276-282`); `daily_target_minutes`'
signature genuinely taking the target and nothing else, per §2.4;
`Default`/`Clone` derives on `WeekLedger` and `Copy`/`Eq` on
`WeekAccounting` (the latter load-bearing for whole-row assertions); the
two-trait split keeping the input abstraction free of any live-DB
assumption — nothing in this module knows SQLite, `chrono::Local`, or a
punch exists, so it does compile and pass its whole suite with
Milestones 4 and 5 absent from this worktree, as required. Idiomatic
Rust throughout: builder-by-value, `&impl Trait` parameters, doc
comments that cite the spec clause each rule comes from, and no `unsafe`,
no `unwrap` outside tests (the two `expect`s at `:50` and `:267` are on
genuine construction-time invariants and are documented as such).

---

## Verdict

**APPROVE WITH NITS.** The arithmetic reproduces SPEC §5 exactly, the
week-walk visits every ISO week with no empty-week shortcut anywhere,
`next()` is correctly date-based and covered across both a 53-week and a
52-week year boundary, nothing is clamped, and the corrected contract 3
is honored with no vestigial `today`/`is_current_week` plumbing left
behind. `cargo build` / `cargo test` (28/28) / `cargo clippy` were run
directly and the report's claims — including that the `-D warnings`
failure is pre-existing in `src/time.rs` — check out against base commit
`638f2d6`. The nine findings are all minor: two are stale-plan
corrections the implementer had already flagged and got right (the
dropped `is_current_week`, and 18-not-19 deficit weeks), and the rest are
integration-time follow-ups — the most substantive being finding 6,
`with_worked`'s overwrite semantics, which is harmless today but becomes
a silent-wrong-carry blocker the moment a per-date `from_daily_totals`
is layered on it.
