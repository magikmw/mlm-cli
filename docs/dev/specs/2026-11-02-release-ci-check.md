# CI check before release — design spec

**Baseline**: written against `main` at `d03bc0a` (`v0.5.0` + the
`prep_release.sh` hint fix and the v0.5.0 NOTES entry, unreleased).
**Branch**: `release-ci-check`.
**Status**: draft.
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

## 1.2 Known issues

- Prep takes about five minutes longer, the length of one CI run.
- A release runs CI twice, once in prep and once in `release.sh`.
- The check runs on the commit before the bump. A failure that only the
  version bump causes shows up in `release.sh`, as today.
- Prep pushes to GitHub. The push only creates the `release-check`
  branch, and the step deletes that branch again.

## 2. Prerequisites

`gh` (authenticated) and `jq` on the release machine, as `release.sh`
already needs, and push access to `origin`.

## 3. CI trigger

`ci.yml`'s `push` trigger adds the branch `release-check`. Nothing else
in the workflow changes.

## 4. `prep_release.sh` step

The check runs after the dependency check and before the version bump.
A stop leaves the tree untouched.

1. Push `HEAD` to `origin` as `release-check`, with force, because the
   branch is throwaway.
2. Wait for the `ci.yml` run on `HEAD`'s sha, polling the way
   `release.sh` does: every 15 seconds, for up to 600 seconds.
   `CI_CHECK_TIMEOUT` (seconds) overrides the 600, for testing.
3. Delete the remote `release-check` branch, whatever the result. A
   failed delete prints a warning and does not stop the script.

| Case | Output | Then |
|---|---|---|
| run succeeds | `CI check: ok (<sha>)` | continue |
| run completes with another conclusion | `CI check failed on <sha> (conclusion: <conclusion>). <run URL>` then `continue anyway? [y/N] ` | `y` or `Y` continues |
| no completed run after 600 seconds | `CI check timed out after 600s on <sha>. <run URL, if a run was found>` then `continue anyway? [y/N] ` | `y` or `Y` continues |
| push fails, or `gh` or `jq` missing | `CI check skipped: <reason>.` then `continue anyway? [y/N] ` | `y` or `Y` continues |

- The prompt follows the dependency check's rules. Any answer but `y` or
  `Y`, end of input, or stdin that is not a terminal stops the script
  with `error: release stopped at CI check`, exit `1`.
- The dependency check's stop line stays
  `error: release stopped at dependency check`. The two steps share one
  prompt function that takes the step's name.
- Progress lines while waiting go to stdout, one per poll, as in
  `release.sh`.
- The script's header comment names the step and its prerequisites.

## 5. Testing

The repo has no tests for its scripts. The implementer checks by hand
in a throwaway clone under `/tmp`, with a stub `gh` and `git push` on
`PATH` where the case needs one. No hand test pushes to the real
`origin`.

- a run that succeeds continues to the version bump
- a failing run, then `n`, an empty answer and `</dev/null`, each stop
  with exit `1` and an unchanged `Cargo.toml`
- the same failure with `y` continues
- a run that never completes (a stub reporting `in_progress`, with a
  short `CI_CHECK_TIMEOUT`) times out and prompts
- a failed push and a missing `jq` each give the skip prompt
- the delete runs in every case where the push succeeded
- the dependency check still stops with its own stop line

The task report records each run's output. `ci.yml` stays valid YAML.

## 6. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `scripts/prep_release.sh`, `.github/workflows/ci.yml` | §3, §4 |

NOTES.md's phase-0 release slot stays with the coordinator, who adds the
CI check at close-out.

## User-visible surface

None. Phase 14 is skipped.
