# export-notes spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Reviewed: `docs/dev/specs/2026-10-24-export-notes.md` (untracked, branch `export-notes`, HEAD `b25bef7`).
Source checked: `src/cli.rs`, `src/date.rs`, `src/storage.rs`, `src/main.rs`, `src/week_view.rs`, `src/db.rs`, `tests/`, `docs/dev/SPEC.md` §1, §2.3, §3, §6, §7, §8, `docs/dev/NOTES.md` entry 125, `AGENTS.md`, `README.md`.

Experiments ran in `/tmp/cd-scratch/esr` against clap `4.6.6` (the locked version), reproducing the planned arg shape (`ExportArgs` with a required subcommand, `NotesArgs` with three `Option<String>` flags in a `multiple = false` group, `-d` with `allow_hyphen_values`) and the existing `NoteArgs` shape from `src/cli.rs:139-157`.

Reading choice: the dispatch says the §5 example is illustrative, so its specific dates and bodies are not checked against anything.

## Findings

### F1 — medium — internal inconsistency: `--date` both mirrors and contradicts `start`/`stop`/`note`

- Claim, spec:45-46: `--date` accepts its forms "as `start`/`stop`/`note` do (SPEC §3.4a)".
- Claim, spec:52-53: "A future date, week or month is not an error."
- Evidence: `start`/`stop`/`note`/`status` all go through `resolve_future_checked_date` (`src/date.rs:158-167`), which rejects a future date. SPEC §3.4a (`SPEC.md:281-282`) and §6.1 (`SPEC.md:623-629`) list that rejection as a hard error. `resolve_date` (`src/date.rs:126`) is the resolver without the check.
- An implementer who follows line 45 rejects `-d 2099-01-01`. One who follows line 52 accepts it.
- The "Interaction with the project spec" list (spec:135-139) does not mention that the §6.1 future-date bullet needs a carve-out for export.
- Fix: say `--date` uses the same grammar as §3.4a, resolved with `resolve_date` (no future check). Add the §6.1 future-date bullet to the interaction list.

### F2 — medium — error-handling gap: hyphen values and repeated flags reach clap, not the §6 table

- Claim, spec:94-97: a bad `WEEK_ID` gets the existing `invalid WEEK_ID "..."` message, exit `1`.
- Evidence (experiment): with no `allow_hyphen_values` on `--week`/`--month`, `mlm ex n -w -1` and `mlm ex n -m -1` fail in clap as `unexpected argument '-1' found`, exit `2`. A user who tries `-1` for "last week" (the `-N` shape `--date` accepts on the same command) gets a usage error, not the WEEK_ID message.
- Further cases from the experiment: `-w 1 -w 2` fails as `the argument '--week <WEEK>' cannot be used multiple times`, exit `2`. Bare `-d` fails as `a value is required`, exit `2`. Neither appears in §6.
- SPEC §1.2 (`SPEC.md:50-53`) makes `-N` for weeks a non-goal, so rejecting it is fine. What is missing is the choice of which layer rejects it, and with which exit code.
- Fix: either set `allow_hyphen_values` on `--week`/`--month` so `-1` reaches the parser (`invalid WEEK_ID "-1": expected YYYY-WW or WW`, exit `1`), or add a §6 row saying a hyphen value for `-w`/`-m` is a clap usage error, exit `2`. Add the repeated-flag and missing-value rows.

### F3 — medium — error-handling gap: piping into `head` can panic

- Claim, spec:9-11: the motivation is plain text "to paste into a report or pipe into another tool". Spec:86 says every success exits `0`.
- Evidence: every command prints with `print!`/`println!` (for example `src/week_view.rs:327`). Rust ignores SIGPIPE, so a write after the reader closes makes `println!` panic with `failed printing to stdout: Broken pipe` and exit `101`. `mlm ex n -m 10 | head -1` hits this once the output exceeds the pipe buffer, or when the timing goes the wrong way.
- Existing commands have the same exposure, but they are not built for pipes. This one is.
- Fix: decide the behavior and put it in §6. Treating a broken pipe as a quiet exit `0` is the usual choice. Add a test.

### F4 — low — missing edge case: MONTH year range and unpadded `YYYY-M`

- Claim, spec:49-51: MONTH accepts `YYYY-MM` and bare `M`/`MM`, "the same pattern as `WEEK_ID`".
- Evidence: `WEEK_ID` also accepts an unpadded `YYYY-W` (`src/date.rs:382-387`, SPEC §6.1 `SPEC.md:634-636`, where `2026-7` equals `2026-07`). Following "the same pattern" literally makes `2026-5` legal, but §3 lists only `YYYY-MM`.
- `WeekId::new` also rejects years outside `1000..=9999` (`src/date.rs:237`). §6 (spec:98-99) has no row for `0999-05` or `0000-05`, so their outcome (shape error, a range error, or an empty export) is unspecified.
- `ArgKind` has only `Date` and `WeekId` (`src/date.rs:21-24`), and `Display` has no month-range wording (`src/date.rs:73-87`). Both MONTH messages are new plumbing, not reuse. The spec does not need to design that plumbing, but "the same pattern" hides that the work exists.
- Fix: state whether `YYYY-M` is accepted, and add a year-out-of-range row with its message.

### F5 — low — factual error: "each note is exactly one line" holds only for v0.3.0+ rows

- Claim, spec:75-76: "Stored bodies are always one line (SPEC §2.3), so each note is exactly one line."
- Evidence: newline collapse landed in `33355f7` (2026-09-14). `git tag --contains` puts it first in `v0.3.0`. `v0.1.0`-`v0.2.0` shipped on 2026-09-13 without it. `src/db.rs` has a single schema migration and no data migration that rewrites old bodies. A prod DB that saw those builds can hold multi-line bodies. `src/commands.rs:260-264` only `debug_assert!`s the invariant.
- `normalize_newlines` also passes U+2028, U+2029, VT, FF and NEL through (`src/storage.rs:277-280`). Some consumers treat those as line breaks.
- Fix: soften the claim to "bodies written since v0.3.0", or say what export does with an embedded newline (pass it through, or collapse it at render).

### F6 — low — spec gap: help value names are not pinned

- Claim, spec:36: surface `--date DATE | --week WEEK_ID | --month MONTH`. Spec:119 tests the `--help` lines.
- Evidence (experiment): with fields named `week`/`month`, clap renders `--week <WEEK>` and `--month <MONTH>`, so the help shows `<WEEK>` while the error says `invalid WEEK_ID`. Existing flags set `value_name` explicitly (`src/cli.rs:124`, `src/cli.rs:144`).
- The help strings at spec:58-60 also drop the `(e.g. 2026-07)`/`(e.g. 7)` examples that the existing WEEK_ID help carries (`src/cli.rs:236`).
- Fix: pin `value_name` for all three flags, and decide on the example style, so the `--help` test has an exact golden.

### F7 — low — testing gap

§7 (spec:104-119) misses these cases:
- `-w -1` / `-m -1` and repeated flags (F2)
- `YYYY-M` and year out of range for MONTH (F4)
- bare `WW`/`M` on a date where the ISO year and the calendar year differ, such as 2025-12-29 (U2)
- broken pipe (F3)
- export against a missing DB path (U4)
- DB open failure through export. `tests/e6_db_failure.rs` covers one write and one read command only.
- `tests/ascii_sweep.rs` sweeps `status` and `week` output only. Spec:118 asks for an ASCII check, but does not say whether the sweep is extended or a new check is added.

### F8 — low — doc targets missing from the interaction list

Spec:133-139 and §8 (spec:121-126) omit these targets:
- SPEC §1.2 non-goals. JSON/CSV, `--output` and ranged export are stated as non-goals here (spec:17-22) but never land in the live spec.
- SPEC §6.1's future-date bullet (F1) and §6.2 "not errors" (empty or future range).
- `AGENTS.md:16`, the "Current state" list of implemented commands, and `AGENTS.md:35`, the `src/date.rs` entry, which gains MONTH.
- The README heading convention `mlm export|ex notes|n ...`, matching `README.md:587`.
- The new module's name, which the spec never gives. The AGENTS.md layout list needs that name.

### F9 — low — scope creep (declared): `WeekArgs` comment rewrap

- Spec:130-131 re-wraps `src/cli.rs:226`, which is 89 columns. The claim is accurate, and the edit is unrelated to export. It is small and declared, so it is acceptable if the user wants it in this changeset.

## Undeclared user-visible gaps

§1.2 says "None." Each item below is a deliberate choice whose visible result the spec does not record. Any of them can go into §1.2. Which ones to accept is the user's call.

### U1 — bare month or week in January resolves to a future period and prints nothing

- Spec:47-51 resolves bare `WW` and `M`/`MM` against the current year. Spec:52-53 makes a future period an empty success.
- In January, `mlm ex n -m 12` means December of this year, not the December just past. It prints nothing and exits `0`. `-w 52` in early January does the same. A user pulling last month's notes for a report gets silence and no hint.

### U2 — bare `WW` and bare `M` use different "current years"

- Bare `WW` uses the ISO year (`src/date.rs:380`). Bare `M` uses the calendar year (spec:50).
- On 2025-12-29, which falls in ISO week 2026-W01, `-w 1` is the current week, `-w 52` is December 2026 (future, empty), and `-m 1` is January 2025. The two flags disagree about the year for the same three days.

### U3 — a future `--date` is a silent empty success, unlike everywhere else

- `mlm status 2099-01-01` errors with a hint (SPEC §3.5, `SPEC.md:286-289`), and `mlm note -d 2099-01-01` errors too. `mlm ex n -d 2099-01-01` prints nothing and exits `0`.
- A typo in the year (2062 for 2026) gives empty output and no error. Spec:52 states the rule but not that it diverges from the other date-taking commands.

### U4 — empty output cannot be told apart from a wrong range or a wrong DB

- `dispatch` opens the DB first (`src/main.rs:48`). A missing DB file is created and migrated silently (SPEC §6.1, `SPEC.md:648-654`).
- A typo in `MLM_DB_PATH`, or a first run on a new machine, gives empty stdout, exit `0`, and a new empty DB file left behind. `status` would at least show an empty page with totals. For export, the empty page is the whole output.

### U5 — range is flags-only, unlike `week`/`status` positionals

- `mlm week 43` and `mlm status -1` take positionals (`src/cli.rs:99-103`, `src/cli.rs:236-239`). `mlm ex n 43` and `mlm ex n -1` are clap usage errors, exit `2`. Users coming from `week` will try the positional first.

### U6 — alias `n` plus the promoted short form makes `mlm n -w 43` write a note

- Spec:38-39 promotes `mlm ex n -w 43`. The top-level `n` is already `note` (`src/cli.rs:85`), and `NoteArgs.body` allows hyphen values (`src/cli.rs:149-155`).
- Experiment: `mlm n -w 43` parses as `note` with body `["-w", "43"]`, so it records the note `-w 43` for today and exits `0`. The same holds for `mlm n -m 10`. Dropping `ex` from the short form writes junk data that the user must then delete.

### U7 — `--help` never says the default range is the current week

- Spec:57-60 help lines carry no "Defaults to ..." text. Every other defaulted argument states its default (`src/cli.rs:100-101`, `src/cli.rs:236-237`). `mlm ex n --help` gives no hint that no flag means this week, not today (the `status` default) and not this month.

### U8 — output lines carry no weekday

- `status` and `week` show the weekday (`format_date_with_weekday`, `src/date.rs:175`; the `Mon`-`Sun` row labels, `src/week_view.rs:42`). Export prints bare `YYYY-MM-DD` (spec:74). A pasted weekly report loses the weekday the user sees everywhere else.

### U9 — one corrupt note row hides the whole month

- Spec:100 makes a failed note read a hard error. `note_from_row` fails the whole read on one bad `date` or `created_at_utc` (`src/storage.rs:448-460`, `src/storage.rs:483-506`). With `--month`, one bad row blocks about 30 days of notes, against 7 days for `week --notes` and 1 day for `status`. NOTES entry 128 dropped this for `week --notes`. The blast radius here is wider, so it is listed again.

### U10 — Windows line endings and redirect encoding

- `println!` writes LF on Windows, which is a target platform (`AGENTS.md`). In Windows PowerShell 5.1, `>` re-encodes the redirect as UTF-16LE with a BOM. A user who "redirects stdout" (spec:20) on that shell gets a file that many tools read as binary. The spec does not record this.

## Verdict

ship-with-followups

F1 is a direct contradiction that the plan phase would trip over, so it has to be fixed in the spec. F2 and F3 need one decision each. The rest are clarifications, test additions, or known-issue candidates for the user to accept or drop.
