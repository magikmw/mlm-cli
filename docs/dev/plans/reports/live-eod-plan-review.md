# live-eod plan review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Reviewed: docs/dev/plans/live-eod-plan.md against the locked spec
docs/dev/specs/2026-10-03-live-eod.md, src/status.rs, src/stint.rs,
docs/dev/SPEC.md, README.md, AGENTS.md.

Verified clean (no finding):

- Plan item 7 recomputed. Completed 445, required-fulfillment 165
  (status.rs:1106 fixture, SPEC.md:625), live 15 gives gap 150 =
  `02h 30m`, est. EOD 17:45 + 165 = `20:30`. At 18:30, live 45 gives
  `02h 00m`, still 20:30. The pinned 18:00 string (plan:109) is
  byte-correct against the SPEC.md:625 layout.
- Plan item 8 recomputed. Week fulfillment 44h35m vs target 40h gives
  baseline gap -275 (`04h 35m over`, README.md:230). Live is 05h19m =
  319, so gap -594 = `09h 54m over`. Correct. The week line text
  (README.md:231) stays unchanged.
- Spec section 4 docs list and section 5 T1-T10 are all present in
  the plan (SPEC 2.4, 7.1 three edits, 1.2a three edits, 8.1 F9/F9b,
  README four edits; T1-T10 plus fixture and assertion re-check).
- Ownership. `DailyTargetHint`, `gap_minutes`, `EodState` occur only
  in src/status.rs (grep over src/, tests/, examples/; hits at
  status.rs:53,66,71,136,142,188-215,620-634 and test fixtures). No
  consumer in tests/ or examples/. Task 1 (status.rs) and Task 2
  (SPEC.md, README.md) are disjoint and no consumer is missing.
- Minute alignment (spec section 1 stability claim) holds.
  storage.rs:208-212 zeroes seconds on every insert, even for
  `start` with no TIME (commands.rs:326 passes `now.time()`).
- `live` source. stint.rs:172 clamps `minutes_so_far` at 0.
  stint.rs:164-175 builds `open` from the LIFO stack, which is in
  ascending start order, so `.last()` is the most recent start.
  `has_open_stint` is `!open.is_empty()` (stint.rs:113-115), so the
  "no ctx.day open means live 0 and no est. EOD" contract is
  consistent. Complexity threshold is 15 (clippy.toml:9).

## Findings

### F1 (medium) Task 2 leaves stale `now + gap` prose in SPEC.md

Claim. Plan:200-204 and spec section 4 replace only the sentence
"Fulfillment here still excludes the open stint's live minutes", and
edit SPEC 2.4 and 7.1 bullets.

Evidence. Passages that contradict the new behavior and are not
listed in either document:

- SPEC.md:692-694, in the est. EOD bullet itself: "it's `now +
  (required − fulfillment)`, i.e. 'if you keep going from right now,
  this is the clock time you'd close out'". It sits in the bullet
  being edited, but the plan names only one sentence to replace.
- SPEC.md:699: "When the projected `now + gap` lands on a later
  calendar date". Wrong under the new formula, and `gap` now means
  something different.
- SPEC.md:243-246 (2.4 Required-by-day): "Compared against the week's
  fulfillment ... as of now" and "Used for ... estimated-EOD figures".
  Needs a `live` mention.
- SPEC.md:322-324 (3.5): "both depend on 'day total so far' and
  'now'". Already loose, now wrong.
- SPEC.md:655-656 (7.1 day total): "ongoing time isn't folded into it
  live (avoids the total silently changing mid-read)". The plan keeps
  the paragraph verbatim (plan:204), so the sentence stays but sits
  beside a figure that does change mid-read. The paragraph needs a
  one-clause pointer that the pace hint beside it does count live.
- SPEC.md:1044-1046 (F9b) and SPEC.md:225 (2.4, which the plan does
  cover).

The plan's re-grep list (plan:231-232) is `est. EOD`, `excludes the
open`, `04h 35m over`. None of these finds `now +`, `now + gap`,
`mid-read`, `as of now`. In README, `est. EOD` has zero hits (README
uses "estimated-EOD", README.md:267), so the grep gives no signal
there.

Fix. Add to Task 2: rewrite the whole est. EOD bullet, not one
sentence; grep both files for `now +`, `as of now`, `mid-read`,
`estimated-EOD`, `day total so far`.

### F2 (low) Stale code comments outside the one named

Claim. Plan:45-46 names only the doc comment at status.rs:54.

Evidence.

- status.rs:69-70 doc on `gap_minutes`: "Signed: `required_minutes -
  fulfillment` ... NOT `day_total_minutes`". Becomes false (post-live).
- status.rs:2093-2094 test comment "push `now + gap` past midnight".
- status.rs:2106 message "gap must push well past midnight" no longer
  describes what decides `(tomorrow)`.
- resolve doc (status.rs:585-589) is fine.

### F3 (low) "Likeliest to need adjusting" is wrong

Claim. Plan:146-149: the assertions near status.rs:2005 and 2105
"encode `now + gap`" and are the likeliest to need adjusting.

Evidence. Neither encodes `now + gap`.

- status.rs:2006 asserts `gap_minutes <= 0`. Baseline gap is 4 - 10 =
  -6, post-live -546. Still passes.
- status.rs:2106 asserts `gap_minutes > 360`. Baseline 1920 (no
  fulfillment), post-live 1920 - 540 = 1380. Still passes. Est. EOD
  = 09:00 + 1920 min = next day 17:00, so `(tomorrow)` still fires.
- status.rs:2037 (-1520) has no open stint, so live is 0.

No existing resolve-level test pins a figure that changes. The
likeliest-to-break items are the hand-built fixtures (F4). Tell the
implementer the assertions should pass unchanged, so a failure there
signals a bug.

### F4 (low) Other hand-built fixtures carry the old figures

Claim. Plan:144-146 and spec section 5 name two fixtures (1102, 1391).

Evidence. The same 165 / `At(20:45)` pair, or the 02h 45m / 20:45
literal, also appears at status.rs:1115 (assert in t6a), status.rs:1439
(golden string in t11, same test as 1391), status.rs:1496-1503 (t12),
status.rs:1562-1568 (t13). The plan updates "the fixtures near 1102
and 1391" only. If t12/t13 stay at 20:45 they contradict the 17:45
open stint they render. Say "every fixture that pairs a 17:45 open
stint with 165/20:45" and list the four tests.

### F5 (low) Contract item 9 vs Task 2 acceptance disagree on README

Evidence. Plan:117-118 says item 9's wording is carried "into SPEC
1.2a and README". Plan:219-221 gives SPEC 1.2a the full set, but
plan:224-229 gives the README known-issues bullet only the week-line
divergence (matches spec section 4). So README never records the
minute-by-minute change, the `left`-to-`over` flip, backdated-punch
jumps, two open starts, or the overnight gap. Pick one. If README is
meant to carry only one item, change item 9's sentence.

### F6 (medium) The merge gate cannot check the pinned literals, and no test pins them

Claim. Plan:79-81 and 233-234: the coordinator checks quoted strings
against the built binary at merge.

Evidence. The binary reads the real clock (main.rs passes `now` into
`status::run`). The 18:00 Thursday line and the README Saturday 23:04
`09h 54m over` sample cannot be reproduced from a binary run, and
tests/status_cli.rs asserts no pace-hint text. The only fixed-`now`
route is `resolve` + `render` in status.rs tests. Task 1's
T1-T10 do not require reproducing the pinned section 7.1 line or the
README figure through `resolve`. The fixture update (plan:144) only
sets hand-built values, which verifies layout and nothing about the
computation (spec section 5 says so). So `02h 30m ... 20:30` and
`09h 54m over` are quoted in docs with no computing test behind them.
Fix. Add a Task 1 criterion: one resolve-level test reproducing the
exact 18:00 line (Thursday, completed 445, open 17:45, target 2400,
carry as in the SPEC sample), and one reproducing the README Saturday
case. Replace the "built binary" gate with those tests.

### F7 (low) Testing gaps against stated behavior

- T6 asserts `live` only. The risk named in spec 3.2 is that est. EOD
  also uses the later start. Assert est. EOD there.
- Spec section 2 justifies `ctx.day` over `ctx.day_raw` by splice
  (spec:82-83). No test covers a splice case, so a regression to
  `day_raw` goes unnoticed.
- Plan:173-174 requires day total / week line / non-status output to
  be byte-identical, but no listed test checks it with an open stint
  (compare `day_total_minutes`, `week_line`, `week` output against a
  no-live baseline).
- The known issues (backdated stop/start jump, overnight after
  midnight at 00:00) have no test, even a characterization one.
- T9 covers a future start with positive baseline gap only. A future
  start with baseline gap <= 0 should yield `target already met`.
- Friday T10 does not say which of Fri/Sat/Sun and which wording
  (`required today`), though the spec does (`required today`).

## Undeclared user-visible gaps

Deliberate choices whose visible consequence is not recorded in plan
item 9 or spec 1.2. Candidates for the spec's known-issues section.

### U1 est. EOD for a future-dated open start

Spec 3.3 / plan item 5 choose `start + (required - fulfillment)` for a
start later than now, while the hint stays at baseline. A user sees an
`est. EOD` that is more than `gap` ahead of the clock, beside a hint
that is not counting down. Spec 3.3 describes the behavior, but 1.2 /
item 9 do not list it as a visible oddity.

### U2 README sample now shows two unequal overages side by side

Item 8 changes `04h 35m over` to `09h 54m over` and keeps the week line
`04h 35m ahead of target (fulfillment 44h 35m / ...)`. The README's own
flagship sample now shows the hint and the week line's fulfillment
disagreeing in magnitude (same direction). Item 9's divergence example
covers direction only (Fri-Sun, `over` beside `left`). Nothing says the
sample shows divergence on purpose.

### U3 `mlm week` for the current week disagrees with `status`

`mlm week`'s current-week row stays frozen on completed time (non-goal,
spec 1.1) while `status` counts live. Spec 1.2 mentions only the
`status` week line. A user comparing `mlm week` and `mlm status`
mid-stint sees different remaining figures for the same day. Not
recorded.

### U4 est. EOD no longer means "if you keep going from now"

The old meaning (SPEC.md:693-694) is retired. The new figure is the
close time of this open stint. After a break, `stop` then `start`
moves est. EOD later by the break length (new start), but only
because a new stint opens. And a user who extends the stint past est.
EOD sees `target already met` and no figure for "when would I stop if
I wanted X more". The spec records the stability, not the loss of the
"from right now" reading.

### U5 `(tomorrow)` timing changes

est. EOD is now fixed per stint, so `(tomorrow)` appears from the
first `status` after the start (not only once `now + gap` crosses
midnight) and never flips on/off while working. Not in 1.2. Related
visible change. The hint can show `left to` with a `(tomorrow)` EOD
that was previously absent until late evening.

No-visible-consequence test (the non-empty case here): the work alters
printed figures, flip timing, and 00:00 behavior, so this category is
not empty.

## Verdict

ship-with-followups

Fix F1 and F6 before dispatching (both change what Task 1/2 must do).
F2-F5 and F7 are plan edits. U1-U5 go to the spec known-issues list if
accepted.
