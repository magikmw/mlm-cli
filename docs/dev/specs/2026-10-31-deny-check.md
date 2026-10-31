# Dependency check before release — design spec

**Baseline**: written against `main` at `33537d1` (`v0.4.0` + `orphan-stop-warning` + `week-notes` + `export-notes` + `panic-fixes` + `relative-ids` + `doc-drift`, unreleased).
**Branch**: `deny-check`.
**Status**: draft.
**Input**: the user's ask on 2026-10-31 for a scan that pauses a release
without breaking the release flow, and the scope agreed in NOTES.md
entry 165.

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
- No change to `scripts/release.sh`, the workflows or the shipped binary.
- No ignore entries. The first advisory that needs one is a decision for
  that release.

## 1.2 Known issues

- An advisory published between releases is caught only at the next
  release.

## 2. Prerequisites

`cargo-deny` 0.19 on the release machine (installed: 0.19.8).

## 3. `deny.toml`

A new `deny.toml` at the repo root configures two checks.

| Check | Fails on |
|---|---|
| `advisories` | a RustSec vulnerability, an unmaintained crate or a yanked version, for any crate in `Cargo.lock` |
| `licenses` | a crate whose license is not on the allowlist |

- The license allowlist is the set the current `Cargo.lock` uses:
  `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSL-1.0`, `EUPL-1.2`,
  `MIT`, `MPL-2.0`, `Unicode-3.0`, `Unlicense`, `Zlib`.
- An allowlisted license that no crate uses is not a finding.
- `cargo deny check advisories licenses` passes on `main` at the
  baseline.

## 4. `prep_release.sh` step

The check runs after the preconditions (on `main`, clean tree, new tag,
tools present) and before the version bump, so a stop leaves the tree
untouched.

| Case | Output | Then |
|---|---|---|
| check passes | `dependency check: ok` | continue |
| check fails | `cargo deny`'s own report, then `dependency check found problems. continue anyway? [y/N] ` | `y` or `Y` continues |
| `cargo-deny` not on PATH | `dependency check skipped: cargo-deny not found. continue anyway? [y/N] ` | `y` or `Y` continues |

- Any other answer, an empty answer, or end of input stops the script
  with `error: release stopped at dependency check`, exit `1`.
- With no terminal on stdin the prompt reads end of input and stops.
- The check's own exit code never stops the script under `set -e`.

## 5. Testing

The repo has no tests for its scripts. The implementer checks each §4
row by hand in a throwaway clone under `/tmp`:

- the baseline passes and the script reaches the version bump
- a `deny.toml` with `MIT` removed from the allowlist fails, and `n`,
  an empty answer and `</dev/null` each stop with exit `1` and an
  unchanged `Cargo.toml`
- the same failure with `y` continues to the version bump
- `PATH` without `cargo-deny` gives the skip prompt

The task report records each run's output.

## 6. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `deny.toml`, `scripts/prep_release.sh` | §3, §4 |

NOTES.md's phase-0 release slot stays with the coordinator.

## User-visible surface

None. Phase 14 is skipped.
