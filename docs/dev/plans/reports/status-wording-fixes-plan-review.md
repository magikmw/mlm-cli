# Adversarial review — status-wording-fixes-plan.md

Reviewed against: `docs/dev/specs/2026-09-23-status-wording-fixes.md` (locked)
and `src/render.rs`, `src/status.rs`, `src/week.rs`, `src/week_view.rs`,
`src/commands.rs`, `README.md`, `docs/dev/SPEC.md` as they exist on `main`
at review time.

## Findings

### 1. [HIGH] Fix C's acceptance criteria are not implementable as written — no data path carries `weekday_number.min(5) == 5` to `day_total_line`

**Claim** (plan `docs/dev/plans/status-wording-fixes-plan.md:156-165`, echoed in
constraints at `:84-86`): `day_total_line`'s daily-target clause is to render
`required today` "when `weekday_number.min(5) == 5`," reusing "the same
work-week-cap check `required_minutes` already performs at its own call
site."

**Evidence**: `weekday_number` is computed and consumed entirely inside
`resolve()` at `src/status.rs:463-464`:

```
let weekday_number = i64::from(today.weekday().number_from_monday());
let required_minutes = daily_target_minutes * weekday_number.min(5);
```

`day_total_line` (`src/status.rs:157-190`) is a pure `&StatusView -> String`
function (its own doc comment at `src/status.rs:156` says so) that never
sees `today` or `weekday_number` — it only has `view.daily_target: Option<
DailyTargetHint>` (`required_minutes`, `gap_minutes` — `src/status.rs:58-64`)
and `view.weekday_name: Option<String>` (a display string like
`"Thursday"`). Neither field encodes whether the work-week cap was reached;
`required_minutes` alone can't be reverse-engineered into that boolean
without also knowing `daily_target_minutes`, which `StatusView` doesn't
carry either.

The plan is explicit and careful about the analogous problem for Fix B — it
says outright that `EodState::At` "gains a second field, a bool ... carry
the result through" (plan `:36-40`). For Fix C it never says the equivalent
thing: neither `StatusView` nor `DailyTargetHint` is named as gaining a new
field, and the constraint at plan `:84-86` ("reuses `status.rs`'s existing
`weekday_number.min(5)` result ... rather than introducing a second bare `5`
literal") is stated as if it were trivially satisfiable by the rendering
function, when in fact it requires a new field threaded from `resolve()`
through `StatusView`/`DailyTargetHint` into `day_total_line` — exactly the
kind of type change the plan spells out for Fix B but omits for Fix C.

Matching against `view.weekday_name`'s three possible strings
(`"Friday"`/`"Saturday"`/`"Sunday"`) is not a fix: it reintroduces the
"second bare work-week-length constant" the plan explicitly forbids, this
time as three literal weekday names instead of a `5`.

This directly hits the review's "are the acceptance criteria concrete
enough to write tests from without a clarifying question" check — an
implementer cannot write `day_total_line`'s new branch without first
inventing where the boolean lives, a design decision the plan should have
pinned but didn't.

### 2. [HIGH] Plan and spec both claim no existing test exercises the Fix C condition — an existing test does, and it isn't in either task's update list

**Claim** (plan `:157-165`; source spec `docs/dev/specs/2026-09-23-status-wording-fixes.md:157-164`):
"no existing test exercises a weekday where `weekday_number.min(5) == 5`" —
the fixture "pins `today` to Thursday 2026-02-12 throughout," so Task 1 must
*add* Friday/weekend coverage.

**Evidence**: `src/status.rs:1759-1780`,
`resolve_f9b_sunday_pin_non_multiple_of_five_target_override`, already pins
a Sunday (`d(2026, 2, 15)`, ISO weekday 7, `min(5) == 5`) and asserts, via
the real rendered output:

```rust
let out = render(&view);
assert!(out.contains("33h 30m required by end of Sunday"));
```

Under Fix C this exact line becomes `33h 30m required today`, so this test
breaks — but it is not Thursday-pinned, it is not in the plan's enumerated
`EodState::At` list (that list is for Fix B, unrelated), and Task 1's
acceptance criteria (plan `:166-167`) requires "every existing test not
touched by the three fixes above passes unchanged," which is false for
this specific test. Nothing in Task 1's acceptance criteria names this test
as needing an update, because the plan's own premise — that no existing
test hits this weekday range — is false. (A companion Saturday test at
`src/status.rs:1739-1757` also pins a capped weekday but happens not to call
`render()`, so it wouldn't break; it's the Sunday one that will.)

This is a factual error about the codebase repeated verbatim from the spec
into the plan, and it produces a genuine testing gap: Task 1 as scoped will
either silently break this test (violating its own regression criterion)
or an implementer will have to discover and fix it without instruction.

### 3. [MEDIUM] Undeclared user-visible gap: an existing, live README example goes stale under Fix C, and Task 2's scope never says to touch it

**What the document decides**: Task 2's acceptance criteria (plan
`:192-229`) enumerate exactly what gets touched in `README.md` — the
`Total still owed` → `Total behind` rename, two new closed-period examples,
and the Known-issues bullet drops. It never mentions auditing or updating
README's *existing* current-period examples for Fix B/Fix C's new wording.

**The consequence a user will see**: `README.md:196-198` already ships a
live worked example queried on a Saturday:

```
Sat 2026-09-12

Day total:     07h 25m (+ ongoing), 04h 35m over 40h 00m required by end of Saturday, est. EOD target already met
```

Saturday is `weekday_number.min(5) == 5` — exactly Fix C's trigger
condition. Once Fix C ships, the real output for this scenario is
`... required today, ...`, not `... required by end of Saturday, ...`. This
is the only `required by end of` occurrence in the whole of `README.md`
(confirmed via `grep -n "required by end of" README.md`), so nothing else
in the file would catch or correct it. Unless Task 2 independently notices
and fixes it — which its own acceptance criteria give it no instruction to
do — the shipped README's flagship `status` example will directly
contradict the tool's actual behavior the moment Fix C lands: a user
running the exact command shown, on a Saturday, will see output that
doesn't match the doc.

This is the "undeclared user-visible gap" this review was specifically
asked to hunt for: a decision the plan makes (Task 2's scope excludes
auditing existing examples for staleness under Fix B/C) that has a
consequence a user will see (a wrong worked example in README), and the
plan records neither the decision nor the consequence.

### 4. [LOW] Minor factual slip in the `EodState::At` test-site enumeration (inherited from spec, low-impact)

**Claim** (plan `:151`, spec `:103-107`): the existing single-argument
`EodState::At(t)` constructions to convert to two-argument form include
`base_view`.

**Evidence**: `base_view()` (`src/status.rs:583-598`) sets `eod: None` and
contains no `EodState::At` construction at all. The actual sites are
`src/status.rs:658` (`t1`), `950` (`t6a`), `1158` (`t11`), `1256` (`t12`),
`1320` (`t13`) — five sites, none of them `base_view`.

Low severity only because the plan already hedges this exact list as a
floor, not a ceiling ("and any others found by search — not just the ones
new test cases touch," plan `:151-152`), and separately calls out in "Open
risks" that the enumeration may drift (plan `:257-261`). Worth a note
since it's a second, independent instance of the plan trusting an
unverified spec enumeration (see also Finding 2) rather than the
"re-run the grep" discipline it otherwise insists on for the string rename.

### 5. [LOW / informational] Pre-existing unrelated defect noticed at the exact spot Task 2 will edit

`README.md:198`'s existing example renders `..., est. EOD target already
met` — a phrase the current renderer cannot produce. `day_total_line`
(`src/status.rs:184-188`) only ever emits either `, est. EOD HH:MM` *or*
`, target already met`, never both concatenated. This looks like a stale
hand-edit unrelated to any of this changeset's four fixes, not a defect in
the plan itself — flagged only because Task 2 will be editing this exact
example's neighborhood (see Finding 3) and could either perpetuate or
silently fix it without anyone having asked either way.

## Categories checked and found clean

- **File ownership vs. spec**: Task 1 owns `render.rs`/`status.rs`
  (production) plus test-only edits in `week_view.rs`/`commands.rs`; Task 2
  owns `README.md`/`SPEC.md`. This matches spec §6 exactly, and the
  `week_view.rs`/`commands.rs` sites checked (`src/week_view.rs:355,386,
  455,476`; `src/commands.rs:1128,1156`) are genuinely test-only literal
  assertions with no adjacent production code — the plan's claim there
  holds up.
- **`Total still owed` → `Total behind` count**: plan/spec claim 8 hits in
  `SPEC.md`; `grep -c "Total still owed" docs/dev/SPEC.md` returns exactly
  8, matching lines 83, 98, 647, 672, 679, 715, 752, 776.
- **Known-issues bullet mapping**: both `README.md`'s and `SPEC.md §1.2a`'s
  "Known issues" sections have exactly 8 bullets each, and the 4 the plan
  says to drop (closed-period wording, EOD-tomorrow, last-weekday
  redundancy, "Total still owed" tone) map cleanly onto Fixes D/B/C/A,
  leaving the other 4 (splice-gate disclosure, anomaly remedy guidance,
  lone-unclosed-start ambiguity, multi-day-old forgotten stop) untouched in
  both files, as claimed.
- **Fix A mirror claim**: `render.rs:67-70`'s two `WeekFraming::Closed`
  arms confirmed — only the `owed_minutes > 0` arm's literal changes, the
  `else` arm (`Total ahead`) is untouched, matching the plan.
- **No accounting-figure changes**: confirmed no fix touches
  `gap_minutes`/`required_minutes`/`owed_minutes` computation sites
  (`status.rs:464-465`, `week.rs:226`) — both fixes are genuinely
  string/field-only where the plan says so, modulo Finding 1's missing
  field.
- **"Scope of work altitude" / no code-pseudocode**: the plan's
  "Architecture (prose)" section and Task 1's acceptance criteria do quote
  exact match-arm literals and one exact boolean condition
  (`(now + gap_minutes).date_naive() != today`), which reads closer to
  algorithm-level detail than pure architecture prose. This is deliberate
  and justified in-document (the "Interface contract to pin" section
  explains Task 2 needs exact strings to quote verbatim in docs), so it is
  not filed as a standalone finding — but it is the same instinct that
  produced Finding 1's problem: the plan is precise about *string* shapes
  down to the character, yet silent on the *type*/data-flow change Fix C
  actually requires.

## Verdict

needs-rework
