# Milestone 14, Task 2 — low-level plan: `src/cli.rs` `delete` CLI surface

**Scope:** `src/cli.rs` only. Clap-level plumbing: new `Command::Delete`
variant, new `DeleteArgs`/`DeleteTarget`/`DeleteEntryArgs` types, their
doc comments/attributes, and their tests. No storage/commands/main
changes (Tasks 1/3/4). Nothing in this file is edited by this plan —
it is a plan document only.

**Sources read:** milestone-14 plan (Task 2 section), delete-punches-
notes spec §3/§6 (and skimmed the rest), backdated-punches spec §2.1,
current `src/cli.rs` in full. `clap = "4.6.6"` per `Cargo.toml` — same
version the backdated-punches spec's §2.1 empirical claims were
verified against, so those claims carry over unchanged.

---

## 1. Where `Command::Delete` goes, and its exact shape

Insert as a new variant in `pub enum Command` (`src/cli.rs`, currently
lines 16–42). Placement: **after `Note`, before `Week`** — it keeps
the "single-entry-recording" commands (`Start`/`Stop`/`Note`) grouped
together ahead of the "view/report" commands (`Week`/`Status`), which
matches the existing ordering's apparent intent (mutating commands
first, read commands last) more closely than appending at the end.
Nothing in clap or the codebase depends on declaration order, so this
is a style call, not a correctness one — but it's the placement this
plan proposes and the implementer should type it there directly rather
than re-deriving.

```rust
/// Delete a punch or note by its ephemeral, per-listing position for a
/// date (run with no ID first to list and number that date's entries).
#[command(visible_alias = "del")]
Delete(DeleteArgs),
```

Notes on this exact text:
- Doc comment describes list-then-delete because that is `delete`'s
  entire user-facing model (spec §1) — matches the style of the
  existing variants' doc comments, which each describe behavior, not
  just repeat the subcommand name.
- `visible_alias = "del"` (not `alias`): every other aliased variant in
  this enum (`Start`→`s`, `Stop`→`e`, `Note`→`n`, `Week`→`w`,
  `Status`→`d`) uses `visible_alias`, which shows the alias in
  `--help` output; matching that convention is required for `del` to
  actually appear in help text like its siblings. `del` does not
  collide with any existing top-level alias (`s`, `e`, `n`, `w`, `d`).
- The variant carries a struct payload `DeleteArgs`, exactly like
  `Start(PunchArgs)`/`Stop(PunchArgs)`/`Note(NoteArgs)`/`Week(WeekArgs)`
  — not an inline `{ .. }` shape like `Status` uses, since `Status` has
  no further nested subcommand and `Delete` does.

---

## 2. `DeleteArgs` — top-level `delete` struct

Placed directly below `Command`'s closing brace, mirroring where
`PunchArgs`/`NoteArgs` sit relative to the enum today. Its own doc
comment should explain the mirrored-`WeekArgs` relationship and why
`args_conflicts_with_subcommands` is *not* needed here (this plan's
draft, refined from the spec's §6 sketch):

```rust
/// `mlm delete note|punch [ID] [--date DATE]`. Mirrors `WeekArgs`'
/// top-level-command-with-its-own-subcommand-enum shape, but simpler:
/// unlike `week`, there is no bare positional that could collide with
/// the subcommand name, so — unlike `WeekArgs` —
/// `args_conflicts_with_subcommands` is not needed: `note`/`punch` is
/// always a required subcommand, there is no bare `mlm delete` to
/// disambiguate against a fallback positional.
#[derive(Args, Debug)]
pub struct DeleteArgs {
    #[command(subcommand)]
    pub target: DeleteTarget,
}
```

Fields, precisely:
- `target: DeleteTarget` — a required subcommand (`#[command(subcommand)]`
  with no `Option<..>` wrapper, unlike `WeekArgs::action` which is
  `Option<WeekAction>`). Making it non-optional is deliberate and is
  what makes `mlm delete` with no further subcommand a clap-level
  `MissingRequiredArgument`-shaped failure (needs no
  `args_conflicts_with_subcommands` because there is no other field on
  this struct providing an alternative path) — matches the spec's "no
  bare `mlm delete`" statement in §6.

---

## 3. `DeleteTarget` — `note`/`punch` subcommand enum

```rust
/// `note` and `punch` targets for `delete`, each taking the same
/// `[ID] [--date DATE]` argument shape.
#[derive(Subcommand, Debug)]
pub enum DeleteTarget {
    /// Delete (or list) a note for a date.
    #[command(visible_alias = "n")]
    Note(DeleteEntryArgs),

    /// Delete (or list) a punch for a date.
    #[command(visible_alias = "p")]
    Punch(DeleteEntryArgs),
}
```

Precise attribute choices:
- `visible_alias = "n"` / `"p"` — spec §3 and the milestone plan both
  name these exact aliases. They are sub-level aliases (scoped to
  `DeleteTarget`, i.e. only meaningful after `mlm delete`/`mlm del`),
  so they cannot collide with the top-level `n` alias already claimed
  by `Command::Note` (`n` under `delete` and `n` under the top level
  are different clap `Command` trees, resolved independently — this is
  the same reasoning the spec's §6 note about "no collision with
  top-level aliases" already makes explicit).
- Both variants carry the same payload type, `DeleteEntryArgs` — there
  is no per-target field difference (no separate note-only or
  punch-only argument), so one shared struct is correct, not a
  simplification that loses information.
- Doc comments say "Delete (or list)" rather than just "Delete" so
  `--help` doesn't imply a bare `mlm delete note` is an error — it
  isn't (§4 of the spec: no `ID` is list mode).

---

## 4. `DeleteEntryArgs` — shared `id`/`--date` struct

```rust
/// Shared `[ID] [--date DATE]` shape for `delete note` and
/// `delete punch`. Unlike `PunchArgs`/`NoteArgs`, there is no
/// free-text trailing positional on this struct, so the
/// backdated-punches spec §2.1 ordering footgun (`--date` must
/// precede free note text or be silently absorbed into it) does not
/// apply to parsing these arguments — see §6 below for why, in detail.
#[derive(Args, Debug)]
pub struct DeleteEntryArgs {
    /// 1-based position from the most recent listing for this date
    /// (run with no ID to list instead of deleting). Position 0 and
    /// anything past the current count are rejected once the app
    /// resolves this against a fresh listing, not here.
    #[arg(value_name = "ID")]
    pub id: Option<u32>,

    /// Date to operate on: YYYY-MM-DD, or `-N` for N days before today
    /// (e.g. `-1` = yesterday). Defaults to today. A malformed or future
    /// date is rejected once the app resolves it with
    /// `date::resolve_future_checked_date`, the same resolver
    /// start/stop/note already use -- not here at the parsing level.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,
}
```

Field-by-field, exact reasoning:

- **`id: Option<u32>`**
  - Type is `Option<u32>`, not `Option<i64>`/`Option<String>`. `u32`
    is the type clap hands to `value_parser` by default for a field
    typed `u32` (clap's derive picks `clap::value_parser!(u32)`
    automatically for a plain integer field — no explicit
    `value_parser` attribute is required). This is what gives clap
    itself, not application code, the "reject negative / non-numeric /
    too-large" behavior the milestone plan requires: clap's built-in
    `u32` parser is a thin wrapper over `str::parse::<u32>()`, which
    fails (and clap turns that into `ErrorKind::ValueValidation`) on a
    leading `-`, non-digit characters, or a value exceeding
    `u32::MAX` (4294967295) — no custom validator needed. This matches
    the spec §5 requirement precisely: `-1`, `abc`, and
    `4294967296` (`u32::MAX + 1`) are all clap-level rejections, before
    `commands.rs` (Task 3) ever runs. `0` parses fine as a `u32` (it's
    a legal `u32` value) — clap has no concept of "positive but not
    zero" built in for a bare integer type, so `0`'s rejection is
    correctly left to application code (Task 3), never encoded here.
    `u32::MAX` itself (4294967295) is accepted by clap — it is a legal
    `u32` — and would only fail later at the application level as
    "past the current count," which is fine and out of this task's
    scope.
  - `Option<..>` (not a bare `u32`) makes it an optional positional —
    absent means list mode (spec §4), present means delete mode (§5).
    This is the field whose presence/absence `commands.rs` branches on
    (milestone plan step 3).
  - `#[arg(value_name = "ID")]` only — no `short`/`long` (it is a
    positional, following the same style as `PunchArgs::time`'s
    `value_name = "TIME"` and `WeekArgs::week_id`'s
    `value_name = "WEEK_ID"`, both positionals with only a
    `value_name` attribute). No `allow_hyphen_values` on this field:
    a bare positional with an integer `value_parser` and a leading
    `-1` is already handled correctly by clap's own negative-number
    detection for numeric positionals (clap recognizes that `-1`
    cannot be a valid flag name and, when the next expected positional
    has a numeric value_parser, tries it as the positional's value) —
    this is different from `Status`'s `date: Option<String>` field,
    which explicitly sets `allow_hyphen_values = true` /
    `allow_negative_numbers` because a `String`-typed field has no
    inherent numeric-ness for clap to infer from. Concretely: `Status`
    uses `#[arg(allow_negative_numbers = true)]` on its `date` field
    for exactly this class of positional-vs-flag ambiguity, but that
    attribute exists because `date`'s type there is `String`, which
    clap cannot otherwise tell apart from a flag-like token; `id`'s
    `u32` value_parser gives clap that signal for free. To be safe and
    explicit rather than relying purely on inference, this plan still
    recommends verifying empirically in the TDD step (§6 below,
    "id=0 accepted" and "negative id rejected" cases) that a bare
    `-1` positional is rejected as a `u32` parse failure and not
    misrouted as an unrecognized flag; if the written test shows clap
    treating `-1` as an unknown flag instead of a value-validation
    error, add `#[arg(allow_negative_numbers = true)]` to this field
    at that point — call this out as a "verify against real clap
    behavior first" step rather than asserting it blind, consistent
    with how the backdated-punches spec itself insists on empirical
    verification (§2.1: "confirmed empirically").

- **`date: Option<String>`**
  - Same type/attribute shape as `PunchArgs::date`/`NoteArgs::date`
    (`Option<String>`, `short`, `long`, `value_name = "DATE"`,
    `allow_hyphen_values = true`) — deliberately identical, since the
    accepted grammar is the same (`YYYY-MM-DD` or `-N`, spec §3).
    `allow_hyphen_values = true` is required here for the same reason
    it's required on `PunchArgs`/`NoteArgs`: without it, clap treats a
    `-N`-shaped value (e.g. `-1`) passed to `--date`/`-d` as an
    attempted flag rather than the option's value, and errors instead
    of accepting it (backdated-punches spec §2, "required for clap to
    accept a `-N` shorthand as the flag's *value*").
  - `short` + `long` (no explicit letter given) — clap derives `-d`
    from the field name `date` automatically, same as
    `PunchArgs`/`NoteArgs` already do; no need to spell out
    `short = 'd'`.
  - Doc comment differs from `PunchArgs`/`NoteArgs` in one substantive
    way: it does not carry those two structs' "must come before NOTE
    text" footgun warning — because `DeleteEntryArgs` has no free-text
    trailing positional for it to collide with (§6 below). Silently
    copying the old warning text here would misdocument this command.
    It otherwise resolves identically to `start`/`stop`/`note`'s
    `--date` (future-checked, hard error on malformed/future input) —
    this task's field/attribute shape is unaffected either way, since
    that resolution happens downstream in `commands.rs` (Task 3), not
    during parsing: clap only captures the raw string here regardless
    of which resolver later consumes it.

- **No `trailing_var_arg` field on this struct at all.** This is the
  key structural difference from `PunchArgs`/`NoteArgs`, covered next.

---

## 5. Why the ordering footgun structurally cannot apply here

The backdated-punches spec's §2.1 footgun is not a general property of
`--date`/`-d` — it is a specific interaction between two clap
mechanisms both present on `PunchArgs`/`NoteArgs`:

1. A field marked `trailing_var_arg = true` (their `note`/`body`
   fields) — once clap starts consuming positionals into this greedy
   trailing collector, it stops re-scanning subsequent tokens for
   named flags like `--date`.
2. `allow_hyphen_values = true` on that same trailing field — which is
   what lets it swallow a token that looks like a flag (`--date`)
   without erroring, instead of `--date` being recognized as a flag at
   parse time.

`DeleteEntryArgs` has neither: its only positional is `id`
(`Option<u32>`, no `trailing_var_arg`, no `allow_hyphen_values`), and
it takes at most one token. There is no free-text, multi-token,
greedy positional on this struct for `--date` to be swallowed into —
once clap has consumed (at most) one token into `id`, every remaining
token is still scanned normally for named flags, `--date`/`-d`
included, regardless of where it appears relative to `id`. This is
exactly what the spec's §6 states and empirically confirms for clap
4.6.6: `mlm delete note 2 --date -1` and `mlm delete note --date -1 2`
both parse to `id: Some(2), date: Some("-1")` — order-independent.

This must still be locked down with an actual parse-level test (§6
below), not just asserted from this reasoning, for the same reason the
backdated-punches spec locked its own (opposite) footgun down: a
future clap upgrade or a refactor that later adds a trailing free-text
field to this struct should be caught by a failing test, not
discovered by a user.

One important asymmetry to flag explicitly in this plan: the *parsed*
independence of `id`/`--date` order does **not** extend to the
recreate-echo string `delete` *prints* (Task 3's concern, spec §5).
That printed string is fed back into `mlm note`, which *does* still
have the footgun on its own args — so the recreate line must place
`--date` before the note body regardless of what order the user typed
`delete`'s own arguments in. This task (`cli.rs` parsing) is unaffected
by that; it's called out here only so the plan doesn't read as
contradicting the milestone plan's footgun-preservation requirement.

---

## 6. Tests to write first (TDD), named per this file's convention

Existing convention observed in `src/cli.rs`'s `#[cfg(test)] mod
tests`: a private `parse(args: &[&str]) -> clap::error::Result<Cli>`
helper wrapping `Cli::try_parse_from`, small per-command extractor
helpers (`start_args`, `note_args`, `week_target_args`) that `match`
on `cli.command` and `panic!("expected .., got {other:?}")` on
mismatch, and test names as descriptive `snake_case` sentences (no
`T<N>`/`P<N>` numeric prefixes on the newer backdated-punches tests —
follow that newer, unnumbered style for these new tests since they're
closer in time/spirit to that block). Add a `delete_target(cli: Cli)
-> DeleteTarget` helper analogous to `start_args`/`note_args`, plus
one further helper `delete_entry_args(target: DeleteTarget) ->
DeleteEntryArgs` (or fold both into one `delete_note_args`/
`delete_punch_args` pair mirroring `start_args`) — either shape is
fine; this plan recommends the two-step version so both `Note(..)`
and `Punch(..)` extraction reuse the same second step instead of
duplicating the `match`.

All new tests belong in the existing `mod tests` block, appended after
the backdated-punches section (a new `// --- delete: cli surface
------` comment banner matching the existing banner style, e.g. `//
--- backdated-punches: --date/-d plumbing ---`).

1. **`delete_note_with_no_id_parses_as_list_mode`** — `parse(&["mlm",
   "delete", "note"])` succeeds; extracted `DeleteEntryArgs.id ==
   None`.
2. **`delete_punch_with_no_id_parses_as_list_mode`** — same, for
   `"punch"`.
3. **`delete_id_and_date_parse_the_same_regardless_of_order`** — the
   direct regression test for §5 above:
   `parse(&["mlm", "delete", "note", "2", "--date", "-1"])` and
   `parse(&["mlm", "delete", "note", "--date", "-1", "2"])` both
   succeed and both yield `id: Some(2), date: Some("-1".to_string())`
   — assert both parses produce equal `DeleteEntryArgs` (derive or
   manually compare `id`/`date` fields; `DeleteEntryArgs` need not
   derive `PartialEq` for this — field-by-field `assert_eq!` calls are
   consistent with how other tests in this file already compare
   `PunchArgs`/`NoteArgs` fields individually rather than deriving
   equality on those structs).
4. **`delete_del_alias_parses_like_delete`** — `parse(&["mlm", "del",
   "note"])` succeeds and matches `Command::Delete(..)`.
5. **`delete_note_n_alias_parses_like_note`** — `parse(&["mlm",
   "delete", "n"])` resolves to `DeleteTarget::Note(..)`.
6. **`delete_punch_p_alias_parses_like_punch`** — `parse(&["mlm",
   "delete", "p"])` resolves to `DeleteTarget::Punch(..)`.
7. **`delete_id_zero_is_accepted_by_clap`** — `parse(&["mlm", "delete",
   "note", "0"])` succeeds; extracted `id == Some(0)`. Comment above
   the test should say explicitly that rejecting `0` is
   `commands.rs`'s job (Task 3), not clap's — this test only pins
   clap's own permissiveness.
8. **`delete_negative_id_is_rejected_by_clap`** — `parse(&["mlm",
   "delete", "note", "-1"]).unwrap_err()`; assert on the resulting
   `clap::error::ErrorKind`. Per §4's caveat: verify empirically what
   kind clap 4.6.6 actually returns for a numeric positional given a
   `-1` token before writing the assertion — expected candidates are
   `ErrorKind::ValueValidation` (value reached the `u32` parser and
   failed) or, if `allow_negative_numbers` turns out to be needed and
   is added, still `ValueValidation` after that fix; do not assume
   `UnknownArgument` without having actually run it, since that would
   indicate the field needs `allow_negative_numbers = true` added
   before this test can pass as written.
9. **`delete_non_numeric_id_is_rejected_by_clap`** — `parse(&["mlm",
   "delete", "note", "abc"]).unwrap_err()`; assert
   `err.kind() == clap::error::ErrorKind::ValueValidation` (a bare
   `str::parse::<u32>()` failure on non-digit input goes through
   clap's value-validation path, same family as the existing
   `WeekTargetArgs` tests' error-kind assertions in this file, e.g.
   `MissingRequiredArgument` in `no_args_is_clap_error`).
10. **`delete_id_past_u32_max_is_rejected_by_clap`** —
    `parse(&["mlm", "delete", "note",
    "4294967296"]).unwrap_err()` (`u32::MAX + 1`, matching the exact
    value the milestone plan and spec §8 both name); assert
    `ErrorKind::ValueValidation`.

Test-writing note: cases 8–10 should each include a short comment
citing which milestone-plan bullet they satisfy ("negative/non-
numeric/overflow id rejected at the clap level"), matching this file's
existing practice of annotating tests with their originating
requirement (e.g. the `T23`/`P1`-style comments above, and the prose
doc-comments on the backdated-punches tests).

No test for "id past current count" or "id == 0 is a hard error"
belongs in this file — those are `commands.rs`/Task 3 concerns (the
milestone plan is explicit that clap-level and app-level rejections
are different code paths with separate coverage); this file only
covers what clap itself accepts or rejects.

---

## 7. Verification

- Targeted test filter for this task's new tests only (fast inner
  loop while writing them):
  ```
  cargo test --lib cli::tests::delete
  ```
  (matches every new test name above, since they all start with
  `delete_`; run without the filter afterward for the full module.)
- Full `cli.rs` test module, to confirm nothing existing regressed:
  ```
  cargo test --lib cli::
  ```
- Full workspace test suite (required before calling the task done,
  per the milestone plan's "Full existing `cli.rs` test suite still
  passes" acceptance criterion):
  ```
  cargo test
  ```
- Clippy, matching this project's existing gate (including the
  cognitive-complexity lint the milestone plan calls out for Task 3 —
  worth running here too since it's a workspace-wide `cargo clippy`
  invocation, even though Task 2's new code is simple plumbing
  unlikely to trip it):
  ```
  cargo clippy --all-targets --all-features -- -D warnings
  ```
  (Adjust flags to match whatever exact invocation the project's CI/
  pre-commit hook uses if it differs — check `.github/workflows/` or
  a pre-commit config for the authoritative flag set before relying on
  this plan's guess verbatim.)

---

## 8. Summary of new/changed items in `src/cli.rs` (for the implementer)

- `Command` enum: one new variant, `Delete(DeleteArgs)`, inserted after
  `Note`, before `Week`, with doc comment and `visible_alias = "del"`
  as given in §1.
- New `DeleteArgs` struct (§2): one field, `target: DeleteTarget`
  (required subcommand, no `Option`, no
  `args_conflicts_with_subcommands`).
- New `DeleteTarget` enum (§3): `Note(DeleteEntryArgs)` (alias `n`),
  `Punch(DeleteEntryArgs)` (alias `p`).
- New `DeleteEntryArgs` struct (§4): `id: Option<u32>` (positional,
  `value_name = "ID"` only), `date: Option<String>` (`short`, `long`,
  `value_name = "DATE"`, `allow_hyphen_values = true`).
- Ten new tests in the existing `mod tests` block (§6), plus one or
  two small extractor helpers (`delete_target`/`delete_entry_args` or
  equivalent) alongside the existing `start_args`/`note_args`/
  `week_target_args` helpers.
- No changes to any existing variant, struct, or test.
