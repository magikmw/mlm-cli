# Scheduled advisory check — design spec

**Baseline**: written against `main` at `ca852e6` (`v0.5.0` + the
`prep_release.sh` hint fix + `release-ci-check` + `changelog-noise`,
unreleased).
**Branch**: `deny-schedule`.
**Status**: locked 2026-11-02 after one review
(`docs/dev/plans/reports/deny-schedule-spec-review.md`, folded in
NOTES.md entry 176).
**Input**: the user's go-ahead on 2026-11-02 for item 2.3 (NOTES.md
entry 169), with the design in entry 175.

## 1. Summary

`cargo deny` runs only in `prep_release.sh`, so an advisory published
between releases waits for the next release. After this changeset a
GitHub workflow runs the advisories check every week. A finding turns
the run red, and GitHub emails the account that last changed the
schedule.

## 1.1 Non-goals

- No `licenses`, `bans` or `sources` check in the job.
- No issue, comment or other write on a finding.
- No change to `ci.yml`, the release scripts or `deny.toml`'s checks.
  Only its header comment changes.

## 1.2 Known issues

None.

## 1.3 Maintenance notes

- GitHub disables scheduled workflows on a repo with no activity for 60
  days. A push or a manual enable turns the job back on.
- `deny.toml` makes unmaintained, unsound and yanked advisories turn
  the run red, the same as a vulnerability.
- The failure email goes to the account that last changed the cron
  line, and only when its Actions notifications are on. A red run
  emails again each week until fixed, and the fix needs its own push.
- The job first runs once the workflow reaches GitHub's default branch,
  which happens at the next release unless pushed sooner. Manual runs
  need the same.
- The action's major tag moves with its releases, so its bundled
  `cargo-deny` stays current with the advisory database.

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
  - `EmbarkStudios/cargo-deny-action@v2`, a major tag like
    `actions/checkout@v6`, with `command: check advisories` and
    `arguments: --locked`, as `prep_release.sh` runs it. It reads
    `deny.toml` at the repo root
- `deny.toml`'s header comment names both places the file is used

## 4. Testing

GitHub Actions cannot run locally here. The implementer:

- checks `deny.yml` is valid YAML and matches §3
- reads the action's `action.yml` at `v2` to confirm the input names,
  that `arguments` takes `--locked`, and that the action fetches the
  advisory database itself, and records them in the report
- runs `cargo deny --locked check advisories` locally and records it
  passes

Real behavior is first seen on a manual run after the workflow reaches
GitHub. The coordinator starts one then and checks it goes green.

## 5. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `.github/workflows/deny.yml`, `deny.toml` (header comment only) | §3 |

## User-visible surface

None. Phase 14 is skipped.
