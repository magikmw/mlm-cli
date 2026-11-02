# Changelog noise — design spec

**Baseline**: written against `main` at `18d4d8e` (`v0.5.0` + the
`prep_release.sh` hint fix + `release-ci-check`, unreleased).
**Branch**: `changelog-noise`.
**Status**: locked 2026-11-02 after one review
(`docs/dev/plans/reports/changelog-noise-spec-review.md`, folded in
NOTES.md entry 173).
**Input**: the user's choice on 2026-11-02 in NOTES.md entry 172.

## 1. Summary

`git-cliff` puts every `docs:` commit under Documentation. Between
`v0.4.0` and `v0.5.0` that was 66 commits, nearly all process commits
(spec drafts, folds, locks, NOTES logs, close-outs). The 0.5.0 section
was rewritten by hand. Separately, `upper_first` capitalizes every
entry, so `mlm` becomes `Mlm` and `prep_release.sh` becomes
`Prep_release.sh`.

After this changeset:

- an unscoped `docs:` commit is left out of the changelog
- a scoped `docs(<scope>):` commit still lands under Documentation
- an entry whose first word is `mlm` or holds a non-letter keeps its
  case
- `build:` commits land in Miscellaneous Tasks
- AGENTS.md names the commit prefixes, so fewer commits land in
  "Other Changes"

## 1.1 Non-goals

- No rewrite of `CHANGELOG.md`'s existing sections. Prep prepends only
  the unreleased section, so they stay as shipped.
- No change to `prep_release.sh` or `release.sh`.
- No commit-message check in the hook or CI.
- No change to how any other prefix is grouped.

## 1.2 Known issues

- A user-facing doc change committed as plain `docs:` drops out
  silently. The author has to remember the scope.
- Commits without a known prefix still land in "Other Changes".
  AGENTS.md asks for prefixes but nothing enforces them.
- A scoped `docs(...)` process commit still shows. Only "adversarial
  review" commits are skipped by content.
- Most releases will have no Documentation group. README and SPEC
  updates folded into a feature's `docs:` commits are no longer listed,
  since the `feat:` entry covers them.
- Running `git-cliff` over an old range gives a different section from
  the shipped one, which keeps its Documentation entries.
- Hand edits at release time get smaller but do not go away. The
  `Changed` group in 0.4.0 and 0.5.0 has no `cliff.toml` parser.
- An entry that starts with `mlm` or an identifier starts lowercase,
  unlike the rest.

## 2. Prerequisites

`git-cliff` 2.14 (installed).

## 3. `cliff.toml`

- A new parser before the Documentation one skips a commit whose
  message starts with `docs:`, `doc:`, `docs!:` or `doc!:`.
- The Documentation parser then matches only scoped `docs(...)` and
  `doc(...)` commits.
- The Miscellaneous Tasks parser also matches `^build`.
- `(?i)adversarial review` still skips first, so
  `docs(review): independent adversarial review ...` stays out.
- The body template applies `upper_first` to an entry only when its
  first word is all ASCII letters and is not `mlm`. Otherwise the
  message prints as written. This is decided in the template, so the
  `*(scope)* ` and `[**breaking**] ` markers do not affect it.
- The comment above `commit_parsers` explains the docs rule in one or
  two lines, and the header comment names the AGENTS.md convention.

## 4. AGENTS.md

A short "Commit messages" section under Conventions:

- Subjects use Conventional Commits prefixes: `feat:`, `fix:`,
  `refactor:`, `perf:`, `style:`, `test:`, `build:`, `ci:`, `chore:`,
  `revert:`, `docs:`.
- `feat:` and `fix:` are for changes a user of `mlm` can see. A code
  change users cannot see takes `refactor:`, `test:`, `build:` or
  `chore:`.
- `docs:` with no scope is internal (specs, plans, reports, NOTES) and
  stays out of the changelog.
- A doc change users should see takes a scope, e.g. `docs(readme): ...`
  or `docs(help): ...`.
- Task commits start with a type too, e.g. `fix: ...`, and name the
  changeset and task in the body, since a scope shows in the changelog.

## 5. Testing

The repo has no tests for `cliff.toml`. The implementer checks by hand
and records each output in the task report:

- `git-cliff --unreleased` on the branch shows no unscoped `docs:`
  entry, and shows the hint fix as `prep_release.sh suggests ...`
- `git-cliff v0.4.0..v0.5.0` shows no unscoped `docs:` entry, keeps the
  `feat:` and `fix:` entries, and shows no `Adversarial review` line
- in a throwaway clone under `/tmp`, commits `docs(readme): x`,
  `doc(help): x2`, `docs: y`, `doc: z`, `docs!: z2`,
  `docs(review): adversarial review`, `feat: mlm does w`,
  `feat(scope)!: mlm v`, `fix: cliff.toml u`, `fix: plain t` and
  `build: s` give:
  - Documentation `*(readme)* X` and `*(help)* X2`
  - Features `mlm does w` and `*(scope)* [**breaking**] mlm v`
  - Bug Fixes `cliff.toml u` and `Plain t`
  - Miscellaneous Tasks `S`
  - nothing for the other four
- in that clone, `git-cliff --unreleased --tag v9.9.9 --prepend
  CHANGELOG.md` (the call `prep_release.sh` makes) adds that section
  above the existing ones and leaves them byte-identical
- `CHANGELOG.md` is unchanged

## 6. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `cliff.toml`, `AGENTS.md` (§4 section only) | §3, §4 |

## User-visible surface

`CHANGELOG.md` content from the next release on, which also feeds the
GitHub Release notes. Phase 14 is skipped, because the coordinator
reads the generated section at prep before it ships.
