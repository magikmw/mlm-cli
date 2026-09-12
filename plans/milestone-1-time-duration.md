# Milestone 1 — Time-of-day and duration parsing/formatting

Implementation plan for a TDD subagent. Scope, per PLAN.md: replace
`src/time.rs`'s placeholder with the real `TIME`/`DURATION` parsers and
the canonical duration formatter. Pure string-in/value-out logic — no
DB, no CLI, no calendar dates.

Spec citations: SPEC.md §3.1 (TIME input), §4.1 (parsing/conversion to
minute granularity), §4.2 (duration display + DURATION input grammar),
§6.1 (TIME/DURATION hard-error cases).

All of this lands in `src/time.rs`, replacing its current contents
entirely (the placeholder `parse_hm`/`between`/`format_duration` using
raw `chrono::ParseError` and unpadded output go away).

---

## 1. Types and signatures

### 1.1 Minute unit — plain `i64`, no newtype

**Superseded by cross-plan reconciliation (PLAN.md interface contract
11):** this milestone originally proposed a `Minutes(i64)` newtype for
every minute-granular value in the project. Milestones 6 and 9 were
built independently in parallel and both used plain `i64` throughout
— it carries no invariant worth enforcing (negative values are legal
and expected everywhere in the accounting math, §5), so retrofitting
a wrapper onto two milestones that never adopted it isn't worth the
churn. **The newtype is dropped.** Every minute-granular value in this
project, including `week_targets.target_minutes` and all of Milestone
6's accounting fields, is a plain `i64`. The formatter and parser
signatures below reflect this (`i64` in, `i64` out) — read any
`Minutes(N)` appearing later in this file's test tables as shorthand
for the plain integer `N`, not a real type.

### 1.2 Time-of-day parsing

```rust
/// Parses a `TIME` argument (§3.1): "HH:MM", "HHMM", or "HH" (minute
/// defaults to 0). Valid range is 00:00-23:59 inclusive; 24:00 is
/// explicitly rejected (§6.1), never treated as next-day midnight.
/// Returns `chrono::NaiveTime` (seconds always 0) so callers combine it
/// directly with a `NaiveDate` (Milestone 2) via `NaiveDateTime::new`
/// with no intermediate conversion — this is the exact value Milestone
/// 4 needs for its UTC-at-write conversion (§2.1).
pub fn parse_time(input: &str) -> Result<chrono::NaiveTime, TimeParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeParseError {
    /// Doesn't match HH:MM / HHMM / HH shape at all (includes non-digit
    /// input like "abc", wrong separators, wrong digit counts).
    InvalidFormat(String),
    /// Matched a shape but hour and/or minute is out of range, including
    /// the explicit 24:00 boundary case.
    OutOfRange { input: String, hour: u32, minute: u32 },
}

impl std::fmt::Display for TimeParseError { /* user-facing message */ }
impl std::error::Error for TimeParseError {}
```

Justification for `chrono::NaiveTime` as the return type rather than a
bespoke `TimeOfDay` struct: the existing scaffold already imports
`NaiveTime`, and Milestone 4's own stated contract is "a local
wall-clock time already parsed by Milestone 1" combined with a date —
`NaiveTime` is what `NaiveDateTime::new(date, time)` wants, so returning
it avoids an extra conversion step at every call site downstream.
Two error variants rather than one string-message error: `OutOfRange`
carries the parsed numeric hour/minute so a CLI layer (Milestone 7)
could theoretically build a more specific message than `InvalidFormat`
without re-parsing.

**Grammar decisions made explicit here** (spec states the three forms
but not every digit-count edge; pinning these down now rather than
leaving them to whoever implements):
- `HH:MM`: hour is 1-2 digits, minute is *exactly* 2 digits (matches
  every spec example: `9:05`, `17:30` — never a 1-digit minute shown).
  `9:5` is therefore rejected as `InvalidFormat`. **Flagged as a
  judgment call** — SPEC.md never explicitly states minute-side padding
  for the colon form; this reading was chosen because "MM" in the
  grammar table implies two digits and all worked examples show two.
- `HHMM`: exactly 4 digits, first two = hour, last two = minute. `905`
  (3 digits) is rejected as `InvalidFormat`, not treated as `0905`.
- `HH`: 1-2 digits, minute defaults to 0.
- No leading/trailing whitespace tolerance, no surrounding quotes —
  reject anything not an exact match of one of the three shapes.
- Range check is uniform after structural parsing: `hour <= 23`,
  `minute <= 59`. `24:00`/`2400`/`24` all fail as `OutOfRange` under the
  same `hour <= 23` rule — no special-cased "24 only valid with :00"
  branch needed.

### 1.3 Duration input parsing

```rust
/// Parses a `DURATION` argument (§3.7/§4.2 input grammar): "Hh",
/// "HhMMm", or "MMm" — unpadded, hours-only and minutes-only both
/// legal standalone. Converts straight to a plain minute count (i64,
/// no newtype — contract 11). Zero is legal; negative is rejected.
pub fn parse_duration(input: &str) -> Result<i64, DurationParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationParseError {
    /// Doesn't match Hh / HhMMm / MMm shape (missing unit, unknown
    /// trailing characters, non-digit content, wrong unit order, etc).
    InvalidFormat(String),
    /// Matched the shape (magnitude + optional leading '-') but the
    /// resulting value is negative.
    Negative(i64),
}

impl std::fmt::Display for DurationParseError { /* user-facing message */ }
impl std::error::Error for DurationParseError {}
```

**Grammar decision, flagged explicitly**: Milestone 1's own acceptance
criteria says "a negative parsed minute value is rejected *even if the
grammar otherwise matches*" — that sentence only makes sense if the
grammar itself can produce a negative value prior to the range check.
So the parser recognizes an optional leading `-` before the digits
(`-5h` structurally matches "Hh" with a negative magnitude), and the
negative check is a distinct, later step from the shape check. This is
why there are two distinct error variants instead of one: `-5h` and
`10x` are both rejected, but for different reasons, and `DurationParseError::Negative`
gives a cleaner user-facing message ("duration cannot be negative")
than folding it into `InvalidFormat`. Without this reading, `Negative`
would be dead code, which is a smell worth surfacing rather than
silently accepting a redundant variant.

**Second grammar decision, flagged as unresolved by SPEC.md**: the
`m` component's own magnitude is not range-checked against 60 (e.g.
`1h90m` is accepted and sums to 150 minutes) — SPEC.md's grammar table
never states each component must individually be a valid clock value
the way `TIME`'s minute must be `<= 59`; `DURATION` is a magnitude, not
a wall-clock offset. No acceptance criterion or §8 flow exercises this
either way. **Recommend confirming this reading before wave-1 sign-off**;
if wrong, it's a one-line range-check addition with no signature
change.

### 1.4 Duration display formatting

```rust
/// The one canonical duration display formatter (§4.2), used
/// identically by `status` and `week` output later (Milestone 9/10/11)
/// and by nothing else — no second formatter should ever be written.
/// Always `HHh MMm`, both sides zero-padded to 2 digits, hour part
/// never dropped even at zero. Negative values keep the same padding
/// with a single `-` in front of the whole thing (not a `-` per side):
/// `-00h 50m`, `-03h 20m`. Plain `i64` in, no newtype (contract 11) —
/// named `format_minutes` to match what Milestones 6/9 already call it.
pub fn format_minutes(minutes: i64) -> String;
```

No error case — every `i64` is representable; this function is total.

---

## 2. Interface-contract compliance

- **Contract 3** (week accounting result shape, target/carry_in/
  worked/fulfillment/owed/carry_out as "signed integer minutes"): this
  milestone's `Minutes` type is the concrete unit Milestone 6 should
  use for all six fields, and its `Add`/`Sub`/`Neg` impls are sized for
  exactly that milestone's arithmetic (folding worked totals, computing
  `owed = target - fulfillment`, etc). Milestone 6's plan should adopt
  `Minutes` rather than re-deriving its own integer wrapper.
- **Contract 5** (shared rendering helpers / Milestone 9's dependency
  on "the duration formatter, already centralized in Milestone 1"):
  `format_duration` above is exactly that single implementation;
  Milestones 9, 10, 11 must call it, never reimplement padding/sign
  logic locally. Its signature (`Minutes -> String`, total, no error)
  is deliberately simple so nothing downstream has a reason to wrap it.
- **Contract 7** (error type/shape for hard errors, spanning
  Milestones 1, 2, 4): PLAN.md leaves the actual shape unspecified
  beyond "agreed ... so CLI wiring doesn't need rework." **This plan
  proposes and flags a concrete completion**: rather than one shared
  enum type edited by three different wave-1 worktrees (a merge-conflict
  risk that undermines the whole point of running them in parallel),
  the agreed contract is a *convention*, not a shared file:
  - Every hard-error-producing function in Milestones 1, 2, and 4
    returns its own domain-local error enum (`TimeParseError`,
    `DurationParseError` here; presumably `DateParseError`/
    `WeekIdParseError` in Milestone 2, a note/DB error in Milestone 4).
  - Every such enum implements `std::fmt::Display` (the exact stderr
    message) and `std::error::Error`.
  - The CLI layer (Milestones 7/8, wave 2/3 — sequential, not
    parallel) is responsible for converting whichever error it catches
    into "print `{err}` to stderr, exit nonzero" uniformly, most likely
    via `Box<dyn std::error::Error>` or a thin `anyhow`-style wrapper
    at the command-dispatch boundary — a decision for Milestone 7/8's
    plan, not this one.
  - This satisfies "CLI wiring doesn't need rework" (it codes against
    the trait bound, not against a specific enum's variant list) while
    letting Milestones 1, 2, 4 land independently with zero shared
    file. **Resolved**: cross-plan review settled contract 7 on
    exactly this shape, then Milestone 7 adopted `anyhow` for the CLI
    boundary specifically — `anyhow::Result<()>` command handlers,
    `?` auto-converting any `std::error::Error` type via `anyhow`'s
    blanket `From` impl. This milestone's parsers need no further
    change either way; the error-type choice at the CLI layer was
    always someone else's decision.

---

## 3. Test cases

### TIME — valid (§3.1, feeds F1-F12 indirectly wherever a punch time is entered)
1. `"9:05"` → `09:05`
2. `"17:30"` → `17:30`
3. `"0905"` → `09:05`
4. `"1730"` → `17:30`
5. `"9"` → `09:00`
6. `"17"` → `17:00`
7. `"0"` → `00:00`
8. `"00:00"` → `00:00`
9. `"23:59"` → `23:59`
10. `"2359"` → `23:59`

### TIME — invalid (E1)
11. `"25:00"` → `OutOfRange`
12. `"24:00"` → `OutOfRange` (explicit boundary, §6.1 — not next-day midnight)
13. `"2400"` → `OutOfRange`
14. `"24"` → `OutOfRange`
15. `"9:75"` → `OutOfRange`
16. `"abc"` → `InvalidFormat`
17. `""` → `InvalidFormat`
18. `"9:5"` → `InvalidFormat` (documented grammar decision, §1.2 above)
19. `"905"` (3 digits) → `InvalidFormat`
20. `"-9:00"` → `InvalidFormat`
21. `"9:00:00"` → `InvalidFormat` (no seconds accepted)
22. `"9: 00"` (embedded space) → `InvalidFormat`

### DURATION — valid (§3.7/§4.2, feeds F7/F7b/E9 at the parsing-unit level)
23. `"20h"` → `Minutes(1200)`
24. `"33h30m"` → `Minutes(2010)`
25. `"45m"` → `Minutes(45)`
26. `"0h"` → `Minutes(0)` (F7b — zero is legal)
27. `"0m"` → `Minutes(0)`
28. `"1h0m"` → `Minutes(60)`
29. `"5h5m"` → `Minutes(305)` (unpadded minute component, per "no padding" rule)
30. `"100h"` → `Minutes(6000)` (no upper bound on hours)

### DURATION — invalid (E4/E9)
31. `"10"` (no unit) → `InvalidFormat`
32. `"10x"` → `InvalidFormat`
33. `"-5h"` → `Negative(-300)` (matches shape, fails on sign — see §1.3 decision)
34. `"-45m"` → `Negative(-45)`
35. `""` → `InvalidFormat`
36. `"h"` (no digits) → `InvalidFormat`
37. `"30m20h"` (wrong order) → `InvalidFormat`
38. `"20H"` (wrong case unit) → `InvalidFormat`
39. `"20 h"` (embedded space) → `InvalidFormat`
40. `"20h-5m"` → `InvalidFormat` (sign only legal as a leading token for the whole value, not mid-string)

### format_duration (§4.2, feeds every later milestone's rendering)
41. `Minutes(465)` → `"07h 45m"`
42. `Minutes(20)` → `"00h 20m"`
43. `Minutes(0)` → `"00h 00m"`
44. `Minutes(-50)` → `"-00h 50m"`
45. `Minutes(-200)` → `"-03h 20m"`
46. `Minutes(2400)` → `"40h 00m"` (default weekly target, sanity check against §5's table)
47. `Minutes(-1)` → `"-00h 01m"` (smallest-magnitude negative, boundary check on the sign/padding interaction)

### Round-trip / consistency
48. `format_duration(parse_duration("33h30m").unwrap())` → `"33h 30m"` (input grammar and output grammar agree on the same value even though the two grammars differ in strictness)

---

## 4. Consumption by later milestones

- **Milestone 4** (punch storage): calls `parse_time` directly on the
  CLI-supplied `TIME` string (via Milestone 7's wiring) and combines
  the returned `NaiveTime` with a local `NaiveDate` (Milestone 2's
  output) to build a `NaiveDateTime`, then localizes/converts to UTC
  per §2.1. No adapter needed — the return type was chosen specifically
  to compose with `NaiveDateTime::new`.
- **Milestone 6** (week accounting): should adopt `Minutes` as the
  type for target/carry_in/worked/fulfillment/owed/carry_out (contract
  3) and use its `Add`/`Sub`/`Neg`/`Sum` impls for the week-walk
  arithmetic instead of a locally-defined integer wrapper.
- **Milestone 8** (`week target` command): calls `parse_duration` on
  the CLI-supplied `DURATION` string before writing
  `week_targets.target_minutes`; the resulting `Minutes.as_i64()` binds
  directly to the SQL integer column.
- **Milestones 9, 10, 11** (rendering): call `format_duration`
  exclusively for every duration shown on screen — day totals, stint
  durations, week target/carry/fulfillment/owed, and the `[!]`-adjacent
  numeric fields. None of them should hand-format a duration string
  independently; this is the entire point of contract 5.
- **Milestone 7** (`start`/`stop` commands): calls `parse_time` on an
  optional `TIME` argument, surfacing `TimeParseError`'s `Display` on
  a hard-error exit per contract 7's convention above.

---

## 5. Ambiguities, risks, and disagreements flagged

1. **Contract 7 completion (trait convention vs. shared enum)** — see
   §2 above. This is a deliberate divergence from a literal "one agreed
   error shape" reading, made to avoid a shared-file merge hazard across
   parallel wave-1 worktrees. Needs sign-off from whoever picks up
   Milestone 2 and Milestone 4 before all three land, since it commits
   them to the same trait-bound convention rather than a shared enum.
2. **`HH:MM` minute-padding requirement** (test case 18) — SPEC.md's
   grammar table doesn't explicitly forbid a 1-digit minute in the
   colon form; this plan rejects `"9:5"` based on inference from the
   worked examples only. Low risk (no §8 flow exercises this string
   either way), but worth a second pair of eyes since it's an inference,
   not a stated rule.
3. **DURATION minute-component upper bound** (test case 30's sibling,
   `"1h90m"`) — genuinely unresolved by SPEC.md; this plan accepts it
   unchecked (sums to a raw minute count) rather than rejecting or
   normalizing it. If the intended behavior is to reject a
   minute-component `>= 60`, that's a small addition to
   `DurationParseError` (a new variant) with no signature change —
   flagging now so it doesn't silently ship one way by default.
4. **`DurationParseError::Negative` reachability** — only reachable
   because this plan deliberately widens the grammar to recognize a
   leading `-` before applying the sign check (§1.3). If a future
   reviewer tightens the regex to disallow `-` entirely (treating any
   sign as `InvalidFormat`), the `Negative` variant becomes dead code.
   This plan's reading is the one that satisfies Milestone 1's own
   acceptance-criteria wording most literally ("even if the grammar
   otherwise matches"), so it's the recommended default, but it's a
   judgment call worth naming rather than leaving implicit.
5. **`Minutes` as a newtype vs. bare `i64`** — this is an addition
   beyond what PLAN.md's Milestone 1 scope literally asks for (it only
   mentions the formatter and the two parsers), justified here because
   contract 3 needs *some* shared minute type and Milestone 1 is the
   first and most natural owner of it (it already owns the formatter
   that consumes it). If Milestone 6's implementer strongly prefers a
   bare `i64` or their own wrapper instead, this is a one-file, no-logic
   rename with no cascading impact on Milestone 1's own tests — flagging
   so it's a conscious choice, not a silent assumption everyone else has
   to discover by reading this milestone's code.
6. **No `TimeOfDay` bespoke type** — reusing `chrono::NaiveTime`
   (always seconds=0 by construction) instead of a custom struct means
   nothing at the type level stops a downstream caller from later
   attaching nonzero seconds to a value that flows through this parser.
   Accepted as low risk (nothing in the plan ever constructs a
   `NaiveTime` with seconds except this parser, and `NaiveDateTime`
   composition doesn't spontaneously add them), but noted as a
   deliberate trade of type-safety for reduced friction with the
   existing scaffold and with Milestone 2/4's date-combination code.
