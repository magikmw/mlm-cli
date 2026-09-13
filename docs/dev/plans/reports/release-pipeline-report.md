# Release pipeline setup report

Date: 2026-09-13

## Summary

Set up a full tag-triggered release pipeline for `mlm`: commit-based
changelog generation (`git-cliff`), multi-platform builds + GitHub Release
creation (`cargo-dist`), a crates.io publish step, minisign-based artifact
signing for `cargo-binstall`, and a local helper script to cut releases.
Everything is committed on `main`; nothing was pushed, tagged-for-real,
released, or published to crates.io.

## Tool versions used

- `git-cliff` 2.14.1 (installed via `cargo binstall`)
- `cargo-dist` 0.32.0 / binary `dist` (installed via `cargo binstall`)
- `minisign` 0.12-3.fc44 (installed via `dnf`, system package)
- `cargo-binstall` 1.19.1 (already present)

All docs were pulled live from the current upstream repos during this
session (axodotdev/cargo-dist's book source, cargo-bins/cargo-binstall's
`SIGNING.md`) rather than relied on from training-data memory, since both
tools are fast-moving.

## 1. `git-cliff` / `CHANGELOG.md`

- `cliff.toml` at repo root. Checked `git log --oneline` first (53 commits):
  a handful use `type:`/`type(scope):` prefixes (`feat`, `fix`, `docs`,
  `review:`, `storage:`), but most are free-form ("Implement Milestone N:
  ...", "Merge milestone-N: ...", "Add ..."). Configured pragmatically:
  `conventional_commits = true`, `filter_unconventional = false`,
  `require_conventional = false`, with a `commit_parsers` list that groups
  recognized prefixes (feat/fix/docs/refactor/perf/style/test/chore-ci/
  review/revert/Merge) and a catch-all `".*" -> "Other Changes"` bucket at
  the end so nothing gets silently dropped.
- Added two `commit_preprocessors` regexes: one keeps only the first
  paragraph of each commit message (several "Merge milestone-N" commits
  have multi-paragraph bodies that were swamping the changelog), the other
  rejoins a couple of old commits whose *subject line itself* was
  hard-wrapped across two lines with no blank separator.
- Generated `CHANGELOG.md` covering the full history as a single `[0.1.0]`
  release (there are no prior tags). Verified output by hand — it reads
  sensibly, groups are non-trivial, and no content was lost or garbled.
- `cargo dist plan` confirms `dist` auto-includes `CHANGELOG.md` in every
  release archive, and (per `create-release` in cargo-dist's config docs)
  it will generate the GitHub Release title/body from it.

## 2. `cargo-dist`

Ran `dist init -y --hosting github` then hand-edited the result. **Note for
the report**: current cargo-dist (0.32.0) does *not* use
`[workspace.metadata.dist]` / `[package.metadata.dist]` in `Cargo.toml` as
the task brief anticipated (that was the pre-1.0 config location) — it now
uses a separate `dist-workspace.toml` file at the repo root, with only a
small `[profile.dist]` (build profile: `inherits = "release"`, `lto =
"thin"`) added to `Cargo.toml`. This is the real, current generated
convention, confirmed by actually running the tool rather than assumed.

`dist-workspace.toml`:
```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.32.0"
ci = "github"
installers = []
targets = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-apple-darwin", "aarch64-apple-darwin"]
hosting = "github"
post-announce-jobs = ["./sign-artifacts"]
publish-jobs = ["./publish-crates-io"]
```

- Targets are exactly the requested four. `dist`'s default runner mapping
  for these is already native-per-OS (`ubuntu-22.04`, `windows-2022`,
  `macos-15-intel` for x86_64, `macos-14` for aarch64) — no
  `github-custom-runners` override needed, no cross-compilation.
- `dist generate` produced `.github/workflows/release.yml`, triggered on
  tag push matching `**[0-9]+.[0-9]+.[0-9]+*` (covers `v0.2.0` etc.). Jobs:
  `plan` → `build-local-artifacts` (matrix, one per target) →
  `build-global-artifacts` → `host` (creates the GitHub Release, uploads
  archives, title/body generated from `CHANGELOG.md`) →
  `custom-publish-crates-io` → `announce` → `custom-sign-artifacts`.
- **crates.io publish**: current cargo-dist has no built-in crates.io
  publisher (only `homebrew` and `npm` are built-in `publish-jobs`
  keywords). Added it as a `publish-jobs` custom job
  (`.github/workflows/publish-crates-io.yml`, a reusable
  `workflow_call` workflow) rather than hand-editing `release.yml` — this
  is cargo-dist's supported extension point, and `dist generate` wired the
  `needs:`/ordering/`secrets: inherit` automatically. It runs `cargo
  publish --locked` using the `CARGO_REGISTRY_TOKEN` secret, after the
  GitHub Release exists (`needs: host`) and before `announce`.
- `dist` also emits `dist-manifest.json` with every release by default
  (left untouched) — recent `cargo-binstall` can consume this directly for
  better resolution; no action needed beyond leaving the default on.

## 3. `cargo-binstall` signing (minisign)

- Fetched `cargo-bins/cargo-binstall`'s current `SIGNING.md` (re-fetched
  this session, cached at `/tmp/binstall-SIGNING.md`) for the exact
  mechanism.
- Generated a real, unencrypted (`-W`) minisign keypair with the system
  `minisign` binary:
  - Public key (committed, in `Cargo.toml`):
    `RWTNO3Iugq4ICzcJhLwnwtOT1T0evn7RYI0951dfE+IkEyRykSvv0V/n`
  - Private key: saved to **`/tmp/mlm-signing.key`** on this machine only.
    **Not committed anywhere.**
- Added to `Cargo.toml`:
  ```toml
  [package.metadata.binstall.signing]
  algorithm = "minisign"
  pubkey = "RWTNO3Iugq4ICzcJhLwnwtOT1T0evn7RYI0951dfE+IkEyRykSvv0V/n"
  ```
- Signing extension point: cargo-dist's `post-announce-jobs` custom job
  (`.github/workflows/sign-artifacts.yml`), which runs after `announce`,
  i.e. after the GitHub Release and all its archives already exist. It:
  1. downloads the release's assets with `gh release download`,
  2. decodes the `MINISIGN_SECRET_KEY` secret (base64) to a temp file,
  3. runs `minisign -S -W -s <key> -x <file>.sig -m <file>` over every
     archive (skipping `dist-manifest.json`/checksum files),
  4. uploads the resulting `.sig` files back to the same release with
     `gh release upload --clobber`,
  5. deletes the temp key file (`trap ... EXIT`).

  This uses the officially documented custom-job mechanism rather than
  hand-editing dist's generated `release.yml` — `dist generate` inserted
  the `custom-sign-artifacts` job itself (with `secrets: inherit` added
  automatically), so `release.yml` stays fully dist-managed and
  `dist generate --check` reports it up to date.

### What you (the user) must do with the private key

1. Base64-encode it:
   ```console
   base64 -w0 /tmp/mlm-signing.key
   ```
2. Add the result as a GitHub Actions repository secret named exactly
   **`MINISIGN_SECRET_KEY`** (Settings → Secrets and variables → Actions →
   New repository secret, on `magikmw/mlm-cli`).
3. Delete the local copy once the secret is saved:
   ```console
   rm /tmp/mlm-signing.key
   ```

If this key is ever lost or thought compromised: generate a new keypair,
update `pubkey` in `Cargo.toml`, and replace the `MINISIGN_SECRET_KEY`
secret. Old releases stay verifiable against the old public key; you'd want
to keep a record of which key signed which release if you ever rotate.

## 4. `SIGNING.md`

Written at `/home/magikmw/projects/mlm-cli/SIGNING.md`. Covers: what's
signed (release archives), algorithm (minisign), where the public key lives
(`Cargo.toml`'s `[package.metadata.binstall.signing]`), how `cargo
binstall` verifies automatically, how to verify manually with the
`minisign` CLI (exact command included), and how the private key is
handled/rotated.

## 5. `Cargo.toml` housekeeping

Added `repository = "https://github.com/magikmw/mlm-cli"`. Also added
`.github/`, `cliff.toml`, `dist-workspace.toml`, and `scripts/` to the
existing `exclude` list, following the project's existing convention of
excluding internal/dev-only files from the published crate (mirrors why
`.githooks/` and `clippy.toml` were already excluded). `SIGNING.md` was
deliberately **not** excluded — it's meant to be discoverable.

## 6. `scripts/release.sh`

Bash script (`chmod +x`). Flow:
1. Verifies `main` branch + clean working tree, aborts otherwise.
2. Validates the version argument looks like semver, aborts if the tag
   already exists.
3. Bumps `Cargo.toml`'s version (`cargo set-version` if `cargo-edit` is
   installed, else a plain `awk` edit of the first `version = "..."`
   line), then `cargo update --workspace` to keep `Cargo.lock` in sync.
4. Regenerates `CHANGELOG.md` via `git-cliff --tag vX.Y.Z`.
5. Commits the version bump + changelog together.
6. Creates an annotated tag `vX.Y.Z`.
7. Prints the exact `git push origin main` / `git push origin vX.Y.Z`
   commands — **does not run them**, does not touch GitHub, does not run
   `cargo publish`.

Tested end-to-end in a throwaway `rsync` copy of the repo in `/tmp`
(committed there, ran `./scripts/release.sh 0.2.0`, confirmed version bump,
regenerated changelog, commit, and tag all worked, then deleted the copy).
Not run for real in this repo.

## Verification performed

- `cargo build` — clean.
- `cargo test` — all passing (unit + integration).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean.
- `dist plan` — validates config, lists exactly the 4 requested target
  archives plus source tarball/checksums, no build performed.
- `dist generate --check` — reports `.github/workflows/release.yml` is up
  to date with `dist-workspace.toml` (exit 0).
- `git-cliff` run for real (read-only w.r.t. git history; only writes
  `CHANGELOG.md` locally) — output inspected by hand.
- Did **not** run `git tag`, `git push`, `gh release create`, or
  `cargo publish` for real in this repo.

## GitHub repository secrets required before any of this works

Add these under `magikmw/mlm-cli` → Settings → Secrets and variables →
Actions:

| Secret name | What it is | How to get it |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | A crates.io API token with publish rights for `mlm` | crates.io → Account Settings → API Tokens → New Token (scope: `publish-update`/`publish-new` as needed) |
| `MINISIGN_SECRET_KEY` | Base64 of the minisign private key generated this session | `base64 -w0 /tmp/mlm-signing.key`, then delete that file (see above) |

`GITHUB_TOKEN` is provided automatically by GitHub Actions and needs no
setup.

## Cutting the first real release

Once both secrets above are added:

```console
./scripts/release.sh 0.2.0        # or whatever the next version should be
git show HEAD                     # sanity-check the version bump + changelog commit
git show v0.2.0                   # sanity-check the tag

git push origin main
git push origin v0.2.0
```

Pushing the tag triggers `.github/workflows/release.yml`, which builds all
four platform archives, creates the GitHub Release (title/body from
`CHANGELOG.md`), publishes to crates.io, and signs every archive with
minisign, uploading the `.sig` files to the same release.
