# deny-schedule Task 1 report

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

**Spec**: `docs/dev/specs/2026-11-02-deny-schedule.md` §3, §4, §5.
**Branch**: `worktree-agent-af2221c48102d9db6`, rebased onto
`deny-schedule` (the worktree started on `main` at `88d9775`, which
lacked the spec; `git rebase deny-schedule` succeeded).

## Files

- `.github/workflows/deny.yml` (new)
- `deny.toml` (header comment only; now names `prep_release.sh` and
  `deny.yml`)

## Action facts (`EmbarkStudios/cargo-deny-action`, tag `v2`)

The `v2` tag exists. Read from `action.yml`, `entrypoint.sh` and
`Dockerfile` at `v2` on raw.githubusercontent.com.

| Input | Default |
|---|---|
| `command` | `check` |
| `arguments` | `--all-features` |
| `command-arguments` | `""` |
| `manifest-path` | `./Cargo.toml` |
| `log-level` | `warn` |
| `rust-version` | `""` |
| `credentials`, `ssh-key`, `ssh-known-hosts` | `""` |
| `use-git-cli` | `"false"` (deprecated, no effect) |

- The action is a Docker action. It runs
  `cargo-deny $*` with the args `--log-level <level> --manifest-path
  <path> <arguments> <command> <command-arguments>`.
- With `arguments: --locked` and `command: check advisories`, the
  resulting call is
  `cargo-deny --log-level warn --manifest-path ./Cargo.toml --locked check advisories`.
  `--locked` sits before the subcommand, as in `prep_release.sh`, so
  §3's assumption holds. Setting `arguments` replaces the
  `--all-features` default, which is intended.
- Advisory database: nothing passes `--disable-fetch`, so
  `cargo-deny check` fetches the database itself.
- The image bundles `cargo-deny` 0.20.2 (`deny_version` in the
  Dockerfile at `v2`) on `rust:1.85.0-alpine`. `deny.toml` is read from
  the repo root, since the manifest path is `./Cargo.toml`.

## Checks

| Check | Result |
|---|---|
| `python3 yaml.safe_load` on `deny.yml` | parses; triggers `schedule` (cron `0 6 * * 1`) and `workflow_dispatch`; `permissions: contents: read`; one job on `ubuntu-latest` with `actions/checkout@v6` and `cargo-deny-action@v2` (`command: check advisories`, `arguments: --locked`). PyYAML shows the `on` key as `True`, a YAML 1.1 quirk that GitHub does not share. |
| `cargo deny --locked check advisories` | `advisories ok` |
| `cargo fmt --check` | exit 0 |
| `cargo build` | finished |
| `cargo test` | all suites ok, 0 failed (main suite 856 passed) |
| `cargo clippy --all-targets -- -D warnings` | finished, no warnings |

## Deviations

None. Workflow test-first was the `action.yml` read, done before writing
`deny.yml`. Real behavior waits for a manual run once the workflow
reaches the default branch (spec §4).
