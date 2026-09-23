# Adversarial review — `docs/dev/specs/2026-09-23-status-wording-fixes.md`

Reviewed against `src/render.rs`, `src/status.rs`, `src/week.rs`, `src/week_view.rs`,
`src/commands.rs`, `docs/dev/SPEC.md`, `README.md`, `docs/dev/NOTES.md` (entry 61),
`Cargo.toml`, `.github/workflows/ci.yml`, and the vendored `chrono` 0.4.45 source.

## Findings

### 1. [High] Internal inconsistency + logic bug — `is_last_workday` fires on Saturday and Sunday, not just "the one day"

- **File:line (claim)**: spec §3, lines 118-119: `is_last_workday = weekday_number.min(5) == 5 (equivalently weekday_number >= 5)`.
- **File:line (evidence)**: `src/status.rs:463-464`:
  ```rust
  let weekday_number = i64::from(today.weekday().number_from_monday());
  let required_minutes = daily_target_minutes * weekday_number.min(5);
  ```
  `number_from_monday()` yields Monday=1 … Sunday=7. So `weekday_number >= 5` is true for
  Friday (5), Saturday (6), **and** Sunday (7) — three days, not one.
- The daily-target/EOD block that would use this flag is gated only by `is_today`
  (`src/status.rs:461: if is_today { ... }`), with no work-day restriction, so a `status`
  query on a Saturday or Sunday already computes and renders a `daily_target` hint today,
  and after this fix would render `required today` on both weekend days too.
- This directly contradicts the spec's own repeated framing: "the one day this wording
  differs" (line 126), "on that one day" (lines 133, 139-140), the section title itself
  ("last-workday … wording"), and the variable's own name `is_last_workday` — Saturday and
  Sunday are not workdays, so a flag with that name being `true` on them is a naming/logic
  contradiction, not an accepted edge case (nothing in §3 acknowledges this).
- Consequence for §3's layout analysis: it's still probably fine (the "no padding" argument
  doesn't depend on how many days trigger the shorter wording), but the *premise* — that this
  changes wording on exactly one calendar day per week — is false as specified, and no test
  is proposed that would catch it (see Finding 4).

### 2. [High] Factual error about the codebase — Fix D's stated problem is already fixed in SPEC.md

- **File:line (claim)**: spec §1 bullet D, lines 38-41: "README's and SPEC.md's `status`/`week`
  output examples only ever show a *current*-period week. A user who queries a closed
  date/week for the first time hits the different sentence shape from **A** unexplained.
  Fixed with a worked closed-period example next to the existing current-period ones, in
  both README and SPEC.md." Same claim repeated in §5 (lines 168-172): SPEC.md "gain[s] one
  worked example each for a closed (past) date/week ... right next to the existing
  current-period example[s]."
- **File:line (evidence)**: SPEC.md already has both closed-period examples:
  - `docs/dev/SPEC.md:707-716` — a full `mlm status 2026-01-05` worked example, explicitly
    introduced as "a past date, different week ... the week line uses the plain closed-week
    form since `2026-01-05`'s week has already ended", rendering
    `Week 2026-02:  Total still owed: 01h 40m`.
  - `docs/dev/SPEC.md:744-766` — a full `Week 2026-06` `mlm week` worked example, introduced
    as "A **past or future** week has no 'today' to frame a deadline against", rendering
    `Total still owed: 03h 10m`.
- The premise is true for `README.md` (checked: both `status` examples at lines 195/215 and
  both `week` examples at lines 233/262 use the current week `2026-37` — no closed-period
  example exists there), but it is false for SPEC.md.
- Practical effect on Task 2: if the SPEC.md portion of Fix D is executed as literally
  described — "gains one worked example" placed "next to" the existing one — an implementer
  following the doc could add a second, redundant closed-period example to §7.1/§7.2 instead
  of doing the thing SPEC.md actually needs, which is an in-place string update to the two
  existing examples (`Total still owed` → `Total behind`, per Fix A). The task-split section
  (§6) doesn't disambiguate "add new" vs "update existing" for SPEC.md.

### 3. [Medium] Scope gap — `"Total still owed"` also appears in SPEC.md body prose outside §1.2a and the two worked examples, and isn't inventoried

- **File:line (claim)**: the "Interaction with the project spec" section names only one
  SPEC.md touch point for this rename: "SPEC.md §7.1/§7.2 (closed-period headline) —
  `"Total still owed"` → `"Total behind"`" (line 202-203). Fix A's own inventory method (§4,
  line 162: `grep -rn "Total still owed" src/`) is explicit and exhaustive for `src/`, but no
  equivalent grep-based inventory is given for the docs.
- **File:line (evidence)**: `grep -n "Total still owed" docs/dev/SPEC.md` returns 8 hits, not
  2: lines 83, 98 (§1.2a, correctly in scope and slated for deletion), 715, 752 (the two
  worked examples, covered by Finding 2), **and also** 647 ("matching decision 18's closed-week
  `"Total still owed"` / `"Total ahead"` split"), 672 ("the plain `Total still owed`/`Total
  ahead` form"), 679 ("past/future `Total still owed`/`Total ahead` form"), and 776 ("a closed
  or not-yet-started week gets the plain `Total still owed`/`Total ahead` form"). These four
  are normative prose describing the wording rule itself, not the deleted §1.2a bullets and
  not the two examples — nothing in Task 2's description (§6) or the interaction section
  calls them out, so a literal reading of the spec can leave SPEC.md self-contradictory after
  the changeset (examples/prose in some places calling it "Total behind", others still saying
  "Total still owed").

### 4. [Medium] Testing gap — no weekend-day test case is specified for Fix C, which is exactly the fixture that would surface Finding 1

- **File:line (claim)**: spec §3 describes only the last-workday/non-last-workday split and
  §6 says Task 1 "fixes A, B, C and their in-file `#[cfg(test)]` assertions" with no mention
  of a weekend fixture.
- **File:line (evidence)**: `src/status.rs`'s existing test fixture `today()` (line 549-551)
  is pinned to Thursday 2026-02-12; every existing daily-target/EOD test in the file
  (`t6a`, `t6b`, `t6c`, `t11`, `t12`, `t13`, etc.) uses this fixed Thursday. Nothing in the
  spec asks for a Saturday- or Sunday-`today` case, so implementing Fix C exactly as
  specified could pass every test in the file while still exhibiting Finding 1's bug on
  weekends.
- Separately (lower-stakes, not a spec defect, just a size note): every existing
  `EodState::At(t)` construction across `status.rs`'s tests (base_view, t1, t6a, t11, t12,
  t13, etc.) is a single-argument call that must become two-argument once Fix B lands; the
  spec doesn't enumerate these call sites the way §4 enumerates the `"Total still owed"`
  ones, though this is implicit in "fixes ... their in-file tests."

### 5. [High] Factual/API error — Fix B's suggested comparison uses a deprecated, non-type-matching `chrono` API that this repo's own CI gate rejects

- **File:line (claim)**: spec §2, lines 72-73: "Compare its `.date()` against `today` ...
  and carry the result into the new bool."
- **File:line (evidence)**:
  - `Cargo.toml:31` pins `chrono = "0.4.45"`.
  - In the vendored source
    (`~/.cargo/registry/.../chrono-0.4.45/src/datetime/mod.rs`, the `date()` method on
    `DateTime<Tz>`), the method carries
    `#[deprecated(since = "0.4.23", note = "Use \`date_naive()\` instead")]` and returns
    `Date<Tz>`, not `NaiveDate`.
  - `today` in `resolve()` is a plain `NaiveDate` (`src/status.rs:409:
    let today = now.date_naive();`), so `(now + ChronoDuration::minutes(gap_minutes)).date()`
    (a `Date<Local>`) does not have a usable `PartialEq` against `today: NaiveDate` — the
    literal comparison as worded won't type-check, only `.date_naive()` (returning
    `NaiveDate` directly) does.
  - `.github/workflows/ci.yml:50` runs `cargo clippy --all-targets --target ${{ matrix.target
    }} -- -D warnings`, so even if a workaround made `.date()` compile, the deprecation lint
    would fail CI outright.
  - The fix is one method-name swap (`.date_naive()`), but the spec's own text names the
    wrong method, and nothing flags it as pseudocode rather than the literal call to make.

## Undeclared user-visible gap

Checked each of the four fixes for a deliberate, correct choice whose visible consequence
isn't written down, distinct from the bugs/gaps above (which are wrong or missing, not
merely undisclosed):

- Fix A (rename) — states plainly that the visible string changes and why; the pairing with
  "Total ahead" is spelled out.
- Fix B (tomorrow suffix) — the imprecision past one calendar day is explicitly called out as
  an "Edge case" (lines 91-97); the never-a-weekday/date choice is explicitly justified
  (lines 85-89).
- Fix C (required today) — the layout/no-padding consequence is explicitly resolved and
  recorded (lines 125-141, "Resolved with the user").
- Fix D (docs) — no rendering behavior change; explicitly says so (line 38).

Nothing survived this pass as a deliberate-and-correct choice with a silently-dropped visible
consequence — the gaps found here (Findings 1, 2, 3, 5) are cases where the document's stated
premise or prescribed mechanism is itself wrong or incomplete, which the task instructions
route to the other categories instead. This category is empty.

## Verdict

**needs-rework** — Findings 1 and 5 are implementation-blocking as specified (a logic bug
that ships incorrect user-facing wording on two extra days per week, and a suggested API call
that won't pass this repo's own `-D warnings` CI gate, and arguably won't compile against
`today`'s type at all). Finding 2 misdescribes the actual state of SPEC.md closely enough that
following §5/§6 literally produces the wrong edit to SPEC.md. These should be corrected in the
spec before Task 1/Task 2 are dispatched.
