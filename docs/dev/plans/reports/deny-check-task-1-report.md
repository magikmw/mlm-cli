# deny-check Task 1 report

Spec: `docs/dev/specs/2026-10-31-deny-check.md` (§3, §4, §5).
Base: the worktree started on `main` and was rebased onto `deny-check` at `79be0b5` before any work.

## What shipped

| File | Change |
|---|---|
| `deny.toml` (new) | `[advisories]` `yanked = "deny"`, `unmaintained = "all"`, `unsound = "all"`. `[licenses]` allowlist of the nine §3 licenses, `unused-allowed-license = "allow"` |
| `Cargo.toml` | `"deny.toml"` added to `exclude` |
| `.github/workflows/ci.yml` | top-level `permissions: contents: read` |
| `scripts/prep_release.sh` | new step 2 "Check dependencies" between the preconditions and the version bump. Header comment names the step and the `cargo-deny` 0.19 + network prerequisite. Later sections renumbered 3-5 (the old numbering jumped from 3 to 6) |

Commits: `f01039d` (config, exclude, CI), `df874d6` (script), plus this report.

### Script step

- `cargo-deny` on PATH → `cargo deny --locked check advisories licenses`, inside `if`, so its exit code never trips `set -e`.
- Pass → `dependency check: ok` on stdout.
- Fail or cannot run → `confirm_or_stop "dependency check found problems."`.
- Not on PATH → `confirm_or_stop "dependency check skipped: cargo-deny not found."`.
- `confirm_or_stop` prints `<msg> continue anyway? [y/N] ` with `printf` to stderr. It reads an answer only when `[ -t 0 ]` holds, and the `read` sits inside the same `if` condition, so end of input never trips `set -e`.
- No terminal, or a failed `read` (EOF, or a partial last line), sets the answer empty and prints a newline so the stop line starts on its own line.
- Only `y` or `Y` continues. Everything else prints `error: release stopped at dependency check` and exits 1.

## Hand tests (§6)

Each run used a throwaway clone at `/tmp/deny-check-test` of this branch, on a local `main` branch so the preconditions pass. The failure cases ran on a clone commit that removes `"MIT"` from `deny.toml` (committed, so the tree stays clean). Between runs `git checkout -- .` reset any bump. The clone was deleted afterwards. Nothing was tagged or pushed.

`script(1)` is not installed here, so the terminal cases ran through a small Python `pty.fork()` driver in the session scratchpad. It sends the answer once `[y/N] ` appears (or `^D` for end of input), and prints the output tail and exit code.

### Red, before the script change

`./scripts/prep_release.sh 0.4.1 </dev/null` with MIT removed bumped the version and changelog with no check, exit 0. That confirmed the step was missing.

Without `deny.toml`, `cargo deny --locked check advisories licenses` on the worktree printed `advisories ok, licenses FAILED` (the default allowlist is empty).

### Green

| # | Case | Command | Tail of output | Exit | Tree after |
|---|---|---|---|---|---|
| 1 | baseline passes | `./scripts/prep_release.sh 0.4.1 </dev/null` | `advisories ok, licenses ok` / `dependency check: ok` / `Upgrading mlm from 0.4.0 to 0.4.1` / ... `Version bumped to 0.4.1.` | 0 | `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` modified (bump reached) |
| 2 | MIT removed, `n` on a tty | pty driver, answer `n` | `advisories ok, licenses FAILED` / `dependency check found problems. continue anyway? [y/N] n` / `error: release stopped at dependency check` | 1 | clean, `version = "0.4.0"` |
| 3 | MIT removed, empty answer on a tty | pty driver, answer `""` | `... continue anyway? [y/N] ` / `error: release stopped at dependency check` | 1 | clean, `version = "0.4.0"` |
| 4 | MIT removed, `^D` on a tty | pty driver, `EOF` | `... continue anyway? [y/N] ` / `error: release stopped at dependency check` | 1 | clean, `version = "0.4.0"` |
| 5 | MIT removed, `</dev/null` | `./scripts/prep_release.sh 0.4.1 </dev/null` | cargo-deny's `error[rejected]: failed to satisfy license requirements` report (540 lines) / `advisories ok, licenses FAILED` / `dependency check found problems. continue anyway? [y/N] ` / `error: release stopped at dependency check` | 1 | clean, `version = "0.4.0"` |
| 6 | MIT removed, `y` on a tty | pty driver, answer `y` | git-cliff progress / `Version bumped to 0.4.1.` / `When ready, trigger ./scripts/release.sh "v0.4.1"` | 0 | `Cargo.toml` (`version = "0.4.1"`), `Cargo.lock`, `CHANGELOG.md` modified |
| 7 | MIT removed, `Y` on a tty | pty driver, answer `Y` | `When ready, trigger ./scripts/release.sh "v0.4.1"` | 0 | bump reached |
| 8 | MIT removed, piped `y` | `printf 'y\n' \| ./scripts/prep_release.sh 0.4.1` | `advisories ok, licenses FAILED` / `dependency check found problems. continue anyway? [y/N] ` / `error: release stopped at dependency check` | 1 | clean, `version = "0.4.0"` |
| 9 | no cargo-deny, `</dev/null` | `env PATH=<dir with cargo, git-cliff, cargo-set-version, rustc>:/usr/bin ./scripts/prep_release.sh 0.4.1 </dev/null` | `Releasing mlm 0.4.1 (currently: version = "0.4.0")` / `dependency check skipped: cargo-deny not found. continue anyway? [y/N] ` / `error: release stopped at dependency check` | 1 | clean |
| 10 | no cargo-deny, `n` on a tty | same PATH, pty driver, `n` | `dependency check skipped: cargo-deny not found. continue anyway? [y/N] n` / `error: release stopped at dependency check` | 1 | clean |
| 11 | no cargo-deny, `y` on a tty | same PATH, pty driver, `y` | `Version bumped to 0.4.1.` / ... | 0 | bump reached |
| 12 | package list | `cargo package --list --allow-dirty` on the worktree | no `deny.toml` line (before the `exclude` edit it listed `deny.toml`) | 0 | n/a |

### CI

- `python3 -c 'yaml.safe_load(...)'` on `ci.yml` parses and gives `permissions == {'contents': 'read'}`.
- The one job's steps are `actions/checkout@v6` and `run:` steps (rustup, fmt, build, test, clippy, the e2e smoke script). None writes to the repo, opens an issue or PR, or uploads to a release, so `contents: read` covers them.

## Verification set (worktree, final tree)

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo build` | ok |
| `cargo test` | all suites ok, 0 failed (856 unit tests plus the integration suites) |
| `cargo clippy --all-targets -- -D warnings` | ok, no warnings |
| `bash -n scripts/prep_release.sh` | exit 0 |
| `cargo deny --locked check advisories licenses` | `advisories ok, licenses ok`, exit 0, no warnings |
| YAML parse of `ci.yml` | ok |
| `cargo package --list --allow-dirty` | no `deny.toml` |
| pre-commit hook (fmt, complexity, coverage) | passed on both commits, coverage unchanged |

No `.claude/cd-verify` exists in the worktree or the main checkout.

## Choices and things not done

- The prompt goes to stderr, like the script's other `error:` lines. `dependency check: ok` goes to stdout, like `Releasing mlm ...`. The spec names the text, not the stream.
- No advisories-failure hand test (a crafted local advisory db). §6 does not ask for one. The advisory keys are visible in `deny.toml` and the pass output above.
- `shellcheck` is not installed, so it was not run.
- The worktree guard refused `git checkout -b main` in the clone when run as `git -C /tmp/deny-check-test ...`. The same command run from inside the clone directory went through. That clone was the throwaway the dispatch asked for, and it is gone now.
- The guard also refused `git remote remove origin` in the clone. The clone kept `origin` pointing at this worktree. The script never pushes or fetches, so the remote did not matter.
- NOTES.md is untouched (coordinator's).
