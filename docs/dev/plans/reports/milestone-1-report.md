# Milestone 1 — completion report

Scope: `src/time.rs` only. No other file touched (`src/main.rs` already
had `mod time;`, so no change was needed there).

## What was implemented

Replaced the placeholder `parse_hm`/`between`/`format_duration` entirely
with:

- `parse_time(&str) -> Result<chrono::NaiveTime, TimeParseError>` —
  `HH:MM` / `HHMM` / `HH` (§3.1), range 00:00–23:59, `24:00` rejected.
- `parse_duration(&str) -> Result<i64, DurationParseError>` —
  `Hh` / `HhMMm` / `MMm` (§4.2), straight to a minute count.
- `format_minutes(i64) -> String` — the single canonical `HHh MMm`
  formatter (§4.2), zero-padded both sides, hour part never dropped,
  single leading `-` for negatives.
- `TimeParseError` (`InvalidFormat` / `OutOfRange { input, hour, minute }`)
  and `DurationParseError` (`InvalidFormat` / `Negative(i64)`), both
  `Debug + Clone + PartialEq + Eq`, both implementing `Display` (one
  ASCII line, no trailing newline) and `std::error::Error` — contract 7's
  convention, so Milestone 7's `anyhow::Result` boundary picks them up
  via the blanket `From` impl with no work here.

Contract 11 honoured: plain `i64` minutes throughout, formatter named
`format_minutes`, no `Minutes` newtype.

## Verification

- `cargo build` — clean, no warnings.
- `cargo test` — **19 tests, 19 passing, 0 failing.**
- `cargo clippy --all-targets -- -D warnings` — exit 0, clean.
- `cargo fmt` — applied; tree is rustfmt-clean.

Note the baseline (pre-change) tree did *not* pass clippy with
`-D warnings`: the three placeholder functions tripped `dead_code`
because nothing in `main` calls them. The same applies to the new API
until wave 2/3 wires it up, so `src/time.rs` carries a module-level
`#![allow(dead_code)]` with a comment naming the consuming milestones.
Worth removing once Milestones 7/8/9 actually call in.

TDD was followed: all 17 planned tests were written first against
`todo!()` stubs and observed failing before any implementation. The two
extra tests below (`accepts_a_minute_component_of_sixty_or_more`,
`treats_negative_zero_as_plain_zero`) were added after green to pin down
two decisions the plan flagged as open — flagged here for honesty, they
are characterization tests, not red-first ones.

## Deviations from the plan

1. **`format_minutes`, not `format_duration`.** The plan's §1.4 and
   PLAN.md contract 11 both say `format_minutes`; only the older test
   table in §3 still said `format_duration`. Used `format_minutes`,
   since Milestones 6/9 were built against that name.
2. **`Minutes` newtype dropped**, per contract 11 — already reflected in
   the plan's own §1.1, but restated since §2 and §5 still discuss it.
3. **Range checking delegated to `NaiveTime::from_hms_opt`** rather than
   a hand-written `hour <= 23 && minute <= 59` pair. Same semantics
   (`24:00`, `2400`, `24`, `9:75` all become `OutOfRange` with the parsed
   numbers preserved), one less place to get a bound wrong.
4. **Added `-0h` / `-0m` → `Ok(0)`.** The plan doesn't say. `-0` is not a
   negative value and zero is explicitly legal (§6.1), so it parses as
   zero rather than as `Negative(0)`. Trivially reversible if reviewers
   disagree.
5. **Overflow handling** (not mentioned in the plan): `parse_duration`
   uses checked arithmetic and returns `InvalidFormat` for a value that
   would overflow `i64` (e.g. a 30-digit hour count); `format_minutes`
   uses `unsigned_abs` so `i64::MIN` cannot panic.

## Open questions / risks for the reviewer

1. **`"9:5"` rejected as `InvalidFormat`** (plan §5.2). Implemented as
   the plan specifies: colon form requires exactly 2 minute digits.
   Mild counter-evidence found in SPEC.md §6.1, which — while describing
   `WEEK_ID` — says unpadded input is accepted "consistent with how
   `TIME`/`DURATION` input is lenient about padding elsewhere". That
   leniency is unambiguous on the *hour* side (`9:05`) but arguably
   extends to the minute side too. Still a judgment call, still
   unresolved by the spec; one-line change (`is_digits(minutes, 1, 2)`)
   plus one test edit if the reviewer prefers the lenient reading.
2. **`"1h90m"` accepted as 150 minutes** (plan §5.3). Implemented
   unchecked as the plan recommends, and now pinned by a test. If the
   intended behavior is rejection, it's a new `DurationParseError`
   variant with no signature change.
3. **`DurationParseError::Negative` reachability** (plan §5.4). Kept
   reachable by letting a leading `-` match the grammar, so `-5h` yields
   `Negative(-300)` and `10x` yields `InvalidFormat` — distinct
   user-facing messages, as the plan intended.
4. **`NaiveTime` return type carries no seconds-are-zero guarantee at
   the type level** (plan §5.6). Accepted as low risk; this parser always
   constructs with `seconds = 0`.
5. Nothing in the plan or SPEC.md blocked implementation — no defect
   found that needed escalation.
