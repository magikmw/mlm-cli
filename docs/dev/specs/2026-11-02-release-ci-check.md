# CI check before release — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.

**Baseline**: written against `main` at `d03bc0a` (`v0.5.0` + the
`prep_release.sh` hint fix and the v0.5.0 NOTES entry, unreleased).
**Branch**: `release-ci-check`.
**Status**: locked 2026-11-02 after one review
(`docs/dev/plans/reports/release-ci-check-spec-review.md`, folded in
NOTES.md entry 170).
**Input**: the user's choice on 2026-11-02 of option A in NOTES.md
entry 169.

## 1. Summary

Changesets merge into `main` locally and reach GitHub only when
`release.sh` pushes the release commit. CI first sees new code there,
after the version bump is committed and pushed. v0.5.0 hit this with a
Windows-only test failure.

After this changeset `prep_release.sh` runs CI on the current `HEAD`
before the version bump. It pushes `HEAD` to a throwaway branch, waits
for CI, and pauses on a failure, as the dependency check does.

## 1.1 Non-goals

- No change to `release.sh`. Its CI wait on the release commit stays the
  hard gate.
- No CI on changeset branches or on `main` between releases.
- No change to what CI runs.
- No flag to skip the check in prep.

## 1.2 Known issues

- Prep takes about five minutes longer, the length of one CI run, and
  every rerun of prep waits again.
- A release runs CI twice, once in prep and once in `release.sh`.
- The check runs on the commit before the bump. A failure that only the
  version bump causes shows up in `release.sh`, as today.
- Prep is no longer local only. The pushed commits become public on
  GitHub. Deleting the branch hides them from the branch list, but they
  stay reachable by sha and from the run's page. The repo's default
  branch and README stay on the released state.
- `release-check` runs, failed ones included, stay in the public Actions
  tab, and GitHub emails the pusher about a failed run.
- A run abandoned by a stop or a timeout keeps running on GitHub.
- A commit message holding `[skip ci]` gets no run, so the check times
  out.
- A green prep check does not make `release.sh --skip-ci-check` safe.
  The release commit's own CI still needs its wait.

## 2. Prerequisites

`gh` (authenticated) and `jq` on the release machine, as `release.sh`
already needs, and push access to `origin`.

## 3. CI trigger

`ci.yml`'s `push` trigger adds the branch `release-check`. Nothing else
in the workflow changes.

## 4. `prep_release.sh` step

The check runs after the dependency check and before the version bump.
A stop leaves the tree untouched.

1. Check for `gh` and `jq`, and that `CI_CHECK_TIMEOUT`, if set, is a
   positive integer. A missing tool gives the skip prompt with nothing
   pushed. A bad `CI_CHECK_TIMEOUT` stops the script with
   `error: CI_CHECK_TIMEOUT must be a positive integer`, exit `1`.
2. Delete any remote `release-check` left from an earlier run, so the
   push below always creates the branch and triggers a run. A missing
   branch is not an error.
3. Note the time, then push `HEAD` to `origin` as `release-check` with
   `--quiet`, so GitHub's pull-request hint does not print.
4. Wait for the `ci.yml` run with `branch=release-check`, `event=push`,
   `HEAD`'s sha, and a creation time no earlier than the push. Poll
   every 15 seconds, for up to 600 seconds. `CI_CHECK_TIMEOUT` (seconds)
   overrides the 600, for testing.
5. Delete the remote `release-check` branch, whatever the result. A
   trap on exit and interrupt does this too, so a stop, an error or
   Ctrl-C after the push never leaves the branch behind. A failed delete
   prints `warning: could not delete remote branch release-check` and
   does not change the exit code.

| Case | Output | Then |
|---|---|---|
| run succeeds | `CI check: ok (<sha>)` | continue |
| run completes with another conclusion | `CI check failed on <sha> (conclusion: <conclusion>). <run URL>` then `continue anyway? [y/N] ` | `y` or `Y` continues |
| no completed run in time | `CI check timed out after <N>s on <sha>.` plus the run URL if a run was found, then `continue anyway? [y/N] ` | `y` or `Y` continues |
| a `gh` or `jq` call fails while waiting | `CI check could not finish: <reason>.` then `continue anyway? [y/N] ` | `y` or `Y` continues |
| push fails, or `gh` or `jq` missing | `CI check skipped: <reason>.` then `continue anyway? [y/N] ` | `y` or `Y` continues |

- Every `gh`, `jq` and `git push` call is guarded, so none of them ends
  the script through `set -e`. Each failure lands in a row above.
- The prompt follows the dependency check's rules. Any answer but `y` or
  `Y`, end of input, or stdin that is not a terminal stops the script
  with `error: release stopped at CI check`, exit `1`.
- The dependency check's stop line stays
  `error: release stopped at dependency check`. The two steps share one
  prompt function that takes the step's name.
- Progress lines while waiting go to stdout, one per poll, as in
  `release.sh`.
- The script's header comment no longer calls prep local only. It names
  the step, its prerequisites, and that prep pushes a throwaway branch.

## 5. Testing

The repo has no tests for its scripts. The implementer checks by hand
in a throwaway clone under `/tmp` whose `origin` is a local bare repo,
so push and delete are real git and never reach GitHub. `gh` is a stub
on `PATH` that returns scripted run JSON.

- a run that succeeds continues to the version bump, and the bare repo
  has no `release-check` afterwards
- a failing run, then `n`, an empty answer and `</dev/null`, each stop
  with exit `1`, an unchanged `Cargo.toml` and no branch left
- the same failure with `y` continues
- a run stuck `in_progress` with a short `CI_CHECK_TIMEOUT` times out
  and prompts, naming that timeout
- a `gh` stub that fails mid-wait gives the could-not-finish prompt and
  deletes the branch
- an older run on the same sha (earlier creation time) is ignored
- a leftover `release-check` at the same sha is deleted before the push
- a failed push and a missing `jq` each give the skip prompt, with
  nothing pushed for the missing tool
- a failed delete prints the warning and keeps the exit code
- Ctrl-C (SIGINT) during the wait deletes the branch
- `CI_CHECK_TIMEOUT=abc` stops with its error before any push
- the dependency check still stops with its own stop line

The task report records each run's output. `ci.yml` stays valid YAML.
Real GitHub behavior is first exercised at the next release.

## 6. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `scripts/prep_release.sh`, `.github/workflows/ci.yml` | §3, §4 |

NOTES.md's phase-0 release slot stays with the coordinator, who adds the
CI check at close-out and drops "reversible" from prep's description.

## User-visible surface

None. Phase 14 is skipped.
