#!/usr/bin/env bash
# Prep a new mlm release.
#
# This script does the *local, reversible* half of a release: bump the
# version, regenerate the changelog.
# Split with `release.sh` to allow for clean changelog curation until I figure
# out how to do it better.
#
# Usage:
#   ./scripts/prep_release.sh 0.2.0

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

# --- 3. Update CHANGELOG.md ----------------------------------------

git-cliff --config cliff.toml --unreleased --tag "$tag" --prepend CHANGELOG.md

# --- 6. Tell the human what to do next -----------------------------------

cat <<EOF

Version bumped to ${new_version}.

CHANGELOG.md updated for review.

When ready, trigger ./scripts/release.sh "$tag"
EOF
