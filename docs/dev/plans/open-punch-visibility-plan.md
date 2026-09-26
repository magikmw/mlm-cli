# Plan: open-punch-visibility

**Status**: adversarially reviewed 2026-09-26
(`docs/dev/plans/reports/open-punch-visibility-plan-review.md`,
ship-with-followups, 5 findings, folded — NOTES.md entry 78).

Spec: `docs/dev/specs/2026-09-26-open-punch-visibility.md` (locked).

## Goal

`mlm` already detects pairing problems (multiple open starts, orphaned
ends, a boundary join that didn't happen) but today it either says
nothing about them or says too little to act on. A forgotten `stop`
from days ago is invisible everywhere except a `status` query against
its exact date; a lone open start is silently fine in one case and
loudly flagged in another with no explanation of the difference; every
`[!]` line names a problem but not a fix; and a refused cross-midnight
join leaves no trace anywhere. After this changeset, `status` for
today surfaces stale open starts from the last two weeks even when
they're not today's problem, every anomaly line on `status` carries
concrete `fix:` commands, `week` marks every anomalous date (stale
starts and refused joins included) and points at `status` for detail,
and a backdated `start` warns on stderr about stale starts it can see.
No pairing math, no exit codes, and no automatic repair change — only
what gets reported.

## Architecture (prose)

The work sits on top of two building blocks that don't exist yet and
one that already does. `stint.rs` already classifies a single date's
punches into completed stints, open stints, and orphaned ends, and
already has one predicate (`splice_candidate`, `pub(crate)`) that
tests whether a boundary between two adjacent dates is the clean 1:1
shape a cross-midnight splice requires. This changeset needs two
siblings to that predicate: one that says whether a date's single open
start counts as *stale* (age-gated, using the date and "today," the
same threshold the existing `(unclosed)` rendering rule already uses),
and one that says whether an adjacent pair of dates is a *refused
join* — the shape `splice_candidate` rejects, made positive instead of
negative. Both predicates are pure functions over data `stint.rs`
already produces (a date's own classification, plus a neighbor's), so
they belong there, next to the predicate they're a sibling of. Keeping
them there is also what stops `status.rs` and `week_view.rs` from
growing two independent copies of "what counts as stale" or "what
counts as refused" that drift apart the way the module's own
`compute_has_anomaly` comment warns about.

The second new building block is a shared 14-day lookback: given
today's date, walk ages 2 through 14 and report every one of those
dates that currently has an open start, oldest first. `status` (for
today only) and `start` (after every write) both need exactly this
scan, worded slightly differently (a stdout anomaly block vs. a single
stderr line per finding), so it must exist in exactly one place.
`status.rs` is that place — it already owns the "resolve: I/O +
arithmetic" step for the command that has the fuller need, and `start`
(in `commands.rs`) calls into it rather than re-deriving the scan.

Everything above feeds `render.rs`, which stays the pure,
DB-free, `now()`-free layer it already is. Its `Anomalies` type (or
whatever replaces it) grows enough shape to carry the new facts a date
can have — a stale lone open start, a refused join naming which
neighbor, and, for today's `status` only, the list of lookback
findings — and grows the `fix:`/`or:` wording for every anomaly kind,
old and new, per §3.4's exact line shapes. `status.rs` and
`week_view.rs` each compute the facts for the dates they care about
(using the new `stint.rs` predicates and their own existing
classification calls) and hand them to `render.rs` to turn into text;
neither file grows its own copy of anomaly wording.

`storage.rs` needs no new function: `punches_in_range` already reads
an inclusive date span, which is all any of the new lookback or
neighbor-checking code needs (extend the requested span by one day on
each side to see a boundary's other half). `main.rs` needs no change:
no new CLI flags, no new dispatch arm — `start`'s warning is `eprintln`
inside the existing `commands::start` path.

## Global constraints

Every task must hold to all of these:

- No change to `stint::classify`/`classify_at`, to `splice_candidate`'s
  own logic, or to any existing pairing/splice test's expected value.
  New predicates are added beside the existing ones; nothing already
  there is rewritten.
- No change to week accounting math, to exit codes (anomalies still
  exit `0`), or to any command's silence on stdout except `status`'s
  documented anomaly block. `stop`, `note`, `delete`, and `week
  target` gain no new output of any kind.
- "Age" of a date is always `today − date` in whole calendar days,
  computed the same way everywhere it's needed (the existing
  `(unclosed)` threshold already fixes the age-2 boundary; reuse that
  reasoning, don't re-derive a different one).
- All new literal strings are plain ASCII (SPEC §7): no em dashes, no
  unicode arrows. Every task that introduces a new literal extends the
  relevant existing plain-ASCII golden test rather than adding a
  parallel check.
- Lookback scope is exactly as narrow as the spec draws it: today's
  plain `status` (DATE omitted or explicitly today) and `start`, only.
  `status DATE` for any other date does no lookback; `week` does no
  lookback (it gets per-row markers and a footer, nothing else new).
  A date whose age exceeds 14 is out of scope everywhere (§1.2 of the
  spec, the known residual gap).
- No automatic repair anywhere — every `fix:`/`or:` line names a
  command the user runs themselves.
- Cognitive complexity budget is 15 (`clippy.toml`, enforced by the
  pre-commit hook). The lookback scan, the §3.4 anomaly-line ordering
  assembly, and the multi-neighbor refused-join check are all shaped
  like they could grow past that threshold if written as one function;
  each task should expect to split its logic into several small
  functions rather than lean on the hook to catch it after the fact.
- Coverage may not regress (ratcheted baseline, same hook). Every new
  branch (each anomaly kind, each age boundary, both fix-command
  variants) needs a test exercising it, not just the happy path.

## Interface contracts (pin before opening parallel worktrees)

Two pairs of tasks run concurrently; each pair needs its shared shape
agreed before either worktree opens, since neither side will compile
against a guess.

**Contract A — pin before Task 1 and Task 2 start together.** Task 2
(`render.rs`) never calls into `stint.rs` directly; it only accepts
facts about a date as plain data. Fix before either task starts:

- Exactly which facts describe one date's anomaly state: the existing
  open-start count and orphaned-end list, plus (new) whether its one
  open start is stale and if so its time and age in days, plus (new)
  whether it has a refused join with the previous calendar date and/or
  the next one, and if so which neighbor date each names.
- That a lookback finding is a separate, smaller shape from the above:
  one window date, its age, and either "one open start, this time" or
  "N open stints, see status for that date" — nothing else.
- The literal wording and line order from spec §3.4 for every kind
  (E7, stale, E8, refused join, lookback), including which value fills
  `D` in each `fix:`/`or:` line (the anomaly's own date for in-date
  lines; the window date for lookback lines) — copied verbatim from
  the spec, not paraphrased, since Task 3 and Task 5 will assert
  against this text exactly.
- That `render.rs` provides two distinct renderings of a lookback
  finding: the multi-line `status`-block form (with its own `fix:`/
  `or:` lines) and the single-line `start`-stderr form (§3.6's
  compressed `warning: ...; fix: ...` shape) — same underlying fact,
  two textual shapes, both owned by Task 2.

**Contract B — pin before Task 3 and Task 5 start together.** Both
consume Task 1's new `stint.rs` predicates and Task 2's new `render.rs`
shapes, but neither touches the other's file. Fix before either
starts:

- The plain-language behavior of the two new `stint.rs` predicates:
  one takes a date's classification plus that date and today, and
  answers whether its lone open start is stale; the other takes two
  adjacent dates' classifications and answers whether together they
  form a refused join, and, when true, which side is which.
- The refused-join predicate takes the two dates' **raw single-date
  classifications** (the same inputs the existing splice gate reads),
  never `classify_at`-resolved ones. It is true when the earlier date
  has at least one open start, the later date has at least one
  orphaned end, and the existing splice gate rejects the pair. Both
  `status` and `week` call it that way, so they always agree on a
  date pair.
- That both predicates are pure and read no clock and no database
  themselves — every date and "today" they need is passed in by the
  caller, matching the rest of `stint.rs`'s existing style.
- The finished shape of Task 2's per-date fact structure and lookback
  finding structure (i.e., Contract A's output), since both `status.rs`
  and `week_view.rs` construct instances of it.

No contract is needed between Task 3 and Task 4: Task 4 is sequenced
strictly after Task 3 and calls the lookback capability Task 3 lands,
so it simply reads the finished code rather than a pinned-in-advance
shape.

## Ordering and parallelization

```
Task 1 (stint.rs)  ---\
                       +--> Task 3 (status.rs)  --> Task 4 (commands.rs) --\
Task 2 (render.rs) ---/\                                                   +--> Task 6 (docs)
                        \--> Task 5 (week_view.rs) ------------------------/
```

- Task 1 and Task 2 run in parallel worktrees, against Contract A.
  Neither imports the other's new code.
- Task 3 and Task 5 both depend on Task 1 and Task 2 landing first
  (they call the new predicates and construct the new render-facing
  shapes). Once both land, Task 3 and Task 5 run in parallel worktrees
  against Contract B — different files, no shared symbol either one
  defines.
- Task 4 depends on Task 3 only (it calls the lookback capability
  `status.rs` lands). It cannot start before Task 3 merges.
- Task 6 (docs) depends on every code task, since it must quote final
  wording verbatim. It can be drafted early from the spec but its last
  pass, and its merge, waits for Tasks 1-5.

**Critical path**: Task 1 or Task 2 -> Task 3 -> Task 4 -> Task 6 (four
sequential steps). Task 5 runs alongside Task 3/Task 4 and does not
lengthen the path unless it individually takes longer than that pair
combined.

`storage.rs` and `main.rs` need no task: no new schema, no new query
shape (`punches_in_range` already covers every read this changeset
needs), no new CLI surface.

## Task 1 — Anomaly predicates

**Spec citations**: §2 (terms: age, stale open start, lookback window,
refused join), §3.1, §3.2.

**Files owned exclusively**: `src/stint.rs`.

**Acceptance criteria**:

- A date with exactly one open start and age 0 or 1 is not stale; age
  2 through 14 is stale; age 15+ is still stale by the same rule (the
  14-day cutoff is a lookback-window boundary, owned by Task 3, not a
  staleness boundary — the predicate itself doesn't stop being true
  past 14 days).
- A date with two or more open starts is never reported stale by this
  predicate (that shape is E7's job, already covered by existing
  code); the predicate only fires for exactly one open start.
- Two adjacent dates form a refused join whenever the earlier date has
  at least one open start and the later date has at least one orphaned
  end, and it is not the case that they'd cleanly splice (mirroring,
  as a positive, the exact shape `splice_candidate` already tests
  negatively) — this holds at every age, including yesterday-today.
- The refused-join predicate reports which date is "the one with the
  open start" and which is "the one with the orphaned end" distinctly,
  since spec §3.4 requires each side's line to name the other
  specifically.
- A clean splice (the shape `splice_candidate` already accepts)
  produces no refused join.
- Both predicates are unit-tested directly, independent of `status.rs`
  or `week_view.rs`, covering the boundary ages (0/1/2/14/15) and the
  refused-join shapes enumerated in the spec's test plan (two opens on
  the earlier date, the orphan not first on the later date, two
  orphans on the later date).

**Out of scope**: any change to `classify`/`classify_at`/
`splice_candidate` themselves; any rendering; any date-range/DB
reading (predicates take already-classified data).

## Task 2 — Anomaly rendering and fix-line wording

**Spec citations**: §3.4, §3.5 (footer wording only, not the row-marker
logic itself), §3.6 (the stderr wording, not the warning trigger),
§3.7.

**Files owned exclusively**: `src/render.rs` (batch 1).

Task 2 is **additive**. It adds the new per-date anomaly type and its
renderers beside the existing ones and leaves every existing render
type and function working, so the build and the pre-commit test gate
stay green after its merge. The binary's output does not change until
Task 3 switches `status` over.

**Acceptance criteria**:

- Every §3.4 line shape renders verbatim against the spec's examples:
  reworded E7 with its two `fix:`/`or:` lines, the new stale-open-start
  line and its two `fix:`/`or:` lines, E8 with its two new `fix:`/`or:`
  lines (existing E8 first line text unchanged), the refused-join line
  on each side (same parenthetical, opposite named neighbor), and both
  lookback line shapes (single open start, and two-or-more open
  stints).
- Line ordering inside one date's anomaly block matches §3.4's fixed
  order: E7, stale, E8 (ascending by instant), refused join
  (previous-date line before next-date line), lookback lines last
  (oldest first) — ordering is exercised by a test that supplies every
  kind at once and asserts the resulting sequence.
- `week`'s footer line renders exactly per §3.5 wording, as a standalone
  piece this task owns (whether it's appended is Task 5's call — this
  task only provides the literal line).
- The `start`-stderr rendering of a lookback finding matches §3.6's
  compressed one-line shape exactly, both the single-open and
  multi-open variants.
- Existing golden-output tests for E7/E8's prior wording either still
  pass unchanged (if any fixture predates the reworded text) or are
  updated in this same task to the new wording — no test elsewhere is
  left asserting the old E7/E8 text.
- Every new literal is added to the module's existing plain-ASCII
  sweep test.

**Out of scope**: computing whether any of these facts is true for a
given date (Tasks 1/3/5's job); anything about *when* `start` runs the
lookback (Task 4); the row-marker boolean itself in `week` (Task 5
decides which dates get marked — this task only supplies the footer
text and the per-date line text used elsewhere).

## Task 3 — `status`: per-date anomalies and the shared lookback

**Spec citations**: §3.1, §3.2, §3.3, §3.4 (assembly, not wording),
terms in §2.

**Files owned exclusively**: `src/status.rs`, `tests/status_cli.rs`.
Task 3 is where the new E7 wording first reaches the binary, so it
updates `tests/status_cli.rs`'s E7 assertion and every old-wording
fixture in `status.rs`'s own tests.

**Acceptance criteria**:

- `status DATE` (any date, not just today) now reflects the new §3.1
  stale-open-start and §3.2 refused-join facts for that date, computed
  via Task 1's predicates against that date's own classification and
  its immediate neighbors' (fetched the same way the existing
  splice-check neighbor data already is, extended to also cover the
  next-day neighbor where not already fetched).
- `status` for today (DATE omitted, or today's date given explicitly)
  additionally runs the 14-day lookback (ages 2-14) and appends one
  finding per window date with any open start, oldest first, after the
  date's own anomaly lines; `status` for any other date runs no
  lookback.
- The lookback capability itself is exposed so Task 4 can call it
  without duplicating the date-walking or classification logic; it
  reports findings as data (date, age, single-or-multiple), leaving
  both textual renderings (block form, stderr form) to Task 2's
  functions.
- Anomaly line assembly for one date follows §3.4's fixed order end to
  end, including the interaction case where a date carries E7, a
  refused join, and (for today) lookback lines simultaneously.
- Every `fix:`/`or:` line produced carries the real date, not a
  placeholder, for every anomaly kind now reachable from `status.rs`.
- Full test-plan coverage for everything in the spec's test plan that
  is a `status`-observable behavior: the stale boundary ages, two-open
  vs. one-open distinction, refused join in both directions and at the
  yesterday-today boundary, lookback ordering and both finding shapes,
  `status` with an explicit today's date doing the lookback, an
  orphaned end alone inside the window producing no lookback line, and
  the fix-command round-trip (running the hinted command then
  re-querying shows no anomaly; the delete hint lists the right
  punch).
- No regression to any existing `status.rs` golden test outside the
  wording Task 2 already changed.

**Out of scope**: `week`'s per-row marker or footer (Task 5); `start`'s
warning (Task 4, calls into this task's lookback but is not this
task's file); any change to the daily-target/pace-hint lines or
anything else in `status` untouched by this spec.

## Task 4 — `start`: stale-start warning

**Spec citations**: §3.6.

**Files owned exclusively**: `src/commands.rs`; and, sequentially
after Tasks 3 and 5 have merged, `src/render.rs` for one job only —
deleting the render type and functions the new per-date type replaced,
once nothing calls them.

**Acceptance criteria**:

- After a successful `start` write (with or without `--date`), the
  command runs Task 3's lookback relative to today and prints one
  stderr line per window date with open starts, oldest first, in
  Task 2's stderr line shape.
- The date just written to is excluded from the warning unless the new
  punch made it an E7 (two or more open starts there now); a single
  open start on the written date (the punch just made) never warns.
- stdout is unaffected (still silent on success) and the exit code
  stays `0` regardless of what the lookback finds.
- If the lookback read fails after the write already committed, `start`
  still exits `0` and prints nothing extra (the check is best-effort,
  never a reason to fail an otherwise-successful write).
- Test coverage: warning present with the right lines and ordering;
  stdout empty and exit `0` throughout; no warning for the just-written
  date when it has a single open start, and the multi-open warning when
  the new punch made it an E7; no warning
  when the window is clean; a backdated `start --date D` into an
  existing stale date warns with the multi-open form, into an empty
  stale date does not warn about `D` itself. The read-failure path is
  tested if a seam exists for it without new production plumbing;
  otherwise the swallowed error is kept to one visibly-commented spot
  and the gap is noted in the task's commit message.

**Out of scope**: any warning from `stop`, `note`, `delete`, or `week
target` (explicitly not in scope per spec §1.1); the lookback's own
implementation (Task 3's file); any change to `start`'s existing
argument handling or its own write path.

## Task 5 — `week`: row markers and footer

**Spec citations**: §3.5.

**Files owned exclusively**: `src/week_view.rs`.

**Acceptance criteria**:

- A row's `[!]` marker now also fires for a stale open start on that
  date and for a refused join touching that date, in addition to the
  existing E7/E8 cases — computed via Task 1's predicates against each
  row date's own classification and its neighbors' (extending the
  existing per-row data fetch to cover the neighbor dates the new
  predicates need, via the existing range-based read extended by one
  day on each side of the week).
- Exactly one footer line (Task 2's wording) appears after the table,
  after a blank line, whenever at least one row carries `[!]`; no
  footer when no row is marked.
- No lookback of any kind appears in `week` output — only the marker
  and the single footer line change.
- Regression: a week with no anomalies of any kind renders
  byte-identical to today's output; a week whose only anomaly is a
  stale date or a refused join newly gets marked, where it previously
  (per the old §7.2 rule) rendered with no marker at all.
- Test coverage: a stale-date row is marked; a refused-join row (both
  the earlier and later date's row, when both fall in the same
  displayed week) is marked; footer presence/absence in both
  directions; no change to E7/E8 marking already covered by existing
  tests.

**Out of scope**: any new column, any per-row detail beyond the
existing marker; the lookback (`status`/`start`-only); any change to
carry-in/worked/fulfillment/target block below the table.

## Task 6 — Docs

**Spec citations**: §4 (the full docs list), which itself points at
§1.2a, §4.3, §4.3.1, §7.1-§7.4, §8.2 of `SPEC.md`, plus README.md's
known-issues, `status`, `week`, and `start` sections.

**Files owned exclusively**: `docs/dev/SPEC.md`, `README.md`.

**Acceptance criteria**:

- `SPEC.md` §1.2a: the four superseded bullets (silent splice gate,
  no remedy guidance, lone-vs-double open start, multi-day-old
  forgotten stop) are removed; the spec's own §1.2 14-day residual
  bullet (already drafted in the design spec's §1.2) is folded in
  verbatim.
- `SPEC.md` §4.3: stale open start is listed alongside the other edge
  cases, with an explicit note that a lone open start at age 0 or 1
  stays intentionally silent.
- `SPEC.md` §4.3.1: the "gate's own on/off condition is undocumented"
  line is replaced with a description of the refused-join anomaly that
  now surfaces it.
- `SPEC.md` §7.1/§7.2/§7.3/§7.4 each gain the relevant new behavior
  (anomaly bullet and lookback lines and the `(unclosed)` cross-
  reference in §7.1; stale/refused-join marking and the footer in
  §7.2, with the old "accepted, silent consequence" passage rewritten
  to match; new line shapes and `fix:` lines in §7.3; the `start`
  stderr exception in §7.4).
- `SPEC.md` §8.2: E7 and E8's documented expected output gain the fix
  lines; new lettered cases (E16 onward, continuing past the existing
  E15) cover the stale start, the refused join, the lookback, and
  `start`'s warning.
- `README.md`: known-issues section drops the four superseded bullets
  and gains the 14-day-residual bullet; the `status` section documents
  `[!]` lines, `fix:` lines, and today's lookback; the `week` section
  documents the footer; the `start` section documents the stderr
  warning; a short plain-language paragraph explains when an overnight
  session joins into one stint and when it doesn't.
- `--help` text is confirmed unchanged (spec says no change needed) —
  this task verifies that claim rather than silently assuming it.
- Every quoted example string in the updated docs matches Task 2's
  actual rendered text exactly (copy from a real test run or the
  merged code, not re-typed from the design spec).

**Out of scope**: any code change; any working-doc archival beyond
what AGENTS.md's documentation convention already requires (this
design spec itself gets archived per that convention once its content
is folded in, but that archival is process, not something this task's
acceptance criteria need to re-litigate).
