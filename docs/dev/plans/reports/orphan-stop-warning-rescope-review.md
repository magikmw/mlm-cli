# orphan-stop-warning rescope — spec review

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.

Reviewed `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (the rescoped
draft, NOTES.md entry 117) against branch `orphan-stop-warning`. The
branch has no source changes over `main` (`git diff main..HEAD -- src
tests README.md docs/dev/SPEC.md` is empty), and `main` is at the stated
baseline `4c451d7`.

Claims checked and found correct:

- `stop` prints nothing and exits 0 today (`src/commands.rs:39-42`).
- Status E8 lines are the two hint lines quoted in spec §4, with the same
  indentation (`src/anomalies.rs:272-287`, SPEC §7.3 lines 1227-1233).
- Orphans come out of `classify` in ascending sorted order
  (`src/stint.rs:144`, `:240-243`), so "ascending by instant" is free.
- The splice removes D's first-punch orphan when D-1 has exactly one raw
  open start (`src/stint.rs:306-311`, `:343-354`), so "splices with
  yesterday's open start → silent" holds.
- Status builds D's facts from the `D-1/D/D+1` trio
  (`src/status.rs:472-482`), so "exactly as D's status page does" names a
  real, reusable path (`anomalies::classify_context` + `build_facts`).
- FE5 example: 2026-10-05 is a Monday; `mlm start 22:00 --date -2` is
  silent because `emit_warnings` suppresses the just-written start
  (`src/commands.rs:72-85`); today's `status` shows the D-2 start through
  the lookback (`src/lookback.rs:251-258`).
- `format_date_with_weekday` gives the `Mon 2026-10-05` header form
  (`src/date.rs:175-177`).
- E27 is unused (SPEC §8.2 ends at E26, line 1583).

## Findings

### F1 — factual error — low

- Claim: spec §1 (lines 22-24): "`start` already warns about the open
  starts it leaves behind (SPEC §7.4), so each write command now reports
  the unmatched punch it leaves."
- Evidence: `start` does the opposite for its own punch. `emit_warnings`
  skips a `SingleOpenStart` whose id is the punch just written
  (`src/commands.rs:76-82`); SPEC §7.4 lines 1344-1348 says the same.
  `start` warns about *other* open starts in the 14-day window. Also
  `note` and `delete` are write commands and report nothing of this kind.
- Fix: say "`start` already warns on stderr about open starts in its
  lookback (SPEC §7.4); `stop` now warns about orphaned ends on its
  date." This sentence is likely to be folded into SPEC §7.4, so the
  wrong parallel would spread.

### F2 — internal inconsistency — low

- Claim: spec §1 line 24 says `stop` reports "the unmatched punch it
  leaves". Spec §3 lines 50-52 says it reports every orphan on D,
  including one "already on D before the write".
- Evidence: the two statements describe different triggers. §3 is the
  rule; the tests (§6 line 98-99, "pre-existing orphan, then a clean pair
  → block for the old orphan") follow §3.
- Fix: reword §1 to "reports the orphaned ends on the date it wrote to".

### F3 — error-handling gap — medium

- Claim: spec §5 lines 79-81: "If the read after the committed write
  fails, `stop` prints nothing extra."
- Evidence: the check needs three dates (D-1, D, D+1) because the splice
  reads D-1 (`src/stint.rs:300-311`). The spec says "the read" but does
  not fix the read shape. If an implementation reads per date
  (`storage::punches_for_date`, `src/storage.rs:381`) and treats a failed
  D-1 read as empty, D's spliced first end classifies as an orphan and
  `stop` prints a false warning. `lookback` avoids this with one ranged
  read (`src/lookback.rs:245-258`, `storage::punches_in_range`,
  `src/storage.rs:398`). NOTES.md entry 115 shows a "partial read
  failure" test existed in the pre-rescope draft, so the concern was
  known and dropped with the cut.
- Fix: one sentence in §5. "Any failed read suppresses the whole check;
  the check never classifies D with a neighbour missing." Or name one
  ranged read over `[D-1, D+1]`.

### F4 — testing gap — low

- Claim: spec §6 line 102: "read failure after the write → nothing
  extra, punch stored", with exact stderr, empty stdout and exit 0
  (line 90).
- Evidence: the existing best-effort test for `start` injects the
  failure with `DROP TABLE punches` (`src/commands.rs:1974-1989`) and
  calls `warn_stale_starts` directly. That route cannot show "punch
  stored" (the table is gone) and does not exercise stdout or the exit
  code. A binary-level test can do all three by seeding a malformed
  `at_utc` row on D-1 (no CHECK on `at_utc`, `src/db.rs:44-49`), then
  reading D's rows back with raw SQL.
- Fix: name the injection in §6, or split the case into "unit: no
  output on read failure" and "binary: exit 0, punch stored".

### F5 — testing gap — low

- Claim: spec §6 lists eight cases.
- Evidence: none pins the behaviour the spec chooses not to have, so a
  later change can add it silently:
  - refused join on D (D-1 has two open starts, `mlm stop` today): stderr
    holds the E8 block only, with no refused-join line, although
    `status` prints one (`src/anomalies.rs:289-298`, SPEC §7.3 line
    1166);
  - a backdated `stop` on D-1 that closes D-1's open start and so
    un-splices today's first end: silent (spec §1.1 line 33);
  - a `stop` that creates an overlap (`start 9`, `start 10`, `stop 11`,
    `stop 12`): silent.
- Fix: add the three as "silent / E8 only" cases, or state that only the
  listed cases are pinned.

### F6 — doc surface missed — low

- Claim: spec "Interaction with the project spec" (lines 115-129) lists
  the SPEC sentences that state write-time silence.
- Evidence: two more sentences say anomalies surface only later.
  - SPEC §6 lead-in, lines 585-589: anomalies "are accepted, stored, and
    surfaced later in `status`/`week` output instead".
  - SPEC §8.2 E8, lines 1488-1492: the orphaned end "only surfaces as an
    anomaly line" (in `status`). E27 is added next to E19, but E8 itself
    gains no pointer.
- Fix: add both to the interaction list (§6 lead-in names the `stop`
  warning; E8 points to E27).

### U1 — undeclared user-visible gap — medium

- Decision: the warning reuses `status`'s E8 hint lines word for word
  (spec §4 lines 63-64) and checks only D (§1.1 line 33).
- Consequence: in FE5, the motivating case, the warning's `fix:` line
  says `mlm start HH:MM --date <today> (a time before 14:16)`. Following
  it creates a stint today that the user never worked and leaves the D-2
  start stale. The real repair is to delete today's end and stop the D-2
  start. The warning never names the D-2 start. Spec §4 line 75 says
  "`status` shows the stale start", but it does not record that the
  warning's first fix points away from the actual repair.
- Unlike `start`, `stop` runs no lookback (spec §1.1, NOTES.md entry 117
  "the lookback lines after the warning" removed), so the stale start
  that caused the orphan is not mentioned at write time.
- Candidate known issue for spec §1.2.

### U2 — undeclared user-visible gap — low-medium

- Decision: a spliced end draws no warning (spec §3 line 53), and the
  long-splice warning was removed (NOTES.md entry 117).
- Consequence: yesterday `start 09:00`, `stop 12:00`, `start 13:00`
  (forgotten stop), today `mlm stop` at 14:16 as the first punch. The
  splice gate passes (`src/stint.rs:343-354`), a 25h stint lands on
  yesterday, and `stop` prints nothing. SPEC §7.3 lines 1294-1297 already
  says that a later punch yesterday means the start "is probably a
  forgotten stop", which is why the yesterday hint drops its `mlm stop`
  line in this shape. The new warning stays silent in exactly that shape.
- `status` shows the 25h stint, so triage may drop this; it is not
  recorded in the spec.

### U3 — undeclared user-visible gap — low

- Decision: `stop` checks only D (spec §1.1 line 33).
- Consequence: `mlm stop 23:00 --date -1` closes yesterday's open start,
  which had spliced with today's first end at 00:45. Today's 00:45 end
  becomes an orphan and `stop` says nothing, so the command that created
  the orphan is silent while a later `stop` today would warn about it.
  The non-goal is stated but this visible effect is not.

### U4 — undeclared user-visible gap — low

- Decision: the warning prints E8 blocks only (spec §3, §4).
- Consequence: when a join is refused (D-1 has two open starts, or D now
  has two orphans), `status` also prints `not joined to ... across
  midnight` (`src/anomalies.rs:289-298`). The `stop` warning gives only
  "add a start before HH:MM", with nothing tying the end to yesterday.
  A `stop 00:30` that breaks an existing 01:00 splice on D warns about
  both ends, including the 01:00 one that was clean a moment ago, with
  no word on why.

### U5 — undeclared user-visible gap — low

- Decision: the warning covers orphaned ends only.
- Consequence: a `stop` that creates an overlap (E21) or leaves an E7 on
  D prints nothing, while a `stop` that creates an orphan warns. A user
  who learns that "`stop` warns when the data goes wrong" will read
  silence as clean data. The spec never says other anomalies stay
  silent.

### U6 — undeclared user-visible gap — low

- Decision: inserts in any order stay legal (SPEC §1.1 line 19, §3.2
  line 254).
- Consequence: the known issue (spec §1.2 lines 37-40) names only
  backfill with `--date -1`. Same-day reverse entry warns the same way:
  `mlm start 09:00`, `mlm stop 12:00`, then `mlm stop 17:00` before
  `mlm start 13:00` warns about 17:00. So does any first `stop` of the
  day typed before its `start`. The known issue should say "any `stop`
  entered before its `start`", not just backfilling.

### U7 — undeclared user-visible gap — low

- Decision: the header names the date and time only (spec §4 lines
  61-62, 71).
- Consequence: `start`'s warnings carry an age, `(5 days ago)` or
  `(yesterday)` (SPEC §7.4 lines 1353, 1368). The `stop` warning has no
  age, so the two write commands' warnings differ in shape in a way the
  spec does not mention. For today's date the block spells out
  `--date <today>` (it does in `status` too). This is cosmetic but
  visible.

## Verdict

ship-with-followups

F3 needs one sentence before planning. F1, F2 and F6 are wording fixes.
F4 and F5 are test-list edits. U1-U7 are candidates for spec §1.2;
whether to accept them is the coordinator's call under "Triaging review
findings".
