# release-ci-check spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Spec: `docs/dev/specs/2026-11-02-release-ci-check.md` (draft, branch `release-ci-check` at `4f31184`).

## Scope and method

- Read the spec, `scripts/prep_release.sh`, `scripts/release.sh`, all four workflows, NOTES.md current state, the phase-0 table and entries 165-169.
- The Bash hook refused `gh` and `curl`, so no Actions API query ran. Claims about GitHub behavior below rest on the workflow source and GitHub's documented behavior. Each one is marked unverified where that matters.
- The hook also refused `git init`, so push and delete behavior on a local bare remote was not tested. Bash `set -e` behavior was tested in `/tmp/cd-scratch/release-ci-check-spec`.

## Checks that hold

- Summary claim (spec:12-15). Only `origin/main` exists remotely (`git branch -a`), and `origin/main` is `88d9775`, so changesets do reach GitHub only at release. NOTES:2295-2299 records the v0.5.0 Windows failure.
- Other workflows stay quiet on a `release-check` push.
  - `release.yml:41-45` triggers on `pull_request` and tag pushes only.
  - `sign-artifacts.yml:10-11` and `publish-crates-io.yml:10-11` are `workflow_call` only.
  - CodeQL default setup scans the default branch, protected branches and PRs to them. `release-check` is unprotected unless a ruleset covers it. Unverified, because the API was unreachable.
- `release.sh`'s gate is unaffected. It queries `head_sha=$(git rev-parse HEAD)` after committing the bump (`release.sh:76,89`). The release commit is a new sha, a child of the checked one, so no `release-check` run can match it.
- `ci.yml` has no `concurrency`, `if:` or ref conditions (grep of `ci.yml`), so adding the branch at `ci.yml:5` is the only change §3 needs.
- GitHub reads the workflow file from the pushed commit. The trigger therefore works only once HEAD contains the §3 edit. That holds for every release after this changeset merges.

## Findings

### F1. A `gh` or `jq` failure mid-wait kills the script and leaves the branch (high, error-handling gap)

- Claim: spec:57 says "Delete the remote `release-check` branch, whatever the result". The spec:65 table covers only "push fails, or `gh` or `jq` missing".
- Evidence: the loop the spec names as its model runs `repo="$(gh repo view ...)"` and `run_json="$(gh api ...)"` unguarded (`release.sh:81,89`). Under `set -euo pipefail` (`prep_release.sh:14`), a failing command substitution in an assignment exits the script. Scratch test: `set -euo pipefail; f(){ local x; x="$(false)"; echo after; }; f` exits 1 without printing `after`.
- Triggers include an expired `gh` token, a network blip, an API 5xx or rate limit, and non-JSON output reaching `jq`. `gh` present but unauthenticated is a runtime failure too, and the spec's "missing" row does not cover it.
- Consequence: the script exits with `gh`'s own error. The maintainer sees no `error: release stopped at CI check` line, and the remote branch is never deleted.
- Fix: require an `EXIT`/`INT`/`TERM` trap that deletes the branch once the push has succeeded. Guard every `gh` and `jq` call with `if !` or `|| …`. Add a table row such as "`gh` or `jq` call fails → `CI check skipped: <reason>.` + prompt".

### F2. `head_sha=…&per_page=1` can return an older run on the same sha (medium, missing edge case)

- Claim: spec:54 says "Wait for the `ci.yml` run on `HEAD`'s sha, polling the way `release.sh` does".
- Evidence: `release.sh:89` queries `actions/workflows/ci.yml/runs?head_sha=${sha}&per_page=1` and reads `.workflow_runs[0]` (`release.sh:90-91`). That filter does not restrict branch, event or creation time. The first poll runs right after the push, before GitHub has registered the new run (the race `release.sh:111` covers with `status: not found`).
- Cases where an older run with the same sha exists:
  - prep rerun on an unchanged HEAD. The tree is dirty after a successful prep (`prep_release.sh:43`), so a redo means `git checkout .`, then the same sha is pushed again.
  - a rerun after a timeout and `n`
  - HEAD already on `origin/main`, e.g. a hand-fix commit like `88d9775` that was pushed during recovery
- Consequence: the first poll reads the old run. A completed old run returns at once, as `ok` or as a stale failure (a cancelled or flaky run). The new run then keeps going unobserved. Spec:54 says the script waits for "the" run, which this does not deliver.
- Unverified against the live API (hook refused `gh`). The API docs say the list is newest-first and that `head_sha` matches across branches.
- Fix: add `&branch=release-check&event=push` plus `&created=>=<push time UTC>`, or capture the run id first seen after the push and poll `actions/runs/<id>`.

### F3. Some pushes create no run, which costs a full timeout (medium, missing edge case)

- Claim: spec:52-53 says "Push `HEAD` ... with force, because the branch is throwaway". Spec:58 lets a failed delete only warn.
- Evidence and cases:
  - A failed delete (spec:58) leaves `origin/release-check` behind. A later prep at the same sha force-pushes to an unchanged ref, and git reports `Everything up-to-date`. No push event fires, so no new run starts. With F2 the script reads the old run; with F2 fixed it times out after 600s.
  - A HEAD commit message containing `[skip ci]` or a sibling marker suppresses push runs.
- Fix: delete the remote branch before pushing (ignoring "remote ref does not exist"), so every push creates the branch. Optionally treat "no run found by N seconds" as its own skip case.

### F4. A non-integer `CI_CHECK_TIMEOUT` silently passes the check (medium, error-handling gap)

- Claim: spec:56 says "`CI_CHECK_TIMEOUT` (seconds) overrides the 600, for testing."
- Evidence: `release.sh:88` and `release.sh:116` use `[ "$elapsed" -lt "$timeout" ]` and `[ "$elapsed" -ge "$timeout" ]`. With a non-integer, `[` returns 2. `set -e` ignores failures in `while`/`if` conditions, so the loop never runs and the timeout branch is skipped. Scratch test with `timeout="10m"` printed `integer expected` twice, then `fell through as success`, exit 0.
- Consequence: an implementation copied from `release.sh` continues to the bump with no CI result, and the only sign is a shell warning.
- Fix: validate `CI_CHECK_TIMEOUT` against `^[0-9]+$` and stop with an error otherwise. Track success in an explicit variable rather than inferring it from `elapsed < timeout`.

### F5. The script header still says prep is local and reversible (medium, internal inconsistency)

- Claim: spec:75 requires only that "The script's header comment names the step and its prerequisites."
- Evidence: `prep_release.sh:4-5` says "This script does the *local, reversible* half of a release". After this changeset prep pushes to GitHub and runs CI there. NOTES:28's phase-0 row also says "reversible bump", and the coordinator owns that row (spec:104-105).
- Fix: have §4 require rewording `prep_release.sh:4` (for example "local half, plus a CI run on a throwaway branch"). Note in §6 that the phase-0 row drops or qualifies "reversible".

### F6. The timeout message hardcodes 600 (low, internal inconsistency)

- Claim: spec:64 prints `CI check timed out after 600s on <sha>.` Spec:56 makes the 600 overridable.
- Fix: write `<timeout>s`.

### F7. Tool checks and push have no stated order (low, missing edge case)

- Claim: spec:65 groups "push fails, or `gh` or `jq` missing" into one skip row. Spec:91 tests "the delete runs in every case where the push succeeded".
- Evidence: §4 steps 1-3 (spec:52-58) do not say whether the `gh`/`jq` check runs before the push. If it runs after, a missing `jq` pushes, then skips, and needs the delete. If it runs before, nothing is pushed.
- Fix: state that the tool check runs before the push, so the skip leaves GitHub untouched.

### F8. The hand tests cannot run as written and miss the risky paths (medium, testing gap)

- Claim: spec:79-82 says to use "a stub `gh` and `git push` on `PATH`" and no push to the real `origin`.
- Evidence:
  - `git push` cannot be stubbed on `PATH` by itself. The only option is a `git` wrapper, and that wrapper also intercepts `git rev-parse`/`git status` in the preconditions (`prep_release.sh:37,43,49`).
  - A local bare remote as `origin` makes real pushes work. But then `gh repo view` (`release.sh:81`) cannot resolve a repo, so `gh` must be stubbed wholesale.
  - Hiding `jq` (spec:90) needs a curated `PATH`, because `jq` sits in `/usr/bin` beside `bash` and `git`. The deny-check report did the same for `cargo-deny` (`deny-check-task-1-report.md:53`). The spec should say so.
- Missing cases:
  - a failed delete prints the warning and continues (spec:58 promises this, and spec:91 only checks that the delete runs)
  - `gh api` failing mid-wait (F1)
  - an older completed run on the same sha (F2)
  - Ctrl-C during the wait (F12)
  - a bad `CI_CHECK_TIMEOUT` (F4)
- Nothing in the plan exercises GitHub itself, meaning the branch-creation push event and what `head_sha` returns. The first real test is the next release, where a mistake shows up as a 10-minute timeout and a prompt.
- Fix: specify a local bare `origin` plus a `gh` stub that takes the run JSON from a file. Add the missing cases. Optionally allow one supervised real push (`release-check` at the current `main`) before lock.

### F9. Pre-release commits become public and stay public (medium, undeclared user-visible gap)

- Claim: spec:34-35 says "The push only creates the `release-check` branch, and the step deletes that branch again."
- Evidence: deleting the branch does not remove the pushed objects. Every commit since `origin/main` stays fetchable by sha. The Actions run page links to the commit and shows its checkout and logs, and the run outlives the branch. This holds even when prep stops and the release is abandoned or delayed.
- NOTES:2316-2317 rejected option C because it "puts unreleased docs on GitHub". Option A does that too, minutes to days earlier than release, and the spec does not say so.
- Fix: add a known issue. "The check publishes every commit since the last release to GitHub. They stay reachable by sha and through the run page after the branch is deleted, even if the release is abandoned."

### F10. Failed runs stay in the public Actions history (low, undeclared user-visible gap)

- Evidence: a failed prep check leaves a red `CI` run labelled `release-check` in the Actions tab permanently, beside the `main` and tag runs. GitHub also emails the pusher about failed runs by default.
- Fix: add a known issue naming the Actions tab entry and the failure email.

### F11. GitHub offers to open a PR for `release-check` (low, undeclared user-visible gap)

- Evidence: a push that creates a branch makes GitHub's remote print `Create a pull request for 'release-check' on GitHub by visiting: …` into prep's output. The repo page may also show a "had recent pushes" compare banner. Opening that PR would also run `ci.yml` (`ci.yml:6`) and `release.yml`'s `pull_request` plan (`release.yml:42`).
- Fix: a known-issue line saying the PR hint is expected and should be ignored.

### F12. Ctrl-C during the wait leaves the branch (low, undeclared user-visible gap)

- Claim: spec:57 says the delete happens "whatever the result".
- Evidence: the wait can run 10 minutes (`release.sh:84`). With no trap (none exists in `prep_release.sh`), an interrupt skips step 3. The branch stays visible in the repo's branch list until the next prep overwrites it, and it then triggers F3.
- Fix: the trap from F1 covers this. If no trap is wanted, record it as a known issue.

### F13. Abandoned runs keep going and every prep pays the wait (low, undeclared user-visible gap)

- Evidence:
  - After a timeout or Ctrl-C, the run keeps going on GitHub. A timeout plus `y` then starts `release.sh`'s run while the prep run is still going.
  - The spec adds no skip flag, unlike `release.sh --skip-ci-check` (`release.sh:16-21`). Each prep rerun, including a redo after `git checkout .`, waits again, and F2 makes that wait unreliable.
- Fix: record these as known issues, or add `--skip-ci-check` to prep if the user wants one. A new flag is a user call, not a fold decision.

### F14. A green prep check tempts `--skip-ci-check` (low, undeclared user-visible gap)

- Claim: spec:32-33 says "A failure that only the version bump causes shows up in `release.sh`, as today."
- Evidence: `release.sh:18-21` says to use `--skip-ci-check` "when you already know CI is green (e.g. it just ran for an identical commit)". After a green prep check that rationale looks satisfied, but the release commit is not identical (it changes `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`). Known issue 3 holds only when the gate is not skipped.
- Fix: extend known issue 3 with "…unless `release.sh --skip-ci-check` is used. The prep check does not cover the release commit."

## Not filed

- Spec:30 says CI takes "about five minutes". Unverified, because the API was unreachable. `release.sh`'s 600s timeout passed at v0.5.0 (NOTES:2297-2299), so 600 is at least not known to be short.
- `gh repo view` and `git push origin` could name different repos if `gh repo set-default` points elsewhere. `release.sh` makes the same assumption, so this is not new.

## Verdict

ship-with-followups

F1-F4 change §4's mechanism (trap and guarded calls, a narrower run query, delete before push, timeout validation) and should fold before lock. F8 should fold with them. F5-F7 are wording. F9-F14 are known-issue candidates, apart from the skip flag in F13, which is a user call.
