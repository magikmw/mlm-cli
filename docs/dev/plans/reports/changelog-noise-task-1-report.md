# changelog-noise Task 1 report

**Status**: green. Every §5 check holds and all four cargo gates pass.

## Base

The worktree started on `88d9775`, not on `changelog-noise`. It was
rebased onto `changelog-noise` (`e2b251b`) before any work.

## Files changed

- `cliff.toml`
- `AGENTS.md` (new "Commit messages" section under Conventions)
- this report

`CHANGELOG.md` is untouched in the repo.

## Final template and parsers

Body entry line:

```
        - {% if commit.scope %}*({{ commit.scope }})* {% endif %}\
            {% if commit.breaking %}[**breaking**] {% endif %}\
            {% set first_word = commit.message | split(pat=" ") | first %}\
            {% if first_word is matching("^[A-Za-z]+$") and first_word != "mlm" %}\
                {{ commit.message | upper_first }}\
            {% else %}\
                {{ commit.message }}\
            {% endif %}\
```

Parsers (changed lines only):

```
    # An unscoped `docs:` commit is internal (specs, plans, NOTES) and is
    # skipped. A scoped `docs(<scope>):` commit is user-facing and is grouped.
    { message = "^docs?!?:", skip = true },
    { message = "^docs?\\(", group = "<!-- 3 -->📚 Documentation" },
    ...
    { message = "^chore|^ci|^build", group = "<!-- 7 -->⚙️ Miscellaneous Tasks" },
```

The `(?i)adversarial review` skip stays first. The header comment now
points at AGENTS.md "Commit messages".

## §5 checks

Scratch files live under
`/tmp/claude-659200001/-home-magikmw-projects-mlm-cli/cf1d0b51-e308-4ab4-aa33-f9324d704282/scratchpad/`.

### 1. Fixture clone, 11 commits

Setup: `git clone` of the worktree, then 11 empty commits on top of
`e2b251b` with the subjects from §5. Command, run in the clone:

```
git-cliff e2b251b55f6a93414c01dac773441ad266548fca..HEAD --tag v9.9.9
```

Before (old config), wrong as expected:

```
### Features
- Mlm does w
- *(scope)* [**breaking**] Mlm v
### Bug Fixes
- Cliff.toml u
- Plain t
### Documentation
- *(readme)* X
- *(help)* Xx
- Y
- Z
### Other Changes
- [**breaking**] Z2
- S
```

After (new config):

```
### Features
- mlm does w
- *(scope)* [**breaking**] mlm v
### Bug Fixes
- cliff.toml u
- Plain t
### Documentation
- *(readme)* X
- *(help)* Xx
### Miscellaneous Tasks
- S
```

All expectations hold. `docs: y`, `doc: z`, `docs!: z2` and
`docs(review): adversarial review` produce nothing.

### 2. `git-cliff --unreleased` on the branch

Before: Bug Fixes `Prep_release.sh suggests the version release.sh accepts`,
a Documentation group with 8 internal entries, and one Other Changes entry.

After:

```
### Bug Fixes
- prep_release.sh suggests the version release.sh accepts
### Other Changes
- release-ci-check task 1: CI check on HEAD in prep_release.sh
```

No unscoped `docs:` entry. The hint fix reads
`prep_release.sh suggests ...`.

### 3. `git-cliff v0.4.0..v0.5.0`

Before: 104 lines, with a Documentation group of about 60 process entries.

After: Features (4 entries, `Warn ...`, `mlm week --notes ...`,
`mlm export notes ...`, `Relative -N ...`), Bug Fixes (4), Testing (1),
Miscellaneous Tasks (2), Other Changes (4). No Documentation group, no
`Adversarial review` line. `doc-drift code items (...)` prints lowercase
because its first word holds a hyphen, as §3 specifies.

### 4. Prepend

In the clone, from `CHANGELOG.md` as shipped:

```
git-cliff --unreleased --tag v9.9.9 --prepend CHANGELOG.md
```

The `[9.9.9]` section lands above `[0.5.0]`. The header (first 4 lines)
and everything from `## [0.5.0]` down compare byte-identical to the
original (`cmp` exit 0 on both). The new section also lists the real
unreleased commits since `v0.5.0` (the `prep_release.sh` hint fix and
the release-ci-check task) next to the fixture commits, since
`--unreleased` covers everything after the last tag.

### 5. `CHANGELOG.md` unchanged

`git status --short` shows only `AGENTS.md` and `cliff.toml` modified
before the report commit.

## Verification gates

- `cargo fmt --check`: exit 0
- `cargo build`: ok
- `cargo test`: all suites ok (856 unit tests, 0 failed)
- `cargo clippy --all-targets -- -D warnings`: clean

## Deviations

1. Scratch clone lives under the session scratchpad dir, not literal
   `/tmp/<name>`, because the harness restricts temp files to it.
2. The harness refuses `git tag`, so the fixture runs use the range
   `e2b251b..HEAD --tag v9.9.9` instead of `--unreleased` for check 1.
   Check 4 uses `--unreleased --tag v9.9.9` as written, against the clone's
   existing `v0.5.0` tag.
3. The Tera `matching` test and `split | first` worked as expected in
   git-cliff 2.14.1; no other filters were needed.
