# doc-drift spec — review

- Spec: `docs/dev/specs/2026-10-31-doc-drift.md`, branch `doc-drift` at `3a396c0`
- Gate run: `cargo test` 1018 passed, 0 failed
- Method: source reads and greps. The read-only hook blocks `cargo run`, running `target/debug/mlm`, `cargo package` and `cp`, so help output and the packaged file list were checked by reading `src/cli.rs`, the tests and `Cargo.toml`, not by running anything.

## Findings

### R1 — medium, factual error / undeclared user-visible gap: F4 would fold a backslash-fallback format the binary does not print

- Claim: spec §4 F4 (`2026-10-31-doc-drift.md:49-55`) folds the archived delete spec §3-§5 "checked against `src/commands.rs` as shipped". Archived §5 (`2026-09-13-delete-punches-notes.md`, "Embedded backslash fallback") says the plain description replaces the recreate command. The sweep's F4 (`doc-drift-sweep-review.md:36`) repeats that as "the backslash fallback `deleted note (<date>): <body>...`".
- Evidence: `note_recreate_line` returns `deleted note (<date>): <body>...` (`src/commands.rs:281-284`), and `delete_note` wraps every return value in the prefix (`src/commands.rs:319-320`). The real line is `deleted. to recreate: deleted note (2026-02-10): path C:\dir\file...`.
- Two more visible quirks come with it. The fallback always prints the date, even for today (`src/commands.rs:283` has no `!= today` check, unlike `:285-287`). It always appends `...`, though nothing is cut, because bodies are single-line (`debug_assert` at `:276-280`).
- Only the helper is tested (`src/commands.rs:1514`). No test pins the emitted line, so a SPEC author reading the archived spec and the tests gets no correction.
- Fix: make F4 name the shipped line, prefix included, or list it in §1.2 as a known issue. Under §1.1 (no behavior change), SPEC must describe the doubled prefix as it is.

### R2 — low, missing edge case: F4 does not pin three more places where the archived spec and the code differ

- Ordering: archived §4 says punches list "by `at_utc`". `punches_for_date` also breaks ties start-before-end, then by `id` (`src/storage.rs:395-397`).
- Count wording: both `ID` errors use `entry` when the count is 1 and `entries` otherwise (`src/commands.rs:184`). The sweep quotes only the plural (`doc-drift-sweep-review.md:35`).
- Clap-level rejections: `-1`, `abc` and values past `u32::MAX` never reach §6.1. They are usage errors with exit 2 (`src/cli.rs:209-214`, SPEC §6.3 `SPEC.md:909-916`). F4 says only "the two `ID` errors in §6.1".
- Fix: name all three in F4's fold list.

### R3 — low, internal inconsistency: F4 adds §6.1 errors with no §8.2 entries and no home for the stale-id rule

- SPEC §8.2 is titled "Error / edge paths (§6 direct mapping)" (`SPEC.md:1885`). F4 adds two §6.1 errors but no E-entries, so the mapping breaks.
- §6 pins §3.9, §6.1 and §7.4 for Task 1's citations (`2026-10-31-doc-drift.md:101-103`). The stale-id rule has no assigned section, yet Task 1 must re-cite `src/commands.rs:297` ("spec §5's stale-id note").
- Fix: add delete E-entries to F4, and name the stale-id rule's section (§3.9, presumably).

### R4 — medium, factual error about the codebase: F20 and the F4 citation fix miss most stale comment citations

F20 (`2026-10-31-doc-drift.md:78-82`) lists four sites. `src/` has more stale or colliding citations, all in Task 1's files:

| Site | Text | Problem |
|---|---|---|
| `src/cli.rs:9` | `glossary (SPEC §4.2)` | The glossary is SPEC §3.5 (`SPEC.md:322-337`). §4.2 is duration formatting. |
| `src/status.rs:525` | `§7.4: only status/week produce stdout output` | False. SPEC §7.4 names `export notes`, `week target` and `delete` (`SPEC.md:1667-1673`). |
| `src/time.rs:6` | `PLAN.md interface contract 11` | Same file as F20's dead-code note, but not listed. |
| `src/storage.rs:395`, `:410` | `PLAN.md interface contract 10` | Archived PLAN.md. |
| `src/storage.rs:292` | `spec §2` | The delete spec's §2. Collides with SPEC §2. |
| `src/week_target.rs:21` | `spec §4` | The panic-fixes spec. Collides with SPEC §4. |
| `src/commands.rs:163`, `:1880` | `spec §5` | Delete spec. F4's "spec §4/§5" covers these only if Task 1 reads it broadly. |
| `src/commands.rs:231`, `:261` | `spec §5/§7` | The archived §7 is the `commands.rs` section, not SPEC §7. |
| `src/cli.rs:225-226`, `src/commands.rs:265-268` | Milestone 8/11/13 | Archived PLAN.md milestones. |

Qualified citations like `backdated-punches spec §3` (`src/time.rs:131`) are at least unambiguous. The unqualified `spec §N` ones read as SPEC.md.

- Fix: either list the sites in F20, or state the rule ("no unqualified `spec §N`, no PLAN.md citation") and let Task 1 grep for it.

### R5 — low, testing gap: no golden quotes the F8 or F9 strings, so the new text goes unpinned

- Claim: §5 says "The help goldens that quote the two §3 strings take the new text" (`2026-10-31-doc-drift.md:86`).
- Evidence: `no start before it` and `most recent listing` appear only in `src/cli.rs:43` and `:205-208`. `long_help_carries_glossary` checks only the term header `"\n  orphaned end\n"` (`tests/week_target_cli.rs:159`). No test renders `delete note|punch --help`.
- Fix: §5 should require new asserts for both strings (for example `flat.contains("An end punch with no open start to pair with.")` and a `delete punch -h` check).

### R6 — low, testing gap: "matches the binary when run" cannot be done as written

- Claim: §5 says "Every README and SPEC example touched by §3 matches the binary when run" (`2026-10-31-doc-drift.md:90-91`).
- Evidence: the examples pin a specific today. F13 needs Sat 2026-10-24 (`README.md:513`), and F12's line needs 2026-09-26. `src/` reads no env var except `MLM_DB_PATH`, so there is no clock override. The review hook blocks `cargo run` (sweep `doc-drift-sweep-review.md:7`).
- Fix: name the method, such as a fixed-`now` render in a unit test, or a shape check against the rendering code.

### R7 — medium, undeclared user-visible gap: F2's fixed dev loop writes to the developer's real database

- F2 changes `README.md:722` to `cargo run -- start 9:00 "working on mlm"`. The old line failed at `parse_time` and wrote nothing. The new line inserts a 09:00 start and a note into the platform app-data DB, because nothing sets `MLM_DB_PATH`.
- `AGENTS.md:101-102` runs the same command with `MLM_DB_PATH=/tmp/mlm-check.db`, and `README.md:112` documents the override.
- Fix: use `MLM_DB_PATH=/tmp/mlm-dev.db cargo run -- start 9:00 "working on mlm"`, or record the write in §1.2.

### R8 — low, missing edge case / undeclared user-visible gap: F12 shows one line of a three-line block, and its date clashes with its neighbours

- The lookback output is three lines: the header, `fix:` and `or:` (`src/lookback.rs:157-169`). There is also a second form for two or more open starts, `[!] N open stints on <Wdy date> (N days ago)` plus `fix: see mlm status <date>` (`src/lookback.rs:97-103`). The README paragraph says "for every date it finds with an unclosed start" (`README.md:297-300`). F12 shows only the single-start header.
- F12's date, Mon 2026-09-21 at 5 days ago, puts today on 2026-09-26. The block just above (`README.md:274-276`) and the `start` warning (`README.md:184`) both treat 2026-09-25 as 5 days old, which puts today on 2026-09-30. A reader sees two different todays in adjacent examples.
- Fix: show the full block with a 2026-09-25 date, or keep SPEC's date and say so.

### R9 — low, missing edge case: one archived file has no title line for F6's "after its title line"

- Claim: F6 places the banner "after its title line" (`2026-10-31-doc-drift.md:60-61`).
- Evidence: `docs/dev/plans/reports/live-eod-fresh-eyes.md:1` is `## Scenarios — what you ran, what it printed`. Every other unbannered archived file starts with `# `.
- Fix: say "at the top" for this file, or give it a title first.

### R10 — low, scope gap: F6 would banner in-flight files and does not stop the gap coming back

- F6 covers "every archived file under `docs/dev/specs/`, `docs/dev/plans/` and `docs/dev/plans/reports/` without a banner" and exempts only the sweep report (`2026-10-31-doc-drift.md:59-65`). That set also holds this spec (`docs/dev/specs/2026-10-31-doc-drift.md`, no banner), this review and any later plan or review report for this changeset. Task 2 would banner its own live spec mid-changeset.
- Counts check out. 99 files lack a `> **Archived` line: the sweep's 97 plus this spec and the sweep report.
- The sweep's fix also said to add reports to the close-out checklist (`doc-drift-sweep-review.md:55`). F6 and F7 drop that step. The NOTES close-out entries name spec, plan and task plans only (`NOTES.md:1458`, `:1553`, `:1756`), so the next changeset's reports go unbannered again.
- Fix: exempt this changeset's own docs by name, and have F7 add reports to the close-out step in `docs/dev/README.md`.

### R11 — low, internal inconsistency: F9 gives one sentence for a three-sentence entry

- The glossary entry reads "An end punch with no start before it. It adds no minutes. Add a start before it, or delete it." (`src/cli.rs:43-44`). F9's table cell gives only the new first sentence (`2026-10-31-doc-drift.md:45`) and does not say the other two stay.
- The F9 wording itself is accurate. It matches LIFO and same-instant group pairing (`SPEC.md:609-628`, `:661-665`), and SPEC does not quote the entry, so Task 2 has nothing to change.
- Fix: say "first sentence only".

### R12 — low, undeclared user-visible gap: F8's new help drops the rejection rule

- The current help says position 0 and anything past the count are rejected (`src/cli.rs:206-208`). The new text drops that. A user reading `--help` no longer learns that `0` is invalid. The errors still explain it at run time (`src/commands.rs:186-191`).
- F8's "taken fresh when this runs" is accurate (`src/commands.rs:310`, `:317`, `:345`).

### R13 — low, scope gap: F3 leaves two more bare `SIGNING.md` references outside both tasks

- `Cargo.toml:45` ("see SIGNING.md") and `.github/workflows/sign-artifacts.yml:9` ("See SIGNING.md") point at the old root path. Neither file is in Task 1's or Task 2's set (`2026-10-31-doc-drift.md:97-98`).
- `Cargo.toml:15-19` also excludes `NOTES.md`, `PLAN.md` and `plans/` by their pre-move names. Gitignore-style matching may still catch `docs/dev/NOTES.md` and `docs/dev/plans/`, but not `docs/dev/specs/` or `docs/dev/SPEC.md`. Not verified, because `cargo package --list` is blocked. This drift is outside the sweep, so it is flagged for the user's call.

### R14 — low, missing edge case: F10 leaves "either platform" in the same sentence

- `AGENTS.md:89-91` continues "no external system dependency needed on either platform". Once F10 lists five targets, "either" no longer fits.
- Fix: include that clause in F10.

### R15 — low, undeclared user-visible gap: after F11, README shows a week/month year contrast and never explains it

- F11 makes a bare `WW` read "current ISO year" (`README.md:428`, `:645`). A bare month stays "current year" (`README.md:648`), which is correct (`src/date.rs:419` vs `:498`). Between 29 Dec and 3 Jan, `-w 1` and `-m 1` land in different years. The adjacent bullets now show the contrast with no explanation.

### R16 — low, undeclared user-visible gap: F14's "every top-level command" includes `help`

- clap's built-in `help` subcommand is top-level and has no alias, so "every top-level command" overstates by one. This is trivial.

### R17 — low, internal inconsistency: the NOTES work in §6 has already landed and points at a section that does not exist yet

- §6 lists the F4 pointer and the F15 header as coordinator work still to do (`2026-10-31-doc-drift.md:100-101`). Both are already in `3a396c0`: the header is at `NOTES.md:3-5`, and decision 5 cites "SPEC.md §3.9" at `NOTES.md:149`. SPEC has no §3.9 until Task 2 merges (`SPEC.md:258-495` ends at §3.8), so that pointer dangles on the branch. If Task 2 picks another number, NOTES goes stale.

### Checked, no drift

- SPEC §3.9 is free. §3 runs §3.1-§3.8 plus §3.4a, and nothing cites §3.9 except `NOTES.md:149` and `:2160`.
- F1 matches `src/commands.rs:244-246`. F5's message matches `src/time.rs:155-158`. F11 matches `src/date.rs:419`. F16 matches `src/db.rs:51-62`. F17 matches `src/status.rs:473` and `:487`. F18's three entries are `so far`, `spans` and `- <note>` (`src/cli.rs:45-56`). F19 matches `src/lib.rs:28-71` and `src/storage.rs:393`, `:443`, `:477`, `:528`. F10's five targets match `dist-workspace.toml:13`.
- No test or script quotes the README lines F1, F2, F3, F11, F13 or F14 change. `refactored the parser` appears only in `examples/seed_test_data.rs:77` as seed data, and the `code review for #41` goldens use other weeks.
- §6 file sets are disjoint. NOTES.md is the coordinator's.
- The F7 claim holds. All 100 existing banners point at `SPEC.md`.

## Verdict

ship-with-followups

R1, R4 and R7 should be settled in the spec before dispatch. R1 would put a wrong format into SPEC. R4 leaves Task 1 with an incomplete list. R7 is a README command that writes real data. The rest are wording or known-issue candidates.
