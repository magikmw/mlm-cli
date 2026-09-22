#!/usr/bin/env bash
# Cut a new mlm release.
#
# Run this only after `prep_release.sh` and a manual review of the
# regenerated CHANGELOG.md — this script commits the version bump,
# pushes `main`, waits for CI to pass on that commit, then creates an
# annotated tag and pushes it to `origin`. Pushing the tag is what
# triggers .github/workflows/release.yml, which does the rest (builds,
# GitHub Release, crates.io publish, artifact signing) — so pushing the
# tag IS the irreversible step (crates.io can't be unpublished, the
# GitHub Release goes public), not just a local/reversible commit.
# Does NOT run `cargo publish` itself and does NOT create the GitHub
# Release directly — both happen inside the triggered workflow.
#
# Usage:
#   ./scripts/release.sh [--skip-ci-check] <new-version>
#
# --skip-ci-check skips waiting for CI on the pushed main commit before
# tagging. Use only when you already know CI is green (e.g. it just ran
# for an identical commit) — this bypasses the safeguard that stops a
# broken build from being released.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

usage() {
    echo "Usage: $0 [--skip-ci-check] <new-version>" >&2
    echo "Example: $0 0.2.0" >&2
    echo "         $0 --skip-ci-check 0.2.0" >&2
    exit 1
}

skip_ci_check=0
new_version=""
for arg in "$@"; do
    case "$arg" in
        --skip-ci-check)
            skip_ci_check=1
            ;;
        -*)
            usage
            ;;
        *)
            [ -z "$new_version" ] || usage
            new_version="$arg"
            ;;
    esac
done
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

# --- 5. Push main -----------------------------------------------------

git push origin main

# --- 6. Wait for CI to pass on the pushed commit -----------------------

sha="$(git rev-parse HEAD)"

if [ "$skip_ci_check" -eq 1 ]; then
    echo "::warning: --skip-ci-check set — bypassing the CI gate for ${sha}. This release will NOT be verified against a passing CI run." >&2
else
    repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
    elapsed=0
    interval=15
    timeout=600

    echo "Waiting for CI to pass on ${sha}..."

    while [ "$elapsed" -lt "$timeout" ]; do
        run_json="$(gh api "repos/${repo}/actions/workflows/ci.yml/runs?head_sha=${sha}&per_page=1")"
        status="$(echo "$run_json" | jq -r '.workflow_runs[0].status // empty')"
        conclusion="$(echo "$run_json" | jq -r '.workflow_runs[0].conclusion // empty')"

        if [ "$status" = "completed" ]; then
            if [ "$conclusion" = "success" ]; then
                echo "CI passed for ${sha} (conclusion: ${conclusion})."
                break
            fi
            echo "error: CI for commit ${sha} completed with conclusion '${conclusion}', not 'success'." >&2
            echo "main was already pushed — that's fine, an ordinary push doesn't trigger a release." >&2
            echo "The tag was NOT created or pushed, so no release was triggered." >&2
            echo "Fix CI and re-run this script (it will make a new commit), or use --skip-ci-check to override." >&2
            exit 1
        fi

        echo "Waiting for CI on ${sha} to complete (current status: ${status:-not found}, ${elapsed}s elapsed)..."
        sleep "$interval"
        elapsed=$((elapsed + interval))
    done

    if [ "$elapsed" -ge "$timeout" ]; then
        echo "error: timed out after ${timeout}s waiting for a completed CI run on commit ${sha}." >&2
        echo "main was already pushed — that's fine, an ordinary push doesn't trigger a release." >&2
        echo "The tag was NOT created or pushed, so no release was triggered." >&2
        echo "Re-run the CI check once it's fixed, or use --skip-ci-check to override." >&2
        exit 1
    fi
fi

# --- 7. Tag -------------------------------------------------------------

git tag -a "$tag" -m "mlm ${new_version}"

# --- 8. Push tag ---------------------------------------------------------

git push origin "${tag}"

# --- 9. Let user know what's next -----------------------------------------

cat <<EOF

Pushed to origin, triggered .github/workflows/release.yml.

Pushing the tag triggers .github/workflows/release.yml, which builds all
platform binaries, creates the GitHub Release (using CHANGELOG.md for the
release notes), signs each artifact with minisign, and publishes to
crates.io.

Track progress here: https://github.com/magikmw/mlm-cli/actions
EOF
