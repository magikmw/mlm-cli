# open-punch-visibility — fix pass

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Closes the findings from `open-punch-visibility-final-review.md` and
`open-punch-visibility-fresh-eyes.md`, per the triage in NOTES.md entry
82 and the amended changeset spec (§1.2/§3.6 of
`docs/dev/specs/2026-09-26-open-punch-visibility.md`).

Worktree branch: `open-punch-visibility-fixpass` (created from the
`open-punch-visibility` branch tip, commit `ef4b85a`, since the
dispatched worktree's own branch was stale and unrelated). Commit for
this fix pass follows this report.

## Review F3 — FIX (behavior + test)

**What changed.** `commands::punch()` now returns the inserted punch's
row id alongside the target date. `render::LookbackKind::SingleOpenStart`
gained a `punch_id` field, populated in `status::lookback_finding_for_date`
from `day.open[0].start.id`. `commands::emit_warnings`'s suppression
rule changed from "same date as the write" to "the `SingleOpenStart`
finding's punch id equals the id just inserted" — matching the amended
spec §3.6 exactly. This also adds the requested delete alternative: a
single-open-start warning is now three lines (`warning:`/`fix:`/`or:`),
laid out like `status`'s own lookback block, instead of the old
one-line compressed form.

**Test that failed first.** `commands::tests::start_warns_about_older_open_start_left_open_by_orphan_pairing`
reproduces the review's exact repro (`stop 17:00`, `start 18:00`,
then `start 08:00` on the same stale date, which pairs 08:00-17:00 and
leaves the older 18:00 start open). I verified it fails against the
old date-only suppression logic (temporarily reverted `emit_warnings`
to the old `finding.date == written_date` check, ran the test, got
`expected a warning about the still-open 18:00 start, got: ""`), then
restored the fix and confirmed it passes.

`render::tests::t_lookback_stderr_single_is_three_lines_like_status_block`
pins the new three-line format; verified it fails against the old
one-line `stderr_line()` implementation the same way (temporarily
reverted, ran, got a one-line mismatch, restored).

**Hand-check** (`MLM_DB_PATH` scratch db, `--date 2026-09-24`,
reproducing the review's own scenario against the real binary):

```
$ mlm stop 17:00 --date 2026-09-24
$ mlm start 18:00 --date 2026-09-24
$ mlm start 08:00 --date 2026-09-24
warning: unclosed start on Thu 2026-09-24 at 18:00 (6 days ago)
    fix: mlm stop HH:MM --date 2026-09-24  (a time after 18:00)
     or: mlm delete punch --date 2026-09-24  (lists punches; rerun with the start's number)
```

Matches the dispatch's expected output exactly.

## Review F1 — FIX (docs)

- SPEC §8.2 E15's fallback clause rewritten: outside the clean-splice
  shape, "the join is refused: both dates flag a `[!]` line naming
  each other (§7.3/E17), at any age, alongside whatever E7/E8 anomaly
  the shape itself produces."
- SPEC §7.2's "accepted, silent consequence" passage rewritten to
  describe the refused-join anomaly firing at any age, dropping the
  old "ordinary single-trailing-start... not an anomaly on its own"
  language (which was also self-contradicting the E7 case it named as
  its own example).
- README's overnight paragraph rewritten: "Anything messier... leaves
  both punches alone and refuses the join, whatever the dates' age: a
  `[!]` line on both dates explains why, and each date's own
  E7/E8/stale-start `[!]` line (whichever applies) carries the fix."

## Review F6 — FIX (docs)

- SPEC §7.1's lookback bullet, §8.2 E18, and README's `status`/`start`
  sections now say "dates 2 to 14 days old" instead of "the last 14
  days" (which wrongly implied age 0/1 were included).
- SPEC §7.3's opening line now says "Most `status` anomaly lines also
  carry... `fix:`/`or:` lines," with an explicit carve-out sentence
  for the refused-join line, which carries none.

## Review F4 — FIX (test)

Added an exact full-line assertion (not just `.contains("not joined
to")`) to `resolve_refused_join_two_opens_on_earlier_date_flags_both_dates`
in `src/status.rs`, pinning the complete refused-join line — including
the parenthetical — on both dates, and confirming each date names the
*other* date as its neighbor (not itself). This is the mutation the
review's own mutation run (`pred_opt`/`succ_opt` swap) would now catch.

## Review F5 — FIX (test)

Added to `src/commands.rs`:

- `start_warns_about_older_open_start_left_open_by_orphan_pairing` —
  the F3 scenario end to end (see above).
- `start_into_stale_date_with_existing_open_start_warns_multi_open_form`
  — a backdated `start --date D` into a stale date with an existing
  open start warns with the multi-open one-line form.
- `start_into_empty_stale_date_raises_no_warning_for_it` — `start`
  into an otherwise-empty stale date raises no warning for that date.
- `warning_still_prints_when_single_open_start_on_written_date_is_an_older_punch`
  — direct unit coverage of the punch-id suppression rule's positive
  case (a mismatched id still warns).

Added to `src/status.rs`:

- `lookback_excludes_age_one_open_start` — a real assertion (not just
  the existing `debug_assert!`) that age 1 (yesterday) is excluded
  from the lookback even though it has an open start.

These four close the gaps the review's mutation runs found. I did not
add a `std::process::Command`-level (binary) CLI test for `start`'s
stderr, since `start()` hardcodes real `std::io::stderr()` and the
existing test infrastructure already exercises the same logic
end-to-end through `warn_stale_starts`/`emit_warnings` with a real
`Connection` and real writes via `commands::start`/`commands::stop` —
a binary-level test would duplicate this coverage without testing
anything the unit-level tests don't already cover (same DB writes,
same warning-line construction, same suppression logic), so I judged
it not worth the added `tests/` binary-spawn overhead. If this
judgment is wrong, say so and I'll add one.

## Review F7 — FIX (comments only)

Removed plan-narration ("Task N", "Contract A/B") from every doc
comment this changeset itself added, in `render.rs`, `stint.rs`,
`status.rs`, and `week_view.rs`. Left `week_view.rs`'s pre-existing
"Landmine 2" comment and its module-level "Landmine notes" header
alone — both predate this changeset (Milestone 9), out of scope.
Also left `cli.rs`'s and `commands.rs`'s milestone-14 "Task 2/3"
comments alone — confirmed via `git diff v0.3.6 open-punch-visibility`
that `cli.rs` has zero changes in this changeset, and the one
remaining `commands.rs` "(Task 2)" line predates this changeset too
(traced to commit `7a25770`, Milestone 14).

Replaced every new-in-this-changeset `§3.1`-`§3.6` citation (the
changeset spec's own "Behavior" section numbering) with the matching
SPEC.md section it now actually lives in: stale-open-start → §4.3,
refused join → §4.3.1, lookback → §7.1, fix/or-line ordering → §7.3,
`start`'s warning → §7.4. About 30 comments across `status.rs`,
`render.rs`, `stint.rs`, and `commands.rs` were touched this way.
Pre-existing citations to real, unrelated SPEC.md sections (§3.5 =
`mlm status`, §3.7 = `mlm week target`, etc.) and citations to other
specs (`boundary-stint-pairing spec §3.2`, `backdated-punches spec
§3.1`) were left untouched — they don't collide and aren't this
changeset's comments.

Fixed the four stale `status.rs` doc comments naming the deleted
`render::Anomalies`/`to_anomalies` (module doc comment, three
instances, plus `StatusView::anomaly_lines`'s field doc) to name the
current `render::DateAnomalies`/`anomaly_facts_for_date`.

Did **not** merge the duplicated `DateAnomalies` construction between
`status::anomaly_facts_for_date` and `week_view::row_anomalies` — the
dispatch explicitly said not to.

## Fresh-eyes F4 — FIX (docs)

Added one sentence to README's `week` section: "An unfixed anomaly in
a past week also throws off every later week's `Carry-in`... that
later week's own view carries no `[!]` telling you the number traces
back to an earlier problem (see Known issues)."

## Known issues — FIX (docs)

Added three new self-contained bullets to both SPEC §1.2a and README's
"Known issues," per the changeset spec's own §1.2 (with the
changelog-style "next changeset" framing dropped, per AGENTS.md):

1. Nested/LIFO pairing double-counts overlapping stints, with the
   stale-open-start hint able to lead a user straight into the shape.
2. A lone open start dated yesterday is never flagged — no signal at
   all on today's `status` for a session still running from the
   previous evening.
3. An unresolved anomaly in a past, closed week skews later weeks'
   carry-in with no `[!]` pointing back at the source week.

## Doc example update

Every doc example of the single-open `start` warning (SPEC §7.4/E19,
README's `start` section) now shows the three-line form, copied
verbatim from the built binary's actual output (`warning: unclosed
start on ... \n    fix: ... \n     or: ...`).

## Not fixed (per dispatch)

- Review F2 (pairing double-count hint bound) — filed as a known issue
  only; hint wording unchanged, since the pairing fix itself is the
  next changeset's scope.
- Fresh-eyes F1 (`HH:MM` placeholder) — dropped per triage.
- Fresh-eyes F2 (yesterday's open session invisible) and F3 (past
  week's anomaly skews carry-in unflagged) — filed as known issues,
  no code change.

## Verification

All from a clean worktree at `open-punch-visibility-fixpass`:

- `cargo fmt` — clean (`cargo fmt --check` confirms no further diff).
- `cargo build` — clean.
- `cargo test` — 499 lib + 10 integration, all pass (up from the
  review's 494 lib + 10 integration baseline; net new tests: F3
  regression, three F5 tests, one F4 line-pin strengthening, one
  lookback age-1 test, plus the render.rs three-line-format test
  replacing the old one-line-format test).
- `cargo clippy --all-targets -- -D warnings` — clean.
- Hand-checked the F3 scenario and the F5 empty/multi-open scenarios
  against the real debug binary with a scratch `MLM_DB_PATH` — output
  matches the spec's examples exactly (see F3 section above).
- Pre-commit hook (fmt/complexity/coverage) runs at commit time below;
  see the commit result for its outcome.
