#!/usr/bin/env bash
# Cut a new mlm release.
#
# This script does the *local, reversible* half of a release: bump the
# version, regenerate the changelog, commit, and create an annotated tag.
#
# It deliberately does NOT push anything, does NOT create a GitHub release,
# and does NOT run `cargo publish`. Pushing the tag is what triggers
# .github/workflows/release.yml, which does the rest (builds, GitHub
# Release, crates.io publish, artifact signing) — that's a separate, human
# decision, made by actually running the `git push` commands this script
# prints at the end.
#
# Usage:
#   ./scripts/release.sh 0.2.0

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

usage() {
    echo "Usage: $0 <new-version>" >&2
    echo "Example: $0 0.2.0" >&2
    exit 1
}

new_version="${1:-}"
[ -n "$new_version" ] || usage

if ! [[ "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "error: '$new_version' doesn't look like a semver version (e.g. 0.2.0)" >&2
    exit 1
fi

tag="v${new_version}"

# --- 1. Preconditions -------------------------------------------------

current_branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$current_branch" != "main" ]; then
    echo "error: not on 'main' (currently on '$current_branch')" >&2
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "error: working tree is not clean" >&2
    git status --short >&2
    exit 1
fi

if git rev-parse "$tag" >/dev/null 2>&1; then
    echo "error: tag '$tag' already exists" >&2
    exit 1
fi

for tool in cargo git-cliff; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "error: required tool '$tool' not found on PATH" >&2
        exit 1
    }
done

current_version_line="$(grep -m1 '^version[[:space:]]*=' Cargo.toml)"
echo "Releasing mlm ${new_version} (currently: ${current_version_line})"

# --- 2. Bump the version in Cargo.toml ---------------------------------

if command -v cargo-set-version >/dev/null 2>&1; then
    cargo set-version "$new_version"
else
    # Fall back to a plain sed edit of the [package] version field if
    # cargo-edit isn't installed. Only touches the first `version = "..."`
    # line, which is the package version.
    tmp="$(mktemp)"
    awk -v newver="$new_version" '
        BEGIN { done = 0 }
        !done && /^version[[:space:]]*=/ {
            print "version = \"" newver "\""
            done = 1
            next
        }
        { print }
    ' Cargo.toml > "$tmp"
    mv "$tmp" Cargo.toml
fi

# Keep Cargo.lock in sync without touching dependency versions.
cargo update --workspace --offline 2>/dev/null || cargo update --workspace

# --- 3. Regenerate CHANGELOG.md ----------------------------------------

git-cliff --config cliff.toml --unreleased --tag "$tag" --prepend CHANGELOG.md

# --- 4. Commit ----------------------------------------------------------

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Release ${tag}

Bump version to ${new_version} and regenerate CHANGELOG.md via git-cliff."

# --- 5. Tag ---------------------------------------------------------------

git tag -a "$tag" -m "mlm ${new_version}"

# --- 6. Tell the human what to do next -----------------------------------

cat <<EOF

Done locally. Nothing has been pushed, released, or published.

Review the commit and tag:
  git show HEAD
  git show ${tag}

When you're ready to actually ship this release, run:

  git push origin main
  git push origin ${tag}

Pushing the tag triggers .github/workflows/release.yml, which builds all
platform binaries, creates the GitHub Release (using CHANGELOG.md for the
release notes), signs each artifact with minisign, and publishes to
crates.io. Nothing in this script or in Claude has run any of that for you.
EOF
