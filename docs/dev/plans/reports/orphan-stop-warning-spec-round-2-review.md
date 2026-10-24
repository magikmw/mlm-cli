# orphan-stop-warning — spec review, round 2

Reviewed `docs/dev/specs/2026-10-24-orphan-stop-warning.md` at `1b5f29f`
against `src/stint.rs`, `src/lookback.rs`, `src/anomalies.rs`,
`src/commands.rs`, `src/render.rs`, `src/storage.rs`, `src/date.rs`,
`docs/dev/SPEC.md` (§1.2a, §3, §4.1-§4.3.1, §6, §7.1, §7.3, §7.4, §8),
`README.md`, the round-1 report and NOTES.md entries 111-112.

The dispatch named `orphan-stop-warning-spec-review-2.md`. The write
hook accepts only names ending in `-review.md` and refused it, so this
report sits at `orphan-stop-warning-spec-round-2-review.md` instead.

The pairing claims below were checked by experiment. A dependency-free
port of `classify`, `close_group`, `splice_candidate` and `classify_at`
(`src/stint.rs:132-354`), with instants as whole minutes, lives in
`/tmp/cd-scratch/osw-review-2/src/main.rs`. Its outputs are quoted where
a finding relies on them. Storage is minute-granular
(`src/storage.rs:207-214`), so "same minute" and "same instant" agree,
and `--date` cannot be in the future (`src/date.rs:161-166`).

## Round-1 findings

| Round 1 | Status | Where |
|---|---|---|
| F1 silent cases | Resolved | spec lines 30, 41-42; repeat rule at 54-55 |
| F2 D+1 un-splice | Resolved | spec lines 75-84 (sibling gap in finding 5 below) |
| F3 no test plan | Resolved, with gaps | spec §7; finding 12 |
| F4 doc list | Mostly resolved | spec §8 and interaction list; finding 11 |
| F5 partial output | Resolved | spec lines 196-197 |
| F6 E27 in §8.2 | Resolved | spec line 244 |
| F7 E8 window | Adopted, rule has holes | spec §5; findings 1, 3, 4, 13 |
| U1 backfill | Resolved | spec lines 50-53 (variant in finding 10) |
| U2 refused join | Partly resolved | consequence 2 fixed (spec 133-136); consequence 1 not recorded, finding 7 |
| U3 long splice | Resolved | spec §3.2 |
| U4 other anomalies | Resolved | spec lines 43-44 |
| U5 backdated lookback | Resolved | spec lines 56-58, 137-138 |
| U6 stderr | Resolved | spec lines 59-60 |

## Verified correct

- The FE5 example (spec 142-153) holds. Sat 2026-10-03 is age 2, Sunday
  is empty, so there is no splice and no refused join
  (`src/stint.rs:399-401` needs an open start on the earlier date). The
  stale line matches `stderr_line_with` (`src/lookback.rs:121-126`).
- The long-splice example (spec 158-166) holds. `start 09:00 --date -1`
  is silent because `emit_warnings` drops the written id
  (`src/commands.rs:78-82`). The splice credits the stint to Sunday
  (`src/stint.rs:315-327`), 09:00 to 14:16 next day is 1756 min, which
  formats as `29h 16m`. Sunday has no open start afterwards, so no
  lookback line follows.
- The §3.1 D+1 example (spec 82-84) holds. `stop 23:00 --date -1` drops
  yesterday's open count to 0, so the gate fails
  (`src/stint.rs:347`) and today's 01:00 is a plain orphan with no
  refused join.
- The §4.3 filter claim (spec 132-136) matches `today_page_lines`
  (`src/lookback.rs:299-311`). The scan already uses the splice
  (`src/lookback.rs:272`), so the filter removes exactly the refused-join
  yesterday line (`src/lookback.rs:217`).
- "Orphans on D" and "a long splice" cannot both fire. The splice needs
  the new end to be D's only raw orphan (`src/stint.rs:347`), and the
  splice then removes it (`src/stint.rs:309-311`). Only D+1 orphans can
  print next to a long-splice block.
- §5's claim that a start at an orphaned P's minute pairs with P at
  zero length holds (`src/stint.rs:215-243`). Step 1 leaves P unclosed
  and step 3 pops the new start.

## Findings

### 1. Medium — Missing edge case / factual error: P can be a start, and the window then produces an overlap

- Claim: spec lines 175-188 assume P is an end. The minute rule (182-187)
  covers only "an end that closed a stint", "an orphaned end" and "the
  spliced first end". Line 188 drops punches at T's minute from the
  search.
- Evidence: an orphan T can share its minute with an end that closed a
  stint. Ends in one group pop in id order (`src/stint.rs:215-224`).
  Take `start 09:00`, `stop 17:00`, `stop 17:00` — a double-tapped bare
  `mlm stop`, the likeliest way to make an orphan by accident. The
  first 17:00 pops 09:00 and the second is orphaned. The 17:00 end is
  excluded by line 188, so P is the 09:00 start and the hint reads
  `(a time between 09:00 and 17:00)`.
- Experiment A: adding `start 10:00` gives `10:00-17:00` and
  `09:00-17:00`, an overlap (E21). No start in the window pairs with T
  only.
- Two orphans at the same minute fail the same way. This happens on a
  double-tapped bare `stop` after a splice. Each gets `(a time before
  01:00)`. Experiment E: following both hints with `00:20` and `00:30`
  nests them, `00:30-01:00` and `00:20-01:00`, another overlap.
- P is a start exactly when an end at T's minute closed it. Otherwise
  the stack is empty at T, so every earlier start was popped before T
  and the latest punch in pairing order is an end.
- Fix: define the case. One option is to drop the `fix:` line, or say
  so, when another end shares T's minute. The `or:` delete line is the
  right fix for a duplicate. State it in §5 and add both cases to §7.

### 2. Medium — Factual error: the long-splice `then:` hint needs a window

- Claim: spec lines 116-118 say nothing on D-1 follows S, "because the
  splice needs S to be D-1's only open start".
- Evidence: the gate checks the open count only (`src/stint.rs:347`).
  One open start does not mean no later punches. D-1 with `start 09:00`,
  `start 10:00`, `stop 11:00` leaves 09:00 as the only open start, and
  today's first-punch end splices with it (experiment C).
- Following the block (delete today's end, then `stop 10:30 --date -1`,
  "a time after 09:00") gives `10:00-10:30` and `09:00-11:00`, an
  overlap (experiment C). The stop-hint rule (`docs/dev/SPEC.md:1178-1189`)
  already handles this with N, the first punch after S, and would print
  `(a time between 09:00 and 10:00)`. A stop at 09:30 pairs cleanly.
- Fix: build `then:` with `render::stop_time_hint(S, next_punch_after)`
  (`src/render.rs:160-178`) and add `mlm stop`'s long-splice block to
  the stop-hint rule's consumer list.

### 3. Low — Internal inconsistency / testing gap: the "spliced first end" case cannot occur

- Claim: spec lines 185-187 treat "the date's spliced first end" as a
  possible P. Spec line 220 asks for a test of it.
- Evidence: a spliced first end is a raw orphan. If T is also an orphan,
  the raw date has two, and the gate refuses
  (`src/stint.rs:347`, `later.orphaned_ends.len() != 1`). Experiment F
  (`22:00` yesterday, `01:00` and `17:00` today) leaves both 01:00 and
  17:00 orphaned and yesterday's start open. P is then an ordinary
  orphan.
- Fix: drop the case from §5 and the test bullet, or replace it with
  "first end under a refused join", which is the real shape.

### 4. Low — Factual error: the included-minute reasoning is wrong when P closed a same-instant pair

- Claim: spec lines 182-184 say a start at P's minute "is pushed after
  the end pops, so it pairs with T".
- Evidence: when P closed a start from its own group (E14), P pops in
  step 3, after the group's starts are pushed (`src/stint.rs:226-243`).
  A new start at P's minute is pushed last and P pops it. Experiment B
  (`start 12:00`, `stop 12:00`, orphan `17:00`, new `start 12:00`) gives
  `12:00-12:00` from the new start and `12:00-17:00` from the old one.
- The result is clean, since T pairs and no overlap is flagged
  (zero-length spans drop, `src/anomalies.rs:142`). But the stated
  mechanism is false, and a test that asserts "the new start pairs with
  T" fails.
- Fix: phrase the rule as "T is paired and nothing overlaps", or note
  the E14 case.

### 5. Medium — Missing edge case: a backdated `stop` that clears an E7 can create a long splice with D+1, silently

- Claim: spec lines 92-95 limit the long-splice check to the end this
  `stop` wrote.
- Evidence: a `stop` on D that closes one of D's two open starts drops
  D's open count to 1, so D's start now splices with D+1's first-punch
  orphan (`src/stint.rs:315-327`). The D+1 orphan disappears, so §3.1
  is silent too.
- Experiment D: D has `start 09:00`, `start 22:00`. D+1 has `stop
  01:00` (E7 and a refused join). The E7 hint says `(a time after 22:00
  closes the 22:00 start)` (`src/anomalies.rs:240-246`). Following it,
  `mlm stop 23:00 --date D` closes 22:00, and 09:00 splices to D+1's
  01:00, a 16h stint. Nothing prints. This is U3's symptom, reached by
  following mlm's own hint.
- Fix: run the 12h check on any splice into D+1 that this write
  created, or record it in §1.2.

### 6. Medium — Undeclared user-visible gap: the yesterday hint's `or: mlm stop` leads into the long-splice warning

- Decision: warn on any splice of 12h or more (spec §3.2).
- Consequence: `status`'s yesterday hint offers `or: mlm stop  (still
  working: ends it now, across midnight)` (`src/lookback.rs:183-185`,
  `docs/dev/SPEC.md:1301`). It prints only when today has no punches,
  which is exactly when a bare `stop` splices. Once the start is 12h
  old, the command mlm suggests answers with a warning whose `fix:`
  says to delete the punch it just made.
- README's own example hits it. The `[i]` hint at `README.md:284-288`
  sits next to a `22:00-now (15h 30m so far ...)` line, so following
  its `or:` line prints a 15h 30m long-splice block. The same holds for
  `docs/dev/SPEC.md:945-949`.
- After the long-splice `fix:` step (delete today's end), today has no
  punches again, so the hint returns with the same `or: mlm stop`.
- The §1.2 bullet at spec lines 61-62 says a genuine 12h+ session warns.
  It does not say that mlm's own hint sends the user there.
- Candidate for §1.2. The hint could also drop or reword its `or:` line
  past 12h.

### 7. Medium — Undeclared user-visible gap: a `stop` that breaks yesterday's splice does not say the overnight stint came apart (round-1 U2, consequence 1)

- Decision: `stop` prints orphan blocks and filtered lookback lines, and
  no refused-join line (spec §4.1, §4.3).
- Consequence: yesterday `start 22:00`, today `stop 01:00`, spliced.
  The user runs `mlm stop 08:59`, or `mlm stop 00:30` (experiment G).
  Today now has two orphans, the join is refused, yesterday's start
  reopens, and yesterday loses the overnight minutes. `stop` prints two
  orphan blocks. The yesterday line is filtered (spec 133-136), and no
  line says the join was refused or that yesterday's stint is now
  uncounted. Today's `status` shows the refused-join line
  (`src/anomalies.rs:288-297`).
- NOTES.md entry 112 folds U2 as "lookback uses today's status filter",
  which covers consequence 2 only. The non-goals (spec 43-44) list
  overlap and E7 but not a refused join.
- Candidate for §1.2 or the non-goals.

### 8. Low — Undeclared user-visible gap: orphan blocks do not separate new orphans from old ones, or name the punch that caused them

- Decision: one block per orphaned end on D and D+1, pre-existing ones
  included (spec 76-86).
- Consequence: a displaced end, a broken D+1 splice and an orphan from
  last week all print the same three lines. In the D+1 case
  (spec 82-84) the block names today's 01:00. Its `fix:` asks for a
  start before 01:00 and its `or:` asks to delete 01:00. Neither names
  the `23:00 --date -1` the user just typed, which is the punch to
  delete if it was the mistake.
- §1.2 records the repeat (spec 54-55) but not that the user cannot
  tell which block their own command caused.
- Candidate for §1.2.

### 9. Low — Undeclared user-visible gap: the long-splice fix deletes the end the user typed

- Decision: `fix:` deletes D's end, `then:` stops D-1's start (spec
  120-124).
- Consequence: a user who forgot yesterday's stop and today's start
  really did end work at 14:16 today. The block's path deletes that end,
  and the user must retype it with a new start. A path that keeps it
  exists. `mlm stop HH:MM --date D-1` first breaks the splice and
  leaves 14:16 orphaned, and `mlm start HH:MM` before 14:16 then fixes
  today. The block offers only the destructive order, and does not say
  what to do about today's own work.
- Candidate for §1.2, or a line under the block.

### 10. Low — Undeclared user-visible gap: backfilling stop-then-start can also print the long-splice block

- Claim: the backfill known issue (spec 50-53) names only the orphan
  warning.
- Evidence: when D-1 has one open start (for example a stale one), a
  backfilled `mlm stop 17:00 --date -1` is D's first punch and only
  orphan, so it splices into a long stint (`src/stint.rs:309-311`).
  The long-splice block prints, and its `fix:` says to delete a correct
  punch. The following `start 09:00 --date -1` clears it.
- Fix: widen the §1.2 bullet to cover both blocks.

### 11. Low — Factual gap: live text still describes the old behavior and is not on the doc list

- Spec §8 and the interaction list (spec 223-247) miss these:
  - `docs/dev/SPEC.md:867-868` says an old start drops "out of `mlm
    start`'s warnings". `stop`'s lookback drops it too.
  - `docs/dev/SPEC.md:875-876`, `docs/dev/SPEC.md:1566` (E22) and
    `README.md:269` say a `stop` as today's first punch "joins the start
    and clears the hint". With §3.2 that `stop` also warns at 12h+.
  - `README.md:43-48` "Possible issues" says "None currently". It is
    the README mirror of SPEC §1.2a (NOTES.md entry 109), and the spec
    adds five known issues to §1.2a only (spec 245).
- Spec line 229 says "any E8 example gains the window". README has no
  E8 example. The only one is `docs/dev/SPEC.md:1228-1233`, already
  covered by spec line 240.

### 12. Low — Testing gap: new paths without a named test

- Spec §7 (lines 202-221) does not name:
  - the long-splice `then:` window when D-1 has a punch after S
    (finding 2)
  - a D+1 orphan and a long-splice block from one `stop`, the only pair
    of kinds that can print together, which pins the order in spec
    100-101
  - the window in `stop`'s orphan block. Spec line 219 tests the window
    in `status` only, and the displaced-end bullet (209) does not say it
    checks the hint text.
  - a positive lookback case where yesterday's lone start is not
    refused, so the yesterday line does print
  - 11h 59m, the boundary just under the threshold
  - a duplicate same-minute stop, and two orphans at one minute
    (finding 1)
- Spec line 220's "spliced first end" fixture cannot be built
  (finding 3).

### 13. Low — Missing edge case: the window can be empty, and the hint does not show its edges

- Claim: spec lines 182-187 exclude P's minute when P is an orphan.
- Evidence: with an orphan P at 12:00 and T at 12:01, the hint reads
  `(a time between 12:00 and 12:01)`, and no `HH:MM` fits. The stop-hint
  rule documents its own empty case at 23:59
  (`docs/dev/SPEC.md:1185-1187`). §5 does not.
- The hint text is the same whether P's minute counts or not. A user
  cannot tell that `12:00` works after a closed stint and fails after
  an orphan.
- Fix: document the empty window in §5. The edge wording is a §1.2
  candidate.

## Undeclared user-visible gaps

This category is not empty. The changeset adds stderr output to `stop`
and changes `status`'s E8 text. Findings 6, 7, 8, 9 and 10 are its hits.
Finding 13's second half is a further candidate.

## Verdict

needs-rework

The round-1 folds hold, and the examples check out against the code.
The new hints are where the spec breaks. The E8 window misleads on a
double-tapped `stop` (finding 1). The long-splice `then:` line skips the
stop-hint window (finding 2). Both print wrong advice at the moment the
user acts on it, and both need spec text before the plan. Findings 3, 4
and 13 tighten §5's wording. Finding 5 is a real silent long splice.
Findings 6-10 are §1.2 candidates for the user to accept or reject.
