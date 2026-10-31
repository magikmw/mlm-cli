# Completion report: relative-ids, Task 2 (docs)

Plan: `docs/dev/plans/relative-ids-task-2-docs.md`.

## Base

- The worktree started off `962ac9f` (main), which does not contain `relative-ids`.
- Rebased onto `relative-ids` at `fbdc9a5` before any edit.

## What was done

- `docs/dev/SPEC.md`: edits 1.1 to 1.13 applied as written, bottom-up.
- `README.md`: edits 2.1 to 2.8 applied as written, bottom-up.
- `AGENTS.md`: checked, no edit (plan §3).

## Deviation

| Where | Plan text | Shipped text | Reason |
|---|---|---|---|
| SPEC §8.2 E36 | "From a fixed today, the last `-N` that reaches week `1000-01` resolves to it, and the next one prints the week offset error." | "On 2026-02-12, `mlm week -53540` is week `1000-01`, and `-53541` prints the week offset error." | Plan risk R5 left the week floor without a number because none was pinned. Task 1's plan now pins it (`relative-ids-task-1-code.md`, the 2026-02-12 row and `relative_week_floor_is_1000_01`), and the dispatch offered it for SPEC. The month half of E36 already uses numbers, so both halves now match. |

The rest of E36 is unchanged and re-wrapped only around the new sentence.

## Check plan results

- 4.1 to 4.8, 4.10, 4.11: every grep gives the planned count. Stale strings return nothing. The self-containedness grep is empty.
- 4.9: `AGENTS.md` has no diff.
- 4.10's binary spot check waits for Task 1 on the integration branch. It is the coordinator's step at merge.
- Gates: `cargo fmt --check`, `cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` all pass. Source is unchanged.

## Open questions for the reviewer

- At merge, check that Task 1's `relative_week_floor_is_1000_01` test still uses 2026-02-12 with `-53540`/`-53541`. If those numbers moved, E36 needs a one-line fix.
- R2 still applies. SPEC §3.7 quotes the `[WEEK_ID]` help row with three spaces of padding, which matches Task 1's plan.
