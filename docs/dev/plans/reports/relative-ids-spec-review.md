# relative-ids spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Spec: `docs/dev/specs/2026-10-25-relative-ids.md` (branch `relative-ids` at `6eb8849`, baseline `main` `77fb19f`)
- Method: read the spec, SPEC.md §1.2/§3.6-§3.8/§6.1/§8, README, and the cited source. I built `mlm` into `/tmp/cd-scratch/relids/mlmtarget` and probed it against a scratch db. I also built a clap 4.6.6 harness in `/tmp/cd-scratch/relids/src/main.rs` that copies `WeekArgs`/`WeekTargetArgs` in three variants: `allow_negative_numbers`, `allow_hyphen_values`, and baseline.

## Clap achievability

These are the harness results for clap 4.6.6, `args_conflicts_with_subcommands = true`.

| Input | baseline (today) | `allow_negative_numbers` on `week_id` | `allow_hyphen_values` on `week_id` |
|---|---|---|---|
| `week -1` | exit 2, unexpected `-1` | `week_id="-1"` | `week_id="-1"` |
| `week -1 -n` | exit 2 | ok, notes=true | ok, notes=true |
| `week -n -1` | exit 2 | ok, notes=true | ok, notes=true |
| `week -01` / `week -0` | exit 2 | reaches parser | reaches parser |
| `week -1x` | exit 2 | **exit 2**, `unexpected argument '-1' found` | reaches parser (exit 1) |
| `week -x` | exit 2 | exit 2 | **reaches parser as WEEK_ID** (exit 1) |
| `week -n` | notes=true | notes=true | notes=true |
| `week -1 target 35h` | UnknownArgument | ArgumentConflict, so `classify` runs the swap check | same |
| `week target -1 35h` | reaches parser | reaches parser | reaches parser |
| `week -v -1`, `week -1 -v` | exit 2 | ok | ok |

`ex n -w -1` and `ex n -m -1` already reach the parsers (`src/cli.rs:325,330`, `allow_hyphen_values`). I confirmed this live: both exit 1 with the shape error today. `week target -1 35h` already reaches `parse_week_id` (`src/cli.rs:286`); live it gives `invalid WEEK_ID "-1": expected YYYY-WW or WW`. Every shape in §3 can be parsed. The swapped reminder works under either variant because `classify` (`src/cli.rs:380-392`) runs on any clap error, and `is_week_token` (`src/cli.rs:443-445`) will accept `-1` once `parse_week_id` does.

## Findings

### F1 — medium — internal inconsistency: `mlm week -1x` cannot give exit 1 under the mechanism the spec names

- Claim: §3 (spec:49-51) says `mlm week -1` takes `-1` "as `mlm status -1` takes it". §4 (spec:73) says "All exit `1`". §5 (spec:89) says "`-0` and `-1x` give the shape errors".
- Evidence: `status` uses `allow_negative_numbers = true` (`src/cli.rs:103`). Live, `mlm status -1x` exits 2 with `error: unexpected argument '-1' found` plus clap's `tip: to pass '-1' as a value, use '-- -1'`. The harness shows the same for `week -1x` under `allow_negative_numbers`. That message names `-1`, which is now a valid value, and the tip is misleading. `allow_hyphen_values` does deliver exit 1 for `-1x`. It also turns `mlm week -x` and `mlm week --bogus` into `invalid WEEK_ID` errors with exit 1 where they exit 2 today. That is a visible change the spec does not record.
- `-1x` does reach the parser for `week target` and `export notes`, which already take hyphen values, so the exit-1 claim holds there.
- Fix: pick the clap setting in §3. With status parity, §4 should say `mlm week -<non-number>` is a parser error with exit 2, and §5's `-1x` CLI case should be scoped to `week target`/`export` or to `parse_week_id` unit tests. With `allow_hyphen_values`, record the `-x`/`--bogus` exit-code change.

### F2 — medium — missing edge case: the offset floor is undefined, and the obvious implementations break `WeekId`'s invariant or panic

- Claim: §4 (spec:79-80) says "`-N` too large to resolve" gives the offset errors. §5 (spec:89-90) says "a huge N gives the offset errors".
- Evidence:
  - `WeekId::new` enforces `1000..=9999` (`src/date.rs:243`). `WeekId::from_date` does not check it (`src/date.rs:260-266`). So `today - 7N days` followed by `from_date` can build a `WeekId` with `iso_year < 1000`, which breaks the invariant documented at `src/date.rs:232-233`.
  - `week target` would then store a key like `0999-52`. `week_targets_in_range` turns any such key in a read range into `CorruptRow` (`src/storage.rs:578`).
  - `WeekId::prev` asserts on underflow below year 1000 (`src/date.rs:355-358`), so an N-times `prev()` loop panics at the floor and takes forever for a huge N.
  - `DATE`'s `-N` clamps to years 1000-9999 for exactly this read-back reason (`src/date.rs:150-157`). `Month::new` accepts years 0-9999 (`src/date.rs:412`, SPEC §3.8:459-460). So the MONTH floor could be 0000-01 or 1000-01, and the spec does not say which.
  - The existing `OutOfRange` arm for `WeekId` prints `week number must be 1 or greater` (`src/date.rs:83`). Reusing `Cause::OutOfRange` for an offset failure would print that text, not §4's.
- Fix: state the floor for each kind (WEEK_ID: the resolved ISO year must be ≥ 1000; MONTH: say 0000 or 1000). State that u64 overflow of the digit run counts as out of range, as it does for DATE (`src/date.rs:137-142`). Add boundary tests in §5 that mirror DATE's (`src/date.rs:1293-1341`): the last accepted N, the first rejected N, and digit-run overflow.

### F3 — low — testing gap: existing tests that assert `-N` is rejected will flip, and §5 does not list them

- Claim: §5 (spec:84-95) lists only new tests.
- Evidence: these tests assert the old rejection.
  - `src/date.rs:743` (`"-5"`, T57), `:745` (`"-07"`, T59), `:752` (`"-1"`, T65) assert WEEK_ID shape errors. T65's comment cites the §1.2 non-goal. `"-07"` now means seven weeks ago because §3 tolerates leading zeros.
  - `src/date.rs:1153` (`"-1"` in the MONTH bad-shape list) and `:1208` (the `-1` MONTH message).
  - `src/export.rs:169` asserts `-w -1` is `(WeekId, Shape)`.
  - 12 `.rs` assertions of `expected YYYY-WW or WW` / `expected YYYY-MM or MM` across `src/date.rs` and `tests/export_notes_cli.rs`.
- Fix: add a §5 bullet saying these become acceptance tests, or move them to inputs that are still malformed.

### F4 — low — internal inconsistency: the doc-edit lists understate what changes

- §6 (spec:97-101) does not name the README non-goals bullet at `README.md:42-43` (`+N`/`-N` relative week notation...). That bullet becomes wrong.
- SPEC §3.8:465-467 says "`-w -1` and `-m -1` reach their own `WEEK_ID` and `MONTH` errors with exit `1`". That sentence becomes false. It needs rewriting, which goes beyond "gain the `-N` form" (spec:106).
- SPEC §3.8:485-486 quotes the old `-w`/`-m` help lines.
- SPEC §6.1:746 quotes `expected YYYY-MM or MM`. That message is replaced, not "gained" (spec:107). The §6.1 WEEK_ID bullet (SPEC:735-741) quotes no message today, so it has nothing to "gain" onto.
- Fix: list each line that changes, so the fold does not leave stale contradictions in SPEC.md and README.

### U1 — undeclared user-visible gap — `1` and `-1` are one keystroke apart and both valid

`mlm week 1` shows ISO week 1 of this year, and `mlm week -1` shows last week. With `week target`, a dropped or extra hyphen writes the wrong week's target and still exits 0. `export notes -m 1` is January, and `-m -1` is last month. §1.2 says "None" (spec:28).

### U2 — undeclared user-visible gap — relative ids change meaning with the calendar

`mlm ex n -w -1 > report.txt` from shell history or a script gives a different week each Monday, and `-m -1` flips at each month's 1st. The swapped reminder echoes `-1` as typed (spec:52-54). If the user runs the suggested command after the week rolls over, it targets a different week than the one they meant. The spec says the current week comes from today (spec:46) but does not record either consequence.

### U3 — undeclared user-visible gap — `export notes` never shows which range `-N` resolved to

`export notes` prints no header (`README.md:635`, `src/export.rs:42`). An empty month or week prints nothing and exits 0 (SPEC §3.8:469-470). So `mlm ex n -m -1` gives no way to confirm which month it meant. `mlm week -1` shows the resolved id in its header, but export does not.

### U4 — undeclared user-visible gap — `mlm week target -1` alone is a DURATION error

With one token, that token is DURATION (`src/cli.rs:458-462`). Live, `mlm week target -1` prints `invalid duration "-1": expected a form like 20h, 33h30m, or 45m` and exits 1. That is not a missing-argument error. The behavior already exists, but the user is more likely to hit it now that `-1` means a week.

### U5 — undeclared user-visible gap — `-0` has no meaning

`mlm week -0`, `ex n -w -0` and `ex n -m -0` fail (spec:77-78), even though "this week" is the default. The rule is declared. Its consequence is not: there is no relative spelling for the current week or month, and a script that computes N must special-case 0.

### U6 — undeclared user-visible gap — `+1` gets a message that lists `-N` but not the way forward

`mlm week +1` prints `expected YYYY-WW, WW or -N (N >= 1)`. Nothing says that future weeks take an absolute id, although §1.1 (spec:21-22) relies on exactly that. The user is left to guess.

### U7 — undeclared user-visible gap — past-week target edits are now easy, and they move today's numbers

`mlm week target -1 35h` changes last week's fulfillment. That changes carry-in, so today's `status` headline and `mlm week` change at once (SPEC §2.4/§5). The absolute form allowed this already. The relative form makes it a one-token habit, and the spec does not mention the knock-on effect on the current week.

## Verdict

ship-with-followups. F1 and F2 need spec text before planning: the clap setting with its exit code, and the offset floors. F3 and F4 are list edits. U1-U7 are candidates for §1.2 Known issues, and which to accept is the user's call.
