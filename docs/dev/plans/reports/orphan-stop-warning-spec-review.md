# orphan-stop-warning — spec review

Reviewed `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (branch
`orphan-stop-warning`, baseline `4c451d7` confirmed as `main`'s head)
against `src/commands.rs`, `src/lookback.rs`, `src/anomalies.rs`,
`src/stint.rs`, `docs/dev/SPEC.md` §3.3/§4.3/§4.3.1/§6/§7.3/§7.4/§8,
`README.md` and `docs/dev/NOTES.md` entries 108 and 111.

## Verified correct

- `stop` today discards the punch id and prints nothing
  (`src/commands.rs:39-42`). `punch()` already returns `(date, id)`
  (`src/commands.rs:315-342`), so §3 needs no new plumbing for the date.
- The `fix:`/`or:` text in spec §4 (lines 74-75) matches `e8_lines`
  word for word (`src/anomalies.rs:278-283`), which always prints
  `--date D`, today included.
- The FE5 example (spec lines 86-95) is right. Mon 2026-10-05 minus 2
  is Sat 2026-10-03. Sunday has no open start, so there is no splice and
  no refused join. The `start` that wrote the punch is silent because
  `emit_warnings` suppresses the written id (`src/commands.rs:78-82`).
  The stale line matches `stderr_line_with` (`src/lookback.rs:121-126`).
- The displaced-end example (spec lines 54-56) matches LIFO pairing
  (`src/stint.rs:205-245`). 12:00 pops 09:00 and 17:00 is orphaned.
- Spec line 81 holds: an end never lands in `LookbackKind`
  (`src/lookback.rs:273-283`).
- Spec line 110 holds: `start --help` says nothing about warnings
  (`src/cli.rs:62-64`, no `warn` anywhere in `cli.rs` help text).

## Findings

### F1 — Medium — Internal inconsistency: "writes no orphan prints nothing" contradicts §3

- Claim: spec line 27 (table) says a `stop` that "pairs with a start" is
  silent. Spec lines 36-37 (non-goal) say "A `stop` that writes no orphan
  prints nothing".
- Evidence: spec §3 lines 54-56 make a `stop 12:00` that pairs with
  `09:00` warn about the displaced `17:00`. Lines 57-59 make every `stop`
  to a date with an older orphan warn, including a `stop` that pairs
  cleanly. Both are stops that pair and write no orphan of their own, and
  both print.
- Fix: phrase the table row and the non-goal in terms of the date's
  state after the write ("a `stop` whose date holds no orphaned end
  afterwards prints nothing"). Add a table row for the
  displaced/pre-existing case.
- Related point: NOTES.md entry 111 (`docs/dev/NOTES.md:1549-1554`)
  records "warn on any orphaned end" but not that a pre-existing orphan
  repeats on every `stop`. Spec line 6 cites entry 111 as its decision
  source, so the repeat rule has no recorded decision behind it. Either
  log the decision or mark it as the spec's own choice.

### F2 — Medium — Missing edge case: a `stop` can orphan the *next* date's end, and the written-date rule never sees it

- Claim: spec lines 50-52 and 62 limit the check to the written date.
- Evidence: the splice gate reads the earlier date's raw open count
  (`src/stint.rs:309-311`, `splice_candidate` at `src/stint.rs:342-354`).
  Take yesterday `start 22:00` and today `stop 01:00`, which splice
  cleanly. Then run `mlm stop 23:00 --date -1`. Yesterday now pairs
  22:00-23:00 and has an open count of 0, so the splice drops and today's
  01:00 becomes an orphan. The written date (yesterday) has no orphan, so
  `stop` stays silent. The new orphan shows up only on today's `status`,
  which is exactly the silent-orphan symptom this changeset sets out to
  fix.
- The only date a `stop` on D can newly orphan besides D is D+1, and
  only by closing D's single open start.
- Fix: also classify D+1 (`classify_at(D, D+1, D+2)`) and warn for its
  orphans. Or record this as a known issue in §1.2, with the example.

### F3 — Medium — Testing gap: no testing section

- Claim: the spec has no test plan (headings at spec lines 8-113).
- Evidence: the four preceding specs all have one
  (`2026-10-03-live-eod.md` §5, `2026-10-03-point-in-time-week.md` §7,
  `2026-10-03-possible-issues-triage.md` §7,
  `2026-10-04-overnight-visibility.md` §8). The existing `start` warning
  tests show the shape a test plan would take
  (`src/commands.rs:1769-2046`).
- Cases a plan should name:
  - stop orphan, empty lookback → orphan block only
  - FE5 → orphan block, then stale block
  - clean pair → silent, even with a stale start in the window
  - splice with yesterday → silent
  - displaced end → warns for the end the user did not type
  - pre-existing orphan + clean pair → warns
  - two orphans on one date → two blocks, ascending
  - `--date` older than 14 days → orphan block, lookback from today
  - read failure after write → nothing extra, exit 0
  - stdout empty, exit 0 in every case
  - refused-join yesterday (see U2)

### F4 — Medium — Factual / doc-surface gap: other live text claiming `stop` is silent is not on the update list

- Claim: spec §6 (lines 108-109) updates README only under `mlm stop`.
  The interaction list (lines 115-120) names only §7.4, §3.3 and §8.
- Evidence of other statements that become false:
  - SPEC §6.2 (`docs/dev/SPEC.md:644-646`) says an orphaned `end` "shows
    up in that date's next `status`/`week` output, not at insert time."
  - SPEC §3.7 (`docs/dev/SPEC.md:356`) says "`start`, `stop` and `note`
    stay silent (§7.4)."
  - SPEC §7.3's stop-hint rule (`docs/dev/SPEC.md:1187-1189`) lists
    "`mlm start`'s warnings" as a consumer. `stop`'s lookback lines are
    now a consumer too.
  - README (`README.md:115-116`) says "Every `start`/`stop`/`note` call
    is silent on success". This was already stale for `start`, and this
    change makes it stale for `stop` as well.
  - AGENTS.md (`AGENTS.md:43-44`) and the `lookback.rs` module and
    function docs (`src/lookback.rs:1-5`, `:107-110`, `:249-250`)
    describe the stderr forms as `mlm start`'s alone.
- Fix: add all of these to §6 and the interaction section.

### F5 — Low — Error handling: partial-output semantics unspecified

- Claim: spec lines 102-104 say "If any read after the committed write
  fails, `stop` prints nothing extra".
- Evidence: `start` makes one read, then prints (`src/commands.rs:56-59`,
  `src/lookback.rs:251-257`). `stop` needs the written date and its
  previous date, plus the lookback range `[today-15, today]`
  (`src/lookback.rs:227-231`). When `--date` falls outside that range,
  these are two reads. The spec does not say whether the orphan block
  may print when the lookback read then fails. "Nothing extra" taken
  literally requires finishing every read before printing anything.
- Fix: state it either way. "All reads first, then print" is the
  literal reading. "The orphan block prints, and the lookback lines are
  skipped on failure" is the looser one.

### F6 — Low — Internal inconsistency: the FE5 flow goes in §8.1 "happy paths"

- Claim: spec lines 119-120 put a "new happy-path flow" for FE5 in §8.1.
- Evidence: `start`'s equivalent warning flow is E19 in §8.2
  (`docs/dev/SPEC.md:1541-1549`). Every anomaly flow there (E7, E8, E16,
  E18) also sits in §8.2.
- Fix: add it as E27 in §8.2.

### F7 — Low — Missing edge case (inherited): the E8 `fix:` hint is wrong for a displaced or mid-day orphan, and this spec now prints it on the write path

- Claim: spec lines 69-75 copy `status`'s E8 lines word for word.
- Evidence: `(a time before T)` (`src/anomalies.rs:279`) has no window.
  This differs from the stop-hint rule (`docs/dev/SPEC.md:1178-1189`).
  In the spec's own displaced example (start 09:00, stop 12:00, orphan
  17:00), `mlm start 10:00` satisfies "before 17:00". The result pairs
  10:00-12:00 and 09:00-17:00, which overlap (E21). Only a start after
  12:00 gives a clean fix.
- This defect already exists in `status`. The spec prints it at the very
  moment the user acts on it, so it should either list this as a known
  issue or add a start window `(a time between P and T)` mirroring the
  stop-hint rule.

### U1 — Medium — Undeclared user-visible gap: backfilling stop-then-start warns

- Decision: warn on any orphaned end (spec lines 19-20).
- Consequence: SPEC §3.2 allows any entry order
  (`docs/dev/SPEC.md:253-254`). A user backfilling yesterday with
  `mlm stop 17:00 --date -1`, then `mlm start 09:00 --date -1`, gets an
  orphan warning on the first command. Its `or:` line suggests deleting
  a punch that is correct.
- NOTES.md entry 111 records this cost ("one needless warning when
  backfilling stop-then-start", `docs/dev/NOTES.md:1552-1553`). The spec
  never mentions it.
- Candidate for §1.2.

### U2 — Medium — Undeclared user-visible gap: refused-join context is dropped, and the yesterday line diverges from `status`

- Decision: the warning carries the E8 `fix:` lines plus the raw
  lookback (spec lines 20-22, 78-82).
- Consequence 1: `status` for that date also prints the refused-join
  line (`src/anomalies.rs:288-297`) that explains why the end did not
  join. `stop` omits it, so the user is not told why the end failed to
  join.
- Consequence 2: `lookback()` (`src/lookback.rs:251-257`) does not drop
  yesterday's single start under a refused join, but `status`'s
  `today_page_lines` does (`src/lookback.rs:299-311` via
  `overnight_stint`, `:217`). Here is a concrete case. Yesterday has
  `start 22:00`. Today has `stop 01:00` (spliced). The user taps
  `mlm stop` a second time. Today now has two orphans, so the splice
  drops. `stop` prints both orphan blocks and `warning: start on ... at
  22:00 still open (yesterday)` with `fix: mlm stop HH:MM --date ...`.
  Today's `status` instead shows the refused-join line and no
  yesterday hint.
- Following `stop`'s yesterday fix line pairs 22:00 with a same-day stop
  and loses the overnight time. The right fix is deleting the extra end.
- Candidate for §1.2, or a reason to filter the lookback like
  `today_page_lines` does.

### U3 — Medium — Undeclared user-visible gap: the age-1 sibling of FE5 splices silently into a long stint

- Decision: an end that splices with the previous date's open start
  draws no warning (spec lines 27, 60-61).
- Consequence: a start forgotten yesterday at 09:00, followed by a bare
  `mlm stop` today at 14:16 as today's first punch, splices into a
  29-hour stint credited to yesterday (`src/stint.rs:315-327`). No
  warning prints. NOTES.md entry 111 rejected option C because it
  "would credit a multi-day stint" (`docs/dev/NOTES.md:1548-1549`). The
  splice does the same thing one day earlier, and stays silent.
- Candidate for §1.2.

### U4 — Low — Undeclared user-visible gap: other anomalies a `stop` creates stay silent

- Decision: only orphaned ends trigger (spec line 52). Non-goals cover
  only "stale starts in the lookback" (lines 36-37).
- Consequence: a `stop` can make an overlap (E21) or leave the wrong
  start open on an E7 date without any warning. Example: start 09:00,
  start 10:00, stop 11:00, stop 12:00 gives 09:00-12:00 and 10:00-11:00.
  The lookback never covers today (ages 1-14, `src/lookback.rs:17-18`),
  so today's E7 or overlap is invisible to `stop`'s output even when an
  orphan warning does fire.
- Candidate for §1.2 or the non-goals.

### U5 — Low — Undeclared user-visible gap: backdated `stop` gets today's lookback and no carry-in marker

- Decision: the lookback runs from today (spec line 80).
- Consequence: `mlm stop 17:00 --date 2026-08-01` that orphans prints
  stale starts near today, unrelated to August. A stale start beside
  the written date, outside the window, does not show. The spec also
  never says that `stop` prints no carry-in marker. SPEC states this for
  `start` (`docs/dev/SPEC.md:1382-1383`).
- Fix: state both explicitly.

### U6 — Low — Undeclared user-visible gap: `stop` now writes to stderr

- Decision: warnings go to stderr, exit stays 0 (spec lines 66-67).
- Consequence: scripts or shell aliases that run `mlm stop` and treat any
  stderr as failure, or merge `2>&1` into a log, now see output where
  there was none (`src/commands.rs:39-42`). The change has no opt-out
  (no `--quiet`).
- Candidate for §1.2.

## Verdict

needs-rework

The design is sound and its claims about the code mostly hold. Before
locking, fix the contradiction between the non-goal/table and §3 (F1).
Decide or record the D+1 un-splice case (F2). Add a testing section
(F3). Complete the doc-update list (F4). U1-U3 are §1.2 candidates.
