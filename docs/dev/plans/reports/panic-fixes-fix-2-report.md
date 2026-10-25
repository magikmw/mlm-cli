# panic-fixes fix 2 — DST boundary minutes

## Base

- Worktree started on `main` (`962ac9f`), not on `panic-fixes`
- Rebased onto `panic-fixes` @ `18ffd6d` before any change (no own commits to carry)

## Cause

Confirmed with a throwaway probe test under `TZ=Europe/Warsaw` (removed before commit).
chrono's unix `Local` vs `chrono_tz`:

| typed | `Local` candidates (UTC) | `chrono_tz` |
|---|---|---|
| 2026-10-25 02:00 | Ambiguous(01:00Z, 00:00Z) | Ambiguous(00:00Z, 01:00Z) |
| 2026-10-25 02:59 | Ambiguous(01:59Z, 00:59Z) | Ambiguous(00:59Z, 01:59Z) |
| 2026-10-25 03:00 | Ambiguous(02:00Z, **01:00Z**) | Single(02:00Z) |
| 2026-10-25 03:01 | Single(02:01Z) | Single(02:01Z) |
| 2026-03-29 01:59 | Single(00:59Z) | Single(00:59Z) |
| 2026-03-29 02:00 | **Single(01:00Z)** | None |
| 2026-03-29 02:30, 02:59 | None | None |
| 2026-03-29 03:00 | Single(01:00Z) | Single(01:00Z) |

- Autumn 03:00 matches the dispatch's cause. `03:00+02:00` (01:00Z) reads back as 02:00 CET.
- Deviation from the stated cause: the same bug also hits spring-forward 02:00.
  `Local` returns `Single(02:00+01:00)` = 01:00Z, which reads back as 03:00 CEST.
  So `mlm start 02:00 --date 2026-03-29` stored 01:00Z silently instead of the
  nonexistent-time error. Same for 2027-03-28 02:00. 02:01-02:59 were already `None`.
- The one rule from the dispatch fixes both. A candidate counts only if it reads
  back in the zone as the typed wall time.

## Change

- `src/storage.rs`: `earlier_instant` replaced by `real_instant(candidates, naive_local, tz)`.
  It keeps candidates whose `tz.from_utc_datetime(..).naive_local()` equals the
  typed time and returns the earliest, or `None`. `to_utc_and_date` maps `None`
  to `NonexistentLocalTime`.
- Unit tests (storage.rs): earlier of two real candidates in either order; a fabricated
  bogus candidate dropped at autumn 03:00 in either order; a lone bogus candidate at
  spring 02:00 rejected; boundary minutes through `punch_from_local` with
  `chrono_tz` (autumn 01:59-03:01, spring 2027-03-28 01:59 and 03:00); gap minutes
  02:00/02:30/02:59 on 2027-03-28 give `NonexistentLocalTime`.
  The `chrono_tz` boundary tests passed before the fix, because `chrono_tz` never
  produces the bogus candidate; the fabricated-candidate tests reach the filter.
- `tests/panic_fixes_cli.rs` (unix, `TZ=Europe/Warsaw`): exact `at_utc` for autumn
  01:59, 02:00, 02:30, 02:59, 03:00, 03:01; spring 2026-03-29 01:59 and 03:00;
  02:00/02:30/02:59 exit 1 with the gap error and write nothing. Spring uses
  2026-03-29 because `--date` rejects future dates.

## Red before green

- CLI autumn: `left: ("2026-10-25T01:00:00Z", ..) right: ("2026-10-25T02:00:00Z", ..)` at 03:00
- CLI spring gap: 02:00 exited `Some(0)`, expected `Some(1)`
- Unit: `real_instant` not found (new function)

## Effect on fresh-eyes findings

- Finding 1: fixed. By hand, `start 01:00`, `stop 03:00` on 2026-10-25 gives `Day total: 03h 00m`.
- Finding 2: fixed for the 03:00 case. Deleting that stop now prints `mlm stop 03:00`,
  which recreates the same instant.
- Finding 4: the `02:00-02:00` stint came from 03:00 being stored as 01:00Z, so that
  example goes away. The repeated hour itself (02:xx always first pass) is unchanged, per SPEC §2.1.

## Side note

During the hand check one command ran without `MLM_DB_PATH` set and wrote to the UAT
database (`~/.local/share/mlm/mlm.db`): a start 01:00 and stop 03:00 on 2026-10-25.
Both were deleted again. That date had no punches before.

## Gates

- `cargo fmt`, `cargo build`, `cargo clippy --all-targets -- -D warnings`: clean
- `cargo test`: all green (lib 826, panic_fixes_cli 17)
