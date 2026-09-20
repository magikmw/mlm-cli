# Delete punches/notes — design spec

> **Archived — historical changeset spec.** Superseded by `docs/dev/SPEC.md`, which folds in the decisions made here.


**Baseline**: written against `v0.2.0`.

**Status**: draft — adversarially reviewed twice, once directly against
this spec and once indirectly via its milestone-13/14 implementation
plans (see §2/§3/§5/§7/§8 for findings folded in both rounds).

## 1. Summary

No way to remove a mis-entered punch or note today; the only fix is
manual DB surgery. This adds `mlm delete note|punch`, delete-only — no
edit command. Correcting a mistake is delete-then-recreate
(`start`/`stop`/`note` already exist for the recreate half); to make
that cheap, `delete` echoes a ready-to-run recreate command on
success.

Punches and notes have no user-facing id today (`id` is an internal
DB surrogate key, SPEC §2.3). Rather than expose that stable id
permanently, `delete` computes **ephemeral, per-invocation** ids: the
1-based position in the existing, already-deterministic storage
order for a given date. Running `mlm delete note`/`punch` with no id
lists that date's entries numbered `1..N` (or "nothing to delete" if
none); running it again with a number deletes that entry.

## 2. Prerequisite: normalize note bodies to single-line (separate commit)

**Ships as its own, independently-scoped commit/PR** — it doesn't
touch any of the CLI/storage code this spec adds, it's a one-line
change to the existing note-insert normalization path, and it stands
on its own merits regardless of whether `delete` ships. Land it ahead
of (or independent from) the rest of this spec; §5's recreate-echo
design assumes it's already true.

- **Change**: `note`'s existing trim-before-storage step (SPEC §2.3:
  "trimmed of leading/trailing whitespace before storage") is extended
  to also collapse any embedded `\r`/`\n` to a single space. Applies
  everywhere a note body is written — the `note` command and the
  inline note on `start`/`stop`.
- **Why**: two independent reasons, neither one delete-specific:
  1. `status`'s existing note rendering is one bullet per line (SPEC
     §7.1: `- <body>`) — that already silently assumes single-line
     bodies; an embedded newline garbles that output today, with no
     feature here involved.
  2. It removes a whole failure mode from this spec's recreate-echo
     (§5): a multi-line body has no valid single-line "ready to run"
     representation, and this way that case can never arise instead
     of needing its own fallback path.
- **Silent normalize, not reject**: a hard error on newline input
  would punish an accidental multi-line paste for no benefit — replace
  is consistent with how the existing trim step already treats
  whitespace non-destructively.
- **Touches base SPEC.md too**: §2.3's "no length cap or charset
  restriction" line needs a carve-out noting embedded `\r`/`\n` are
  normalized away, not preserved. `docs/dev/NOTES.md`'s decision log
  makes the same unqualified "no charset restriction" claim — update
  it alongside SPEC.md, not just SPEC.md, or the decision log goes
  stale the moment this lands. Needs its own test
  (`"line one\nline two"` stores as `"line one line two"`) independent
  of anything in §7.
- **Tie-break rule for interrupted runs**: only a *maximal run of
  `\r`/`\n` characters* collapses to one space — ordinary whitespace
  (a plain space/tab) between two such runs is left exactly as typed,
  never merged into the run's replacement space. E.g. `"a\n\n \nb"`
  (newline-run, one literal space, newline) becomes `"a   b"` (three
  spaces: one from the first run, the untouched literal space, one
  from the second run) — not `"a b"`. This is the concrete answer to
  "what happens when a run is interrupted by plain whitespace,"
  otherwise underspecified.
- **Unicode line separators (U+2028/U+2029) are explicitly out of
  scope**: this normalization targets ASCII `\r`/`\n` only, consistent
  with this project's existing plain-ASCII conventions elsewhere
  (SPEC §7). An embedded U+2028/U+2029 passes through untouched — not
  a silent gap, a deliberate boundary.
- **Applies to newly-written notes only — no backfill/migration.**
  This is a pure insert-path change (§1: "no schema change"). This
  project has no installs/users predating this normalization, so there
  is no pre-existing data to worry about in practice — a note body can
  never contain `\n` from this point forward, full stop, not merely
  "for anything written after this ships." §5's recreate-echo relies on
  that guarantee directly (a `debug_assert!` there is a cheap tripwire
  for a future regression, not a runtime fallback for legacy data).

## 3. CLI surface

- `mlm delete note [ID] [--date DATE]`
- `mlm delete punch [ID] [--date DATE]`
- `--date`: same grammar *and* same resolver as `start`/`stop`/`note`'s
  existing `--date` — absolute `YYYY-MM-DD` or `-N` shorthand, defaults
  to today, resolved via `date::resolve_future_checked_date` (a future
  `--date` is a hard error, `Cause::Future`, same as those three
  commands). Reusing the resolver `commands.rs` already imports and
  uses beats introducing a second one (`date::resolve_date`, the
  permissive resolver `status.rs` uses) purely to be marginally more
  lenient — there's no real permissiveness lost: a future date can
  never have punches/notes to delete anyway, since nothing can insert
  data into the future in the first place, so the two resolvers would
  behave identically in practice for every actually-reachable input.
  (Superseded decision: an earlier draft of this spec picked the
  permissive resolver instead — reverted after review flagged it as
  needless complexity with no behavioral payoff.)
- No `ID` → list mode (§4). `ID` given → delete mode (§5).
- No ranges, no bulk delete, no `--all`.
- Aliases: `delete` → `del` (`d` is already `status`'s), `note` → `n`,
  `punch` → `p` (sub-level, no collision with top-level aliases).
- Landing this feature also makes SPEC.md §7.4's current claim ("Only
  `status` and `week` produce stdout output") false — `delete`'s list
  mode and its success echo (§5) both print to stdout. §7.4 needs a
  one-line update alongside this spec, not just this spec's own docs
  (§8).

## 4. List mode (no `ID`)

- Resolve target date (§3).
- Fetch that date's rows via the existing `storage::punches_for_date`
  / `storage::notes_for_date` — no new storage read needed, both
  already take `date: NaiveDate` and return an already-ordered
  `Vec<Punch>`/`Vec<Note>` (punches by `at_utc`, notes by
  `created_at_utc`/`id`, §2.3's tiebreak note).
- Number that returned order `1..N` — the ephemeral id is just the
  1-based position in the existing storage order, no new sort, no new
  column.
- Zero rows → print `nothing to delete for <YYYY-MM-DD>.` to stdout,
  exit `0` (not an error — mirrors the "empty section, not an error"
  tone used elsewhere, e.g. SPEC §6.2).
- Non-zero rows → print each numbered:
  - Punch: `<n>  <kind> <HH:MM>`, e.g. `1  start 09:00`, reusing
    `kind.as_str()` and existing time formatting.
  - Note: `<n>  <body>`, e.g. `2  fixed migration runner bug`.
- Pure read — no writes on this path, same property `status` has.

## 5. Delete mode (`ID` given)

- `ID` is a positional `u32`-shaped arg, 1-based, resolved against a
  **freshly re-run** §4 listing query for the resolved date — i.e.
  `mlm delete note 2` re-fetches and re-numbers, then deletes whatever
  currently lands at position 2. No caching of a previous listing:
  worst case of a stale id is deleting the wrong day's *current*
  position 2, never a row that doesn't exist. (Stale-id risk after a
  same-sitting add/list/add/delete sequence is a documentation/
  onboarding note — belongs in README alongside the rest of `delete`'s
  documentation, §8 — not something to code around.)
- A malformed or future-dated `--date` is a hard error, exactly like
  `start`/`stop`/`note` (§3's resolver choice) — nothing deleted,
  nonzero exit.
- `ID` of `0`, or `> N` for that date (including `N == 0`): hard
  error, nothing deleted, nonzero exit — same shape as existing hard
  errors (message on stderr, no write). This is the app-level
  validation path. **Not every bad `ID` reaches it**: `-1`,
  non-numeric input (`abc`), and anything past `u32::MAX` are rejected
  by clap itself, before `commands.rs` ever runs, with clap's own
  error text/exit path rather than the app's — both paths are real and
  both need covering (§8), not just the app-level one.
- No confirmation prompt on delete. (Possible future config option if
  a config system ever exists; out of scope here.)
- Needs two new `storage.rs` functions (none exist yet):
  - `delete_punch(conn: &Connection, id: i64) -> Result<Punch, StorageError>`
  - `delete_note(conn: &Connection, id: i64) -> Result<Note, StorageError>`

  Both take the row's *internal* DB id (resolved from the ephemeral
  position first), and return the deleted row for the recreate-echo
  below. Implementation: `SELECT` then `DELETE ... WHERE id = ?`, or a
  single `DELETE ... RETURNING *` — implementation's choice, no
  behavior difference. A delete affecting zero rows maps to a new
  `StorageError::NotFound { id: i64 }` variant rather than panicking
  or silently succeeding — cheap defensive coding, not a claim of
  concurrency support: SPEC §2.2 explicitly puts concurrent
  multi-process access out of scope for MVP (no WAL/busy-timeout
  tuning planned), and the only realistic way this race fires is
  exactly that unsupported scenario, or external DB tampering. Adding
  `NotFound` means "doesn't panic if it somehow happens" — it isn't
  reopening §2.2's scope decision. Like the backdated-punches spec did
  for its own new `TimeParseError::Required` variant, remember
  `StorageError`'s `Display`/`source()` are exhaustive `match` blocks
  (`src/storage.rs`) — adding `NotFound` needs a new arm in both or it
  won't compile.
- **Deleting a date's only/earliest data can shift week figures
  elsewhere.** SPEC §2.4's carry walk starts from
  `storage::earliest_data_date` — whichever date currently has the
  earliest punch/note. Deleting all remaining rows on what was that
  date moves the anchor forward, which silently changes every later
  week's carry-in/fulfillment/owed on the next `status`/`week` view.
  Same mechanism, opposite direction, as the backdated-punches spec's
  §3.1 "accepted consequence of retroactive week figures" — accepted
  here too, not guarded against, called out explicitly rather than
  left for someone to discover.
- **Echo on delete** (deliberate exception to SPEC §7.4's "write
  commands print nothing", §3): on success, print a ready-to-run
  recreate command, not just a description:
  - Punch: `deleted. to recreate: mlm start 09:00 --date 2026-09-10`
  - Note: `deleted. to recreate: mlm note --date 2026-09-10 'fixed
    migration runner bug'`
  - `--date …` is only included when the deleted entry's date isn't
    today (no clutter on same-day deletes).
  - For a note, `--date` **must precede** the body text in the
    printed string, per the backdated-punches spec §2.1 footgun: that
    string gets fed back into `note`, which *does* have the ordering
    footgun, even though `delete`'s own args (§6) don't.
  - **The note body is wrapped in single quotes as one shell token**,
    not printed as bare whitespace-separated words. Verified against
    clap 4.6.6: printing the body as bare tokens lets any word that
    happens to match a *registered global flag* — `--verbose`
    (`src/cli.rs`, `global = true`) — get silently re-parsed as that
    flag instead of note text on replay (word lost from the body, no
    error, verbose logging turns on as a side effect); a body
    containing a literal `--` token is separately swallowed by clap's
    own end-of-options marker. Wrapping the whole body as a single
    quoted argv token sidesteps both: clap matches global flags and
    `--` against whole tokens, never against substrings of one.
  - Single quotes specifically, not double: double quotes still let
    the shell interpolate `$(...)`, backticks, and `$VAR` *inside* the
    printed line before the token even reaches `mlm` — since note
    bodies have no charset restriction (SPEC §2.3, aside from §2's
    newline carve-out), a body containing shell metacharacters
    combined with double-quoting could execute something on paste.
    Single quotes suppress all interpolation in POSIX shells and in
    fish.
  - Embedded `'` characters in the body are escaped for the printed
    line to stay valid: each `'` becomes `'"'"'` (standard POSIX
    close-quote/literal-quote/reopen-quote sequence).
  - Multi-line bodies are ruled out entirely by §2's prerequisite (this
    project has no pre-existing data, so there is no legacy case to
    defend against) — a stored body can never contain `\n`/`\r`.
  - **Embedded backslash fallback**: there is no single-quoting scheme
    that round-trips a literal `\` identically in both bash and fish —
    verified that fish's single-quote parsing recognizes `\\`/`\'` as
    escapes even inside `'...'`, where POSIX shells treat single quotes
    as 100% literal with no escapes at all. A note body containing a
    literal `\` (a Windows path, a regex, anything) would either
    silently corrupt on replay under fish or fail to parse
    (`quotes are not balanced`). This is a live, ongoing case — any
    note typed from now on can trigger it, nothing to do with legacy
    data. Fix: if the deleted body contains a literal `\` (checked at
    echo time), print a plain description instead of a quoted command —
    `deleted note (2026-09-10): <first line of body>...` — rather than
    emitting a command that's broken in at least one common shell. No
    shell detection is involved: the plain description is safe to print
    as-is regardless of what shell is running, and the quoted-command
    path is only ever used for bodies that don't contain a backslash,
    where the existing single-quote scheme is provably safe in both
    bash and fish.
- Exit `0` on success.

## 6. CLI wiring (`src/cli.rs`)

Mirrors the existing `week`/`WeekAction` pattern (top-level command
with its own subcommand enum), simpler since there's no bare-positional
fallback ambiguity to guard (`WeekArgs`'s
`args_conflicts_with_subcommands` exists because `WEEK_ID` and
`target` could collide; here `note`/`punch` is always required, no
bare `mlm delete`):

```rust
#[command(visible_alias = "del")]
Delete(DeleteArgs),
```

```rust
pub struct DeleteArgs {
    #[command(subcommand)]
    pub target: DeleteTarget,
}

pub enum DeleteTarget {
    #[command(visible_alias = "n")]
    Note(DeleteEntryArgs),
    #[command(visible_alias = "p")]
    Punch(DeleteEntryArgs),
}

pub struct DeleteEntryArgs {
    /// 1-based position from the most recent listing for this date
    /// (run with no ID to list). Omit to list instead of deleting.
    pub id: Option<u32>,

    /// Date to operate on: same grammar as elsewhere (YYYY-MM-DD or
    /// -N). Defaults to today.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,
}
```

No `trailing_var_arg` on this struct — unlike `PunchArgs`/`NoteArgs`
there's no free-text `NOTE` positional to fight with, so the
backdated-punches §2.1 ordering footgun (`--date` must precede
`NOTE`) **does not apply to parsing `delete`'s own args**: `mlm delete
note 2 --date -1` and `mlm delete note --date -1 2` both parse the
same way — verified against clap 4.6.6, both produce
`id: Some(2), date: Some("-1")`. (It does still matter for the
*printed recreate string*, since that gets fed into `note` — §5.)
Worth an explicit regression test (§8) precisely because it's easy to
assume the footgun applies here by analogy when it doesn't.

## 7. `commands.rs`

New `delete_note`/`delete_punch` handlers, parameterised the same way
`punch()`/`note()` already are:

1. Resolve date via `date::resolve_future_checked_date` (§3).
2. Fetch `storage::notes_for_date` / `storage::punches_for_date` for
   that date (already correctly ordered, §4).
3. `args.id == None` → list mode (§4), return `Ok(())`.
4. `args.id == Some(n)`:
   - `n == 0` or `n > rows.len()` → hard error (`anyhow::bail!`,
     consistent with the crate's error convention — no dedicated
     error enum needed, same as `note()`/`punch()`'s own validation).
     Message names both the requested position and how many entries
     exist for that date. Nothing deleted.
   - Otherwise: `rows[n - 1].id` is the internal id → call
     `storage::delete_punch`/`delete_note` → print the recreate line
     (§5) → `Ok(())`.
5. Recreate-line construction:
   - Punch: convert the deleted row's `at_utc` to local time using the
     same **per-instant** timezone conversion `status.rs`'s existing
     rendering already uses (`at_utc.with_timezone(tz)`, per row) —
     **not** a single offset grabbed once from `now` and reused. SPEC
     §2.1 explicitly forbids that pattern generally; reusing `now`'s
     offset here would echo the wrong `HH:MM` for a backdated punch
     that falls on the other side of a DST transition from today.
     Format `HH:MM` with the existing time-formatting helper, pick
     `start`/`stop` from `kind`.
   - Note: body text as stored, single-quoted and escaped, `--date`
     prefixed — all per §5.

## 8. Testing plan

- **`storage.rs`** unit tests: `delete_punch`/`delete_note` remove the
  row and return it; deleting a nonexistent id returns `NotFound`
  rather than panicking; deleting one row doesn't touch other rows
  for the same or other dates.
- **`commands.rs`** unit tests, per subcommand:
  - List mode: empty date → "nothing to delete" message, exact date
    formatting; non-empty date → correct numbering, correct
    punch/note formatting, order matches storage order.
  - Delete mode: valid id deletes the right row (verified via a
    follow-up `punches_for_date`/`notes_for_date` read) and prints
    the correct recreate line; `--date` omitted → recreate line has
    no `--date`; `--date` given (incl. `-N` shorthand) → recreate
    line carries the resolved absolute date.
  - Out-of-range id: `0` and `N+1` for an N-row date both rejected,
    nothing deleted (row count unchanged on a follow-up read).
  - `--date` scoping: deleting from date A never touches date B's
    rows/numbering (two dates with overlapping counts, e.g. both have
    a "position 1").
  - Recreate-line correctness for a **note**: `--date` precedes the
    body text in the printed string, order checked explicitly (§5/§7
    footgun case), not just substring presence.
  - **Quoting round-trip** (the actual regression test for §5's
    flag-collision/`--`-swallowing fix): **the bug is
    position-dependent** — `trailing_var_arg` only misparses a
    flag-like token when it's the *first* token of the printed body
    region (immediately after `--date`'s value, or at the very start
    with no `--date`); once at least one ordinary word has already
    been captured, clap stops re-checking later tokens against flags,
    so a flag-lookalike *mid*-body was never actually broken. The test
    must use a **leading** flag-lookalike — a body of `"--verbose
    logging bug"`, not `"fixed --verbose logging bug"` — or it passes
    identically with or without the quoting fix and proves nothing.
    Once deleted, this body and a separate body containing an embedded
    `'` both, run through the recreate line and re-parsed via
    `Cli::try_parse_from`, must yield the exact original body back,
    unmodified. Not just a formatting/appearance check.
  - **Embedded backslash**: a note body containing a literal `\`
    (e.g. a Windows path or a regex), once deleted, produces the
    plain-description fallback (§5), not a quoted command that would
    misparse under fish.
  - **Future `--date` rejected**: `mlm delete note --date <a future
    date>` is a hard error, nothing deleted/listed — confirms the §3
    resolver choice, mirroring the equivalent `start`/`stop`/`note`
    test.
  - **DST-crossing punch delete**: delete a backdated punch whose
    date sits on the other side of a DST transition from today's
    date; recreate line's `HH:MM` matches the punch's actual local
    wall-clock time, not a `now`-offset-shifted one (§7's fix,
    mirroring the main SPEC's existing DST test convention).
  - **Earliest-data-date shift**: delete the only remaining
    punch/note on the currently-earliest tracked date; a subsequent
    `status`/`week` view reflects the new earliest date and updated
    carry/owed figures (§5's called-out consequence).
- **`cli.rs`** parse-level tests: `mlm delete note 2 --date -1` and
  `mlm delete note --date -1 2` parse identically (§6's "footgun does
  not apply here" claim, locked down the same way the
  backdated-punches spec locked down its own footgun); `del`/`n`/`p`
  aliases all parse to the same result as the full names; bare `mlm
  delete note`/`mlm delete punch` (no id) parse with `id: None`;
  clap-level rejection of a bad `ID` (`-1`, `abc`,
  `4294967296`/`u32::MAX + 1`) — distinct from the app-level `0`/`>N`
  cases already covered above, per §5's split.
- **e2e smoke test** (existing script): extend with add → bare
  `delete note`/`delete punch` listing → delete by listed id → confirm
  gone from `status` → confirm the printed recreate line actually
  works when re-run.
- **Docs**: README and `--help` text (`///` doc comments on the new
  `cli.rs` items, §6) updated alongside the code, not as a follow-up —
  README's coverage includes the stale-id caveat (§5) explicitly, not
  just the command syntax. Also update SPEC.md §7.4's now-inaccurate
  "only `status` and `week` produce stdout output" claim (§3), and
  `docs/dev/NOTES.md`'s matching "no charset restriction" claim (§2).

Newline-normalization's own test (§2) ships with that separate
commit, not duplicated here.

## 9. Out of scope

- Edit command — no edit story; correction path stays delete +
  recreate.
- Fuzzy-match / text-based entry selection — only numeric ephemeral
  ids.
- Confirmation prompt on delete — none for this change (§5).
- Interactive TUI (ratatui) — parked, unrelated to this change.
- Stable/persistent ids for punches/notes — ids stay ephemeral,
  recomputed per listing.
- Bulk delete / `--all` / ranges (`2-4`) — single id per invocation.
- Rendering changes to `status`/`week` beyond deleted rows no longer
  appearing — no new display logic, though §5 notes the figures
  themselves can shift as a consequence of what's deleted.
- Note-body newline normalization's implementation — real and
  required (§2), but scoped as its own separate commit, not part of
  this change's diff.
