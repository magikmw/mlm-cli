# Changelog noise — design spec

**Baseline**: written against `main` at `18d4d8e` (`v0.5.0` + the
`prep_release.sh` hint fix + `release-ci-check`, unreleased).
**Branch**: `changelog-noise`.
**Status**: draft.
**Input**: the user's choice on 2026-11-02 in NOTES.md entry 172.

## 1. Summary

`git-cliff` puts every `docs:` commit under Documentation. Between
`v0.4.0` and `v0.5.0` that was 66 commits, nearly all process commits
(spec drafts, folds, locks, NOTES logs, close-outs). The 0.5.0 section
was rewritten by hand.

After this changeset:

- an unscoped `docs:` commit is left out of the changelog
- a scoped `docs(<scope>):` commit still lands under Documentation
- an entry that starts with `mlm` keeps it lowercase
- AGENTS.md names the commit prefixes, so fewer commits land in
  "Other Changes"

## 1.1 Non-goals

- No rewrite of `CHANGELOG.md`'s existing sections.
- No change to `prep_release.sh` or `release.sh`.
- No commit-message check in the hook or CI.
- No change to how any other prefix is grouped.

## 1.2 Known issues

- A user-facing doc change committed as plain `docs:` drops out
  silently. The author has to remember the scope.
- Commits without a known prefix still land in "Other Changes".
  AGENTS.md asks for prefixes but nothing enforces them.
- `build:` has no group and lands in "Other Changes".

## 2. Prerequisites

`git-cliff` 2.14 (installed).

## 3. `cliff.toml`

- A new parser before the Documentation one skips a commit whose
  message starts with `docs:` or `doc:`.
- The Documentation parser then matches only scoped `docs(...)` and
  `doc(...)` commits.
- `(?i)adversarial review` still skips first, so
  `docs(review): independent adversarial review ...` stays out.
- A postprocessor turns `Mlm` back into `mlm` where `upper_first` made
  it, at the start of an entry, after an optional `*(scope)* ` marker.
  `Mlm` anywhere else is left alone.
- The comment above `commit_parsers` explains the docs rule in one or
  two lines, and the header comment names the AGENTS.md convention.

## 4. AGENTS.md

A short "Commit messages" section under Conventions:

- Subjects use Conventional Commits prefixes: `feat:`, `fix:`,
  `refactor:`, `perf:`, `test:`, `chore:`, `ci:`, `docs:`.
- `feat:` and `fix:` are for changes a user of `mlm` can see.
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
  entry, and still shows the `fix:` entry for the hint fix
- `git-cliff v0.4.0..v0.5.0` shows no unscoped `docs:` entry, keeps the
  `feat:` and `fix:` entries, and shows no `Adversarial review` line
- in a throwaway clone under `/tmp`, commits `docs(readme): x`,
  `docs: y`, `doc: z`, `feat: mlm does w` and `docs(review): adversarial
  review` give one Documentation entry `*(readme)* X`, one Features
  entry `mlm does w`, and nothing for the other three
- the same clone with `feat(scope): mlm v` shows `*(scope)* mlm v`
- `CHANGELOG.md` is unchanged

## 6. Tasks

One task, no plan beyond this spec.

| Task | Owns | Covers |
|---|---|---|
| 1 | `cliff.toml`, `AGENTS.md` (§4 section only) | §3, §4 |

## User-visible surface

None. Phase 14 is skipped.
