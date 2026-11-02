#!/usr/bin/env bash
# Prep a new mlm release.
#
# This script does the first half of a release: check the dependencies,
# check CI on HEAD, bump the version, regenerate the changelog.
# The dependency check needs `cargo-deny` 0.19 and the network.
# The CI check needs `gh` (authenticated), `jq` and push access to
# `origin`. It pushes HEAD to a throwaway `release-check` branch, waits
# for its CI run, then deletes the branch. CI_CHECK_TIMEOUT (seconds,
# default 600) caps the wait.
# A failed or skipped check asks whether to continue.
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

# --- 2. Check dependencies ---------------------------------------------

# Prompt on stderr and continue only on `y`/`Y`. Off a terminal, or at
# end of input, stop at the named step without reading an answer.
confirm_or_stop() {
    local step="$1" message="$2" answer=""
    printf '%s continue anyway? [y/N] ' "$message" >&2
    if ! { [ -t 0 ] && read -r answer; }; then
        answer=""
        echo >&2
    fi
    case "$answer" in
        y | Y) ;;
        *)
            echo "error: release stopped at ${step}" >&2
            exit 1
            ;;
    esac
}

if command -v cargo-deny >/dev/null 2>&1; then
    if cargo deny --locked check advisories licenses; then
        echo "dependency check: ok"
    else
        confirm_or_stop "dependency check" "dependency check found problems."
    fi
else
    confirm_or_stop "dependency check" "dependency check skipped: cargo-deny not found."
fi

# --- 3. Check CI on HEAD -------------------------------------------------

ci_branch="release-check"
ci_interval=15
ci_timeout="${CI_CHECK_TIMEOUT:-600}"
ci_sha="$(git rev-parse HEAD)"
ci_pushed=0
ci_outcome=""
ci_reason=""
ci_status=""
ci_conclusion=""
ci_url=""

if ! [[ "$ci_timeout" =~ ^0*[1-9][0-9]*$ ]]; then
    echo "error: CI_CHECK_TIMEOUT must be a positive integer" >&2
    exit 1
fi
ci_timeout=$((10#$ci_timeout))

# Delete the pushed branch. The traps call this too, so it disarms first.
delete_ci_branch() {
    [ "$ci_pushed" -eq 1 ] || return 0
    ci_pushed=0
    if ! git push --quiet origin --delete "$ci_branch"; then
        echo "warning: could not delete remote branch ${ci_branch}" >&2
    fi
}

trap delete_ci_branch EXIT
trap 'delete_ci_branch; exit 130' INT
trap 'delete_ci_branch; exit 143' TERM

# Poll for the push's run until it completes or the timeout passes. Sets
# ci_status, ci_conclusion and ci_url. Fails with ci_reason set when a
# gh or jq call fails.
wait_for_ci() {
    local repo="$1" since="$2" elapsed=0 step run_json fields parts
    local query="repos/${repo}/actions/workflows/ci.yml/runs?branch=${ci_branch}&event=push&head_sha=${ci_sha}&per_page=20"
    while :; do
        if ! run_json="$(gh api "$query")"; then
            ci_reason="gh api failed"
            return 1
        fi

        # Runs created before the push belong to an earlier prep.
        if ! fields="$(jq -r --arg since "$since" '
                [.workflow_runs[] | select(.created_at >= $since)][0] // {}
                | .status // "", .conclusion // "", .html_url // ""' <<<"$run_json")"; then
            ci_reason="jq could not read the run list"
            return 1
        fi
        mapfile -t parts <<<"$fields"
        ci_status="${parts[0]:-}"
        ci_conclusion="${parts[1]:-}"
        ci_url="${parts[2]:-}"

        if [ "$ci_status" = "completed" ] || [ "$elapsed" -ge "$ci_timeout" ]; then
            return 0
        fi
        echo "Waiting for CI on ${ci_sha} (status: ${ci_status:-not found}, ${elapsed}s elapsed)..."
        step=$((ci_timeout - elapsed < ci_interval ? ci_timeout - elapsed : ci_interval))
        sleep "$step"
        elapsed=$((elapsed + step))
    done
}

# Push HEAD as the throwaway branch and wait for its run. Sets ci_outcome
# to skipped, error or done, with ci_reason for the first two.
run_ci_check() {
    local tool repo since
    for tool in gh jq; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            ci_outcome="skipped"
            ci_reason="${tool} not found"
            return 0
        fi
    done
    if ! repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"; then
        ci_outcome="error"
        ci_reason="gh repo view failed"
        return 0
    fi

    # A leftover branch at HEAD would turn the push into a no-op with no run.
    git push --quiet origin --delete "$ci_branch" >/dev/null 2>&1 || true

    since="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    ci_pushed=1
    if ! git push --quiet origin "HEAD:refs/heads/${ci_branch}"; then
        ci_pushed=0
        ci_outcome="skipped"
        ci_reason="push to origin failed"
        return 0
    fi

    echo "Waiting for CI on ${ci_sha} via branch ${ci_branch}..."
    if wait_for_ci "$repo" "$since"; then
        ci_outcome="done"
    else
        ci_outcome="error"
    fi
    delete_ci_branch
}

run_ci_check
case "$ci_outcome" in
    skipped)
        confirm_or_stop "CI check" "CI check skipped: ${ci_reason}."
        ;;
    error)
        confirm_or_stop "CI check" "CI check could not finish: ${ci_reason}."
        ;;
    *)
        if [ "$ci_status" != "completed" ]; then
            confirm_or_stop "CI check" "CI check timed out after ${ci_timeout}s on ${ci_sha}.${ci_url:+ ${ci_url}}"
        elif [ "$ci_conclusion" = "success" ]; then
            echo "CI check: ok (${ci_sha})"
        else
            confirm_or_stop "CI check" "CI check failed on ${ci_sha} (conclusion: ${ci_conclusion:-none}). ${ci_url}"
        fi
        ;;
esac
trap - EXIT INT TERM

# --- 4. Bump the version in Cargo.toml ---------------------------------

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

# --- 5. Update CHANGELOG.md ----------------------------------------

git-cliff --config cliff.toml --unreleased --tag "$tag" --prepend CHANGELOG.md

# --- 6. Tell the human what to do next -----------------------------------

cat <<EOF

Version bumped to ${new_version}.

CHANGELOG.md updated for review.

When ready, trigger ./scripts/release.sh ${new_version}
EOF
