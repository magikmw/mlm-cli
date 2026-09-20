# Adversarial review — `docs/dev/specs/2026-09-20-boundary-context-cues.md`

Baseline checked: `v0.3.3` / commit `f04e77c`, against `src/status.rs`,
`src/render.rs`, `src/week_view.rs`, `src/stint.rs`,
`docs/dev/SPEC.md`.

## Findings

### F1 — High — literal ASCII-output rule violation in the spec's own example
**File:line**: `docs/dev/specs/2026-09-20-boundary-context-cues.md:63`
**Claim**: the §4 "yesterday" open-stint caption example, inside a code
fence (i.e. presented as literal rendered CLI output):
```
  23:10-now    (10h 35m, ongoing — elapsed since now, not a running total)
```
**Evidence**: the separator before "elapsed" is an em dash, `—`
(U+2014), confirmed by byte-scanning every fenced block in the spec
file (`awk` extraction + `grep -P '[^\x00-\x7F]'` finds exactly this
one non-ASCII byte sequence, nowhere else). `docs/dev/SPEC.md:562-564`
(§7) states as a hard rule: "Plain ASCII throughout (no
box-drawing/unicode dashes)". Three existing tests enforce this byte
range on real render output: `src/status.rs`
`t13_plain_ascii_output_and_non_ascii_notes_pass_through` (asserts
every output byte is `\n` or `0x20..=0x7E`), `src/week_view.rs`
`a11_plain_ascii_audit`, and `src/render.rs` `t33_every_produced_string_is_ascii`.
If this caption is implemented verbatim as written, it fails the
project's own plain-ASCII invariant and would need a code fix (plain
hyphen) that the spec itself doesn't specify. This is a factual/
internal-consistency defect in the spec text, not a stylistic nit —
the fenced block is presented as exact output, the same convention
used by every other example in this document and in `SPEC.md` §7.

### F2 — High — arithmetic error in §5's second worked example
**File:line**: `docs/dev/specs/2026-09-20-boundary-context-cues.md:86-88`
**Claim**:
```
Week 2026-09:  52h 00m left by end of Monday (fulfillment -02h 00m = worked 03h 00m + carry-in -05h 00m / target 40h 00m)
```
**Evidence**: `docs/dev/SPEC.md:463-465` (§5) defines `owed = target -
fulfillment`, and this is exactly what `status`'s week-line headline
renders (`src/render.rs:60-72` `week_headline`, called from
`status_week_line`, called from `src/status.rs:211-213` `week_line` —
the sole call site). With `target = 2400` (40h) and `fulfillment =
-120` (-02h 00m, itself correctly derived as `worked(180) +
carry_in(-300) = -120` per the line's own decomposition), `owed =
2400 - (-120) = 2520` minutes = **42h 00m**, not 52h 00m. The first
worked example in the same section (§5, lines 78-80) is internally
consistent (`owed = 2400 - 1755 = 645` = 10h 45m, matches "10h 45m
left"); only the second example's headline figure is wrong. This is
exactly the kind of worked example §6's testing plan says a test must
reproduce ("the case where `fulfillment` itself goes negative from a
carry-in deficit larger than `worked`") — as written, that test would
be written against a wrong expected value.

### F3 — Medium — internal inconsistency: §5 claims closed-week framing is covered, but no closed-week parenthetical exists to expand
**File:line**: `docs/dev/specs/2026-09-20-boundary-context-cues.md:96`
("Same rule for both current and past/future week framings.")
**Evidence**: both of §5's worked examples use the deadline
("... left by end of ...") headline, i.e. `WeekFraming::Current`
only. `src/render.rs:85-92` (`status_week_line`) currently emits the
`(fulfillment .../target ...)` parenthetical **only** for
`WeekFraming::Current`; for `WeekFraming::Closed` the parenthetical is
`String::new()` — nothing is appended at all, confirmed by the golden
test `src/status.rs:689-723` `t7_f10_golden_second_spec_example`,
which explicitly asserts `!out.contains("fulfillment")` for a
closed/past week (`Week 2026-02:  Total still owed: 01h 40m`, no
parenthetical whatsoever). §5's flat claim that the carry-in expansion
applies identically to "past/future" framings therefore either (a) is
vacuous — there is nothing there to expand, since closed weeks never
show a parenthetical today — or (b) silently implies closed weeks must
newly grow a fulfillment/target parenthetical they've never had, which
would be a real behavior change nowhere else described, exampled, or
added to the §6 testing plan or the §7 SPEC.md-update list (which only
mentions §7.1's inline parenthetical, not a new closed-week
parenthetical). Either reading leaves the spec under-specified for
exactly the case it explicitly claims to cover.

### F4 — Medium — undeclared user-visible gap: §3's fix creates a new status/week asymmetry that the document never names
**File:line**: `docs/dev/specs/2026-09-20-boundary-context-cues.md:38-49`
(§3) and `:115-123` (§7, the SPEC.md-update list)
**Claim/choice**: §3 gives `status`'s date header a suffix
disclosing that a date's first punch was consumed by a splice.
§7 confirms the scope of the doc update: "§7.1: document the header
suffix (§3) ... §7.2: note the caption (§4) at the existing
stale-open discussion" — §7.2 (`week`) is updated only for §4, never
for §3.
**Consequence a user will see**: after this change, `mlm status
<receiving-date>` shows a trace of the absorbed punch
(`Fri 2026-02-13  (00:45 continues previous day's stint)`), but `mlm
week` for the same week still shows that date's row with no marker at
all — confirmed in `src/week_view.rs`: `build_rows` (:126-169) and
`b2_bucketing_now_splices_across_midnight` (:642-665) show the
spliced-away date renders as an ordinary `00h 00m` row, no `(ongoing)`,
no `[!]`, nothing — and nothing in §3/§7 proposes a `week`-side
analog. So the same underlying fact (a punch on this date existed and
was consumed) is now visible in one view and invisible in the other, a
new inconsistency introduced by this very changeset. Notably, the
document *does* make the analogous cross-reference for §4 ("note the
caption (§4) at the existing stale-open discussion" — SPEC.md §7.2
already discusses that residual asymmetry), which shows the author was
tracking status/week symmetry as a concern elsewhere in this same
document, but no equivalent note exists for §3. This is a deliberate,
correct-as-far-as-it-goes choice (scope it to `status` only, per §2's
identical "status's stint list only" scoping) whose visible consequence
— a new status/week disclosure gap for this one date — is nowhere
recorded.

### F5 — Low — testing gap: §3 + stint-list omission interaction untested
**File:line**: `docs/dev/specs/2026-09-20-boundary-context-cues.md:98-113`
(§6)
**Claim**: §6 tests "header suffix present exactly when a splice
consumed the date's first punch ... never more than one suffix" but
never calls out the degenerate case already covered by existing code:
when the consumed punch was the receiving date's *only* punch, the
whole stint-list section is omitted per `SPEC.md` §7.1 ("Section
omitted entirely when the date has zero stints") — confirmed by
`src/status.rs:1159-1177`
`resolve_midnight_splice_later_date_shows_no_orphan_anomaly`
(`view.stints.is_empty()` on the receiving date). The resulting
`status` output for that date is header (with new suffix) + day total
+ week line, and nothing else — a very sparse page whose shape isn't
named anywhere in §6's test list, even though it's the most common
real-world shape this fix will produce (a `stop`-only punch typed
right after midnight).

### F6 — Low — testing gap: §5's positive-carry-in case has no worked example
**File:line**: `docs/dev/specs/2026-09-20-boundary-context-cues.md:74-96`
(§5)
**Claim**: §6 says the expanded form must be tested for "`carry_in !=
0` (either sign)", but §5 itself gives two worked examples and both
use a negative carry-in (`-02h 10m`, `-05h 00m`). A surplus
(positive) carry-in exercises a different sign combination in "each
figure is signed independently" (§5's own framing) and has no example
to pin the expected rendering against (e.g. whether `+` is ever
printed, or the SPEC.md §4.2 convention of "positive numbers carry no
sign" is meant to hold for `carry-in` too) — left implicit rather than
shown.

## Verdict

**needs-rework** — F1 and F2 are concrete, verifiable defects in the
spec's own literal examples (one violates a hard, tested project
invariant; the other is a wrong arithmetic result in a worked example
the testing plan says must become a test), so this text should not be
carried into implementation as-is. F3 leaves a stated scope claim
("same rule for ... past/future framings") without any example or
mechanism reconciling it with the current code's closed-week
rendering. F4 is a real, spec-introduced status/week disclosure
asymmetry with no acknowledgment anywhere in the document, despite the
author demonstrably tracking this exact kind of asymmetry for §4.
