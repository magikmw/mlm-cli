# Review: 2026-09-24-current-week-ahead-wording.md

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Adversarial review of the design spec against the actual source it
describes. Baseline: `v0.3.6` (matches HEAD's recent commits).

## Findings

### 1. Testing gap — no end-to-end coverage of the new wording through either real command surface (medium)

**Claim** (§4, spec lines 93-104): the test plan lists only `render.rs`
unit-test changes (rewrite `t11`, add `status_week_line` ahead cases, add
an `owed_minutes == -1` boundary case) plus a re-grep of `src/`/`tests/`.
Task 1 (§6) scopes all test changes to `src/render.rs` only.

**Evidence**: grepping the whole crate for the headline literals shows
`week_headline`/`status_week_line` are called from exactly two places
outside `render.rs`: `src/status.rs:301` (`week_line`, wrapping
`status_week_line`) and `src/week_view.rs:252` (`render::week_headline`
directly). Neither module's own test suite, nor `tests/status_cli.rs`,
nor `tests/ascii_sweep.rs`, ever constructs a scenario where the real
`resolve()`/`build_week_view()` pipeline produces a negative `owed` for
the *current* week. Every `status.rs`/`week_view.rs` fixture that
exercises the current-week branch uses a positive `owed` (645, 165, 0,
etc. — see `current_week_acct(645, 1755, 2400)` reused throughout
`status.rs`, and `example_a()` in `week_view.rs` with a hardcoded
positive headline string). `status.rs:1586`
(`resolve_f11_is_today_false_but_week_is_current`) and
`week_view.rs:1070` (`c10_current_vs_closed_headline_end_to_end`) are
the two closest candidates and both assert `"left by end of Thursday"`
only.

So after this changeset ships, the only test asserting a real user
sees `"X ahead of target"` will be a call to the pure `week_headline`/
`status_week_line` functions with a hand-fed negative `i64`. Nothing
proves `week::week_accounting` can actually produce a negative `owed`
for a current week reachable via `resolve()`/`build_week_view()` and
that it renders correctly once wired through `build_ledger`,
`week_line()`, and `render()`/`render_week()`. This mirrors a
pre-existing gap: the "Total ahead" closed-week branch (decision 18,
already shipped) has zero end-to-end coverage either — this changeset
doesn't introduce the pattern, but it does add a second, more
frequently-hit branch (current week going ahead is a routine outcome,
not an edge case) to the same blind spot without flagging it.

### 2. Internal inconsistency — README/SPEC line citations are accurate but the doc never checks whether other rendered lines on the same page still carry the dropped weekday (see finding 4 — related, not the same issue)

Not filed as a separate item; folded into finding 4 below since it's the
same root cause viewed from the "is this internally consistent" angle
as well as the "does the user see it" angle.

### 3. Minor — stale wording in the NOTES.md text this changeset's own decision-log entry will sit next to (low)

**Claim** (spec §"Interaction with the project spec", line 136-138):
"NOTES.md decision 18 framed the current week only as `X` left by end
of `<weekday>`." True as far as it goes, but decision 18's own text
(`docs/dev/NOTES.md:138-143`) describes the closed-week form as
`"Total still owed X" / "Total ahead X"`, which was never the actual
implemented wording — the real strings are `Total behind: {}` /
`Total ahead: {}` (`src/render.rs:68,70`). This mismatch predates this
changeset and isn't something it needs to fix, but the plan explicitly
says a new decision-log entry "supersedes" 18 "for `owed < 0`" — whoever
writes that entry should not copy 18's already-wrong `Total still owed`
phrasing forward. Worth a one-line flag in the task so the person
writing the NOTES.md entry doesn't propagate the error.

### 4. Undeclared user-visible gap — the ahead form makes two adjacent status lines disagree on whether "today" has a deadline at all (the main finding)

**The choice**: the current-week headline drops the weekday entirely in
the `owed < 0` branch (spec §1, line 36: "The ahead form drops the
weekday deadline entirely").

**The consequence nowhere recorded**: `status`'s output puts the
day-total pace hint immediately above the week line
(`src/status.rs:262-267`, `render()`'s `lines` vec: `day_total_line`
then `week_line`). The day-total pace hint computes its own `gap` from
`required` vs. `fulfillment` (§2.4) — a different quantity from the
week line's `owed` — and SPEC.md §7.1 (lines 645-651) explicitly says
these two can and do disagree mid-week ("both are correct answers to
different questions, not a contradiction"). That means a perfectly
normal status page can now read:

```
Day total:     ..., 02h 45m left to 32h 00m required by end of Thursday, ...
Week 2026-37:  04h 35m ahead of target (fulfillment ... / target ...)
```

Before this change, both lines referenced "Thursday" whenever the week
was current (the pace hint always does, Monday-Thursday; the week line
always did too, since it never dropped the weekday for the current
week). After this change, the same page can show one line naming a
weekday and the very next line naming none, for the same date, in the
same command's output, with nothing anywhere — not in §1, not in §1.1's
non-goals, not in §1.2's known issues — noting that the two lines'
deadline framing can now visibly diverge on an ahead week. This is
exactly the deliberate-and-correct-choice-with-an-unrecorded-consequence
this category asks for: dropping the weekday is the right call for the
week line (a met target has no deadline left to state), but the doc
never says a reader now gets one deadline-framed line sitting directly
above one deadline-free line, on the same screen, describing related
but distinct numbers.

**Severity**: presentation-only, no functional impact, but it's a real
first-read “wait, why does only one of these mention a day” moment,
squarely the kind of thing this project already documents proactively
when it notices it elsewhere (e.g. the exact same file's own §7.1 text
already calls out the day-total/week-line divergence in *values*, just
not in *whether a weekday appears at all*).

I tested the "is this actually empty" possibility before writing this:
the closed-week form already omits the weekday entirely and coexists
with a still-weekday-bearing day-total line under the *existing*
Monday-Thursday closed-week behavior (`status` for a past date has no
daily-target/EOD block at all, so the two lines never appear together
for a closed week — `t7_f10_golden_second_spec_example`,
`src/status.rs:1149-1156`, has no day-total pace-hint clause). So the
juxtaposition this finding describes is new specifically to the
*current*-week ahead case — it did not already exist for closed weeks,
because closed weeks never show the daily-target line next to their
plain-total headline. That rules out "already true today, not a new
consequence of this doc."

## Verdict

ship-with-followups
