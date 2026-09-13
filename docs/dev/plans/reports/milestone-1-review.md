# Milestone 1 — independent adversarial code review

Reviewer: independent pass, did not write the code. Read against
`plans/milestone-1-time-duration.md`, `plans/reports/milestone-1-report.md`,
`PLAN.md` §"Milestone 1", and SPEC.md §3.1/§4.1/§4.2/§6.1.
Target: commit `123c4aa` on `milestone-1`, `src/time.rs` only.

## Verification of the report's claims (re-run, not taken on faith)

All four claims confirmed in this worktree:

- `cargo build` — clean, zero warnings.
- `cargo test` — `19 passed; 0 failed`. The report's "19/19" is exact.
- `cargo clippy --all-targets -- -D warnings` — exit 0, no diagnostics.
  Re-run with `touch src/*.rs` first to defeat the result cache, since a
  cached clippy run prints nothing and looks falsely clean.
- `cargo fmt --check` — exit 0, tree is rustfmt-clean.

`git status` is clean; the commit touches `src/time.rs` and the plan/report
files only, matching the report's "no other file touched".

## Acceptance criteria — checked against behavior, not against prose

Each `PLAN.md` Milestone 1 criterion was exercised directly by compiling the
module into a scratch harness and running ~110 inputs through it (far beyond
the test suite), not just by reading the tests.

- Valid `TIME` forms (`9:05`, `17:30`, `0905`, `1730`, `9`, `17`) — all parse
  to the correct hour/minute, seconds always 0 (§4.1's "no seconds
  precision"). **Met.**
- `24:00`, `25:00`, `9:75`, `abc` rejected, never clamped — `24:00`/`2400`/`24`
  all yield `OutOfRange { hour: 24 }`; no silent wrap to `00:00`, no clamp to
  `23:59`. Boundaries spot-checked exhaustively at `00:00`, `0`, `0000`,
  `23:59`, `2359`, `23:60`, `24:59`, `99:99`. **Met.**
- Valid `DURATION` (`20h`, `33h30m`, `45m`, `0h`) → 1200 / 2010 / 45 / 0;
  `0h` and `0m` both `Ok(0)`. **Met.**
- Malformed `DURATION` (`10`, `-5h`, `10x`) rejected, and a negative value
  rejected *even when the grammar matches* — `-5h` → `Negative(-300)`,
  distinct from `10x` → `InvalidFormat`. This is the reading the criterion's
  wording demands. **Met.**
- Formatter renders `07h 45m`, `00h 20m`, `-00h 50m`, `-03h 20m`. **Met.**
- E1/E4 covered at the parsing-unit level. **Met.**

Sign handling in `format_minutes` was probed specifically for the failure
modes named in the review brief: exact `-1` → `-00h 01m`; `-59` → `-00h 59m`;
`-60` → `-01h 00m`; `-61` → `-01h 01m`; `0` → `00h 00m` with no stray sign
(i64 has no negative zero, and the `minutes < 0` test excludes 0 correctly —
a `<=` there would have produced `-00h 00m`); `i64::MIN` → `-153722867280912930h 08m`
with no panic, because `unsigned_abs` is used instead of `abs`. No off-by-one
or sign defect found in the formatter.

Test-table coverage: all 48 plan cases (1–48) are present and each assertion
actually tests what its case claims. Case 41–47 map onto the two formatter
tests, 48 onto the round-trip test. Nothing in the plan's table is silently
skipped, and no test asserts a weaker property than its case states.

## The two self-flagged items

**`#![allow(dead_code)]` (src/time.rs:11) — expected, temporary, masking
nothing.** Verified empirically by copying the tree, stripping the attribute,
and re-running clippy: the only nine warnings are the not-yet-wired public
API (`parse_time`, `parse_duration`, `format_minutes`, both error enums) and
its four private helpers, which are dead only transitively because nothing in
`main` reaches the module. No genuinely unreachable logic hides behind it.
The comment above it names the consuming milestones. Reasonable as-is.

**`"9:5"` → `InvalidFormat` — defensible, not a bug.** SPEC.md §3.1 spells
the colon form `HH:MM` and every worked example (`9:05`, `17:30`) uses two
minute digits. The §6.1 leniency aside the implementer honestly flagged is
written about `WEEK_ID` and its concrete claim is about *leading-zero padding
of a whole field* (`2026-7` ≡ `2026-07`); the TIME-side leniency it gestures
at is already satisfied — the hour is accepted unpadded (`9:05`) and the
whole-minute-field `HH` form is accepted. Reading it as also licensing a
1-digit minute is a stretch, and §6.1/§8.2's E1 example list never includes
`9:5` as a case that must be accepted. Rejection is the correct call.

## Findings

- **src/time.rs:51–54 — `parse_u32` returns a `Result` whose error arm is
  unreachable, and if it were reachable it would report the wrong string.**
  Every caller pre-validates the digit shape (`is_digits`) and passes at most
  4 digits, so `s.parse::<u32>()` cannot fail; the doc comment says as much.
  The error it would build is `InvalidFormat(s.to_string())` — the *fragment*
  (e.g. `"9"`), not the full user input, so the rendered message would be
  `invalid time "9": expected HH:MM, HHMM, or HH`, which is actively
  misleading. Why it matters: dead error plumbing that propagates a `?`
  through the happy path, plus a latent wrong-message bug if the digit
  precondition is ever loosened. Preferred shape is an infallible
  `fn parse_u32(s: &str) -> u32` using `expect` on an already-validated
  precondition, or folding the bound check into `is_digits`. *Severity:
  minor.*

- **src/time.rs:73, 79, 92–96 — a well-shaped `DURATION` that overflows `i64`
  is reported as `InvalidFormat`, producing a misleading message.**
  `"999999999999999999h"` and `"153722867280912931h"` both return
  `InvalidFormat`, which renders as `invalid duration "...": expected a form
  like 20h, 33h30m, or 45m`. The input *does* match that form; the real
  problem is magnitude. Why it matters: user-facing message tells the user to
  fix something that isn't wrong. The overflow guarding itself is correct and
  is a genuine improvement over the plan (which never mentions overflow) — the
  complaint is only the variant/message choice. A third variant, or reusing
  `Negative`'s "magnitude" framing with an out-of-range message, would be
  honest. *Severity: minor.*

- **src/time.rs:76 — the minutes-only form has no magnitude ceiling, unlike
  the hours form.** `"9223372036854775807m"` returns `Ok(i64::MAX)`, while the
  arithmetically equivalent hour count is rejected. Why it matters: the value
  flows to `week_targets.target_minutes` (Milestone 8) and then into
  Milestone 6's unchecked `target - fulfillment` / carry-walk arithmetic,
  where it will panic in debug builds or wrap in release — i.e. this parser
  hands downstream code a value it cannot safely add to. SPEC.md states no
  upper bound so this is not a spec violation, and it needs a deliberately
  absurd input to reach; but the asymmetry (hours bounded, minutes not) is
  accidental rather than reasoned. A sanity ceiling here would be cheap.
  *Severity: minor.*

- **src/time.rs:70–76 — `"-9223372036854775808m"` returns `InvalidFormat`
  rather than `Negative`.** The sign is stripped before parsing, so the
  magnitude `9223372036854775808` fails to fit `i64` even though the signed
  value would. Cosmetic consequence of the (correct, plan-mandated)
  strip-sign-then-check-magnitude design; the input is rejected either way.
  *Severity: minor.*

- **src/time.rs:31–32 — redundant length arguments to `is_digits` inside the
  `len()` match.** The `1 | 2` arm already guarantees the length, so
  `is_digits(input, 1, 2)` re-checks it; likewise `is_digits(input, 4, 4)` in
  the `4` arm. Harmless and arguably defensive (the `4` arm's slicing depends
  on ASCII-ness, which `is_digits` does establish), but the length half of
  each call is dead. *Severity: minor.*

- **src/time.rs:363–377 — `errors_render_as_a_single_ascii_line` asserts
  shape but never content, so no test pins any user-facing message text.**
  It checks non-empty / ASCII / no-newline only. The plan left exact wording
  unspecified (`/* user-facing message */`), so nothing is violated, but the
  four messages — which are the entire §6.1 stderr surface and which
  Milestone 12's docs will quote — can be reworded silently without a test
  failing. One `assert_eq!` on each string would close that. *Severity:
  minor.*

- **src/time.rs:223–233 — `matches!` inside `assert!` discards the actual
  value on failure, and the `input` field goes unasserted.** The custom
  message names the input but not what was actually returned, so a regression
  here reports "expected \"2400\" to be out of range" without showing what it
  got. The sibling tests use `assert_eq!` with a full expected error and are
  better for it. *Severity: minor.*

Explicitly checked and found clean, no finding raised: no `unwrap`/`expect`/
`panic!`/indexing on any user-input path (the only `unwrap` is the `t()` test
helper; the one string slice at line 32 is guarded by a preceding
length-and-ASCII check, so it cannot split a UTF-8 boundary — confirmed with
multibyte inputs such as `"9:０5"` and `"٩:٠٥"`, all of which reject
cleanly); no whitespace or quote tolerance anywhere (` 9:00`, `9:00 `,
`"5h "`, `"20 h"` all rejected, per plan §1.2); non-ASCII digits rejected
(`is_digits` tests bytes, not `char::is_numeric`); no `24:00`-as-midnight
alias; unit case strictly lowercase (`20H`, `1h30M` rejected); unit order
enforced (`30m20h`, `1h2h3m` rejected); mid-string signs rejected
(`20h-5m`, `1h-0m`); `-0h`/`-0m` → `Ok(0)` (report deviation 4 — correct,
`-0` is not a negative value and §6.1 makes zero legal); `1h90m` → 150
accepted unchecked and now pinned by a test, exactly as plan §5.3 recommends;
`format_minutes` naming matches contract 11 (report deviation 1 is correct —
the plan's §1.4 and PLAN.md agree on `format_minutes`, only the stale §3 test
table said `format_duration`); range checking delegated to
`NaiveTime::from_hms_opt` (report deviation 3) is semantically identical to
the plan's `hour <= 23 && minute <= 59` and strictly harder to get wrong;
both error enums implement `Display` + `std::error::Error` per contract 7, so
Milestone 7's `anyhow` boundary needs nothing here. Code is idiomatic —
closure-based error constructors, `split_once`/`strip_prefix`/`strip_suffix`
instead of hand-rolled indexing, inline format args, no `unsafe`, no
allocation in the hot path beyond the error strings.

The report's stated honesty about the two post-green characterization tests
(`accepts_a_minute_component_of_sixty_or_more`,
`treats_negative_zero_as_plain_zero`) is accurate and the right call — both
pin plan-flagged open decisions and neither papers over a failure.

## Verdict

**APPROVE WITH NITS** — every acceptance criterion is genuinely met, all 48
planned test cases are present and test what they claim, the report's build/
test/clippy/fmt claims all reproduce exactly, and both self-flagged items are
handled correctly (the `dead_code` allow masks nothing but the not-yet-wired
API, verified by removing it; the `"9:5"` rejection is the better reading of
§3.1 against §6.1's WEEK_ID-scoped leniency aside). No blocker and no
correctness bug found in parsing, boundary handling, or the formatter's sign
logic. The seven findings are all minor: dead error plumbing in `parse_u32`
with a latent wrong-input message, overflow reported as a format error, an
unbounded minutes-only magnitude that downstream accounting cannot safely add
to, and three test/style nits — none of which need to block the merge, and
all of which are natural follow-ups when wave 2/3 wires the module in and the
`allow(dead_code)` comes off.
