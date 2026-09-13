# Milestone 14 — Task 5 low-level plan: `docs/dev/SPEC.md` §7.4 +
`docs/dev/NOTES.md` + `README.md`

**Scope:** Documentation only, three files: `docs/dev/SPEC.md` §7.4,
`docs/dev/NOTES.md` (two decision-log entries — see the "second NOTES.md
edit" finding below, which is *not* the same entry Milestone 13's Task 2
touches), and `README.md`'s command reference. No code. This plan does
not itself edit any of the three files — it specifies each edit
precisely enough to apply without further judgment calls, and a review
step to catch drift between this plan's wording and whatever Task 3
actually shipped.

**Depends on (per the milestone plan):** Task 2 (finalized CLI syntax/
aliases) and Task 3 (finalized output strings) merged. Also — not
explicit in the milestone plan's own Task 5 section, but required by
the milestone's Global Constraints — **Milestone 13 must already be
merged**, including its own Task 2 doc edit to `docs/dev/NOTES.md`
decision 35. As of this writing (2026-09-14), `docs/dev/NOTES.md` lines
209–211 still carry the *pre*-Milestone-13 wording (verified by reading
the file directly — see §2 below), i.e. Milestone 13 has not shipped
yet. **Every line number in this plan for `docs/dev/NOTES.md` is
therefore only valid against the file's current (pre-Milestone-13)
state.** If Milestone 13's Task 2 edit lands first (as the milestone's
Global Constraints require), decision 35's entry grows from 3 lines to
6 (`docs/dev/plans/milestone-13-task-2-spec-doc.md` §7), which shifts
every later decision — including decision 43, this plan's second
NOTES.md edit — down by 3 lines, from 253–256 to 256–259. **Whoever
executes this task must re-locate both entries by their bold headings
("Note content policy" / "Write commands are silent on success") and
decision numbers (35 / 43), not by blindly trusting the line numbers
quoted below, if Milestone 13 has landed by execution time.** The
quoted *text* of decision 43 is unaffected either way (Milestone 13's
edit doesn't touch it) — only its line position may move.

---

## 1. `docs/dev/SPEC.md` §7.4 — exact edit

### Location

Section: `### 7.4 Write-command output`, heading at **line 641**.

**Current text, lines 641–646, quoted verbatim:**

```
### 7.4 Write-command output

`start`, `stop`, `note`, and `week target` print nothing on success —
silent, Unix-conventional, exit `0` (§6.3) is the only signal. A hard
error (§6.1) still prints its message to stderr as usual. Only
`status` and `week` produce stdout output (§7.1, §7.2).
```

### Exact proposed replacement (full section, not a diff)

Replace lines 641–646 in full with:

```
### 7.4 Write-command output

`start`, `stop`, `note`, and `week target` print nothing on success —
silent, Unix-conventional, exit `0` (§6.3) is the only signal. A hard
error (§6.1) still prints its message to stderr as usual. `status` and
`week` produce stdout output (§7.1, §7.2); `delete note`/`delete
punch` are a narrow, deliberate exception to the write-command-silence
rule above rather than a repeal of it — list mode prints that date's
numbered entries (or `nothing to delete for <date>.` when there are
none), and a successful delete prints a ready-to-run recreate command
(`docs/dev/specs/2026-09-13-delete-punches-notes.md` §5).
```

Changes from current text, in order of appearance:
1. `Only \`status\` and \`week\` produce stdout output` → `\`status\`
   and \`week\` produce stdout output` (drops "Only", since it's no
   longer true).
2. New clause appended in the same sentence, naming `delete`'s two
   stdout cases as a narrow exception and cross-referencing the delete
   spec's §5, per the milestone plan's Task 5 acceptance criteria
   ("corrected to also name `delete`'s two stdout cases, as a narrow,
   deliberate exception, cross-referencing the delete spec").
3. Nothing else in the section changes.

---

## 2. `docs/dev/NOTES.md` — two edits, not one

The milestone plan's Task 5 acceptance criteria names only decision 35
("no charset restriction"). Grepping `docs/dev/NOTES.md` broadly for
anything relevant to *this* task's actual subject — stdout/write-command
conventions, not note-body charset — turns up a **second, genuinely
different** decision-log entry that also duplicates SPEC.md §7.4's
soon-to-be-false claim and is not mentioned anywhere in the milestone
plan or the delete spec. Both are covered below; they are unrelated
decisions and there is no risk of double-editing the same lines.

### 2a. Decision 35 ("Note content policy") — NO EDIT for this task

**Current text, lines 209–211, quoted verbatim (as of this writing):**

```
35. **Note content policy**: no length cap, no charset restriction —
    §7's plain-ASCII commitment is about rendered layout characters,
    not what a user can type into a note body.
```

**This is the SAME decision item Milestone 13's Task 2 already fully
specifies an edit for** — see
`docs/dev/plans/milestone-13-task-2-spec-doc.md` Part B (§§6–10),
which quotes these exact same lines 209–211 and specifies replacing
them with:

```
35. **Note content policy**: no length cap, no charset restriction
    beyond embedded `\r`/`\n` being collapsed to a single space rather
    than preserved verbatim (so a stored body is always exactly one
    line, applies to notes written from that point forward only, no
    backfill of rows already stored) — §7's plain-ASCII commitment is
    about rendered layout characters, not what a user can type into a
    note body.
```

**No new edit is proposed here.** Per the milestone plan's own wording
("If Milestone 13's own doc task already updated NOTES.md, this is a
no-op confirmation; if it didn't, this task is the backstop"), Task 5's
job for decision 35 is a **review/confirmation step, not a second,
independently-worded edit**: whoever executes this task should check
whether decision 35 (find it by its "Note content policy" heading, not
assumed line number) already reads exactly as the replacement block
quoted above.
- If yes — Milestone 13's Task 2 landed as planned; nothing to do here,
  note that in the PR description as a confirmation.
- If no (still reads as the "current text" block above, or reads as
  something else entirely) — apply the exact replacement block quoted
  above, verbatim, as the backstop. **Do not invent different wording**
  for this entry; reuse Milestone 13 Task 2's already-reviewed text
  exactly, so the two plans can never end up proposing conflicting
  edits to the same lines.

**Overlap/conflict check (explicitly requested):** this is the one spot
where Milestone 13's Task 2 and Milestone 14's Task 5 touch the exact
same lines. There is no conflict because Milestone 14's Task 5 does not
propose independent wording — it either finds the edit already applied,
or applies the identical, already-specified text as a backstop. Nothing
in this plan overrides or diverges from `milestone-13-task-2-spec-doc.md`
Part B.

### 2b. Decision 43 ("Write commands are silent on success") — NEW EDIT, this task's actual concern

This is a **different** entry from decision 35 — it's the NOTES.md
mirror of SPEC.md §7.4 (§1 above), not of SPEC.md §2.3's charset row.
Milestone 13's Task 2 plan searched NOTES.md only for note-body
content/charset terms (`note body`, `newline`, `\n`, `charset`, `length
cap`, `multi-line`, `single-line` — see that plan's §9) and explicitly
concluded "no other decision... requires an edit" *for that search*;
it never looked for write-command-silence terms, so it correctly never
found or touched decision 43. **This entry is not named anywhere in the
milestone-14 plan's Task 5 section or in the delete spec's §3/§8 Docs
bullets** — it is a gap this review surfaced, not an already-planned
edit, and it should be fixed now rather than left to go stale silently
the same way §1 above was caught.

**Current text, lines 253–256, quoted verbatim (as of this writing —
see the line-number caveat at the top of this plan if Milestone 13 has
landed by execution time):**

```
43. **Write commands are silent on success**: `start`, `stop`, `note`,
    `week target` print nothing; exit `0` is the only success signal
    (new §7.4). Previously unspecified, and two independent milestone
    plans had proposed two different confirmation-line wordings.
```

**Exact proposed replacement (full entry, not a diff):**

```
43. **Write commands are silent on success**: `start`, `stop`, `note`,
    `week target` print nothing; exit `0` is the only success signal
    (§7.4). Previously unspecified, and two independent milestone
    plans had proposed two different confirmation-line wordings.
    `delete note`/`delete punch` are a deliberate, narrow exception
    added by the delete-punches/notes milestone: list mode and a
    successful delete both print to stdout (§7.4,
    `docs/dev/specs/2026-09-13-delete-punches-notes.md` §5) — the
    silence rule above still governs every other write command
    unchanged.
```

Changes from current text, in order of appearance:
1. `(new §7.4)` → `(§7.4)` — "new" is stale phrasing by this point (the
   section has existed since before this milestone); trivial, included
   for accuracy but not load-bearing.
2. New sentence appended, naming `delete`'s exception and
   cross-referencing both SPEC.md §7.4 (§1 above) and the delete spec's
   §5, mirroring the SPEC.md wording so the two docs agree.
3. Decision number (`43`) and bold lead-in are untouched.

**Why this doesn't need to touch decisions 34/37/42/44** (its
immediate neighbors) — none of them mention stdout, write-command
silence, or `delete`; confirmed by reading lines 195–259 directly.

---

## 3. `README.md` — exact edits

Two separate spots need changing: the intro paragraph's own "silent on
success" claim (which will also go false), and a new command-reference
entry for `delete`.

### 3a. Intro paragraph — exact current text (lines 112–115)

```
Every `start`/`stop`/`note`/`week target` call is silent on success —
nothing prints unless something went wrong. `status` and `week` are
the commands that produce output, so a `status` after punching in/out
is how you confirm things landed correctly.
```

**Exact proposed replacement:**

```
Every `start`/`stop`/`note`/`week target` call is silent on success —
nothing prints unless something went wrong. `status` and `week` are
the commands that produce output; `delete note`/`delete punch` are a
narrow exception too (see below) — a `status` after punching in/out is
still how you confirm things landed correctly.
```

Change: inserts the `delete` exception into the existing sentence
rather than leaving the paragraph flatly wrong once `delete` ships;
keeps the "confirm via `status`" advice for the ordinary commands
unchanged.

### 3b. New command-reference entries

**Location:** insert as two new `###` sections, after the existing
`### \`mlm week|w target [WEEK_ID] DURATION\`` section (which currently
ends at line 259, immediately before `## Build (local dev)` at line
261) — i.e. `delete` becomes the last command documented, matching its
position as the newest/most specialized command, and mirroring how
`week`/`week target` are two consecutive `###` sections for one
top-level command with a subcommand, the closest existing precedent for
`delete note`/`delete punch`.

**Exact proposed insertion** (matches the existing entries' voice:
one-paragraph description, a `-d`/`--date` grammar line reusing the
exact phrasing already used for `start`/`note`, a fenced `sh` example
block, and a callout box for the stale-id caveat in the same spirit as
`start`'s existing **Footgun** callout):

```markdown
### `mlm delete|del note|n [ID] [-d/--date DATE]`

List or delete today's (or another date's) notes. Run with no `ID` to
list that date's notes numbered `1..N`; run again with a number to
delete that entry — deleting prints a ready-to-run command to recreate
it. `-d`/`--date` targets a different date the same way as
`start`/`stop`/`note` (`YYYY-MM-DD` or `-N`), and defaults to today.

```sh
$ mlm delete note
1  fixed migration runner bug
2  reviewed open PRs
$ mlm delete note 1
deleted. to recreate: mlm note --date 2026-09-10 'fixed migration runner bug'
```

**Stale-id caveat**: `ID` is always resolved against a fresh listing
at the moment you run `delete`, not whatever listing you last looked
at. If notes were added or removed for that date since you last ran
`mlm delete note` with no `ID`, an old number may no longer point at
the entry you think it does — worst case is deleting the wrong entry
at that position, never a nonexistent one. Re-run with no `ID` right
before deleting if you're not sure the listing is still fresh.

### `mlm delete|del punch|p [ID] [-d/--date DATE]`

Same list/delete shape as `delete note`, for punches instead —
`-d`/`--date` and the stale-id caveat above both apply identically.

```sh
$ mlm delete punch
1  start 09:00
2  stop 13:00
$ mlm delete punch 2
deleted. to recreate: mlm stop 13:00 --date 2026-09-10
```
```

Notes on this draft, for whoever finalizes it:
- The two example output blocks above (`1  fixed migration runner
  bug`, `deleted. to recreate: mlm note --date 2026-09-10 '...'`,
  `1  start 09:00`, `deleted. to recreate: mlm stop 13:00 --date
  2026-09-10`) are taken verbatim from the delete spec's §4/§5 example
  strings — **not yet verified against Task 3's actually-merged code**.
  See §4 below.
- `--date` is omitted from both example commands themselves (matching
  how `stop`'s existing entry doesn't show `--date` in its one-liner
  example either) but is included in the *recreate-line output*,
  because the example assumes the deleted entry isn't today's — pick
  whichever framing the finalized examples read most naturally with;
  not a substantive choice.
- Top-level alias is `del`, not a single letter (`d` is already
  `status`'s, per the milestone plan's Global Constraints) — this
  differs from every sibling entry's single-letter alias and is called
  out explicitly in the heading (`delete|del`) rather than implied.

---

## 4. Required review step: verify quoted output strings against the merged Task 3 branch

This plan's SPEC.md, NOTES.md, and README text above quotes these
output strings directly from the design spec
(`docs/dev/specs/2026-09-13-delete-punches-notes.md` §4/§5):

- `nothing to delete for <YYYY-MM-DD>.`
- `<n>  <kind> <HH:MM>` (e.g. `1  start 09:00`)
- `<n>  <body>` (e.g. `2  fixed migration runner bug`)
- `deleted. to recreate: mlm start 09:00 --date 2026-09-10`
- `deleted. to recreate: mlm note --date 2026-09-10 'fixed migration
  runner bug'`

**These are spec-stage strings, not implementation-verified ones.**
Task 3's own acceptance criteria only pins down *structural* properties
of these strings (ordering, quoting round-trip, `--date` placement) —
it never pins the literal wording (e.g. whether it's `deleted.` vs.
`Deleted.` vs. `deleted:`, or two spaces vs. one between the ephemeral
number and the entry). Wording can legitimately shift during Task 3's
own code review without violating any of its acceptance criteria.

**Before applying any of the edits in §§1–3 above, whoever executes
Task 5 must:**
1. Check out (or read) the actually-merged Task 3 branch/commit — not
   this plan, not the spec.
2. Run `mlm delete note`/`mlm delete punch` (list mode, empty-date
   mode, and delete mode, for both a same-day and a backdated entry)
   against a real seeded db, or read the exact `println!`/`format!`
   strings directly in the merged `src/commands.rs`.
3. Diff those exact strings against every quoted string in §§1–3 above.
4. If anything differs (even punctuation/spacing), update §§1–3's
   quoted strings to match the merged code's actual output *before*
   editing `SPEC.md`/`NOTES.md`/`README.md` — the three docs must quote
   the real output byte-for-byte, per the milestone plan's own Task 5
   acceptance criteria ("checked against the merged Task 3 branch, not
   this plan's wording, in case wording changed during that task's own
   review").

---

## Summary of edits proposed by this plan

| File | Location | Action |
|---|---|---|
| `docs/dev/SPEC.md` | §7.4, lines 641–646 | Replace (§1) |
| `docs/dev/NOTES.md` | decision 35, lines 209–211 | Confirm only — apply Milestone 13 Task 2's already-specified text as backstop if not already landed (§2a); no independent wording |
| `docs/dev/NOTES.md` | decision 43, lines 253–256 | Replace — new finding, not named in the milestone plan (§2b) |
| `README.md` | intro paragraph, lines 112–115 | Replace (§3a) |
| `README.md` | after line 259, before `## Build (local dev)` | Insert two new `###` sections (§3b) |

All five quoted-output strings used across these edits must be
re-verified against the merged Task 3 branch before the edits are
actually applied (§4) — this plan's strings are spec-stage placeholders
until then.
