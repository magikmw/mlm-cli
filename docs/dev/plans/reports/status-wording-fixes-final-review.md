# Final review: status-wording-fixes

Diff reviewed: `v0.3.5..status-wording-fixes` (full changeset, both
Task 1 and Task 2 merges plus the NOTES.md log commit). Spec:
`docs/dev/specs/2026-09-23-status-wording-fixes.md`. Plan:
`docs/dev/plans/status-wording-fixes-plan.md`.

## Method

Read the spec and plan in full, then diffed `src/render.rs`,
`src/status.rs`, `src/commands.rs`, `src/week_view.rs`, `README.md`,
and `docs/dev/SPEC.md` against them line by line. Built the release
binary, ran the full test suite and `cargo clippy --all-targets -- -D
warnings`, both clean.

The real system clock (Wednesday 2026-09-23) doesn't naturally land on
Friday/Saturday/Sunday, and `status`/`week`'s `--date`/`-d` flag only
changes which date's punches are displayed — it does not change what
the code treats as "today" (that's always `Local::now()`, captured
once in `main()`), so it can't be used to fake a weekday for the
capped-week hint. `faketime` isn't installed and changing the system
clock isn't appropriate here. Instead, wrote a throwaway
`examples/review_probe.rs` (deleted before finishing, not part of the
diff) that calls the library's actual public `status::run`/
`week_view::run` against a real temporary SQLite DB, with `now`
pinned to specific dates — this exercises the real code path
end-to-end, not a mock. Output below. The file was removed afterward;
`git status` confirms nothing besides the pre-existing
`docs/dev/NOTES.md` modification (present at session start, unrelated
to this review) is left dirty in the tree.

## Fix A — "Total behind" rename

`render.rs:68`'s `WeekFraming::Closed if owed_minutes > 0` branch now
reads `format!("Total behind: {}", format_minutes(owed_minutes))`,
byte-identical wording to the spec, correctly mirroring the unchanged
`"Total ahead"` sibling. All 8 `grep`-able `Total still owed` sites in
`src/` (render.rs x5 incl. tests, status.rs, week_view.rs x3,
commands.rs x2) are updated; confirmed zero hits for `Total still
owed` anywhere in `src/` or `docs/dev/SPEC.md` post-diff.

Verified live: closed week/status query renders
`Week 2026-35:  Total behind: 02h 10m` exactly.

## Fix B — `est. EOD ... (tomorrow)`

`EodState::At` is now `(NaiveTime, bool)`, matching the spec's
no-new-variant constraint. The bool is computed at the one
construction site in `resolve()` via `.date_naive()` comparison
against `today` (not the deprecated `.date()` — confirmed by
`clippy --all-targets -D warnings` passing clean). `day_total_line`
renders `, est. EOD {t}` / `, est. EOD {t} (tomorrow)` on the two
values, exactly as specified — no weekday name, no date, just the
literal suffix.

Verified live: an open stint with a large gap (started 20:00, ~32h
still owed) renders `est. EOD 04:05 (tomorrow)`.

## Fix C — `required today` on Fri/Sat/Sun

`day_reaches_week_cap` is computed once in `resolve()` as
`weekday_number.min(5) == 5` (reusing the existing `required_minutes`
computation's own `.min(5)`, no second bare `5` literal), threaded
through `DailyTargetHint`, and `day_total_line` branches on it with no
padding, matching the spec's explicit "no padding" resolution.

Verified live, all three trigger days plus the two boundary cases:

- Friday: `4h 00m worked → 36h 00m left to 40h 00m required today`
- Saturday: `38h 00m left to 40h 00m required today`
- Sunday: `38h 00m left to 40h 00m required today` (same `40h 00m`
  required figure as Friday/Saturday, confirming the "not one day"
  correction)
- Wednesday (Mon-Thu control): unchanged, `required by end of
  Wednesday`

The previously-breaking test
`resolve_f9b_sunday_pin_non_multiple_of_five_target_override` is
correctly updated to assert `required today` and explicitly asserts
`day_reaches_week_cap`. New Friday/Saturday render-level tests assert
both the wording and that the `required_minutes` figure matches
across the adjacent weekday cases — these are the load-bearing
assertions the spec called for, not just "does it compile."

## Fix D — README closed-period examples + Saturday correction

Both new README examples (closed `status` date, closed `week`) were
reproduced byte-for-byte against the real binary output using data
constructed to match the README's own numbers (week 2026-35, Mon 8:15
+ Tue 7:50 + Wed 8:00 + Thu 7:45 + Fri 6:00 = 37h50m worked/
fulfillment against a 40h target, "Total behind: 02h 10m"). Both
matched exactly, including the per-day table and the
carry-in/worked/fulfillment/target block.

The existing Saturday `status` example's line was fixed for two
independent things in the same edit, both correctly resolved:

1. `required by end of Saturday` → `required today` (Fix C's own
   scope, since Saturday triggers the cap).
2. The pre-existing `est. EOD target already met` concatenation bug
   (the renderer only ever emits one of `, est. EOD HH:MM` or `,
   target already met`, never both) is fixed to the single correct
   phrase, `, target already met` — consistent with the given
   `04h 35m over` (gap already negative) figure.

I did not reconstruct byte-identical underlying punch data for this
specific example (it predates the changeset and its literal figures
are untouched by any of the four fixes), but verified algebraically
against the code: `gap_minutes = required_minutes(2400) -
week_fulfillment(2675) = -275` → `TargetAlreadyMet` branch (matches
"target already met"), magnitude `275min = 04h 35m` "over" `40h 00m`
(matches), and `day_reaches_week_cap` is true on Saturday (matches
"required today"). The wording-shape fix is correct; the underlying
numbers were already correct pre-changeset and are untouched here.

SPEC.md: both existing closed-period worked examples (§7.1, §7.2) got
their string updated, no redundant new example added, matching the
spec's explicit "SPEC.md needs no new example" instruction. §1.2a and
README's "Known issues" both correctly drop exactly the four bullets
this changeset closes, leaving the other four (splice-gate,
anomaly-remedy, lone-unclosed-start, multi-day-old forgotten stop)
untouched in both files.

## Code quality

- No production logic changed — confirmed by inspection and by `git
  diff`'s hunks touching only string literals, the new bool field, and
  its one computation site plus one comparison site. `gap_minutes`/
  `required_minutes`/`owed_minutes` derivations are byte-identical to
  `v0.3.5`.
- `day_reaches_week_cap` and the EOD bool are both threaded as plain
  fields on existing structs rather than new enum variants, per the
  spec's explicit non-goal.
- Test coverage is substantive, not just presence-checking: new tests
  assert both the string shape and the underlying figures (e.g. the
  Friday/Saturday tests assert the *same* `40h 00m required today`
  figure on both days, directly testing the spec's own claim about
  redundancy rather than just each day in isolation).
- `cargo fmt`, `cargo build --release`, `cargo test --release` (449 +
  2 + 1 + 2 + 5 unit/integration tests), and `cargo clippy
  --all-targets -- -D warnings` all pass clean on the branch tip.

## Security

Presentation-only string formatting over already-correct, internally
computed integers (minute counts) and a fixed weekday-name formatter —
no user-controlled string data reaches any of the four changed
`format!`/`push_str` call sites. No injection surface introduced.

## Findings

None. Every one of the four fixes matches its spec section exactly,
every literal string checked against the real binary's output is
correct, the Saturday example's double-fix is genuinely resolved (not
just relabeled), tests assert the figures the spec cares about, and
the full verification set is clean.

## Verdict

Ship.
