# changelog-noise spec review

Reviewed: docs/dev/specs/2026-11-02-changelog-noise.md against cliff.toml, AGENTS.md, CHANGELOG.md, scripts/prep_release.sh, scripts/release.sh, NOTES.md entry 172, and `git log`.

Limit: the review sandbox refused `git-cliff` (not on the read-only list) and refused `git clone`/`cp` for a scratch repo. No git-cliff output was observed. Semantics claims below come from reading cliff.toml and from knowledge of git-cliff 2.x, and are marked "unverified" where they matter. The implementer should confirm each by running it.

## Findings

### F1 (medium) Postprocessor rule covers only `mlm`, but `upper_first` mangles any lowercase-leading entry
- Claim: spec.md:20, :51-53 fix `Mlm` only.
- Evidence: cliff.toml:37 applies `upper_first` to every message. The unreleased commit `fix: prep_release.sh suggests the version release.sh accepts` (git log, `fix: prep_release.sh ...`) will render as `Prep_release.sh suggests ...`. The spec's own test (spec.md:76-77) runs `git-cliff --unreleased` and expects that `fix:` entry to be right, but only checks it is present.
- Suggestion: drop `upper_first` on the message, or add other known lowercase identifiers (`prep_release.sh`, `release.sh`, `cargo`, `git-cliff`) to the postprocessor, or record it as a known issue in 1.2.

### F2 (medium) Postprocessor anchor ignores the `[**breaking**]` marker
- Claim: spec.md:51-53 matches `Mlm` "at the start of an entry, after an optional `*(scope)* ` marker".
- Evidence: cliff.toml:35-37 emits `*(scope)* ` then `[**breaking**] ` then the message. A breaking `feat!: mlm ...` renders `[**breaking**] Mlm ...` and the rule misses it. No breaking commit exists in history (`git log` grep found none), so the risk is future only.
- Suggestion: allow an optional `[**breaking**] ` in the pattern, or state the gap.

### F3 (medium) Postprocessor regex details are unspecified and easy to get wrong
- Claim: spec.md:51-53.
- Evidence: postprocessors run on the rendered text (cliff.toml:46-47, unverified for `--prepend`), not per commit. A start-of-entry anchor needs a multiline flag and the leading `- ` (cliff.toml:35). It also needs a word boundary so `Mlmfoo` is untouched. The spec gives none of this.
- Suggestion: give the intended pattern and add a test with `Mlm` mid-line and one with a word that merely begins with `Mlm`.

### F4 (medium) Testing never exercises the real invocation
- Claim: spec.md:71-85.
- Evidence: prep_release.sh:248 runs `git-cliff --config cliff.toml --unreleased --tag "$tag" --prepend CHANGELOG.md`. Every spec test uses stdout. Whether postprocessors run on the new section only, or on the whole prepended file (which would touch the hand-written 0.5.0 text), is untested. "CHANGELOG.md is unchanged" (spec.md:85) is only meaningful if a `--prepend` run on a scratch copy is checked.
- Suggestion: add a scratch-copy `--prepend --tag v0.5.1` run and diff only the new section. Also add `feat!: mlm x` (F2), `docs!: x`, and `doc(scope): x` cases.

### F5 (low) `docs!:` and `docs :` fall through to "Other Changes"
- Claim: spec.md:45-48 skip rule covers `docs:` and `doc:`.
- Evidence: the kept parser `^docs?(\(|:)` (cliff.toml:90) does not match `docs!:`. A new skip rule of `^docs?:` would not match it either, so it lands under the catch-all (cliff.toml:97). Same for `docs(scope)!:` which matches the scoped rule but is marked breaking.
- Suggestion: have the skip rule match `^docs?!?:`.

### F6 (low) AGENTS.md prefix list omits prefixes the repo and cliff.toml use
- Claim: spec.md:61-62 lists `feat fix refactor perf test chore ci docs`.
- Evidence: cliff.toml:93,96 group `style` and `revert`. History uses `build:` (e.g. `build: run cargo deny in prep_release.sh ...`), which the spec itself says has no group (spec.md:37-38). An agent following the list is told nothing about `build:`/`style:`/`revert:`.
- Suggestion: either tell agents to use `chore:`/`ci:` for build work and add `style`/`revert`, or give `build:` a group (which conflicts with non-goal spec.md:29).

### F7 (low) Scoped `docs(...)` can leak process commits
- Claim: spec.md:49-50 relies on `adversarial review` text to skip `docs(review):`.
- Evidence: only commits with that phrase are skipped (cliff.toml:84). Nothing stops an agent from writing `docs(spec): ...` or `docs(plan): ...` and showing it in Documentation. AGENTS.md text at spec.md:64-67 says a scope means "users should see it" but names no list of allowed scopes. History already has `docs(scripts): fix release.sh header comment ...`, an internal change that would show.
- Suggestion: name the user-facing scopes in AGENTS.md (`readme`, `help`) or skip scopes like `review|spec|plan|notes`.

### F8 (low) "Phase 14 skipped / user-visible surface: none" is wrong for the changelog
- Claim: spec.md:95-97.
- Evidence: CHANGELOG.md ships to users in the GitHub Release (release.sh:146) and the crates.io package. The spec changes what that file will contain. The user-visible surface is the changelog output.
- Suggestion: state the changelog as the surface, even if Phase 14 is still skipped.

### F9 (low) Scope creep and mild inconsistency in task commit guidance
- Claim: spec.md:68-69 "Task commits start with a type too ... since a scope shows in the changelog".
- Evidence: the non-conventional task commits in history (`release-ci-check task 1: ...`, `panic-fixes: ...`, `doc-drift fix pass: ...`) are parsed as a type/scope-less prefix and land in Other Changes. The rule is fine, but the spec's goal ("fewer commits land in Other Changes", spec.md:21-22) depends on agents obeying it, and spec.md:1.2 says so. Also an internal task typed `feat:`/`fix:` for non-user-visible work will show; the spec says `feat`/`fix` are user-visible only but gives no type for internal code work beyond `refactor`/`chore`/`test`.
- Suggestion: say which type an internal code change takes.

## Undeclared user-visible gaps (candidates for known issues)

- G1: `Prep_release.sh ...` capitalisation of lowercase identifiers other than `mlm` stays in the changelog (see F1). Not in 1.2.
- G2: Changelog sections after this change contain no Documentation group at all in most releases. Docs work like "README and `--help` match what the commands print" (CHANGELOG.md 0.5.0 Documentation) came from `docs:` commits (e.g. `docs: relative-ids SPEC and README for -N ...`). After this change those commits vanish unless scoped. 1.2 item 1 covers the mechanism but not that past-style README/SPEC updates folded into feature work will no longer be mentioned.
- G3: History rendered by `git-cliff` for old ranges (v0.4.0..v0.5.0, spec.md:78) now differs from the shipped CHANGELOG.md sections; regenerated old sections would lose their Documentation entries. Non-goal 1.1 line 26 mentions not rewriting but not that regeneration diverges.
- G4: Hand curation of release sections (the 0.5.0 section rewrite, the `🔄 Changed` group, CHANGELOG.md:12-18) is not reduced by this change. `Changed` is not a cliff group at all. The spec's summary (spec.md:13-14) implies the hand rewrite goes away; it only shrinks.
- G5: `docs(review): ...` without the words "adversarial review" appears in Documentation (F7).
- G6: Breaking-change entries (`[**breaking**] Mlm`) keep the capital (F2).
- G7: Entries that start with `mlm` now render lowercase at the start of a bullet, unlike every other bullet. This is a deliberate choice and nowhere noted as a style difference.

## Verified claims
- 66 unscoped-`docs:` commits in v0.4.0..v0.5.0 (of 108), matches spec.md:12 (`git log --format=%s v0.4.0..v0.5.0 | grep -cE '^docs?:'` = 66).
- `docs(review): independent adversarial review of Milestone N` exists and is skipped by cliff.toml:84.
- Two `feat: mlm ...` commits exist, so the `Mlm` bug is real (cliff.toml:37).
- AGENTS.md has a `## Conventions` section for the new subsection; prep_release.sh:248 is the only git-cliff call.
- NOTES entry 172 matches the spec summary (NOTES.md:2359-2368).

## Verdict
ship-with-followups
