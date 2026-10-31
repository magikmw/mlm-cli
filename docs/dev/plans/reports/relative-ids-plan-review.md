# relative-ids plan review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Plan: `docs/dev/plans/relative-ids-plan.md` (branch `relative-ids` at `8d159cb`)
- Spec: `docs/dev/specs/2026-10-25-relative-ids.md` (locked)
- Method: read the plan, the spec, the spec review, NOTES.md 150-153, and the cited source and tests. Built `mlm` from this branch into `/tmp/cd-scratch/relids-plan/mlmtarget` and probed it against a scratch db with `MLM_DB_PATH` and `TZ=UTC`. Built a clap 4.6.6 harness in `/tmp/cd-scratch/relids-plan/src/main.rs` that copies `WeekArgs`/`WeekTargetArgs` with `allow_negative_numbers` on `week_id`, the new help text, and two ways to build the lone-`WEEK_ID` error.

## What checks out

- File sets are disjoint. Task 1 owns all source, tests and `coverage-baseline.json`. Task 2 owns `SPEC.md`, `README.md`, `AGENTS.md`. No other file needs to change: the only help goldens are `tests/week_target_cli.rs:218-235,256-268` and `tests/export_notes_cli.rs:537-547`, and the only old-shape-text assertions are in `src/date.rs` and `tests/export_notes_cli.rs`. `CHANGELOG.md` is written at release time (NOTES.md:27).
- Contracts 2-5 match spec §3 and §4 word for word. Contract 1's examples are right: 2026 has 53 ISO weeks, 2026-01-05 is in 2026-02, 2027-01-04 is in 2027-01. `-m -24313` from February 2026 is `0000-01` (2026*12+1 = 24313).
- Every spec §5 bullet has an acceptance criterion. The coverage of bullet 1 is partial (finding 4).
- Every call site goes through `parse_week_id`/`parse_month`: `src/week_view.rs:329`, `src/week_target.rs:90`, `src/export.rs:30,33`, `src/cli.rs:444`.
- `mlm week` under `allow_negative_numbers` works in the harness. `-1`, `-1 -n`, `-n -1` parse. `-1x` and `--bogus` give exit 2. `-1 target 35h` gives `ArgumentConflict`, so `classify` (`src/cli.rs:384-396`) runs the swap check. Doc-comment backticks render literally, and the long help lines stay on one line (no `wrap_help` feature, `Cargo.toml:32`).
- The lone-`WEEK_ID` usage error can be built. In the harness, re-parsing argv without the token and building `clap::Error::new(MissingRequiredArgument).with_cmd(&target)` with `InvalidArg` and `Usage` context both render byte-identical to bare `mlm week target` once `rename_args_context` runs. The parse fails before `db::connect` (`src/main.rs:19-22,57`), so nothing is written.
- Cognitive complexity of 15 or less is plausible. `resolve_date` (`src/date.rs:132-162`) already has the shape the new helper needs and passes the gate. A `match` counts once however many arms it has, so a `+` branch in `Display::fmt` adds about 2.
- The Risks entry is right that no `WEEK_ID` is a well-formed `DURATION`. `parse_duration` requires an `h` or `m` suffix (`src/time.rs:70-82`).
- `Cargo.toml:5` is `rust-version = "1.85"`. The coverage baseline is 99.505 (`coverage-baseline.json`).
- Performance at the floor is fine. `mlm week 1000-01` runs in 4 ms on the scratch db.

## Findings

### 1. medium. Factual error: contract 6 says only `-` plus digits reaches the `mlm week` parser, but clap also passes float-shaped tokens

- Claim: contract 6 (plan:58) says "On `mlm week`, only `-` plus digits reaches the parser." Spec §3 (spec:54-57) says the same, and Task 2 must fold it into SPEC §3.6 byte for byte (plan:102,108).
- Evidence: `allow_negative_numbers` uses clap_lex's `is_number` (`clap_lex-1.1.0/src/lib.rs:316-321,492-522`). That function accepts `1.`, `1.5` and `1e3`. The harness gives `week_id = "-1.5"`, `"-1e3"` and `"-1."`, so each reaches `parse_week_id` and exits 1 with the shape error. `mlm status -1.5` already does this: `error: invalid DATE "-1.5": expected YYYY-MM-DD or -N (N >= 1)`, exit 1.
- Visible consequence: `mlm week -1x` exits 2 but `mlm week -1.5` exits 1, and the plan does not record the split.
- Fix: reword contract 6 to say "tokens clap reads as a negative number (`-1`, `-1.5`, `-1e3`) reach the parser, and only `-` plus digits resolves". Add one binary case, `mlm week -1.5`, which exits 1 with contract 2's message.

### 2. low. Internal inconsistency: the `+` hint mechanism cannot be both parser-side and keep the shape cause

- Claim: plan:15 says "The `+` hint is a parser-side branch on `+` plus digits that keeps the shape cause and adds the suffix."
- Evidence: `DateWeekError` holds only `arg`, `input` and `cause` (`src/date.rs:52-57`). `Cause::Shape` is a unit variant (`src/date.rs:33`). Display picks the text from `(arg, cause)` alone (`src/date.rs:77-93`). So there is nowhere for the parser to put a suffix while the cause stays `Shape`. Only two options work. Display can inspect `input` in the `WeekId`/`Month` `Shape` arms, which is not parser-side. Or a new cause or field can carry the hint, which flips the `"+1"` `Cause::Shape` assertions at `src/date.rs:751` (T65) and `src/date.rs:1153` (MONTH list). The plan names neither as a flip.
- Fix: say which option. If it is a new cause, list `src/date.rs:751,1153` with the flips.

### 3. low. Factual error: "4 sites in `src/date.rs`" counts the source as assertions

- Claim: plan:71 says "Every assertion of the old shape text takes contract 2's text (4 sites in `src/date.rs`, 8 in `tests/export_notes_cli.rs`)."
- Evidence: `src/date.rs` has two assertions (`:1207`, `:1208`, and `:1208` is the `-1` test that flips anyway). The other two sites are the `Display` arms at `:79-80`. The count of 8 in `tests/export_notes_cli.rs` is right (`:256,262,380,384,388,392,396,400`).
- Fix: say "2 assertions plus the 2 `Display` arms".

### 4. low. Testing gap: `-01`, `-2` and `-07` never go through `mlm week`'s clap layer

- Claim: spec §5 bullet 1 (spec:109-110) asks for `-1`, `-2`, `-01` and a year crossing "for each row of the §3 table". Plan AC 1 (plan:74) tests them at parser level only. The binary cases (plan:79) use only `-1`.
- Evidence: `mlm week` is the only row whose clap setting changes (plan:19). Whether `-01` or `-07` is a negative number to clap is decided by `is_number`, not by the parser. Nothing pins that a leading-zero token reaches `week_id`.
- Fix: add `cli.rs` parse tests that `mlm week -01`, `-07`, `-2` and `-0` land in `week_id` verbatim.

### 5. low. Testing gap: the lone-`WEEK_ID` conversion is not tested with the alias, with `-v`, or after `--`

- Claim: plan:85 covers `mlm week target -1`, `7` and `2026-07` alone.
- Evidence: the swapped-order detector works over raw argv and has tests for `w` and every `-v` position (`src/cli.rs:896-932`, `tests/week_target_cli.rs:312-342`). A re-parse-without-the-token mechanism is argv-based in the same way. Cases like `mlm w target -1`, `mlm -v week target -1`, `mlm week target -v 7` and `mlm week target -- -1` should all give bare `mlm week target`'s stderr. No criterion pins them.
- Fix: add those four to the binary or `super::parse` tests.

### 6. low. Testing gap: nothing guards `DATE` against the new `+` hint

- Claim: spec §1.1 says "No change to `DATE`." Contract 4 limits the hint to `WEEK_ID` and `MONTH`.
- Evidence: the only `DATE` `+` tests check the cause, not the text (`src/date.rs:572` `"+2026-02-12"`, `:1280` `"+1"`). A hint added to the generic `Shape` path would change `mlm status +1` (today `error: invalid DATE "+1": expected YYYY-MM-DD or -N (N >= 1)`, verified live), and no test would fail.
- Fix: assert the exact `DATE` message for `+1`, as a unit test or as `mlm status +1`.

### 7. low. Internal inconsistency: the Risks reading of lone invalid tokens is not carried into Task 2

- Claim: Risks (plan:117) says `mlm week target -0`, `-99999999999999999999` and `53` in a 52-week year keep `invalid duration "<token>"` with exit 1.
- Evidence: Task 2's AC (plan:105) only says "E10 names the lone-`WEEK_ID` case". Nothing asks SPEC to say which lone tokens convert and which do not. So SPEC.md can come out describing the conversion without its boundary, and the plan already treats that boundary as settled.
- Fix: add to Task 2's AC that §3.7 or E10 states the boundary. Only a token that parses as a `WEEK_ID` converts. Everything else, `-0` and an out-of-range `-N` included, keeps `invalid duration`.

### U1. Undeclared user-visible gap: `mlm week -1x` names `-1` and suggests `-- -1`, which now runs last week

- Decision: contract 6 keeps `mlm week -1x` a clap usage error, for parity with `status` (NOTES.md entry 152).
- Visible consequence: stderr reads `error: unexpected argument '-1' found` and `tip: to pass '-1' as a value, use '-- -1'` (harness, and `mlm status -1x` live). Once this changeset lands, `-1` is a valid `WEEK_ID`, so the message names a token that is fine. Following the tip runs `mlm week -- -1`, which shows last week, not what the user typed. The spec review raised the message text (F1). The fold recorded only the exit code.

### U2. Undeclared user-visible gap: a lone duration with a forgotten unit loses the format hint

- Decision: a lone `week target` token that is a valid `WEEK_ID` becomes the missing-`DURATION` usage error (contract 8).
- Visible consequence: `mlm week target 40` (the user meant `40h`) today prints `invalid duration "40": expected a form like 20h, 33h30m, or 45m`, exit 1 (verified live). After the change it prints `the following required arguments were not provided: <DURATION>`, exit 2. So the user is told no duration was given when they did type one, and the hint about `h`/`m` is gone. The outcome now depends on the number. `1`-`52` give the usage error, `53` gives it only in a 53-week year, and `0` or `54` and up keep the format hint. A lone `-5` (a negative duration without its unit) also switches to the usage error. Any script checking for exit 1 sees 2.

### U3. Undeclared user-visible gap: `+0` gets the "future week" hint

- Decision: contract 4 includes `+0` in the hint (plan:49).
- Visible consequence: `mlm week +0` prints `...; a future week takes YYYY-WW or WW`, and `-m +0` prints the month twin. `+0` names no future week. A user who meant "this week" is told how to reach a future one.

### U4. Undeclared user-visible gap: `DATE` `+1` gets no hint, `WEEK_ID` and `MONTH` `+1` do

- Decision: no change to `DATE` (spec §1.1), plus the `+` hint for `WEEK_ID`/`MONTH` (contract 4).
- Visible consequence: `mlm week +1` and `mlm ex n -m +1` point at the absolute form. `mlm status +1`, `mlm start -d +1` and `mlm ex n -d +1` print the bare `expected YYYY-MM-DD or -N (N >= 1)`. So three flags of one command answer the same typo differently.

### U5. Undeclared user-visible gap: the lone-`WEEK_ID` error does not mention the token the user typed

- Decision: the stderr is byte-identical to bare `mlm week target` (contract 8).
- Visible consequence: `mlm week target 2026-07` prints exactly what `mlm week target` prints, so the output does not show that `2026-07` was read as the week. The text says an argument was "not provided" after the user provided one.

### U6. Undeclared user-visible gap: the offset error names no limit, and the limits differ by kind

- Decision: contract 3's text is `week offset is out of range` / `month offset is out of range`, with floors `1000-01` and `0000-01`.
- Visible consequence: a user who hits the error cannot tell how far back is allowed. The two floors are about a thousand years apart, so `-w -N` and `-m -N` fail at very different N for what reads as the same rule. `DATE`'s `date offset is out of range` has the same shape, so this matches house style, but nothing records it.

## Verdict

ship-with-followups. Finding 1 should be fixed before Task 2 folds contract 6 into SPEC.md, because otherwise SPEC states something the binary contradicts. Findings 2-7 are wording or test-list edits. U1-U6 are known-issue candidates for the user, and choosing which to accept is the user's call.
