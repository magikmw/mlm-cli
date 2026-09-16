# Adversarial review — required-by-day pace hint (decision 52)

Scope: SPEC.md §2.4 (Daily target / Required-by-day bullets), §5's note
after the worked example, §7.1 (status sample + pace-hint/est.-EOD
bullets), F9b, NOTES.md decision 52. Adjacent files read for
consistency: SPEC.md §1/§2.1 (ISO week), §7.2, §4.2, §3.5; src/status.rs,
src/week.rs, src/week_target.rs, src/time.rs.

## Findings

### 1. [High] Worked example's `est. EOD` is stale — contradicts the formula given three lines later

- Claim: SPEC.md:503 — the §7.1 sample output shows `est. EOD 18:35`
  for the line `02h 45m left to 32h 00m required by end of Thursday`.
- Claim: SPEC.md:538 — "it's `now + (required − fulfillment)`".
- Evidence: from the sample's own stint list (SPEC.md:506-508), the
  open stint is `17:45-now (00h 15m, ongoing)`, so `now = 18:00`. The
  gap the same line already states is `02h 45m` (`required` 32h00m −
  `fulfillment` 29h15m, per the week line's own fulfillment figure,
  SPEC.md:504). `now + gap = 18:00 + 02h45m = 20:45`, not `18:35`.
  `18:35` is exactly `18:00 + 00h35m` — the *old* gap (`08h00m` daily
  target − `07h25m` day total = `00h35m`), carried over unedited from
  the pre-decision-52 example. This is confirmed against
  src/status.rs:766-799 (`t11_golden_full_first_spec_example`), which
  still encodes the pre-change numbers (`gap_minutes: 35`,
  `EodState::At(t(18, 35))`) — i.e. the new SPEC.md text is internally
  inconsistent with its own stated formula, not just out of sync with
  not-yet-updated source.
- This is the flagship illustration for §7.1; an implementer following
  the example literally will hand-verify against a wrong number.

### 2. [Medium-High] "Sat/Sun required == full week target" is false whenever the week target isn't a multiple of 5 minutes

- Claim: SPEC.md:202-204 — "Sat/Sun both pin to 5 (... so the
  requirement is the full week target, same as the week's own
  `owed`)."
- Evidence this can fail: `daily target` is defined as `today's week
  target ÷ 5 (floor to the minute)` (SPEC.md:197). `required` on
  Sat/Sun is `daily target × 5` = `5 × floor(target / 5)`, which equals
  `target` only when `target % 5 == 0`. The default target (2400 min)
  and the §5 example's override (2000 min) both happen to be multiples
  of 5, masking this — but nothing in the schema or CLI enforces that.
  `week_targets.target_minutes` has only `CHECK (target_minutes >= 0)`
  (SPEC.md:173); the parser backing `mlm week target` (src/time.rs:59-88,
  `parse_duration`) accepts arbitrary `HhMMm`/`MMm` strings with no
  multiple-of-5 rounding (e.g. `33h17m` → 1997 minutes; confirmed no
  such constraint in src/week_target.rs either). With target = 1997:
  `daily target = floor(1997/5) = 399`; Sat/Sun `required = 399×5 =
  1995 ≠ 1997`. So the Sat/Sun pace hint would read 2 minutes short of
  the week's own `owed` target — directly contradicting the "same as
  the week's own `owed`" claim in the same bullet.
- Either the claim needs an explicit multiple-of-5 caveat, or
  Sat/Sun's `required` should be defined as the *actual* week target
  (not `daily target × 5`) to make the equivalence true unconditionally
  — the latter also matches the stated rationale ("the requirement is
  the full week target") more directly than re-deriving it through a
  floored intermediate.

### 3. [Medium] F9b doesn't exercise the case that would catch finding #2

- Claim: SPEC.md:724-733 (F9b) covers a large-carry-in negative-pace
  case and a Sat/Sun pin case ("`required` pins to the full 5-weekday
  target... matching the week's own `owed` for that day").
- Evidence: F9b's Sat/Sun sub-case doesn't specify a target that's
  non-divisible-by-5, so as written it can pass while the equivalence
  it's asserting is false in general (finding #2). A target like
  `33h17m` (1997 min) in that sub-case would have caught it before
  implementation.

### 4. [Low] Minor terminology note, not a defect

- SPEC.md:202 defines "ISO weekday number, Mon=1 … Fri=5" inline
  rather than by cross-reference; §2.1's "Always Monday-start" bullet
  (SPEC.md:91-92) doesn't itself number weekdays. This isn't
  ambiguous — src/week.rs:301 and chrono's `Weekday::number_from_monday`
  agree Mon=1 — but the spec's own numbering claim rests on chrono
  convention rather than being stated as a project-wide definition
  anywhere else. Not worth blocking on; flagging only because the task
  asked to check for exactly this kind of implicit-convention gap.

Nothing else in the changed sections misfires: the §5 note's
"share the fulfillment number but not the target" framing is accurate
and matches week::week_accounting's `fulfillment`/`owed` fields
(src/week.rs:69-72, reused as-is at src/status.rs:329-330,
`acct.fulfillment`); the required-by-day math needs no new plumbing —
`acct.fulfillment` is already computed in `resolve` right next to
where `daily_target_minutes` is currently used (src/status.rs:337-339),
so decision 52's implementation is a small in-place change, not a
new data dependency. Zero-target weeks, first-ever-week carry_in=0,
and mid-week target overrides are all handled by pre-existing,
already-tested machinery (`week::week_accounting`/`week_series`) that
this change doesn't touch or need to touch — no new edge case there.
No scope creep found: the diff stays within the sections it claims to
touch.

## Verdict

needs-rework — finding #1 is a wrong number in the document's own
canonical example, and finding #2 is a claim in normative spec text
that's false for a real, currently-unconstrained input (non-multiple-
of-5 week target override). Both are cheap to fix (recompute the EOD
figure; either caveat the Sat/Sun equivalence or define Sat/Sun's
`required` directly as the week target instead of `daily target × 5`)
but both need a text change, not just review sign-off.
