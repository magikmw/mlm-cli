# Milestone 13 — Task 2 low-level plan: `docs/dev/SPEC.md` §2.3 and `docs/dev/NOTES.md` decision 35 wording

> **Archived — historical planning record.** Not authoritative. Current spec: `docs/dev/SPEC.md`. Design/process history: `docs/dev/NOTES.md`.


**Scope:** Documentation only. Two files, one spot each: the `notes.body`
row inside the `**notes**` table in `docs/dev/SPEC.md` §2.3 ("Tables"),
and decision 35 ("Note content policy") in `docs/dev/NOTES.md`'s
decision log. No code, no other file/row/decision. This plan does not
edit `SPEC.md` or `NOTES.md` itself — it specifies both edits precisely
enough for whoever executes Task 1's sibling task to apply them without
further judgment calls.

**Forward-only, no backfill (applies to both edits below).** Per the
milestone plan's Goal/Architecture caveat, Task 1's normalization only
ever touches notes written *after* it lands — there is no migration
touching already-stored rows. Neither replacement wording below may
claim or imply that the "no charset restriction" carve-out covers notes
already in the table before this ships; both are worded as a
going-forward content-policy statement only. This is called out again
inline at each edit's acceptance criteria, not just here.

**Part A** (§§1–5 below) covers `docs/dev/SPEC.md` §2.3. **Part B**
(§§6–10) covers `docs/dev/NOTES.md` decision 35, same tier of rigor,
added because the parent milestone plan's Task 2 scope names both
documents explicitly (§2 of the spec calls out both by name — NOTES.md's
decision log makes the same unqualified "no charset restriction" claim
independently and goes stale the moment this lands if only SPEC.md is
touched).

## Part A: `docs/dev/SPEC.md` §2.3

### 1. Exact location

File: `docs/dev/SPEC.md`
Section: `### 2.3 Tables` (heading at line 142), inside the `**notes**`
table (heading at line 155), the `body` row.

**Current text, line 161, quoted verbatim:**

```
| `body` | `TEXT NOT NULL` | free text, trimmed of leading/trailing whitespace before storage (§6.1); project-name prefix stays *in* the text for MVP (no `project` column — that's the deferred stretch, adding it later is a plain migration); no length cap or charset restriction — the plain-ASCII rule in §7 is about layout characters in *rendered* output, not what a user can type into a note |
```

Nothing else on that line, and no other row in either table needs
touching for this task.

### 2. Exact proposed replacement (full row, not a diff)

Replace line 161 in full with:

```
| `body` | `TEXT NOT NULL` | free text, trimmed of leading/trailing whitespace before storage (§6.1); embedded `\r`/`\n` are collapsed to a single space rather than preserved verbatim, so a stored body is always exactly one line (this collapse never affects whether a body counts as empty — that check happens first, against the pre-normalization text, and rejects a whitespace-only body, including one that's only newlines, before either step touches it, §6.1); project-name prefix stays *in* the text for MVP (no `project` column — that's the deferred stretch, adding it later is a plain migration); no length cap or charset restriction beyond that newline collapse — the plain-ASCII rule in §7 is about layout characters in *rendered* output, not what a user can type into a note |
```

Changes from current text, in order of appearance:
1. New clause inserted right after the existing trim clause: `embedded
   \`\r\`/\`\n\` are collapsed to a single space rather than preserved
   verbatim, so a stored body is always exactly one line`.
2. New parenthetical immediately after that, tying the new clause back
   to the pre-existing rejection rule so a reader doesn't have to infer
   the ordering themselves: `(this collapse never affects whether a
   body counts as empty — that check happens first, against the
   pre-normalization text, and rejects a whitespace-only body,
   including one that's only newlines, before either step touches it,
   §6.1)`.
3. The trailing claim changes from `no length cap or charset
   restriction` to `no length cap or charset restriction beyond that
   newline collapse` — the old, unqualified wording would otherwise
   directly contradict the new clause (embedded newlines *are* now a
   restriction, of a kind: they don't survive storage as typed).

Everything else in the row (the `§6.1` trim citation, the project-name-
prefix clause, the plain-ASCII/§7 clause) is untouched, copied forward
verbatim.

### 3. Why this doesn't contradict the empty/whitespace-rejection rule

**That rule's exact location and text.** It appears twice in
`SPEC.md`, both already citing §6.1/§2.3 for each other; neither needs
to change for this task, they're quoted here only to check the new
row's wording against them.

`§6.1 Hard errors` (heading at line 421), lines 442–444, verbatim:

```
- Empty `NOTE`/note `body`: whitespace-only text is rejected rather
  than stored as a blank log line (checked *before* the trim in §2.3
  — a note that's nothing but whitespace has nothing left to trim to).
```

`§8.2 Error / edge paths` (heading at line 700), **E5**, lines 711–713,
verbatim:

```
- **E5** — Empty/whitespace-only `NOTE` → hard error (checked before
  the trim, §2.3/§6.1). A padded-but-non-empty note (`"  did a
  thing  "`) is accepted and stored trimmed.
```

**Why there's no contradiction.** Both existing passages fix the
rejection check *before* the trim step, on the untouched input. The
newline-collapse this task documents is a second, later normalization
step (per the milestone plan, it only fires on what's left *after* the
existing trim/empty-check has already passed — Task 1 doesn't move the
rejection check). A body that is only `\n`/`\r`/spaces is still caught
by the existing "whitespace-only, nothing left to trim to" rule and
never reaches the collapse step at all; the collapse only ever touches
a body that already contains at least one non-whitespace character (an
*embedded* run of newlines between real text). So:

- The rejection rule's scope (whitespace-only bodies) and the collapse
  rule's scope (embedded newlines inside an otherwise non-empty body)
  are disjoint — they can never both fire on the same input.
- The new row's parenthetical says this explicitly ("this collapse
  never affects whether a body counts as empty... before either step
  touches it, §6.1") specifically so a future reader of just the
  `notes.body` row isn't left to work out the ordering for themselves
  or wonder whether "collapsed to a single space" might mean a
  newline-only body now survives as a lone space — it doesn't.
- No wording in §6.1/§8.2 needs to change: neither passage claims
  anything about what happens to embedded (non-empty-making) newlines,
  so the new row is additive, not corrective, with respect to both.

### 4. Other `SPEC.md` sections checked for a matching update

Searched the whole file for other statements/implications about note-
body content restrictions or newline handling (`note body`, `NOTE`,
`body`, `newline`, `\n`, `multi-line`, `single-line`, `bullet`).
Findings:

- **Line 161 (`notes.body` row, §2.3)** — the only spot, needs the
  update above.
- **§6.1 line 442–444 and §8.2 line 711–713** — the empty/whitespace
  rejection rule (quoted in §3 above). Neither states or implies
  anything about *embedded* newlines specifically (only about a body
  that is nothing but whitespace); nothing to change.
- **§3.2/§3.3/§3.4 (`mlm start`/`stop`/`note` command grammar, lines
  216–239)** — describe `NOTE` as "free text" for CLI-parsing purposes
  (word-splitting/`trailing_var_arg` concerns), not storage content
  rules. No claim about newlines one way or the other; nothing to
  change.
- **§7.1 `mlm status` (lines 475–529)**, specifically the `Notes:`
  block (lines 490–493, `  - <body>` bullet format) and its prose
  ("Notes render only if any exist for the date"). This section
  already *assumes* one bullet line per note — it doesn't assert
  "bodies contain no newlines" as a documented invariant, it just
  formats each note as a single `- <body>` line and would have silently
  garbled a multi-line body before this milestone. Since it makes no
  explicit claim that this task's new SPEC wording could now
  contradict or duplicate, and the milestone plan doesn't ask for a
  rendering-section rewrite, **no change needed here for Task 2** —
  this section becomes *actually* accurate as a side effect of Task 1's
  code landing, with nothing in its text now false or needing a
  caveat. (Contrast with the separate, out-of-scope-for-this-task
  §7.4 "only `status`/`week` produce stdout" claim, which the
  `delete`-feature spec — not this milestone — calls out as needing
  its own future edit for unrelated reasons.)
- No other section (§1, §2.1, §2.2, §2.4, §4.x, §5, §7.2, §7.3, §7.4,
  §8.1, §9+) mentions note-body content, charset, length, or newlines
  at all.

**Conclusion:** only the one row (line 161) needs a matching update for
this task; no other `SPEC.md` section requires an edit.

### 5. Acceptance check for whoever applies this edit

- Line 161 replaced with the exact block in §2 above, nothing else in
  the file touched.
- The rendered Markdown table still parses correctly (no stray `|`
  introduced by the new backticked `` `\r` ``/`` `\n` `` tokens — the
  replacement text above was written to keep every embedded backtick
  pair matched and no bare `|` inside the new clauses).
- A reader of just this row, with no other context, can state: (a)
  embedded newlines are removed, not kept; (b) that removal doesn't
  turn a newline-only body into a valid one-space body; (c) exactly
  which existing hard-error rule (§6.1) governs that.
- The replacement text makes no claim, explicit or implied, that this
  carve-out covers notes already stored before Task 1 lands — it reads
  as a content-policy statement about what gets written from here on,
  not a retroactive guarantee over existing rows (see the top-level
  "Forward-only, no backfill" note above).

## Part B: `docs/dev/NOTES.md` decision 35

### 6. Exact location

File: `docs/dev/NOTES.md`
Section: the decision log entry numbered **35**, "Note content policy"
(currently at lines 209–211, immediately after decision 34
"Concurrency" and before the "More decisions (from implementation-plan
review, round 6)" heading).

**Current text, lines 209–211, quoted verbatim:**

```
35. **Note content policy**: no length cap, no charset restriction —
    §7's plain-ASCII commitment is about rendered layout characters,
    not what a user can type into a note body.
```

Nothing else in the decision log needs touching for this task — decision
35 is the only entry that mentions note-body content/charset (see §9
below for the full search).

### 7. Exact proposed replacement (full entry, not a diff)

Replace lines 209–211 in full with:

```
35. **Note content policy**: no length cap, no charset restriction
    beyond embedded `\r`/`\n` being collapsed to a single space rather
    than preserved verbatim (so a stored body is always exactly one
    line, applies to notes written from that point forward only, no
    backfill of rows already stored) — §7's plain-ASCII commitment is
    about rendered layout characters, not what a user can type into a
    note body.
```

Changes from current text, in order of appearance:
1. The claim changes from `no length cap, no charset restriction` to
   `no length cap, no charset restriction beyond embedded \`\r\`/\`\n\`
   being collapsed to a single space rather than preserved verbatim` —
   the old, unqualified wording would otherwise directly contradict
   Task 1's normalization (embedded newlines *are* now a restriction,
   of a kind: they don't survive storage as typed). This mirrors the
   Part A/SPEC.md change exactly, adapted to this entry's sentence
   shape.
2. New parenthetical appended to that clause: `(so a stored body is
   always exactly one line, applies to notes written from that point
   forward only, no backfill of rows already stored)` — the forward-only
   half is new relative to the Part A/SPEC.md wording, added here
   because this is prose in a decision log (read standalone, out of
   the table-row context that SPEC.md's own surrounding rows/sections
   provide) and is more likely to be read as a blanket claim about
   every row in the table if the caveat isn't stated inline.
3. The trailing `§7's plain-ASCII commitment...` sentence is untouched,
   copied forward verbatim.
4. The decision number (`35`) and bold lead-in (`**Note content
   policy**`) are untouched.

### 8. Why this doesn't contradict the empty/whitespace-rejection rule

`docs/dev/NOTES.md` is a running decision log, not the spec — it does
not itself restate the empty/whitespace-only-note rejection rule (that
rule lives only in `SPEC.md` §6.1/§8.2 and in the actual `insert_note`
code, quoted in Part A §3 above). Decision 35 has never made any claim
about empty/whitespace-only bodies one way or the other, and the
replacement text above doesn't add one — it only describes what happens
to a body that already has at least one non-whitespace character (an
*embedded* run of newlines between real text), same disjoint scoping
as the Part A/SPEC.md change. So there is nothing in decision 35,
before or after this edit, for the rejection rule to contradict.

### 9. Other `NOTES.md` entries checked for a matching update

Searched the whole file for other decisions/prose about note-body
content restrictions or newline handling (`note body`, `newline`,
`\n`, `charset`, `length cap`, `multi-line`, `single-line`). Findings:

- **Decision 35 (lines 209–211)** — the only entry that mentions
  note-body charset/length/content policy, needs the update above.
- No other decision (1–34, 36+) or surrounding prose mentions note-body
  content, charset, length, or newline handling at all — decision 34
  ("Concurrency," immediately preceding) and decision 37 ("Totals
  exclude open-stint live time," the next numbered entry after the
  "round 6" heading) are both about unrelated topics and don't need
  touching.

**Conclusion:** only decision 35 needs a matching update for this task;
no other `NOTES.md` entry requires an edit.

### 10. Acceptance check for whoever applies this edit

- Lines 209–211 replaced with the exact block in §7 above, nothing else
  in the file touched (in particular, the decision number `35` and the
  surrounding decisions 34/37 are left exactly as they are).
- The replacement reads as a standalone decision-log entry that makes
  sense without cross-referencing SPEC.md — a reader of just this entry
  can state: (a) embedded newlines are removed, not kept; (b) that only
  applies to notes written after this change ships, not a retroactive
  guarantee over rows already in the table; (c) everything else about
  the "no length cap/no charset restriction" claim is unchanged.
- The updated wording does not contradict SPEC.md's §2.3 `notes.body`
  row (Part A above) or the §6.1/§8.2 empty/whitespace-rejection rule —
  both entries now say the same thing about the newline carve-out, and
  neither claims retroactive coverage of pre-existing rows.
