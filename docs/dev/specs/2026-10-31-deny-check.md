# Dependency check before release — design spec

**Baseline**: written against `main` at `33537d1` (`v0.4.0` + `orphan-stop-warning` + `week-notes` + `export-notes` + `panic-fixes` + `relative-ids` + `doc-drift`, unreleased).
**Branch**: `deny-check`.
**Status**: locked 2026-10-31 after one review
(`docs/dev/plans/reports/deny-check-spec-review.md`, folded in NOTES.md
entry 166).
**Input**: the user's ask on 2026-10-31 for a scan that pauses a release
without breaking the release flow, the scope agreed in NOTES.md
entry 165, and the open CodeQL alert the user added to scope.

## 1. Summary

Nothing checks mlm's dependencies today. A release can ship a crate with
a known vulnerability, a yanked crate, or a crate whose license clashes
with EUPL-1.2, and nobody is told.

After this changeset `scripts/prep_release.sh` runs `cargo deny` over
`Cargo.lock` before it changes anything. A clean check continues. A
finding prints the report and asks whether to continue.

## 1.1 Non-goals

- No CI job. The check runs only in `prep_release.sh`.
- No `bans` or `sources` checks.
- No change to `scripts/release.sh`, the shipped binary, or any workflow
  besides §5.
- No ignore entries. The first advisory that needs one is a decision for
  that release.

## 1.2 Known issues

- An advisory published between releases is caught only at the next
  release.
- The check needs the network to fetch the advisory database and the
  crates.io index. Offline, it cannot run and prompts like a finding.
- An error running the check (no network, a bad config) prompts like a
  finding. `cargo deny`'s output shows which.
- A finding is fixed on a branch, since prep needs a clean `main`. A new
  dependency under a license off the allowlist pauses the next release
  until `deny.toml` allows it.
- Answering `y` leaves no record in the changelog, commit or tag.
- `release.sh` run alone skips the check.
- The CodeQL alert stays open until the fix is pushed with the next
  release.

## 2. Prerequisites

`cargo-deny` 0.19 on the release machine (installed: 0.19.8), and
network access during prep.

## 3. `deny.toml`

A new `deny.toml` at the repo root configures two checks.

| Check | Fails on |
|---|---|
| `advisories` | a RustSec vulnerability, unsound or unmaintained advisory affecting a locked version, or a yanked version, for any crate in `Cargo.lock` |
| `licenses` | a crate whose license is not on the allowlist |

- `[advisories]` sets `yanked = "deny"`, `unmaintained = "all"` and
  `unsound = "all"`, so none of them is only a warning.
- The license allowlist is the set the current `Cargo.lock` uses:
  `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSL-1.0`, `EUPL-1.2`,
  `MIT`, `MPL-2.0`, `Unicode-3.0`, `Unlicense`, `Zlib`.
- `unused-allowed-license = "allow"`, so an allowlisted license no crate
  needs prints nothing.
- `cargo deny --locked check advisories licenses` passes on `main` at
  the baseline with no warnings.
- `Cargo.toml`'s `exclude` lists `deny.toml`, so the crates.io package
  does not ship it.

## 4. `prep_release.sh` step

The check runs after the preconditions (on `main`, clean tree, new tag,
tools present) and before the version bump. It runs as
`cargo deny --locked check advisories licenses`, so it never rewrites
`Cargo.lock`, and a stop leaves the tree untouched.

| Case | Output | Then |
|---|---|---|
| check passes | `cargo deny`'s own lines, then `dependency check: ok` | continue |
| check fails or cannot run | `cargo deny`'s own report, then `dependency check found problems. continue anyway? [y/N] ` | `y` or `Y` continues |
| `cargo-deny` not on PATH | `dependency check skipped: cargo-deny not found. continue anyway? [y/N] ` | `y` or `Y` continues |

- Any other answer, an empty answer, or end of input stops the script
  with `error: release stopped at dependency check`, exit `1`.
- When stdin is not a terminal, a case that would prompt prints its
  message and the stop line, and stops with exit `1`. A piped `y` does
  not continue.
- Neither the check's exit code nor `read` at end of input stops the
  script through `set -e`. Every stop goes through the stop line.
- The script's header comment names the step and the `cargo-deny`
  prerequisite.

## 5. CI token permissions

- CodeQL's one open alert is `actions/missing-workflow-permissions` at
  `.github/workflows/ci.yml:14`. CI declares no `permissions`, so its
  token gets the repo default.
- `ci.yml` gains a top-level `permissions: contents: read`. No CI step
  writes to the repo, so every job still passes.
- The alert closes on the next CodeQL run after the fix reaches `main`.

## 6. Testing

The repo has no tests for its scripts. The implementer checks each §4
row by hand in a throwaway clone under `/tmp`:

- the baseline passes and the script reaches the version bump
- a `deny.toml` with `MIT` removed from the allowlist fails, and `n`,
  an empty answer and `</dev/null` each stop with exit `1` and an
  unchanged `Cargo.toml`
- the same failure with `y` continues to the version bump
- the same failure with stdin from a pipe holding `y` stops
- `PATH` without `cargo-deny` gives the skip prompt
- `cargo package --list` does not include `deny.toml`

The task report records each run's output. `ci.yml` stays valid YAML,
and the job steps are read to confirm none needs a write scope.

## 7. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `deny.toml`, `scripts/prep_release.sh`, `.github/workflows/ci.yml`, `Cargo.toml` (`exclude` only) | §3, §4, §5 |

NOTES.md's phase-0 release slot stays with the coordinator, who adds the
check and its prerequisite at close-out.

## User-visible surface

None. Phase 14 is skipped.
