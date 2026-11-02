# release-ci-check Task 1 report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Spec: `docs/dev/specs/2026-11-02-release-ci-check.md` (§3, §4, §5).
Decisions: NOTES.md entries 169-170. Review: `release-ci-check-spec-review.md`.

## Base

- The worktree started on `main` (`88d9775`).
- Rebased onto `release-check` tip `3f5e9ad` before any change.

## What shipped

### `.github/workflows/ci.yml`

- `push.branches` is now `[main, release-check]`. Nothing else changed.

### `scripts/prep_release.sh`

- Header comment no longer says "local, reversible". It names the CI step, its prerequisites (`gh` authenticated, `jq`, push access to `origin`), the throwaway `release-check` branch and `CI_CHECK_TIMEOUT`.
- `confirm_or_stop` now takes the step name and the message. The dependency check passes `"dependency check"`, so its stop line is unchanged.
- New section 3, "Check CI on HEAD", between the dependency check and the bump. Later sections are renumbered 4-6.
  1. `CI_CHECK_TIMEOUT` (default 600) must match `^0*[1-9][0-9]*$`. Otherwise the script prints `error: CI_CHECK_TIMEOUT must be a positive integer` and exits 1. The value is normalized with `10#` so a leading zero is not read as octal.
  2. `run_ci_check` checks for `gh` and `jq`. A missing tool gives `CI check skipped: <tool> not found.` with nothing pushed.
  3. It resolves the repo with `gh repo view` before pushing. A failure gives `CI check could not finish: gh repo view failed.` with nothing pushed.
  4. It deletes any remote `release-check` (`git push --quiet origin --delete`, output silenced, failure ignored), notes the UTC time, then pushes `HEAD:refs/heads/release-check` with `--quiet`. A failed push gives `CI check skipped: push to origin failed.`, and git's own error stays visible above it.
  5. `wait_for_ci` polls `actions/workflows/ci.yml/runs?branch=release-check&event=push&head_sha=<sha>&per_page=20`. One `jq` call picks the first run whose `created_at` is no earlier than the push time and prints its status, conclusion and URL. It polls every 15 s, but the last sleep is cut short so the final poll lands on the timeout. One progress line per poll goes to stdout.
  6. Every `gh` and `jq` call sits in an `if !` guard. A failure sets `ci_reason` (`gh api failed`, `jq could not read the run list`) and gives `CI check could not finish: <reason>.`.
  7. The branch is deleted right after the wait, before the result line and prompt. `delete_ci_branch` is also the `EXIT` trap, and the `INT` and `TERM` traps call it and then exit 130 or 143. It disarms itself (`ci_pushed=0`) first, so it runs once. It is armed just before the push and disarmed if the push fails. A failed delete prints `warning: could not delete remote branch release-check` and keeps the exit code. The traps are cleared after the step.
- Outcome lines, all as in the §4 table:
  - `CI check: ok (<sha>)`
  - `CI check failed on <sha> (conclusion: <c>). <url> continue anyway? [y/N] `
  - `CI check timed out after <N>s on <sha>.` plus ` <url>` when a run was found
  - `CI check could not finish: <reason>.`
  - `CI check skipped: <reason>.`
- Every prompt stops with `error: release stopped at CI check`, exit 1, unless the answer is `y` or `Y`.

## Choices the spec left open

- `gh repo view` runs before the push. Its failure counts as "could not finish", since it is a `gh` call failing, and nothing is pushed. The spec's table does not place this call. Running it first keeps an unauthenticated `gh` from pushing anything.
- The created-time filter runs in `jq` on the client, not as the API's `created=` parameter. The hand tests can then exercise it against stub data, and `>=` needs no URL encoding. ISO 8601 UTC strings compare correctly as strings.
- A failed pre-push delete is silent. The usual cause is a missing branch, which is not an error. If the remote is unreachable, the push fails next and gives the skip prompt.
- `CI_CHECK_TIMEOUT` is checked inside the CI step, after the dependency check, as §4 step 1 orders it. A bad value still stops before any push.

## Not done

- No `--skip-ci-check` flag, per NOTES 170 (F13).
- No margin for clock skew between the local clock and GitHub's `created_at`. The spec says "no earlier than the push", and the time is taken before the push, which gives a small margin. A local clock running ahead of GitHub's by more than the push and registration time would hide the new run, and the check would time out and prompt. Real behavior is first seen at the next release, as §5 says.
- No shellcheck run, because `shellcheck` is not installed here.

## Hand tests

### Harness

- Driver: `/tmp/claude-659200001/-home-magikmw-projects-mlm-cli/cf1d0b51-e308-4ab4-aa33-f9324d704282/scratchpad/drive.py` (Python). The `gh` and `cargo-deny` stubs sit in `.../scratchpad/stubs/`.
- Each full run rebuilds `.../scratchpad/hand/`:
  - `origin.git` is a `git init --bare` repo. Its `pre-receive` hook rejects a `release-check` create or delete when a flag file exists. Its `post-receive` hook logs every accepted ref change.
  - `clone` is a clone of this worktree, on `main`, with the script and `ci.yml` under test committed in it. `origin` is set to the bare repo.
- Before every case the driver asserts that `git remote get-url origin` is the bare repo path and holds no `github.com`. Nothing reached GitHub, nothing was tagged, and `~/.local/share/mlm` was not touched.
- PATH is the stubs dir, `~/.cargo/bin`, then `/usr/bin:/usr/local/bin`. The `jq missing` case swaps the last part for a symlink farm of `/usr/bin` without `jq`.
- The `gh` stub serves `api.<n>.json` for poll n, or `api.json`, with `@NOW@` replaced by the current UTC time. It fails poll n when `fail.<n>` exists, and fails `gh repo view` when `fail.repo` exists. It logs the bare repo's branches at each call, which shows the branch existed during the wait.
- The `cargo-deny` stub passes, or fails when `DENY_FAIL` is set.
- A terminal answer goes through a pty on stdin, written before the run. A pty's echo goes to the driver's side and is not captured, so in the outputs below the stop line follows the prompt on the same line. `</dev/null` cases print the newline themselves.
- The command for every case is `./scripts/prep_release.sh 9.9.9` in the clone. `<sha>` below is `fd15ca3421b35f350b6f6b5372aafdb760eccafe`, the clone's HEAD.

### Red run

Against the unchanged script, 15 of 16 cases failed. Every failure was the missing CI step, for example no `CI check: ok` line and no `release-check` create in the bare repo's log. The `dependency check stop` case passed, as it covers existing behavior.

### Green run

All 16 cases passed. The two git-cliff `INFO` lines in the continue cases are trimmed to `[git-cliff INFO lines]`, and the cargo-deny stub's `advisories ok, licenses ok` line is kept.

#### 1. success continues (`</dev/null`)

```
Releasing mlm 9.9.9 (currently: version = "0.5.0")
advisories ok, licenses ok
dependency check: ok
Waiting for CI on <sha> via branch release-check...
CI check: ok (<sha>)
   Upgrading mlm from 0.5.0 to 9.9.9
[git-cliff INFO lines]

Version bumped to 9.9.9.

CHANGELOG.md updated for review.

When ready, trigger ./scripts/release.sh 9.9.9
```

- Exit 0. Cargo.toml bumped.
- Bare repo log: `create refs/heads/release-check fd15ca3`, `delete refs/heads/release-check`. No branch left.

#### 2. failure, answer `n` (tty)

```
Releasing mlm 9.9.9 (currently: version = "0.5.0")
advisories ok, licenses ok
dependency check: ok
Waiting for CI on <sha> via branch release-check...
CI check failed on <sha> (conclusion: failure). https://github.com/test-owner/mlm-cli/actions/runs/3 continue anyway? [y/N] error: release stopped at CI check
```

- Exit 1. Cargo.toml unchanged. Created then deleted, no branch left.

#### 3. failure, empty answer (tty)

- Output identical to case 2.
- Exit 1. Cargo.toml unchanged. No branch left.

#### 4. failure, `</dev/null`

```
...
Waiting for CI on <sha> via branch release-check...
CI check failed on <sha> (conclusion: failure). https://github.com/test-owner/mlm-cli/actions/runs/3 continue anyway? [y/N] 
error: release stopped at CI check
```

- Exit 1. Cargo.toml unchanged. No branch left.

#### 5. failure, answer `y` (tty)

```
...
CI check failed on <sha> (conclusion: failure). https://github.com/test-owner/mlm-cli/actions/runs/3 continue anyway? [y/N]    Upgrading mlm from 0.5.0 to 9.9.9
[git-cliff INFO lines]

Version bumped to 9.9.9.
...
```

- Exit 0. Cargo.toml bumped. No branch left.

#### 6. stuck `in_progress` times out (`CI_CHECK_TIMEOUT=2`, answer `n`)

```
...
Waiting for CI on <sha> via branch release-check...
Waiting for CI on <sha> (status: in_progress, 0s elapsed)...
CI check timed out after 2s on <sha>. https://github.com/test-owner/mlm-cli/actions/runs/4 continue anyway? [y/N] error: release stopped at CI check
```

- Exit 1. Two polls, at 0 s and 2 s. Cargo.toml unchanged. No branch left.

#### 7. `gh api` fails on the second poll (`CI_CHECK_TIMEOUT=5`, answer `n`)

```
...
Waiting for CI on <sha> via branch release-check...
Waiting for CI on <sha> (status: in_progress, 0s elapsed)...
gh: HTTP 502: Bad Gateway
CI check could not finish: gh api failed. continue anyway? [y/N] error: release stopped at CI check
```

- Exit 1. The stub saw `refs/heads/release-check` on the bare repo during both polls. It was deleted afterwards, and no branch is left. Cargo.toml unchanged.

#### 8. older run on the same sha is ignored (`CI_CHECK_TIMEOUT=3`, `</dev/null`)

- Poll 1 returns only a completed `failure` run created `2020-01-01T00:00:00Z`. Poll 2 returns a new `success` run plus that old one.

```
...
Waiting for CI on <sha> via branch release-check...
Waiting for CI on <sha> (status: not found, 0s elapsed)...
CI check: ok (<sha>)
   Upgrading mlm from 0.5.0 to 9.9.9
...
```

- Exit 0. No `conclusion: failure` in the output. No branch left.

#### 9. leftover `release-check` at the same sha (`</dev/null`)

- Setup pushed `HEAD:refs/heads/release-check` to the bare repo first.
- Output matches case 1, with exit 0.
- Bare repo log, in order: `delete refs/heads/release-check`, `create refs/heads/release-check fd15ca3`, `delete refs/heads/release-check`. The leftover was deleted, so the push created the branch again. No branch left.

#### 10. push fails (pre-receive rejects the create, answer `n`)

```
...
dependency check: ok
remote: hook: push rejected        
To <scratch>/hand/origin.git
 ! [remote rejected] HEAD -> release-check (pre-receive hook declined)
error: failed to push some refs to '<scratch>/hand/origin.git'
CI check skipped: push to origin failed. continue anyway? [y/N] error: release stopped at CI check
```

- Exit 1. `gh api` never called. Cargo.toml unchanged. No branch left.

#### 11. `jq` missing (curated PATH, answer `n`)

```
Releasing mlm 9.9.9 (currently: version = "0.5.0")
advisories ok, licenses ok
dependency check: ok
CI check skipped: jq not found. continue anyway? [y/N] error: release stopped at CI check
```

- Exit 1. The bare repo log is empty, so nothing was pushed. Cargo.toml unchanged.

#### 12. `gh repo view` fails (answer `n`)

```
...
dependency check: ok
gh: not logged in
CI check could not finish: gh repo view failed. continue anyway? [y/N] error: release stopped at CI check
```

- Exit 1. Nothing pushed. Cargo.toml unchanged.

#### 13. delete fails (pre-receive rejects the delete, `</dev/null`)

```
...
Waiting for CI on <sha> via branch release-check...
remote: hook: delete rejected        
To <scratch>/hand/origin.git
 ! [remote rejected] release-check (pre-receive hook declined)
error: failed to push some refs to '<scratch>/hand/origin.git'
warning: could not delete remote branch release-check
CI check: ok (<sha>)
   Upgrading mlm from 0.5.0 to 9.9.9
...
Version bumped to 9.9.9.
...
```

- Exit 0, so the warning keeps the exit code. Cargo.toml bumped. The branch stays on the bare repo, and the driver removed it afterwards.

#### 14. SIGINT during the wait (default timeout)

- The driver sent SIGINT to the script's process group 1 s after the first poll, during the 15 s sleep.

```
...
Waiting for CI on <sha> via branch release-check...
Waiting for CI on <sha> (status: in_progress, 0s elapsed)...
```

- Exit 130. The bare repo log shows the create, then the delete from the trap. No branch left. Cargo.toml unchanged.

#### 15. `CI_CHECK_TIMEOUT=abc` (`</dev/null`)

```
Releasing mlm 9.9.9 (currently: version = "0.5.0")
advisories ok, licenses ok
dependency check: ok
error: CI_CHECK_TIMEOUT must be a positive integer
```

- Exit 1. Nothing pushed. Cargo.toml unchanged.

#### 16. dependency check stop (`DENY_FAIL=1`, answer `n`)

```
Releasing mlm 9.9.9 (currently: version = "0.5.0")
error[vulnerability]: stub advisory
dependency check found problems. continue anyway? [y/N] error: release stopped at dependency check
```

- Exit 1. No CI output, nothing pushed. Cargo.toml unchanged.

## Verification

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean, exit 0 |
| `cargo build` | finished, exit 0 |
| `cargo test` | all suites ok, 0 failed (856 unit tests plus 17 integration suites) |
| `cargo clippy --all-targets -- -D warnings` | finished, no warnings |
| `bash -n scripts/prep_release.sh` | exit 0 |
| YAML parse of `ci.yml` (`yaml.safe_load`) | ok, `push.branches` = `['main', 'release-check']` |
| `.claude/cd-verify` | not present in the worktree or the main checkout |
