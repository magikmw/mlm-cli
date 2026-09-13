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

# --- 4. Commit ----------------------------------------------------------

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Release ${tag}"

cat <<EOF

Bump version to ${new_version}.
EOF

# --- 5. Tag ---------------------------------------------------------------

git tag -a "$tag" -m "mlm ${new_version}"

# --- 6. Push commits and tags ---------------------------------------------

git push origin main
git push origin "${tag}"

# --- 7. Let user know what's next -----------------------------------------

cat <<EOF

Pushed to origin, triggered .github/workflows/release.yml.

Pushing the tag triggers .github/workflows/release.yml, which builds all
platform binaries, creates the GitHub Release (using CHANGELOG.md for the
release notes), signs each artifact with minisign, and publishes to
crates.io.

Track progress here: https://github.com/magikmw/mlm-cli/actions
EOF
