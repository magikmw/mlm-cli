# deny-check spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Spec: `docs/dev/specs/2026-10-31-deny-check.md` (working copy, with the uncommitted §5 edit).
Baseline checked: `main` = `33537d1`, matching spec line 3.

## Method and limits

- Read the spec, `scripts/prep_release.sh`, `scripts/release.sh`, all four workflows, `Cargo.toml`, `Cargo.lock`, `dist-workspace.toml`, NOTES.md current state, phase-0 table and entry 165.
- The Bash hook refused `cargo deny` and `bash` (scratch box included), so nothing was run through cargo-deny.
- License check, done by hand instead: read the `license` field of every `Cargo.lock` package from the local registry (`~/.cargo/registry/src/index.crates.io-*/<name>-<ver>/Cargo.toml`). All 168 third-party packages were present, and every one has a `license` field (none uses `license-file`).
- Advisory check, done by hand instead: compared each locked version against the local advisory-db snapshot (`~/.cargo/advisory-dbs/advisory-db-3157b0e258782691`, last commit 2026-10-30).
- Yanked status needs the crates.io index online, so it was not verified.
- The CodeQL alert location (`ci.yml:14`) needs the GitHub API, so it was not verified.

## Verified (no finding)

- Allowlist is complete. Every expression in the lock is satisfied by the §3 list (spec line 47-49). Tally:
  - 105 `MIT OR Apache-2.0`, 31 `MIT`, 9 `MIT/Apache-2.0`, 7 `Apache-2.0 OR MIT`, 1 `Apache-2.0/MIT`
  - 5 `Unlicense OR MIT`, 3 `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`, 1 `Apache-2.0 OR BSL-1.0`
  - 2 `Apache-2.0` (`approx`, `rusqlite_migration`), 1 `MPL-2.0` (`option-ext`), 1 `Zlib` (`foldhash`)
  - 1 `(MIT OR Apache-2.0) AND Unicode-3.0` (`unicode-ident`), 1 `MIT OR Apache-2.0 OR LGPL-2.1-or-later` (`r-efi`)
  - mlm itself is `EUPL-1.2` (`Cargo.toml:8`). cargo-deny checks workspace crates unless `[licenses.private] ignore` applies, and that only covers unpublished crates. mlm is published, so `EUPL-1.2` must be on the list, and it is.
- Advisories as of the 2026-10-30 snapshot: 21 locked crates have advisory files, and every locked version is patched or unaffected. Recent ones checked: `anyhow` 1.0.104 vs RUSTSEC-2026-0190 (patched >=1.0.103), `lru` 0.18.4 vs RUSTSEC-2026-0253 (>=0.18.2), `time` 0.3.55 vs RUSTSEC-2026-0009 (>=0.3.47). The one `unmaintained` hit (`directories`, RUSTSEC-2020-0054) is withdrawn.
- §5: `ci.yml` has no `permissions` key (`.github/workflows/ci.yml:1-14`). Its steps are `actions/checkout@v6` and `run:` steps (`ci.yml:31-281`), and none writes to the repo. `contents: read` is enough. `release.yml:17` and `sign-artifacts.yml:20` already declare permissions. `publish-crates-io.yml` is `workflow_call` only (`:10-11`), so it inherits the caller's permissions.
- Step placement: putting the check after `prep_release.sh:52-57` and before `:64` leaves the tree untouched on stop, because nothing before line 64 writes. Finding 16 covers the one exception.
- `cargo update --workspace` (`prep_release.sh:84`) only touches workspace members, so the lock that was checked is the lock that ships.

## Findings

### 1. Medium — factual error: "no terminal" is not "end of input", and the prompt text is not shown off a terminal
- Claim: spec line 68, "With no terminal on stdin the prompt reads end of input and stops." Rows at lines 63-64 list the prompt text as output.
- Evidence: a non-terminal stdin is often a pipe with data in it. `yes | ./scripts/prep_release.sh 0.5.0` or `echo y | ...` reads `y` and continues. Only `</dev/null` gives end of input. Also, bash `read -p` prints its prompt only when input comes from a terminal (bash manual, `read` builtin), so a piped run prints no prompt at all.
- Fix: either stop outright when `[ -t 0 ]` is false, or reword line 68 to "end of input stops; piped input is read as the answer". Print the prompt with an explicit `printf ... >&2` rather than `read -p`. Add a piped-`y` case to §6.

### 2. Medium — error-handling gap: `read` fails at end of input under `set -e`
- Claim: spec line 66 says end of input prints `error: release stopped at dependency check` and exits 1. Line 69 only guards the check's own exit code.
- Evidence: `prep_release.sh:12` sets `set -euo pipefail`. `read` returns nonzero at EOF, so an unguarded `read -r answer` kills the script with exit 1 and no message. A final line without a newline (`printf y |`) also makes `read` return nonzero, even though `answer=y`.
- Fix: the spec should say the read is guarded (`read -r answer || answer=""`), and that a partial last line counts as end of input and stops. §6's `</dev/null` case should check that the message is printed, not just the exit code.

### 3. Medium — error-handling gap: a cargo-deny error looks the same as a finding
- Claim: spec line 63, "check fails → `dependency check found problems`".
- Evidence: §4 sends every nonzero exit to the same row. cargo-deny also exits nonzero when:
  - the advisory-db git fetch fails, for example offline
  - the yanked lookup cannot reach the crates.io index
  - `cargo metadata` fails
  - `deny.toml` has a key the installed version does not know (§2 asks only for "0.19", spec line 36)
- In each of those cases the maintainer is told "found problems" when none were found.
- Fix: word the prompt as "dependency check failed or found problems", or tell exit code 1 (findings) apart from other codes. Otherwise record it in §1.2.

### 4. Medium — missing edge case: unsound advisories are not addressed
- Claim: spec line 44 lists exactly three failing kinds (vulnerability, unmaintained, yanked).
- Evidence: cargo-deny 0.19 has a separate `unsound` setting next to `unmaintained`. To my knowledge both default to `all`, which fails the check. This category is live in this lockfile. Ten locked crates have historical `informational = "unsound"` advisories (`anstream`, `anyhow`, `bumpalo`, `iana-time-zone`, `lock_api`, `lru` x2, `mio`, `smallvec`, and others), all patched today. `lru` alone got two new ones in 2026.
- Fix: state in §3 whether unsound fails, and name the setting.

### 5. Low — testing gap: §3 names no config keys, and §6 never runs the advisories failure path
- Claim: spec line 44, "Fails on ... a yanked version".
- Evidence: in cargo-deny 0.19, `[advisories] yanked` defaults to `warn`, so a default config does not fail on yanked crates. The spec gives no keys, so the implementer must know to set `yanked = "deny"` and `unmaintained = "all"`. §6 (lines 85-90) breaks only the license check (removing `MIT`). Nothing exercises the advisories settings.
- Fix: put the intended keys in §3 (`[advisories] unmaintained`, `unsound`, `yanked`; `[licenses] allow`, `unused-allowed-license`). In §6, add one advisories failure using `db-path` and a crafted local advisory, or at least have the report quote the `deny.toml` keys and cargo-deny's pass output.

### 6. Medium — scope gap: `deny.toml` ships in the crates.io package
- Claim: spec line 24 says "No change to ... the shipped binary". Spec line 101 says Task 1 owns `deny.toml`, `prep_release.sh` and `ci.yml`.
- Evidence: `Cargo.toml:15-25` excludes the other repo-tooling files (`clippy.toml`, `cliff.toml`, `dist-workspace.toml`, `scripts/`, `.github/`, `coverage-baseline.json`). There is no `include`, so a new root `deny.toml` goes into the published `.crate`.
- Fix: add `"deny.toml"` to `exclude`, and add `Cargo.toml` to Task 1's Owns column. Or record the shipped file as a known issue.

### 7. Low — internal inconsistency: unused allowlist entries may print warnings
- Claim: spec line 50, "An allowlisted license that no crate uses is not a finding." Spec line 47 says the list is "the set the current `Cargo.lock` uses".
- Evidence: from the tally above, the smallest set that satisfies every crate is `Apache-2.0`, `MIT`, `MPL-2.0`, `Zlib`, `Unicode-3.0`, `EUPL-1.2`. `BSL-1.0`, `Unlicense` and `Apache-2.0 WITH LLVM-exception` appear only as `OR` alternatives that `MIT`/`Apache-2.0` already satisfy. cargo-deny's `unused-allowed-license` defaults to `warn` (`license-not-encountered`). Whether a license that is only an `OR` alternative counts as encountered was not confirmed, because cargo-deny could not be run. If it does not count, every run prints three warnings.
- Fix: require `unused-allowed-license = "allow"` in §3, which makes line 50 true by config.

### 8. Low — undeclared user-visible gap: the pass-path output is unspecified
- Claim: spec line 62 gives the pass output as `dependency check: ok`.
- Evidence: cargo-deny writes its own summary and any warn-level diagnostics to stderr. The spec does not say whether that output is shown or suppressed on pass.
- Consequence:
  - If it is shown, the maintainer sees more than line 62 says.
  - If it is suppressed, warn-level items stay hidden. That covers finding 7's warnings, plus any category later set to `warn`.

### 9. Medium — undeclared user-visible gap: every release prep needs the network
- Evidence: `cargo deny check advisories` fetches the advisory db into `~/.cargo/advisory-dbs`, and the yanked check queries the crates.io index.
- Consequence:
  - Release prep gets slower by one git fetch.
  - Offline prep always hits the "found problems" prompt (finding 3).
- §1.2 (lines 31-32) records neither.

### 10. Low — undeclared user-visible gap: any finding needs a full branch cycle to fix
- Evidence: the script requires `main` and a clean tree (`prep_release.sh:35-45`).
- Consequence: to fix a finding, the maintainer must answer `n` and then do the fix on a branch. That means bumping a dependency for an advisory or yanked crate, or adding a license to `deny.toml`. Then merge to `main` and rerun. The fix cannot happen in place.
- The allowlist is frozen at today's lock. A new dependency under a common license not on the list (`BSD-3-Clause`, `BSD-2-Clause`, `ISC`, `0BSD`, `CC0-1.0`) pauses the next release.
- §1.2 records neither.

### 11. Low — undeclared user-visible gap: a `y` answer leaves no record
- Evidence: §4 line 63 continues on `y`. Nothing is written to `CHANGELOG.md`, the release commit (`release.sh:63`) or the tag message (`release.sh:133`).
- Consequence: a release can ship with a known advisory or license problem, and nothing in the repo shows that it was accepted.

### 12. Low — undeclared user-visible gap: `release.sh` bypasses the check
- Evidence: `release.sh:60-63` commits whatever is staged and does not require that `prep_release.sh` ran.
- Consequence:
  - Running `release.sh` alone skips the check entirely.
  - Lock edits made between prep and release are not checked.
- Spec line 24 keeps `release.sh` unchanged but does not record either consequence.

### 13. Low — undeclared user-visible gap: the CodeQL alert stays open until the release
- Claim: spec line 78, "The alert closes on the next CodeQL run after the fix reaches `main`."
- Evidence: the NOTES.md phase-0 Branches row says changesets merge to `main` at release, and NOTES.md line 16 lists six unreleased changesets ahead of this one.
- Consequence: the alert stays open for the whole changeset and its release wait. The spec should say so.

### 14. Low — undeclared user-visible gap: no live doc names the cargo-deny prerequisite after archive
- Claim: spec line 36 (prerequisite) and line 103 ("NOTES.md's phase-0 release slot stays with the coordinator").
- Evidence:
  - The `prep_release.sh:4-5` header still describes the script as "bump the version, regenerate the changelog".
  - The NOTES.md phase-0 Release row still says "reversible bump + changelog".
  - AGENTS.md "Documentation" requires settled content to fold into SPEC.md/NOTES.md before the working spec is archived.
- Consequence: once the spec is archived, nothing live says the release machine needs cargo-deny 0.19, or that prep can now pause for input.
- Fix: put the script header comment in Task 1's scope, and name the NOTES fold explicitly.

### 15. Low — internal inconsistency: the decision log misses the §5 scope addition
- Claim: spec line 8 says "the open CodeQL alert the user added to scope", and lines 24-25 widen the workflow non-goal.
- Evidence: NOTES.md entry 165 (`docs/dev/NOTES.md:2241-2250`) records only advisories/licenses and "Out: ... a scheduled CI job". It has no CI-permissions decision, so the spec cites a scope the log does not hold.

### 16. Low — missing edge case: cargo-deny can rewrite `Cargo.lock`
- Claim: spec line 57, "a stop leaves the tree untouched".
- Evidence: cargo-deny runs `cargo metadata`, which rewrites `Cargo.lock` when it is out of date with `Cargo.toml`. The clean-tree check (`prep_release.sh:41`) does not prove the lock is in sync.
- Fix: run `cargo deny --locked check ...`, which turns the stale-lock case into an error instead of a silent edit.

## Verdict

ship-with-followups

Fold findings 1, 2, 4, 5 and 6 into the spec before dispatching Task 1. They change what the implementer writes. Findings 8-14 are candidates for §1.2 known issues. 3, 7, 15 and 16 are small wording or flag fixes.
