# Milestone 9 — Independent adversarial review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Reviewer: independent pass, not the implementer. Scope: `src/render.rs`
against `plans/milestone-9-shared-rendering.md`, the implementer's own
report, SPEC.md §7.1-7.3, NOTES.md decisions 18/25/26/32, and PLAN.md
interface contracts 2/3/5/6, plus the already-merged `src/week.rs`,
`src/stint.rs`, `src/date.rs` this milestone consumes.

Verification performed directly (not taken on faith):
- `cargo build`: clean.
- `cargo test`: **252 passed, 0 failed** — matches the report's claim exactly.
- `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
- Traced `format_minutes`, `WeekId::iso_year`/`week`/`Display`,
  `format_weekday_full`, `DayStints`/`Anomaly` in `stint.rs`, and
  `WeekAccounting` in `week.rs` by reading the real merged source rather
  than trusting the plan's guessed shapes.
- Hand-computed every worked-example string in `render.rs`'s test module
  against SPEC.md §7.1/§7.2/§7.3's literal text (format_minutes(645) =
  "10h 45m", format_minutes(190) = "03h 10m", format_minutes(100) =
  "01h 40m", magnitude of -50 = "00h 50m", etc.) — all byte-for-byte
  matches, including the colon in `Total still owed:`/`Total ahead:`
  and the two-leading-space `"  [!]"` row marker.

## Findings

1. **`src/render.rs:76-100` — `status_week_line`'s three `i64` minute
   parameters (`owed_minutes`, `fulfillment_minutes`, `target_minutes`)
   are same-typed and adjacent, so a transposed call compiles silently.**
   E.g. a Milestone 10 call site that accidentally writes
   `status_week_line(week, fulfillment, owed, target, today)` type-checks
   fine and produces a plausible-looking but wrong parenthetical
   (`(fulfillment <owed> / target <target>)`) or a wrong headline number
   — exactly the "silent wrong output, no crash" failure mode this review
   was asked to hunt for, and exactly the milestone this signature is
   handed to verbatim. The plan's own §1.5 anticipated this by proposing
   a `status_week_line_for(acct: &WeekAccounting, today)` adapter that
   takes named struct fields instead of positional minutes — but that
   adapter was explicitly deferred to integration time and does not
   exist yet, so today the only callable surface is the
   easy-to-misuse one. Not a bug in the code as written (all of T1-T33
   call it correctly), but a real landmine for Milestone 10's first call
   site. Severity: **significant** (signature-hygiene risk for a
   verbatim-call function, not a present defect).

2. **`src/render.rs:70` — `format_minutes(-owed_minutes)` negates before
   calling the formatter, rather than delegating the sign-flip to a
   checked/wrapping path.** `format_minutes` itself is overflow-safe
   (`unsigned_abs`), but the call site's `-owed_minutes` is plain `i64`
   negation, which panics in a debug build (or silently wraps in
   release) if `owed_minutes == i64::MIN`. Given `owed_minutes` is always
   a small week-scale minute count in practice (thousands, never
   anywhere near `i64::MIN`), this is not reachable via any real
   accounting path — `WeekAccounting`'s walk cannot produce such a value.
   Flagging only because Milestone 9 is explicitly the "no imprecision
   survives to two downstream milestones" module. Severity: **minor**.

3. **`src/stint.rs:63-95` already exposes its own independent
   `has_anomaly()` bool and `anomalies() -> Vec<Anomaly>` on
   `DayStints`, computed by the identical `open.len() > 1 ||
   !orphaned_ends.is_empty()` rule that `render::Anomalies::has_any()`
   uses.** Today the two formulas agree (both were read directly and
   compared), but this means "what counts as an anomaly" is currently
   defined in **two** places in the merged code, not one — the exact
   divergence risk PLAN.md contract 2 and this milestone's whole
   existence are supposed to close. The plan explicitly deferred the
   `From<&StintClassification> for Anomalies` adapter to integration
   time (§2.4, correctly out of this milestone's scope per R9), but as
   merged, nothing stops a Milestone 10 or 11 implementer from calling
   `DayStints::has_anomaly()`/`DayStints::anomalies()` directly instead
   of converting to `render::Anomalies` first — both are public, both
   look equally authoritative, and only one of them (`render::Anomalies`)
   is guaranteed by test (T29-T31) to keep `has_any()`/`detail_lines()`/
   `row_marker()` in lockstep. If a future edit changes one formula and
   not the other, they will silently disagree and nothing in the test
   suite would catch it, since the two modules currently have no tests
   binding them together. Severity: **significant** — worth a one-line
   note in Milestone 10/11's briefs (or a `#[deprecated]`/doc-comment
   steer on `DayStints::has_anomaly`/`anomalies()`) directing callers to
   go through `render::Anomalies` exclusively, since this milestone's
   own report says "no adapter added — left for 10/11's worktrees",
   which is the right call for scope but leaves the seam unguarded.

4. **F11 structural guarantee verified, not just read.** `week_headline`
   and `status_week_line` take `today: NaiveDate` and no "date being
   displayed" parameter anywhere in their signatures (confirmed by
   reading the actual function signatures, not just the doc comments) —
   there is no argument path by which a caller could make the weekday
   in `"<owed> left by end of <weekday>"` come from anything but `today`.
   T4/T5 exercise exactly the scenario asked for (rendering Monday's
   status headline on a Thursday) and the weekday in the output is
   Thursday. This is a real, structural (not just documented) guarantee.
   No finding — noted because the review brief asked for explicit
   confirmation.

5. **Anomaly rendering split verified against the same underlying
   data.** `Anomalies::has_any()`, `detail_lines()`, and `row_marker()`
   all read only `open_stint_count`/`orphaned_end_times`, and
   `has_any()` is the sole predicate the other two are built from
   (`row_marker` delegates to `has_any()` directly; `detail_lines`'s
   emptiness is asserted equal to `has_any()` by T29 and the 20-case
   sweep in T31). No divergence path exists within this module. No
   finding.

6. **E8 (multiple orphaned ends never coalesce) traced through an
   actual 2+-orphan case.** T18 (3 orphans, 3 distinct lines in punch
   order) and T19 (2 orphans sharing an identical timestamp, still 2
   distinct identical lines, not deduplicated via e.g. a `HashSet`) both
   exercise this directly against the real `Vec`-based implementation,
   which has no dedup/sort step. No finding.

7. **`src/render.rs:44-51` (`week_framing`) correctly compares
   `(iso_year, week)` tuples, never a calendar year or a Mon-Sun range
   check**, and the three year-boundary tests (T13: Dec 29 2025 is
   ISO-current for 2026 week 1; T14: Jan 1 2027 is ISO-current for 2026's
   week 53; T15: same calendar year, different ISO week, is Closed) all
   pass against the real `WeekId`/`chrono` `iso_week()` machinery, not a
   stub. No finding.

8. **Interface contract 3 confirmed structurally.** `WeekAccounting`
   (`src/week.rs:59-77`) carries no `is_current_week` field — read
   directly, confirmed absent — and `week_framing`/`week_headline`
   compute currency solely from `(WeekId, NaiveDate)`, never from a
   caller-supplied boolean. Matches PLAN.md contract 3 and the report's
   claim exactly. No finding.

9. **`WeekId` accessor names (`iso_year()`, `week()`) and `format_weekday_full`
   were verified against the real `src/date.rs`, not the plan's guessed
   `iso_week()` name.** The implementation correctly uses the real names;
   the report's stated deviation from the plan's guess is accurate. No
   finding.

10. **Negative-zero test (T12) is dead weight, not a real regression
    guard.** `format_minutes(-0i64)` — there is no negative zero for a
    signed integer in Rust; `-0i64 == 0i64` is true by construction, so
    this assertion can never meaningfully fail regardless of
    `format_minutes`'s implementation (it would only catch a bug that
    doesn't `unsigned_abs()`-normalize inside `format_minutes` itself,
    which is Milestone 1's code, already correct and out of scope here).
    Harmless, but slightly misleading as a "guard" comment. Severity:
    **minor** (test-quality nit only).

## Verdict

**APPROVE WITH NITS** — the implementation is a precise, byte-for-byte
faithful realization of the plan and SPEC.md's worked examples; build,
full test suite (252/252), and clippy are all independently confirmed
clean; the F11/contract-3/E8 guarantees the milestone exists to
provide were traced structurally, not taken on faith. The two
significant items (positional same-typed minute parameters on
`status_week_line`, and the unguarded seam between `DayStints`'s own
anomaly predicate and `render::Anomalies`) are real risks for
Milestones 10/11 but are follow-up/integration-time concerns, not
defects in this milestone's own code or tests — they should be called
out explicitly in Milestone 10 and 11's briefs before those worktrees
open.
