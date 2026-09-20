# Milestone 2 — independent adversarial code review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Reviewer: independent agent (did not write the code). Target: commit
`9953b37` on branch `milestone-2` (`src/date.rs`, `src/main.rs` `mod date;`).
Sources of truth read: `plans/milestone-2-date-week.md`,
`plans/reports/milestone-2-report.md`, SPEC.md §1.3/§2.3/§3.5/§3.6/§6.1,
PLAN.md "Interface contracts" (esp. contracts 7 and 9).

All verification below was **run**, not read: `cargo build`, `cargo test`,
`cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, plus an
out-of-tree probe harness (a scratch crate including `src/date.rs` verbatim)
used to exercise paths the in-tree tests do not.

---

## Findings

### 1. `src/date.rs:73` — `Cause::OutOfRange` renders a wrong message for an out-of-range *year*

`Cause::OutOfRange` is produced by `WeekId::new` for two distinct
conditions (`src/date.rs:183`): `week == 0` **and** `iso_year` outside
`1000..=9999`. `Display` maps the whole variant to a single string:

```
(_, Cause::OutOfRange) => write!(f, "week number must be 1 or greater"),
```

Observed (probe): `parse_week_id("0999-07", …)` and
`WeekId::from_key("0999-07")` both render

```
invalid WEEK_ID "0999-07": week number must be 1 or greater
```

The week number is `07`; the year is what was rejected. The message tells the
user to fix the part that is already correct. Reachable from the CLI
(`mlm week 0026-07`, `mlm week target 0999-07 20h`) because the `YYYY-WW`
shape check only requires four ASCII digits, not a value in `1000..=9999`.

Also note the `Display` arms for `OutOfRange` / `NoSuchCalendarDate` /
`NoSuchIsoWeek` match on `_` for `arg`, so a mismatched (arg, cause) pair
would silently render a message for the wrong argument kind. Not reachable
today, but it removes the compiler's help if a variant is reused later.

**Why it matters**: the error text is the entire user-facing contract for a
§6.1 hard error, and contract 7 makes those messages a pinned interface.
There is also **no test** covering the year-bound rejection at all (plan T61
only covers the 3-digit *shape* case), so this path is both wrong and
unexercised.

**Severity: minor** (wrong wording on an unusual-but-reachable input;
one extra `Cause` variant or one extra match arm fixes it).

### 2. `src/date.rs:104` + `src/date.rs:200` — `WeekId`'s documented `1000..=9999` invariant is reachably violated, and the plan's justification for not enforcing it is factually wrong

Plan §7.6 asserts: *"The `YYYY-WW` / `YYYY-MM-DD` shapes are exactly four
digits, so parsing cannot produce anything outside `1000..=9999` anyway."*
That is false — four digits include `0000`–`0999`. Verified (probe):

- `parse_date("0000-01-01")` → `Ok(0000-01-01)`; `parse_date("0500-06-01")` → `Ok`.
- `WeekId::from_date(0500-06-01)` → `0500-22`, i.e. a `WeekId` that breaks the
  struct-level invariant documented at `src/date.rs:171-172`.
- `WeekId::from_key(WeekId::from_date(0500-06-01).to_key())` → **`Err(OutOfRange)`**.
  `to_key`/`from_key` are therefore not inverses for every value the type can
  hold, contradicting the doc comment "Strict inverse of `to_key`" and the
  round-trip property the `week_targets` `TEXT PRIMARY KEY` design rests on.
- Worse for a year `< 0` (not reachable from `parse_date`, but `from_date`
  accepts any `NaiveDate` and is the API contract 9 pins):
  `WeekId::from_date(-0010-06-01).to_key()` → `"-010-22"`. `{:04}` on a
  negative year silently truncates to a 7-character string that looks
  canonical, is not, and sorts before every real key — exactly the failure
  mode `to_key`'s zero-padding exists to prevent.
- `WeekId { iso_year: 1000, week: 1 }.prev()` would trip the `MIN_YEAR`
  assertion (`src/date.rs:296`), so a sub-1000 week reaching Milestone 6's
  walk is a panic path rather than an error path.

The report (risk 3) discloses that `from_date` does not enforce the bound,
which is honest, but the disclosure is paired with the plan's incorrect
reasoning that no input can get there. `parse_date` is the counterexample.

**Why it matters**: `from_date`, `start`, `to_key` and `Ord` are consumed
verbatim by Milestones 6, 8, 9 and 11 *right now*, on the strength of the
"validated once, trusted everywhere" invariant. Two cheap fixes exist —
bound the year in `parse_date` (or in `from_date`, returning the clamped-away
case as an error), or drop the `1000..=9999` invariant from the doc and from
`WeekId::new` so the type's promise matches its behaviour. Either is a
few lines; leaving both the claim and the hole is the problem.

**Severity: significant** (latent, absurd-input-gated, but it invalidates a
documented invariant four downstream milestones are coding against).

### 3. `src/date.rs:69` — `Display` echoes raw user input, so the "one plain-ASCII line" promise does not hold for hostile input

`write!(f, "invalid {} \"{}\": ", name, self.input)` interpolates the input
verbatim (as the plan mandates: "verbatim and untruncated"). Verified:

| input | rendered message |
|---|---|
| `"2026-02-12\n"` | contains a literal newline → **two lines** |
| `"🙂"` | non-ASCII in the message |
| `"2026-02-12\x1b[31m"` | ANSI escape reaches stderr unfiltered |

Contract 7 specifies "`Display` (one ASCII line, no trailing newline)" and
SPEC §7 imposes a plain-ASCII rendering rule. The existing tests
(T122/T123) assert single-line-ASCII only for already-clean inputs, and
T125's junk corpus asserts `is_err()` without looking at the message, so the
suite cannot catch this. Impact is confined to garbled/colour-injected
stderr — no memory or logic issue.

**Severity: minor** (the plan asked for verbatim echo, so this is a plan
defect the implementation faithfully inherited; a `escape_debug`-style or
control-char-stripping pass at the `Display` boundary would close it).

### 4. `src/date.rs:162` — public `iso_weeks_in_year` panics on an out-of-representable year, undocumented

`NaiveDate::from_ymd_opt(iso_year, 12, 28).expect("December 28 exists in every year")`
— true for every year `chrono` can represent, false for `i32::MAX`.
Verified: `iso_weeks_in_year(i32::MAX)` panics. The function is `pub` with
no `# Panics` note and no year validation, while `WeekId::new` (its only
in-crate caller) happens to validate afterwards, not before.

**Severity: minor** (no MVP caller can reach it; a `# Panics` line or an
internal-only visibility would settle it).

### 5. `src/date.rs:209` — `WeekId::current` has zero test coverage

`grep` over the test module shows no call to `WeekId::current`. It is one of
the functions the plan (§4.2, §6) and the report hand to Milestones 8, 9 and
11 as *the* "is this the current week" primitive (contract 3 note). It is a
one-line delegation to `from_date`, so the risk is low, but it is the only
exported item in the module with no test at all.

**Severity: minor.**

### 6. `src/date.rs:11` — module-wide `#![allow(dead_code)]`

Justified today (consumers land in Milestones 4/6/8/9/10/11) and explained
in a comment. But it is module-*wide* and permanent unless someone
remembers: once Milestone 11 lands, a genuinely unused private helper or an
abandoned method in `date.rs` will never be reported, and `cargo clippy -D
warnings` will silently stop covering unused-code lints for the largest
module in the crate. Prefer per-item `#[allow]`/`#[expect]`, or a wave-5
checklist entry to delete the module attribute.

**Severity: minor.**

### 7. `plans/milestone-2-date-week.md:151`, `:698` — the plan still carries the superseded shared-`src/error.rs` design

The **implementation is correct on this point**: `DateWeekError`, `ArgKind`
and `Cause` are local to `src/date.rs`, there is no `src/error.rs` anywhere
in the tree, and `grep -rn "error.rs\|InputError"` over `src/` finds nothing.
Contract 7's final form is followed exactly.

The *plan document*, however, was only half-patched. §3's reconciliation
paragraph reads "the shared `src/error.rs` proposed below" — referring to a
design block that no longer exists below it — and §7.8 survives intact as a
whole subsection titled "`src/error.rs` is a shared file across two parallel
worktrees", ending in a bolded instruction: "**Action required before
opening the worktrees: paste §3 verbatim into Milestone 1's plan.**" A
reader arriving at §7.8 first is told to build precisely the design §3 now
forbids. The report (risk 5) notes §7.8 is moot, which is the right call but
does not fix the document.

**Severity: minor** (documentation only; delete §7.8 and reword §3's
"proposed below").

### 8. `coverage-baseline.json` — still `{"line_coverage_percent": 0}`

AGENTS.md describes the pre-commit gate as ratcheting the baseline upward
whenever coverage improves. The milestone added 976 lines with 42 tests and
the baseline is unchanged at 0, so either the hook was not enabled
(`git config core.hooksPath .githooks` is per-clone and this is a fresh
worktree) or `cargo-llvm-cov`/`jq` was missing and the check SKIPped. Not a
code defect; it means the quality gate is currently toothless for every
subsequent milestone that inherits the 0 baseline.

**Severity: minor** (process).

---

## Verified correct (checked by execution, not by reading)

- **Contract 9 API, verbatim**: `WeekId` has private fields `iso_year: i32`,
  `week: u8`; `new(i32, u32, &str) -> Result`, `from_date(NaiveDate)`,
  `current`, `iso_year() -> i32`, `week() -> u32`, `start()`, `end()`,
  `span()`, `dates() -> [NaiveDate; 7]`, `contains`, `next()`, `prev()`,
  `to_key()`, `from_key()`. No `year()` / `iso_week()` / `monday()` aliases
  exist. Derives `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`
  with field order `(iso_year, week)`; `Ord` is chronological (pinned in-tree
  against sorting by `start()`, and cross-checked out-of-tree over every
  week from 1900-01 to 2199-xx: `w.next() > w` and
  `w.next().start() == w.start() + 7 days` for all 15 653 weeks).
- **ISO week count is a real per-year check, not `1..=53`.** `2026 → 53`
  (53-week year, so `2026-53` is *accepted*), `2027 → 52` (the spec's own
  `2027-53` counterexample rejected with `NoSuchIsoWeek{2027, 52}`),
  `2020 → 53` (leap + Jan 1 Wednesday branch), `2024 → 52` (leap but not 53),
  `2015/2009/2004 → 53`, `2021/2025/2019 → 52`. The in-tree 200-year property
  test (T38/T39) compares against a genuinely *independent* derivation
  (count of Thursdays in the calendar year, plus the Dec-31-weekday rule),
  not a restatement of the Dec-28 rule. I ran the suite: it passes.
- **`WEEK_ID` padded/unpadded/bare equivalence**: `2026-07` == `2026-7` ==
  `7` == `07` (single assertion, T44), confirmed independently.
- **Bare `WW` defaults to today's ISO year, not calendar year**, and the
  exact Dec 29–31 boundary is tested: `parse_week_id("7", 2025-12-30)` →
  `2026-07` (T67) and the mirrored `2027-01-02` → `2026-07` (T68). Probe
  confirms 2025-12-29/30/31 all default to ISO year 2026 and 2024-12-30
  defaults to 2025. Bare forms are still validated against the defaulted
  year (`"53"` with today 2027-06-15 → `NoSuchIsoWeek{2027,52}`, with the
  message naming the *defaulted* year — the reason for the documented
  deviation of adding `iso_year` to the variant, which is a correct call).
- **`DATE` strictness**: shape is byte-checked before `from_ymd_opt`, so
  `2026-02-30` → `NoSuchCalendarDate`, `13/02/2026` / `2026-2-12` /
  `+2026-02-12` / trailing-space / non-ASCII-digit → `Shape`. Byte indexing
  is safe: the shape check guarantees all ten bytes are ASCII before any
  `s[0..4]` slice, so no char-boundary panic is possible (probed with
  full-width and mathematical digits, and with a NUL byte).
- **Mon–Sun span, including ISO-year boundaries**: all of §7.1/§7.2's worked
  examples match (`2026-07 → 2026-02-09..2026-02-15`,
  `2026-06 → 2026-02-02..2026-02-08`, `2026-02 → 2026-01-05..2026-01-11`),
  as do both boundary directions (`2026-01 → 2025-12-29..2026-01-04`,
  `2026-53 → 2026-12-28..2027-01-03`, `2020-53 → 2020-12-28..2021-01-03`).
  Out-of-tree exhaustive check over 1900–2199: every week's `start()` is a
  Monday, `end()` a Sunday six days later, and both round-trip through
  `from_date`.
- **`from_key` is strict / `parse_week_id` lenient**, as designed; `to_key`
  is zero-padded so lexicographic order agrees with `Ord` (tested).
- **`week_range`** is inclusive, empty (not looping or panicking) when
  `from > to`, gapless across 52↔53 rollovers, and counts 53 for 2026 and 52
  for 2027.
- **`DateWeekError` is local to `src/date.rs`**, implements a hand-written
  `Display` and `std::error::Error` (compile-time assertion T124), no
  `thiserror`, no shared error file, no `InputError` left anywhere.
- **Build/test/lint, run here**:
  - `cargo build` — clean, 3 pre-existing `dead_code` warnings in
    `src/time.rs`.
  - `cargo test` — **42 passed, 0 failed**. The report's "42/42" claim is
    accurate.
  - `cargo clippy --all-targets -- -D warnings` — fails with exactly 3
    errors, all `dead_code` on `src/time.rs`'s `parse_hm` / `between` /
    `format_duration`. Confirmed pre-existing against the base commit:
    `git diff 638f2d6 HEAD -- src/time.rs` is empty and base `main.rs` had no
    call sites either, so this milestone did not introduce them. With
    `-A dead_code` the workspace is clean — **zero clippy findings in
    `src/date.rs`**, matching the report.
  - `cargo fmt --check` — clean.
  - Complexity gate (`-W clippy::cognitive_complexity`, threshold 15 from
    `clippy.toml`) — no hits in `src/date.rs`.
- Code reads idiomatically: no `unwrap` on user input, `expect` used only
  where an invariant makes it total (with a message saying so),
  `std::array::from_fn` for `dates()`, `Option<WeekId>` cursor in
  `WeekRange`, doc comments citing the spec sections they implement.

---

## Verdict

**APPROVE WITH NITS** — the contract-9 API is exact, the 52/53 check is a
real per-year calendar check (independently verified for both kinds of year),
the padded/unpadded/bare-`WW` and ISO-year-default behaviours are correct and
tested at the Dec-30 boundary, the Dec/Jan spans are right, and the 42/42
test and clippy claims hold up on re-run. Nothing here blocks Milestones 6,
8, 9 and 11, which depend on this API verbatim. Follow-ups, in priority
order: finding 2 (make `parse_date`/`from_date` agree with the `1000..=9999`
invariant `WeekId` advertises, or stop advertising it), finding 1 (an
`OutOfRange` message that names the year when the year is what failed, plus a
test for that path), then findings 3–8.
