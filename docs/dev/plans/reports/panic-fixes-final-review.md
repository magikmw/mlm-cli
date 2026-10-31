# panic-fixes — final review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- **Baseline**: `main` at `dd91952`
- **Changeset**: `panic-fixes` at `5ec515c` (clone `/tmp/cd-scratch/panic-fixes-final`, HEAD verified)
- **Spec**: `docs/dev/specs/2026-10-25-panic-fixes.md`, SPEC §1.2a, §2.1, §3.7, §4.1, §6.1, §6.3, §7 intro, §8.2 E32-E34
- **Verdict**: ship-with-followups (2 low findings, no correctness or security issues)

## 1. Verification set

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | clean |
| `cargo test` | 978 passed, 0 failed |
| `cargo clippy --all-targets -- -D warnings` | clean |

## 2. DST conversion (`src/storage.rs` `to_utc_and_date` / `real_instant`)

### 2.1 Exhaustive probe

I wrote a probe (`examples/dst_probe.rs` in the clone, not on the branch). It finds every offset transition in 2025-2028 and checks two things for each.

- Every local minute within 4h of the transition, typed. `punch_from_local` must return the earliest UTC instant that reads back as that wall time, with `date` equal to the typed date and a display that round-trips. When no such instant exists, it must return the gap error.
- Every UTC minute within 4h of the transition, as a `now` capture. Its own local wall time, re-stored, must give the earliest real instant, never later than the real one.

Each zone runs twice, once through unix `Local` (TZ set) and once through `chrono_tz`. Each source is checked against brute-force truth built from its own `from_utc`. A first version used `chrono_tz` as truth for `Local` too, and found 0 display or storage mismatches except Africa/Casablanca. There the box's tzdata 2026e and chrono-tz's bundled data disagree about Morocco after 2026, so the mismatch is in the data, not the code.

| Zone | Why | Local | chrono_tz |
|---|---|---|---|
| Europe/Warsaw | named case | 0 bad / 7680 | 0 bad / 7680 |
| America/New_York | named case | 0 / 7680 | 0 / 7680 |
| Australia/Lord_Howe | 30-minute shift | 0 / 7680 | 0 / 7680 |
| America/Santiago | southern, midnight transitions | 0 / 7680 | 0 / 7680 |
| Australia/Sydney | southern | 0 / 7680 | 0 / 7680 |
| Europe/Dublin | negative DST in tzdata | 0 / 7680 | 0 / 7680 |
| Africa/Casablanca | negative DST, Ramadan | 0 / 4800 | 0 / 7680 |
| America/St_Johns, Pacific/Chatham, America/Havana | odd offsets, midnight shifts | 0 | 0 |
| Asia/Tokyo, Asia/Kolkata, Asia/Tehran, UTC | no DST | no transitions | no transitions |
| POSIX rules (`CET-1CEST,...`, `EST5EDT,...`, a +10:30/+11 rule, the test file's `AAA0BBB-1,...`, a midnight `/24` rule) | `Local` rule-string path | 0 / 7680 each | n/a |

### 2.2 Through the binary

`dst_manual.sh` in the clone runs typed `start` at every boundary minute below. For each one it then runs `status DATE`, `week`, `delete punch` list mode and `delete punch 1`. Last, it replays the printed recreate line through `bash` and compares the stored `(at_utc, date)` before and after.

- Europe/Warsaw 2026-10-25: 01:59, 02:00, 02:30, 02:59, 03:00 (stores `02:00Z`), 03:01
- Europe/Warsaw 2026-03-29: 01:59, 03:00 stored; 02:00 and 02:59 rejected with the gap error
- America/New_York 2025-11-02: 00:59-02:00. America/New_York 2026-03-08: gap rejected, edges stored
- Australia/Lord_Howe 2026-04-05: 01:29-02:01 (repeated 01:30-01:59 takes the +11 pass). Australia/Lord_Howe 2025-10-05: 02:00 and 02:29 rejected, 02:30 stored
- America/Santiago 2026-04-04 23:00-23:59 (repeated hour before midnight) and 2026-04-05 00:00. America/Santiago 2025-09-07: 00:00-00:59 rejected, 01:00 stored
- Asia/Tokyo: 00:00, 02:30, 03:00, 23:59

Every stored punch showed its typed wall time in `status`, `week` and the list. Every recreate line printed the typed `HH:MM` with the right `--date`, and every replay stored the identical row. No mismatches.

I also ran untyped `start`, `stop` and `note` under a TZ rule that put the real clock in the second pass. All exited 0. The untyped `start` stored the first-pass instant, one hour before the real clock (§2.1, §4.1), and the note kept the real `created_at_utc`. `status` and `week --notes` rendered without error.

### 2.3 Mutations

| Mutation | Caught by |
|---|---|
| `real_instant` filter removed | 2 unit tests, `fall_back_boundary_minutes_store_the_real_instant`, `spring_forward_gap_minutes_are_rejected` |
| `.min()` → `.max()` | 3 unit tests, 3 integration tests (typed and untyped second pass) |
| `now` capture reverted to `Local::now().with_second(0).unwrap()` | both repeated-hour integration tests |
| cap `>` → `>=` | `cap_boundary_values_are_accepted` |
| broken-pipe arm → exit 1 | `broken_pipe_output_error_exits_141_silently` |

The `real_instant` unit tests feed it the bad `LocalResult`s that unix `Local` produces, built from `chrono_tz`. So the storage-level tests still bite on a runner where `Local` is UTC. The binary tests with `TZ=Europe/Warsaw` exercise the real `Local`.

## 3. Week target cap (E32)

By hand, all of these matched SPEC §3.7 and E32 exactly:

- `168h`, `10080m` and `167h60m` echo `168h 00m`
- `168h1m`, `200h` and `99999999999h` print `(got 168h 01m)`, `(got 200h 00m)` and `(got 99999999999h 00m)`, exit 1, empty stdout, and leave the stored row unchanged
- `-5h`, `10x` and `9999999999999999999h` keep their messages
- `mlm week 2026-41 target 200h` prints the reminder and exits 2
- the `--help` `DURATION` line matches §7
- `status` with a 168h target and an open stint exits 0 and shows `est. EOD 20:11 (Sun 2026-11-01)`

## 4. Output write failures (E33, E34, §6.3)

- **Stdout sites.** Every stdout write in `src/` goes through `emit` (`status`, `week`, `week target`, `export notes`, both `delete` list modes and recreate lines). No `println!` or `print!` is left outside `examples/`. `start`, `stop` and `note` write nothing to stdout.
- **Stderr sites.** Every stderr site ignores a failed write: `warn_stale_starts`, `warn_orphaned_ends`, main's `error:` line, the swapped-target reminder, and clap's `err.print()`.
- **Closed reader.** With `{ sleep; mlm ...; } | true`, `week`, `status`, `export notes`, `week target 38h`, `delete note` (list), `delete note 1` and `delete punch 1` all exit 141 with empty stderr. The target was stored and the deletions held.
- **`/dev/full`.** `week` and `week target 30h` print `error: cannot write output: No space left on device (os error 28)` and exit 1. The target was stored.
- **Failing stderr.** With stderr on `/dev/full` or closed, a malformed `TIME` exits 1, the swapped shape exits 2, a usage error exits 2, and `start`/`stop` with warnings exit 0.
- **Exit mapping.** `exit_code` in `src/main.rs` matches §6.3. It returns 141 only when the top-level error is an `OutputError` of kind `BrokenPipe`. A storage-side I/O error keeps its own message and exits 1 (`storage_io_error_keeps_its_own_message`).

## 5. Security

The diff does not change how either recreate line is built. It only moves where the line is written. I replayed a note body containing `'`, `$(touch ...)`, backticks and `--date x` through `bash`. Nothing ran, and the body came back byte-identical. Punch recreate lines contain only `HH:MM` and a validated date. The diff adds no SQL built from strings.

## 6. Findings

### F1 — low — repeated-hour integration tests skip silently about an hour a day

- **Location**: `tests/panic_fixes_cli.rs:323` (`repeated_hour_tz`), `:346` (`skip`), `:363`, `:373`
- **Problem**: The TZ rule uses standard offset 0, so local time equals UTC. `repeated_hour_tz` returns `None` whenever the repeated hour would cross midnight. For the second-pass test that happens from 23:30 to 00:30 UTC; for the first-pass test, from 22:30 to 24:00 UTC. Both tests also skip on 1 January. In those windows the tests pass without asserting anything, and the skip note is swallowed by the test harness. One of them, `repeated_hour_second_pass_status_and_untyped_start`, is the only end-to-end check of spec §6's untyped-start row.
- **Fix**: Choose the rule's standard offset from `now` so the local clock sits near midday (e.g. `std = 12 - now.hour()`, with DST one hour ahead). The repeated hour then never crosses a date, and the `None` and skip paths can go.

### F2 — low — `mlm::exit_code` disagrees with the binary's §6.3 mapping

- **Location**: `src/lib.rs:28-35`, used from `src/week_view.rs:1292` and `src/commands.rs:693,697`
- **Problem**: The library still exports `exit_code`, which maps every `Err` to 1, while `src/main.rs` has its own `exit_code` that adds 141. The doc comment now calls the library one "test-side", but it is `pub` and named as if it were the real mapping. A future caller or test can get 1 for a broken pipe with no hint that it is wrong.
- **Fix**: Move the 141 arm into `mlm::exit_code` (or a pure `mlm::exit_status(&Result) -> i32`) and have `main` call it and print the `error:` line separately. Or drop the library function and change the three test call sites to `is_ok()`.

## 7. Notes for merge (not findings)

- Archive the changeset spec and plans with the `Archived` banner per `docs/dev/README.md`, as done for `export-notes`, once this is merged.
- The probe files `examples/dst_probe.rs` and `dst_manual.sh` exist only in the review clone. They are not on the branch.

## 8. Verdict

**ship-with-followups.** All three panics and the boundary-minute storage bug are fixed. Behavior matches the spec sections, and every load-bearing test fails under mutation. F1 and F2 are low-severity test and code-shape follow-ups.
