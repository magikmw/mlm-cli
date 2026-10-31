# open-punch-visibility — final review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Range**: `v0.3.6..open-punch-visibility` (tip `3b4d345`)
- **Changeset spec**: `docs/dev/specs/2026-09-26-open-punch-visibility.md`
- **Living spec**: `docs/dev/SPEC.md`
- **Reviewed**: 2026-09-30
- **Verdict**: ship-with-followups (7 findings, none blocking)

## Scope note

- `v0.3.6..open-punch-visibility` also contains the current-week-ahead-wording changeset (`267060b` and earlier). That work has its own final review and is out of scope here.
- The changeset spec says "Baseline v0.3.6 (`main` at `267060b`)". The `v0.3.6` tag is older than `267060b`, so the baseline line names two different commits.

## Mechanical gates

| Check | Result |
|---|---|
| `cargo build` | clean |
| `cargo test` | 494 lib + 10 integration, all pass |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |

## Hand runs against the binary

All runs used a scratch `MLM_DB_PATH`, backdated punches via `--date`, and local today = Wed 2026-09-30 (CEST).

| Scenario | Result |
|---|---|
| Lone open start at ages 0, 1, 2, 14, 15 | Ages 0/1 silent. Age 2 flagged on its own `status`. Ages 2 and 14 listed in today's lookback. Age 15 flagged on its own date only. Matches spec. |
| `start` into ages 2/14/15 | Warns about every *other* window date, oldest first. Stays silent about the date just written. stdout empty, exit 0. |
| Stale fix round trip (`stop 17:00 --date 2026-09-28`) | Anomaly gone afterwards. |
| `delete punch --date D` hint | Lists the offending punch. |
| Refused join, spec example (Mon 21 two opens, Tue 22 `00:45` end) | Both pages print byte-for-byte the SPEC §7.3 lines, E7/E8 fix lines included. Week 39 marks both rows and prints the footer. |
| Clean splices (mid-week Tue 22 to Wed 23, week boundary Sun 27 to Mon 28) | No `[!]` on any row or page. `spans to next day` and `continues previous day's stint` intact. |
| Splices at lookback edges (age 15 to 14, age 2 to 1) | No lookback lines. `start` warns about nothing. |
| Yesterday/today refused join | Today's `status` shows E8 plus `not joined to Tue 2026-09-29`. Week row marked. |
| `status 2026-09-30` vs `status` | Identical output, lookback included. `status -1` runs no lookback. |
| Orphan-only window date | No lookback line. |
| Backdated `start` into a stale date that already has an open start | Prints the multi-open warning form. |
| Backdated `start` into an empty stale date | No warning. |
| `TZ=Pacific/Kiritimati` (local today 10-01, UTC 09-30) | Ages and the window follow the local date. Age 2 = 09-29, stale. |
| Same DB under `TZ=America/Los_Angeles` | Ages follow the stored `date` column against LA's today. Times display in LA local. Consistent with SPEC §2.1. |
| ASCII | No non-ASCII bytes in any new output. |

## Security

- Every value interpolated into `fix:`/`or:` lines and the `start` stderr warning is a `NaiveDate` (via `format_date`/`format_date_with_weekday`), a `NaiveTime` (`%H:%M`), or an integer.
- No note text or raw CLI string reaches these lines. `--date` input is parsed to `NaiveDate` before it is formatted back out. No injection path exists.
- The hints are not verbatim-runnable by design, because `HH:MM` is a literal placeholder.
- Pasting a whole hint line, parenthetical included, fails to parse in bash, fish, and PowerShell. Two things cause the failure. The `'` in `start's` leaves a quote unbalanced, and the parenthetical opens a subshell or command substitution. So nothing runs.
- In fish or PowerShell, a user who edits `HH:MM` but keeps the `(a time after 09:00)` tail would trigger a command substitution of `a time after 09:00`. That does nothing harmful on ordinary systems. This is not a finding.

## Findings

### F1 — SPEC E15, SPEC §7.2 and the README overnight paragraph contradict refused-join behavior (medium, docs)

- **SPEC §8.2 E15**: when the splice shape fails, it still says day one falls back to "an ordinary (non-anomalous) open `start` ... silent in `week`'s view (no `(ongoing)`, no `[!]`, §7.2)".
  - The binary now marks both rows `[!]` at every age and prints refused-join lines. E17 says the same.
  - E15's own example is "two dangling starts", which has always been E7, an anomaly.
- **SPEC §7.2**, the rewritten "accepted, silent consequence" passage:
  - It still calls the fallback date "an ordinary single-trailing-start under §4.3 (not an anomaly on its own)".
  - It says the date gets `[!]` only "once that date turns 2 or more calendar days old".
  - A refused-join date gets `[!]` at age 0 and age 1 too. Verified on the yesterday/today pair.
- **README "An overnight session..." paragraph**: "a second stray start ... leaves ... an ordinary open start on the first date". A second start on the first date makes it E7, not an ordinary open start.
- **Fix**: rewrite E15's fallback clause and the §7.2 passage to say that a refused shape flags both dates (§7.3 refused join) whatever its age. Correct the README sentence to match.

### F2 — The stale and E7 `stop` hint bound can make overlapping stints (low-medium, behavior; spec-level)

- The hint names only the open start ("a time after 09:00"). Pairing is LIFO (§4.3).
- When the date has completed stints *after* the open start, an end placed between the open start and those stints re-pairs everything into nested, overlapping stints. The day total inflates and no anomaly fires.
- Reproduced:

```
2026-09-24: start 09:00, start 10:00, stop 12:00
status  -> [!] unclosed start at 09:00 ... (a time after 09:00); Day total 02h 00m
mlm stop 11:00 --date 2026-09-24      # follows the hint
status  -> 09:00-12:00 (03h 00m), 10:00-11:00 (01h 00m); Day total 04h 00m, no [!]
```

- E7's "a time after 14:00" has the same hole when a completed stint sits after the latest open start.
- **Fix**: bound the hint by the date's last punch, e.g. `(a time after 12:00, the date's last punch)`. Otherwise SPEC §7.3's rationale for the wording is incomplete.

### F3 — The `start` written-date rule wrongly silences a pre-existing stale start (low, behavior; spec-level)

- Changeset §3.6 and SPEC §7.4 assume that "a single open start there is the punch just written". That is false when the new `start` pairs with an existing orphaned end on the same date. The date's remaining single open start is then the *old* one.
- Reproduced by following an E8 hint on a stale date:

```
2026-09-24: stop 17:00, start 18:00      (E8 + stale 18:00)
mlm start 08:00 --date 2026-09-24        # E8 fix hint
-> no warning about the still-unclosed 18:00 start on 2026-09-24
```

- `status` still shows it, so the only loss is the one warning.
- **Fix**: suppress the line only when the single open start's punch id equals the id just inserted. `punch()` can return the id.

### F4 — The test suite does not pin which neighbor a refused join names (low, tests)

- Spec §5 asks for "both dates with the right neighbor named".
- The `status` tests only assert `contains("not joined to")`. The `render.rs` tests pin the text for given facts. The `week_view` tests check only `has_anomaly`.
- Mutation run: swapping `pred_opt` for `succ_opt` in `status::anomaly_facts_for_date`'s `refused_join_with_previous` left all 494 tests passing.
- The hand runs confirm the current wiring is right.
- **Fix**: assert the full refused-join line on both dates in at least one `status` test.

### F5 — `start()`'s written-date wiring and the CLI-level warning are untested (low, tests)

- Mutation run: passing `now.date_naive()` instead of `written_date` into `warn_stale_starts` left all tests passing.
- The `emit_warnings` tests feed synthetic findings. Nothing covers these spec §5 items end to end:
  - backdated `start --date D` into a stale date that has an open start warns with the multi-open form
  - `start` into an empty stale date does not warn
- `start_stdout_stays_empty_and_exit_is_zero` checks the exit code only and captures neither stdout nor stderr. No `tests/` CLI test covers `start`'s stderr at all.
- The lookback's lower bound (age 1 excluded) is caught only by `debug_assert!`s in `render.rs`. No assertion covers it.
- **Fix**: add one CLI test that seeds a stale open start, runs `start --date` against it and another date, and asserts stdout empty, stderr lines exact, and exit 0.

### F6 — SPEC and README misstate the lookback window and the fix-line rule (low, docs)

- SPEC §7.1's lookback bullet, §8.2 E18, and README's `status`/`start` sections all say "every date in the last 14 days that still has an open start".
  - The window is ages 2 to 14. Yesterday's open start is excluded.
  - SPEC never states the age-2 lower bound for the lookback outright.
- SPEC §7.3 opens with "Every `status` anomaly line also carries one or two indented `fix:`/`or:` lines". A few paragraphs later it shows the refused-join line with none.
- **Fix**: say "dates 2 to 14 days old". Qualify "every" in §7.3.

### F7 — Code-comment and structure nits (nit, code quality)

- New doc comments narrate the plan: "Task 1's predicate", "Task 3's concern", "Contract B", "Landmine 2". They should describe present behavior.
- Code comments cite changeset-spec section numbers (§3.1, §3.6) that collide with `SPEC.md`'s own numbering. For example, `commands::start`'s "(§3.6)" means the changeset spec, but SPEC §3.6 is `mlm week`.
- `status::anomaly_facts_for_date` and `week_view::row_anomalies` build the same `DateAnomalies` twice. They already differ slightly: `latest_open_start` uses `.max()` over local times in one and `.last()` in the other. One shared builder would stop the two from drifting.

## Spec compliance summary

| Item | Status |
|---|---|
| Changeset §2 terms (age, open start post-`classify_at`, window 2 to 14, refused join) | Implemented as defined. Raw-classification refused-join check is equivalent to the post-`classify_at` definition. |
| §3.1 stale anomaly, E7 suppresses the stale line | OK |
| §3.2 refused join on both dates, all ages | OK (see F4 for test gap) |
| §3.3 lookback, today only, explicit today included | OK |
| §3.4 line shapes and order | OK. Byte-identical to spec examples. |
| §3.5 week `[!]` and footer | OK |
| §3.6 `start` stderr warning, best-effort | OK per spec text (see F3 for the rule's flaw, F5 for test gaps) |
| §3.7 ASCII | OK |
| SPEC §1.2a, §4.3, §4.3.1, §7.1 to §7.4 | Updated. See F1 and F6. |
| SPEC §8.2 E7, E8, E16 to E19 | Present. E15 is now stale (F1). |
| README examples vs binary | Line shapes match the binary exactly. The overnight paragraph is off (F1). |

## Verdict

Ship with follow-ups. The behavior is correct against the spec, and every worked example reproduces exactly. The window and age boundaries hold under hand runs, and so does the local-date handling. Clean splices, including across week boundaries, raise no false refused joins. F1 and F6 are doc fixes. F2 and F3 are small spec-level refinements to hint wording and warning suppression. F4 and F5 close test gaps that mutation runs showed.
