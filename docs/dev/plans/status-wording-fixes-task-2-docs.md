# Task 2 low-level plan: docs (`README.md` + `docs/dev/SPEC.md`)

Changeset: `status-wording-fixes`.

## Read in full before writing this plan

- `docs/dev/plans/status-wording-fixes-plan.md` (locked, one round of
  adversarial review folded in — treated as settled).
- `docs/dev/specs/2026-09-23-status-wording-fixes.md` (locked, two
  rounds of adversarial review folded in — treated as settled).
- `README.md` (current file, full).
- `docs/dev/SPEC.md` (current file, the relevant ranges: §1.2a
  lines 58-102, and §7 lines 594-834, plus the four normative-prose
  hits at lines 647/672/679/776 read in their surrounding paragraphs).
- `src/render.rs` (full) — `week_headline`'s `WeekFraming::Closed`
  branch (lines 60-72) and its test literals, to confirm the exact
  closed-period output shape.
- `src/status.rs` lines 1-260 — `EodState`, `DailyTargetHint`,
  `day_total_line` (lines 157-190), to confirm the exact `est. EOD`
  and day-total rendering logic, including that `TargetAlreadyMet`
  renders `, target already met` alone, never prefixed with
  `est. EOD`.

There is no Task 1 task plan or completion report yet (being written
in parallel). Task 1's exact string shapes are pinned instead by the
changeset plan's "Interface contract to pin before opening worktrees"
section, quoted verbatim where used below. Per the changeset plan's
own soft-ordering note, this plan's derivations should still be
spot-checked against Task 1's actual completion report / test
assertions once it exists, before this task's edits are executed —
see "Verification plan" below.

## Scope

**This task owns**: `README.md`, `docs/dev/SPEC.md`. Doc-only, no
Rust.

**Task 1 owns exclusively** (do not touch): `src/render.rs`,
`src/status.rs`, `src/week_view.rs`, `src/commands.rs`. This plan
describes what output Task 1's code will produce, because that output
is pinned by the changeset plan's interface contract, not because this
task derives or verifies it independently.

**Interface contract this task's strings depend on** (from the
changeset plan, restated here so the edits below aren't decoupled from
their source):

- Closed-period deficit headline: exactly `"Total behind: {formatted
  minutes}"`.
- EOD tomorrow marker: exactly `" (tomorrow)"` appended after the
  existing `HH:MM` time.
- Day-total capped-week wording: exactly `"required today"`, replacing
  the entire `"required by end of {weekday}"` phrase, firing on
  Friday, Saturday, and Sunday (`weekday_number.min(5) == 5`).

## Current occurrence counts (grepped fresh, not trusted from the spec)

```
$ grep -n "Total still owed" docs/dev/SPEC.md
83:    different sentence shape (`Total still owed: Xh Ym`, no
98:  "`Total still owed`" (the closed-week/closed-date headline, §7.1/§7.2)
647:  inline `-` (matching decision 18's closed-week "Total still owed" /
672:  or the plain `Total still owed`/`Total ahead` form when `DATE`'s
679:  past/future `Total still owed`/`Total ahead` form never gets this
715:Week 2026-02:  Total still owed: 01h 40m
752:Total still owed: 03h 10m
776:  a closed or not-yet-started week gets the plain `Total still owed`/
```

8 hits, confirming the spec's own count and matching (not contradicting)
its line-number citations at this snapshot — re-run this grep
immediately before editing, since Task 1 landing first would not move
these lines (Task 1 never touches SPEC.md) but another doc edit
landing first could.

```
$ grep -n "required by end of" README.md
198:Day total:     07h 25m (+ ongoing), 04h 35m over 40h 00m required by end of Saturday, est. EOD target already met
```

One hit, confirming the spec's claim that this is the only occurrence
in `README.md`.

## Edit 1 — SPEC.md §1.2a: drop 4 of 8 bullets

Current state (`docs/dev/SPEC.md:64-101`), 8 bullets in this order:

1. splice-gate disclosure (KEEP)
2. anomaly remedy guidance (KEEP)
3. lone-unclosed-start ambiguity (KEEP)
4. multi-day-old forgotten stop (KEEP)
5. closed-period undocumented sentence shape (DROP — closed by Fix D's
   new README example)
6. `est. EOD` tomorrow, no date shown (DROP — closed by Fix B)
7. day-total/week-line redundancy on the last weekday (DROP — closed
   by Fix C)
8. `"Total still owed"` punitive tone (DROP — closed by Fix A)

Delete bullets 5-8 verbatim (`docs/dev/SPEC.md:82-101`, the four
bullets from `- \`status\`/\`week\`'s output for a past (closed)
date/week...` through `...("fulfillment," "carry-in," "shortfall/
surplus" per the README).`). Bullets 1-4 (lines 64-81) are left
byte-identical. The section's intro prose (lines 58-63) is unchanged.

Resulting section reads:

```markdown
### 1.2a Known issues to revisit

Shipped, working-as-designed behavior that's rough or confusing, not
committed to a fix. Kept separate from §1.2: a non-goal isn't built
yet, a known issue works but has a rough edge.

- Whether an unclosed stint reaching into the next day silently merges
  or gets flagged and left unmerged depends on an internal
  1:1-unambiguous gate (§4.3) the user has no way to observe — nothing
  in `--help`, `status`, or `week` explains why the same-looking
  situation sometimes resolves silently and sometimes doesn't.
- `[!]` anomaly flags (`status` and `week`) name the problem but give
  no remedy guidance — no pointer to `delete`, no suggested next step.
- Whether a lone unclosed `start` gets flagged depends on whether a
  *second* one also exists that date (one is the ordinary open-stint
  case, two-or-more is E7) — the same surface signal ("still open, no
  stop yet") is silent in one case and loudly flagged in the other,
  and the distinction isn't explained anywhere.
- A multi-day-old forgotten `stop` is invisible everywhere except the
  exact calendar date it started: `status` for today, `status` for any
  date in between, and `week`'s per-day table (that date's row just
  reads `00h 00m`) all show zero trace of it. Nothing says "you have an
  open punch from N days ago" anywhere except a `status` query against
  that exact date.

### 1.3 Terminology
```

(The two `Total still owed` occurrences inside bullets 5 and 8 above
vanish with the deletion — they are not renamed, per Fix A's own
description of this pair as "deleted along with the rest of that
bullet.")

## Edit 2 — SPEC.md: the remaining 6 `Total still owed` → `Total behind`

Normative prose, three sites:

`docs/dev/SPEC.md:647` — before:
```
  inline `-` (matching decision 18's closed-week "Total still owed" /
```
after:
```
  inline `-` (matching decision 18's closed-week "Total behind" /
```

`docs/dev/SPEC.md:672` — before:
```
  or the plain `Total still owed`/`Total ahead` form when `DATE`'s
```
after:
```
  or the plain `Total behind`/`Total ahead` form when `DATE`'s
```

`docs/dev/SPEC.md:679` — before:
```
  past/future `Total still owed`/`Total ahead` form never gets this
```
after:
```
  past/future `Total behind`/`Total ahead` form never gets this
```

`docs/dev/SPEC.md:776` — before:
```
  a closed or not-yet-started week gets the plain `Total still owed`/
```
after:
```
  a closed or not-yet-started week gets the plain `Total behind`/
```

Worked examples, two sites:

`docs/dev/SPEC.md:715` (§7.1's closed-date example) — before:
```
Week 2026-02:  Total still owed: 01h 40m
```
after:
```
Week 2026-02:  Total behind: 01h 40m
```

`docs/dev/SPEC.md:752` (§7.2's closed-week example) — before:
```
Total still owed: 03h 10m
```
after:
```
Total behind: 03h 10m
```

No other content on any of these six lines changes. No new example is
added at either site — Fix D explicitly does not touch SPEC.md's
examples beyond this string swap (see Edit 4 below).

## Edit 3 — SPEC.md §7.1: document the `(tomorrow)` and `required today` cases

Two prose insertions into §7.1's bullet list (`docs/dev/SPEC.md:637-
665`), each folded into the bullet already describing that line so the
section doesn't grow a redundant third bullet per fix.

**3a — `required today` case**, appended to the end of the day-total
pace-hint bullet (the one starting `"X left to \`<required>\`
required by end of \`<weekday>\`"...`, ending at
`docs/dev/SPEC.md:657` with `...the pace hint is "today's slice of the
whole week's math."`). New sentence appended after that bullet's final
sentence, same bullet:

```
  On Friday, Saturday, and Sunday — the three days on which
  `required` has already plateaued at the week's full target
  (`weekday_number.min(5) == 5`) — this trailing phrase drops the
  weekday name entirely and reads `required today` in place of
  `required by end of <weekday>`, since the week line directly above
  already names the same weekday and, on those three days, the same
  figure. Monday through Thursday render `required by end of
  <weekday>` unchanged.
```

**3b — `(tomorrow)` case**, appended to the end of the "Estimated EOD"
bullet (`docs/dev/SPEC.md:658-665`, ending `...replaced with \`target
already met\` when the gap is already zero or negative.`). New
sentence appended after that bullet's final sentence, same bullet:

```
  When the projected `now + gap` lands on a later calendar date than
  today — a large enough `gap` to cross midnight — the line gains a
  literal ` (tomorrow)` suffix after the clock time, e.g. `est. EOD
  01:15 (tomorrow)`, so the time can't be misread as landing before
  midnight tonight. The marker is always the bare word `(tomorrow)`,
  never a weekday name or calendar date, even when `gap` is large
  enough to cross more than one midnight — an accepted imprecision
  past one day out, not a bug.
```

Both insertions are prose-only, no new fenced code block — SPEC.md's
existing §7.1 example (`docs/dev/SPEC.md:605-618`) already shows the
ordinary `est. EOD 20:45` and `required by end of Thursday` shapes;
adding a second full worked example for the `(tomorrow)`/`required
today` variants would duplicate what a sentence-level description
already covers, and README's new closed-period examples (Edit 4) are
where this changeset's only new fenced examples belong.

## Edit 4 — README.md: two new closed-period worked examples

Placed after the existing past-date `status` example
(`README.md:214-222`, `$ mlm status 2026-09-08`) and after the
existing `week target`-override example (`README.md:259-278`) — i.e.
appended at the end of each command's own example block, not
interleaved with the current-period examples, so the current-period
examples stay the reader's first, unqualified reference point.

Both examples use a shared fictional past week, `2026-35`
(`2026-08-24` – `2026-08-30`), distinct from the `2026-37` week the
existing examples already use, so the two blocks are visibly
consistent with each other (same week, same per-day figures) without
colliding with the current-period week id.

**4a — new `status` example**, inserted directly after
`README.md:222` (the closing `` ``` `` of the `2026-09-08` example),
before the `### mlm week|w [WEEK_ID]` heading:

```markdown

A date in an already-closed week shows the same plain total the week
line uses (Fix A/§7.1 in `docs/dev/SPEC.md`) instead of the
current-week's deadline framing — no fulfillment/target parenthetical
either, since that only applies to the current week:

​```sh
$ mlm status 2026-08-25
Tue 2026-08-25

Day total:     07h 50m
Week 2026-35:  Total behind: 02h 10m

  09:10-17:00  (07h 50m)
​```
```
(Render without the zero-width-space escaping shown above for the
fence markers — that escaping is an artifact of nesting a fenced
example inside this plan document, not something to write into
README.md itself.)

**4b — new `week` example**, inserted directly after `README.md:278`
(the closing `` ``` `` of the `week target` example), before the
`### mlm delete|del note|n [ID] [-d/--date DATE]` heading:

```markdown

A past (or future) week has no "today" to frame a deadline against, so
its headline is the same plain total `status` showed above, leading
the output instead of appearing inline — everything else is the same
shape:

​```sh
$ mlm week 2026-35
Week 2026-35 (2026-08-24 - 2026-08-30)

Total behind: 02h 10m

  Mon 2026-08-24   08h 15m
  Tue 2026-08-25   07h 50m
  Wed 2026-08-26   08h 00m
  Thu 2026-08-27   07h 45m
  Fri 2026-08-28   06h 00m
  Sat 2026-08-29   00h 00m
  Sun 2026-08-30   00h 00m

Carry-in:      00h 00m
Worked:        37h 50m
Fulfillment:   37h 50m
Target:        40h 00m
​```
```

**Arithmetic check on these two new examples** (so they're internally
consistent even though they can't be checked against the real binary
yet — see "Verification plan"):

- Per-day sum: 08h15 + 07h50 + 08h00 + 07h45 + 06h00 + 00h00 + 00h00
  = 37h50m.
- Carry-in 00h00m, so Worked = Fulfillment = 37h50m (matches the
  per-day sum, as it must with zero carry-in).
- Target 40h00m; owed = target − fulfillment = 40h00 − 37h50 =
  02h10m, positive (short of target) → `Total behind: 02h 10m`,
  matching both the `week` headline and the `status` example's week
  line (same week, so the same headline).
- The `status` example's `Day total: 07h 50m` matches the `week`
  example's `Tue 2026-08-25   07h 50m` row exactly, and the single
  stint line `09:10-17:00  (07h 50m)` sums correctly
  (09:10→17:00 = 7h50m).

**Shape check against `render.rs`**: `WeekFraming::Closed`
(`src/render.rs:60-72,105`) produces bare `Total behind: {}` /
`Total ahead: {}` with an empty parenthetical string — no
`(fulfillment .../target ...)` clause at all, regardless of carry-in.
Both new examples omit that parenthetical, matching. `day_total_line`
(`src/status.rs:157-190`) only appends its `, ... required by end of
{weekday}`/`, est. EOD ...` clauses when `view.daily_target`/`view.eod`
are `Some`, which `resolve()` only sets when `DATE == today`
(`status.rs:56,64,119-126`) — a closed past date never satisfies that,
so the new `status` example's `Day total:` line correctly has no
trailing clause at all, just the bare total.

## Edit 5 — README.md's existing Saturday `status` example (Fix C + pre-existing bug)

`README.md:198`, current text:

```
Day total:     07h 25m (+ ongoing), 04h 35m over 40h 00m required by end of Saturday, est. EOD target already met
```

Two independent problems, both corrected in this one edit since both
are Task 2's to fix and both are on the same line:

1. **Fix C staleness**: Saturday is exactly `weekday_number.min(5) ==
   5`'s trigger. Post-fix, `required by end of Saturday` becomes
   `required today`.
2. **Pre-existing, changeset-independent bug**: `est. EOD target
   already met` is not a string `day_total_line` can produce under
   *any* fix, old or new. Reading `src/status.rs:184-188`:
   `Some(EodState::At(t))` renders `, est. EOD {t}`;
   `Some(EodState::TargetAlreadyMet)` renders `, target already met`
   — with no `est. EOD` prefix at all. The two branches are mutually
   exclusive (one `match` arm each) and neither one is capable of
   emitting `est. EOD target already met`; that exact substring has
   never been producible output, in the pre-changeset code or the
   post-changeset code alike. It is not "internally consistent but
   about to go stale" — it was already wrong.

   Which single branch is correct here, derived from the example's
   own numbers: the pace hint reads `04h 35m over 40h 00m`, i.e.
   `gap_minutes` is negative (fulfillment exceeds required by 4h35m).
   `EodState::TargetAlreadyMet` is defined as firing exactly when
   `gap <= 0` (`src/status.rs:51`, doc comment on the variant) — this
   example is already past target, so `TargetAlreadyMet` is the
   correct branch, not `EodState::At`. Corrected clause: `, target
   already met` (no `est. EOD` prefix).

Corrected line, replacing `README.md:198` verbatim:

```
Day total:     07h 25m (+ ongoing), 04h 35m over 40h 00m required today, target already met
```

No other part of the line changes — `(+ ongoing)`, the `07h 25m` day
total, the `04h 35m over 40h 00m` pace-hint figures, and the
surrounding week line (`README.md:199`, unaffected by either fix)
stay byte-identical. The three stint lines and header above/below this
line are untouched.

## Edit 6 — README.md "Known issues": drop the same 4 bullets

Current state (`README.md:47-65`), same 8-bullet order as SPEC.md's
§1.2a (this is deliberate per NOTES.md's original triage — the two
lists mirror each other bullet-for-bullet):

1. splice-gate disclosure (KEEP)
2. anomaly remedy guidance (KEEP)
3. lone-unclosed-start ambiguity (KEEP)
4. multi-day-old forgotten stop (KEEP)
5. closed-period undocumented sentence shape (DROP)
6. `est. EOD HH:MM` tomorrow, no date shown (DROP)
7. last-weekday day-total/week-total redundancy (DROP)
8. `"Total still owed"` punitive tone (DROP)

Delete bullets 5-8 verbatim (`README.md:57-65`, from `- \`status\`/
\`week\`'s output for a past (closed) date/week uses a...` through
`...simply ended under target`). Bullets 1-4 (`README.md:47-56`) stay
byte-identical.

Resulting section:

```markdown
### Known issues

Shipped behavior that's rough or confusing in a way worth fixing
later:

- Whether a stint reaching into the next day auto-resolves or gets
  left flagged depends on an internal rule the output doesn't explain
- `[!]` anomaly flags describe the problem but not how to fix it
- Whether a lone unclosed `start` gets flagged depends on whether a
  *second* one also exists that date — the difference is just how
  many, but it isn't explained anywhere
- A forgotten `stop` from several days ago is invisible everywhere
  except a `status` query against the exact date it started — no
  warning on today's `status`, on any date in between, or in `week`'s
  per-day table
```

## Verification plan

Cannot run `cargo build`/`cargo test`/the binary for these edits —
this task touches no Rust and the harness is docs-only. Verification
here is necessarily string-derivation, not execution, done two ways:

1. **Internal**: every new/changed line's own arithmetic is checked
   by hand above (Edit 4's per-day sum / carry-in / fulfillment /
   owed chain; Edit 5's `gap <= 0` branch selection) — done in this
   plan, to be re-checked by whoever executes it before committing.
2. **Cross-reference against source**: every string shape claimed
   above is traced to the specific `render.rs`/`status.rs` line that
   produces it (cited inline in Edits 4 and 5), not just paraphrased
   from the spec's prose. This substitutes for running the binary,
   which isn't possible yet since Task 1 hasn't landed the code that
   makes these branches reachable with the new wording.

**Documented risk / open dependency**: this plan's string derivations
are Task 2's own reading of the *current* `render.rs`/`status.rs` (the
pre-Task-1 code) plus the changeset plan's pinned interface contract —
they are not yet checked against Task 1's actual landed diff, because
Task 1 is being written concurrently and has no completion report yet.
Before this task's edits are considered done, re-verify every quoted
string in Edits 2-5 against one of, in order of preference:

1. Task 1's completion report, if it quotes exact rendered strings
   (not just describes them — per the changeset plan's own risk note,
   a report that only describes the change isn't sufficient).
2. Task 1's actual test assertions in `src/render.rs`/`src/status.rs`
   (e.g. the literal strings in `#[test]` functions once Task 1 has
   updated them).
3. The built binary, run against the worked examples' own inputs.

If any of those three surfaces a mismatch against this plan (e.g. a
different `(tomorrow)` placement, a different capped-week trigger, or
`format_minutes` rendering differently than assumed), the affected
edit above is wrong and needs correcting before landing — this plan's
derivation is a best-effort stand-in for that check, not a replacement
for it.

## Risks, ambiguities, and disagreement

- **No disagreement with the changeset plan or spec.** Both are
  locked, already through one adversarial round each, and this task's
  scope (doc strings only) has no technical decision left in it that
  the interface contract doesn't already settle.
- **Line-number drift risk (flagged, not a problem)**: every line
  number cited above (SPEC.md's 8 hits, README's 1 `required by end
  of` hit) was grepped fresh at plan-writing time, per both the
  changeset plan's and spec's own instruction not to trust static line
  numbers. Re-grep immediately before executing each edit, since any
  other change landing on `main` between now and execution (unlikely,
  since these are the only two files under active edit in this
  changeset, but not impossible) would shift them.
- **The Edit 5 "already-wrong" finding is a pre-existing bug, not
  caused by this changeset** — flagging this explicitly so it isn't
  mistaken for scope creep: `est. EOD target already met` predates all
  four fixes here (confirmed by reading `day_total_line`'s `match`
  arms, which have never had a branch producing that concatenation).
  Task 2 fixes it because it's already editing this exact line for
  Fix C and leaving a visibly-impossible string in a shipped README
  would be worse than the one-line fix, but it is not itself one of
  the four numbered fixes (A/B/C/D) and isn't claimed as such anywhere
  in this plan or in commit messages that follow it.
- **New-example week choice (`2026-35`) is this plan's own invention,
  not dictated by the spec.** The spec only requires "one new worked
  `status` example (closed date) and one new worked `week` example
  (closed week)" with the right shape; the specific fictional week id
  and figures are free choices made here for internal consistency with
  each other and with the existing `2026-37`-based examples. If Task 1
  or the coordinator would prefer these match some other existing
  fixture (e.g. reusing SPEC.md's own `2026-02`/`2026-06` closed-week
  numbers instead of inventing a new `2026-35`), that's a trivial
  substitution against this same plan structure, not a design change.
- **SPEC.md Edit 3's placement (folded into existing bullets vs. new
  bullets) is a judgment call.** The spec says these cases must be
  "documented" without specifying bullet structure. This plan folds
  both additions into their existing parent bullet (day-total pace
  hint bullet gains the `required today` sentence; Estimated-EOD
  bullet gains the `(tomorrow)` sentence) rather than adding two new
  top-level bullets, on the grounds that each addition is a variant of
  the behavior its parent bullet already describes, not a separate
  concern — consistent with how the existing bullet list already
  handles sub-cases (e.g. the stint-line bullet's own nested
  sub-bullets for spliced/open-stint variants). If reviewed and
  rejected, the alternative (two new sibling bullets) is a mechanical
  restructure, not a content change.
