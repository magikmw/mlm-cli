# Scheduled advisory check — design spec

**Baseline**: written against `main` at `ca852e6` (`v0.5.0` + the
`prep_release.sh` hint fix + `release-ci-check` + `changelog-noise`,
unreleased).
**Branch**: `deny-schedule`.
**Status**: draft.
**Input**: the user's go-ahead on 2026-11-02 for item 2.3 (NOTES.md
entry 169), with the design in entry 175.

## 1. Summary

`cargo deny` runs only in `prep_release.sh`, so an advisory published
between releases waits for the next release. After this changeset a
GitHub workflow runs the advisories check every week. A finding turns
the run red, and GitHub emails the repo owner.

## 1.1 Non-goals

- No `licenses`, `bans` or `sources` check in the job.
- No issue, comment or other write on a finding.
- No change to `ci.yml`, `deny.toml` or the release scripts.

## 1.2 Known issues

None.

## 1.3 Maintenance notes

- GitHub disables scheduled workflows on a repo with no activity for 60
  days. A push or a manual enable turns the job back on.
- `unmaintained = "all"` in `deny.toml` makes an unmaintained advisory
  turn the run red, the same as a vulnerability.
- The job first runs once the workflow reaches GitHub's default branch.

## 2. Prerequisites

None on the release machine.

## 3. Workflow

A new `.github/workflows/deny.yml`:

- `name: Advisories`
- triggers: `schedule` with cron `0 6 * * 1` (Monday 06:00 UTC), and
  `workflow_dispatch`
- top-level `permissions: contents: read`
- one job on `ubuntu-latest`:
  - `actions/checkout@v6`, as in `ci.yml`
  - `EmbarkStudios/cargo-deny-action`, pinned to its latest release tag
    with an exact version (looked up at implementation time, recorded
    in the report), `command: check advisories`, reading the repo's
    `deny.toml`

## 4. Testing

GitHub Actions cannot run locally here. The implementer:

- checks `deny.yml` is valid YAML and matches §3
- reads the pinned action release's `action.yml` (or its README) to
  confirm the input names used
- runs `cargo deny --locked check advisories` locally and records it
  passes, so the first scheduled run is expected green

Real behavior is first seen on the first scheduled or manual run after
the workflow reaches GitHub.

## 5. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `.github/workflows/deny.yml` | §3 |

## User-visible surface

None. Phase 14 is skipped.
