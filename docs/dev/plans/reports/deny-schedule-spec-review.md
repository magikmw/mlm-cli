# deny-schedule spec review

Reviewed: docs/dev/specs/2026-11-02-deny-schedule.md against ci.yml, deny.toml, scripts/prep_release.sh:88-96, Cargo.toml, NOTES.md 165-167, 175.

Verification limits: no web tool was available. Everything about GitHub Actions and cargo-deny-action below comes from prior knowledge and is marked UNVERIFIED. The implementer's step to read the pinned release's `action.yml` is the only real check, and the spec should say what it must confirm (F3, F4).

## Findings

### F1 (medium, factual) Failure email goes to the cron editor, not "the repo owner"
- Claim: spec.md:15-16, NOTES.md:2412 "GitHub emails the repo owner".
- Evidence (UNVERIFIED, GitHub docs): notifications for a scheduled run go to the user who last committed a change to the `cron` line of the workflow, and only if that user has Actions notifications enabled for failed runs. Here the committer and the owner are likely the same person, but the spec states the mechanism wrongly and does not list the notification setting as a precondition. Manual `workflow_dispatch` runs notify the dispatching actor.
- Fix: say "the user who last edited the cron line" and add the Actions-notification setting to §1.3 or §2.

### F2 (medium, scope/behavior) The schedule does not start until a release, so it only guards the gap after the first release
- Claim: spec.md:34 "first runs once the workflow reaches GitHub's default branch"; spec.md:65-66.
- Evidence: NOTES.md:2288-2290 (entry 168) says changesets reach `main`'s remote only at release. `deny-schedule` merged to local `main` therefore starts nothing. The first scheduled run follows the next release push, which already ran `cargo deny` in `prep_release.sh:89` minutes earlier. The maintenance note states the condition but not that, in this repo's flow, it means "after the next release". The user also cannot trigger `workflow_dispatch` before then (the manual trigger only appears for a workflow on the default branch).
- Fix: state in §1.3 that the job is dormant until the next release push, and that nothing validates `deny.yml` until then.

### F3 (medium, error-handling) Pinned action ages; an old cargo-deny breaks on new advisory-DB content and turns the run red with no finding
- Claim: spec.md:50-52 pins an exact version, no update path; no dependabot (`.github/` holds only `workflows/`).
- Evidence (UNVERIFIED): the action ships a fixed cargo-deny. Old cargo-deny releases have failed on advisory-DB schema additions (e.g. CVSS 4.0 in 2024), so every run goes red as a tool error. The email then reads like a vulnerability. The pin also never updates, and the spec names no owner or trigger for bumping it. prep_release.sh requires cargo-deny 0.19 (prep_release.sh:6), so the two checks can diverge in tool version.
- Fix: add a §1.3 note (or known issue) that a red run may be a tool error, and that the pin needs a manual bump. Say which cargo-deny version the pinned action bundles and record it next to the local 0.19.

### F4 (medium, testing gap) The local check does not prove the action's first run is green
- Claim: spec.md:62-63 "runs `cargo deny --locked check advisories` locally ... so the first scheduled run is expected green".
- Evidence: the action runs its own bundled cargo-deny and (UNVERIFIED) its docker image's Rust toolchain, with default `arguments: --all-features` and no `--locked`. `Cargo.toml:4-5` sets `edition = "2024"` and `rust-version = "1.95"`. If the image's cargo is older than 1.95, `cargo metadata` fails and the run is red from day one. The spec's toolchain question is not addressed anywhere, and the local run uses a different cargo-deny and a different cargo. The advisory DB also moves between the local run and the first scheduled run (up to a week or more), so "expected green" is a guess either way.
- Fix: the implementer must confirm the pinned image's Rust version is at least 1.95 and the bundled cargo-deny can read the current DB. If the action cannot meet this, §3 needs an alternative (a `cargo install --locked cargo-deny` step, or a pinned version via `rust-version`/toolchain input).

### F5 (low, spec gap) The workflow's lockfile behavior is unspecified
- Evidence: §4 uses `--locked` locally; §3 gives no `arguments`. The action (UNVERIFIED) does not pass `--locked`. `Cargo.lock` is committed (AGENTS.md "Conventions"), so a non-locked run on the checked-out lock should not change it, but the spec should say which behavior it intends. If it intends `--locked`, set `arguments: --locked`; note that setting `arguments` replaces the default `--all-features`.

### F6 (low, inconsistency) deny.toml header becomes stale while §1.1 forbids touching it
- Claim: spec.md:22 "No change to ... `deny.toml`".
- Evidence: deny.toml:1-2 says "run by scripts/prep_release.sh as `cargo deny --locked check advisories licenses`". After this change the file is also read by the scheduled job. The comment becomes inaccurate. Same for the prep_release.sh:6 header only in spirit (still true).
- Fix: allow a one-line comment edit, or record that the comment is left stale.

### F7 (low, factual precision) 60-day disable applies to public repos
- Claim: spec.md:30-31.
- Evidence (UNVERIFIED): GitHub disables scheduled workflows after 60 days of no repository activity in public repos only. The repo URL is github.com/magikmw/mlm-cli (Cargo.toml:10); its visibility is not stated in the spec. The note should say which case applies. A release push is also the only activity this repo has between releases (NOTES.md:2288), so gaps over 60 days are plausible and the job silently stops (no email). That is the exact "published advisory waits" gap the spec set out to close.

### F8 (low, consistency) Pinning style differs from `ci.yml`
- Claim: spec.md:49-52 pins one action to an exact version while `actions/checkout@v6` stays floating "as in ci.yml".
- Evidence: ci.yml uses `actions/checkout@v6`. The spec gives no reason for pinning cargo-deny-action precisely but not checkout, and a tag is not an immutable pin (a commit SHA is). Minor; if the aim is supply-chain safety, say so.

### F9 (low, scope) "Latest release tag, looked up at implementation time" leaves the key design value to the implementer
- Evidence: spec.md:50-52. The version is a decision input to F3 and F4, yet it is recorded only in a report. §3 should list the inputs to use (`command: check advisories`; `manifest-path` default is `./Cargo.toml`, `arguments` default noted) rather than "reading the repo's `deny.toml`", which is implicit behavior (the action runs in the workspace and finds `deny.toml` by default).

## Undeclared user-visible gaps (candidates for §1.2 / §1.3)

The work adds a CI workflow, so the "no user-visible consequence" shortcut does not apply to the owner. For end users of `mlm` it is empty. The items below are what the maintainer will see.

- U1: Dormant until the next release push (F2). The maintainer will see no Monday runs in the Actions tab for the whole period before then, and no manual trigger.
- U2: Licenses, bans and sources are not scheduled (spec.md:20 states it, NOTES.md:2413 gives the reason). The consequence is that a yanked or relicensed crate is only caught by the prep run, and a `licenses` problem on a new advisory-free dependency version appears first at release time. Declared as a non-goal, but the consequence is not.
- U3: A scheduled failure produces only a red run and an email (spec.md:21). Nothing links the finding to a fix, and a prolonged unfixed finding re-emails every Monday. Not written down.
- U4: `unmaintained = "all"` and `unsound = "all"` (deny.toml:5-6) plus `yanked = "deny"` make three classes of finding red. The note at spec.md:32-33 names only unmaintained.
- U5: The run is red for the whole repo state until a fix lands on `main`, and per NOTES.md:2288 `main` only reaches the remote at release, so a fix commit needs a push of its own. Related to the "a fix needs a branch" known issue of entry 166.
- U6: Scheduled workflows can be delayed or dropped at the top of the hour under load (UNVERIFIED). 06:00 UTC is such a time. The advisory latency is therefore "about a week, sometimes more".
- U7: Silent stop on the 60-day pause (F7) gives no email, so absence of mail is not evidence of a clean repo.

## Checked and fine

- `ci.yml` already has `permissions: contents: read` (ci.yml:9-10); spec §3 matches.
- No existing workflow named "Advisories"; no filename clash (`ls .github/workflows`: ci, publish-crates-io, release, sign-artifacts).
- Baseline `ca852e6` is `main` (git rev-parse main). `.github/` and `deny.toml` are in the package `exclude` (Cargo.toml:15-26), so the new workflow does not affect the crate package.
- `cargo deny --locked check advisories` is a valid subset of prep_release.sh:89.
- Cron `0 6 * * 1` is Monday 06:00 UTC; GitHub cron is UTC.
- Schedule runs only on the default branch (spec.md:34 agrees).

## Verdict

ship-with-followups

Fold F2, F3 and F4 into the spec before lock (they change what the implementer must check). F1, F5-F9 are wording fixes. U1-U7 are candidates for §1.3; the user decides which to accept.
