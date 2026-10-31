# doc-drift Task 2 (docs) — completion report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

- Base: worktree was not on `doc-drift`; rebased onto `7b283e5` before any edit.
- Commits: `ee88c00` content edits (README, AGENTS, SPEC, `docs/dev/README.md`), `7ac77a4` F6 banner pass (97 files, banners only).
- Gates: `cargo fmt --check` clean, `cargo build` clean, `cargo test` 1018 passed / 0 failed, `cargo clippy --all-targets -- -D warnings` clean. Pre-commit hook passed on both commits (coverage unchanged at 99.5166%).

## Deviations from the spec

| Item | Spec says | Shipped | Why |
|---|---|---|---|
| F2 | "the following `status` line takes the same prefix" | README had no `status` line, so one was added, with the prefix, plus one sentence saying `MLM_DB_PATH` keeps the dev loop off the real database | The spec assumes a line that did not exist |
| R1 in README | §3 lists R1 for the command only, with no README row | README's `delete note` section gains one paragraph and an example of `deleted note (<date>): <body>` | README documents every other delete line; this one is user-visible. It describes Task 1's R1 as it will ship, so it is wrong until Task 1 merges |
| F12 | Show the lookback block, every line | Also shows the two-line multi-open-start form (`[!] 2 open stints on ...`, `fix: see mlm status ...`) | R8 of the spec review names it as part of the same paragraph's claim |
| F11 | Bare `WW` uses the ISO year, one sentence on the month contrast | Also adds a `mlm week 5` on 2025-12-30 example under `week`, and says "calendar year" on the `-m` bullet | Keeps the two bullets self-explaining |

## Small edits outside the F list

- SPEC §1.2 and §7.3 now point `delete` at §3.9 (§7.3 used to cite §7.4 for the listing).
- SPEC §6.1's future-`--date` bullet now names `delete note|punch`, which uses the same resolver.

## Checked against the binary

Run against `MLM_DB_PATH=/tmp/mlm-doc-drift-t2.db` (dated examples checked by reading the renderer):

- `delete punch` listing and `deleted. to recreate: mlm stop 13:00` (F1)
- `id must be 1 or greater (got 0); 1 entry for this date`, `id 5 is out of range; only 1 entry for this date`
- `deleted. to recreate: mlm note 'it'"'"'s done'`
- `nothing to delete for 2026-10-31.`
