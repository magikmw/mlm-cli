# Milestone 14, Task 4 — low-level plan: `src/main.rs` dispatch wiring

**Scope:** `src/main.rs` only. Add one new `match` arm to `dispatch()`
routing `Command::Delete` to Task 3's `commands::delete_note`/
`commands::delete_punch` handlers, plus whatever `use` adjustments that
requires. No `cli.rs`/`commands.rs`/`storage.rs` changes (Tasks 1–3
already own those). Nothing in this file is edited by this plan — it
is a plan document only.

**Sources read:** milestone-14 plan (Task 4 section, plus the plan's
own "Architecture"/Global-Constraints framing of this task as "one new
dispatch arm"), `docs/dev/specs/2026-09-13-delete-punches-notes.md`
(skimmed for context — no dispatch-level detail beyond what the
milestone plan already states), `docs/dev/plans/milestone-14-task-2-cli.md`
in full (source of truth for `Command::Delete`/`DeleteArgs`/
`DeleteTarget`/`DeleteEntryArgs`'s exact shapes — Task 3's own plan
file does not exist yet, see below), current `src/main.rs` in full,
and `src/commands.rs`'s existing handler signatures (`start`/`stop`/
`note`, all `pub fn(conn: &mut Connection, now: DateTime<Local>, args:
&SomeArgs) -> anyhow::Result<()>`) as the pattern Task 3's
`delete_note`/`delete_punch` are assumed to follow, per the parent
milestone plan's Task 3 description ("a list/delete handler for notes
and one for punches, matching this file's existing handler shape").
`docs/dev/plans/milestone-14-task-3-commands.md` does not exist in the
tree at the time of writing this plan — Task 3 is upstream of Task 4
and may not have landed yet. **This plan's exact match-arm code
depends on two assumptions about Task 3's finished handlers that must
be re-verified against the actual merged `commands.rs` before writing
the arm for real** (see §3 below): the two function names
(`delete_note`/`delete_punch`) and their signature shape
(`&Connection, DateTime<Local>, &DeleteEntryArgs`) — **`&Connection`,
not `&mut`**: Task 3's plan is explicit that `delete_note`/
`delete_punch` take `&Connection`, since the underlying
`storage::delete_punch`/`delete_note` each do their work as one atomic
`DELETE ... RETURNING` statement with no multi-statement transaction,
unlike `start`/`stop`/`note`. If Task 3 lands with different names or a
different signature, adjust the arm to match reality rather than this
plan's guess — everything else
in this plan (import list, test-module reasoning, smoke-check
sequence, verification commands) is unaffected by that detail.

---

## 1. Exact current dispatch match (quoted verbatim, as the pattern to follow)

Current `src/main.rs`, lines 39–57:

```rust
fn dispatch(cli: &Cli, now: DateTime<Local>) -> anyhow::Result<()> {
    // E6: `db::connect()` returns `Err`, never panics.
    let mut conn = db::connect()?;
    log::debug!("db path: {:?}", db::default_db_path());

    match &cli.command {
        Command::Start(a) => commands::start(&mut conn, now, a),
        Command::Stop(a) => commands::stop(&mut conn, now, a),
        Command::Note(a) => commands::note(&mut conn, now, a),
        // `week`/`week target` full dispatch (Milestone 11's wiring
        // pass): `action: Some(Target(..))` routes to Milestone 8's
        // `week_target::run`; `action: None` renders the week view.
        Command::Week(args) => match &args.action {
            Some(WeekAction::Target(t)) => week_target::run(&conn, now.date_naive(), t),
            None => week_view::run(&conn, now, args),
        },
        Command::Status { date } => status::run(&conn, now, date.as_deref()),
    }
}
```

Two existing arms are the pattern to follow, precisely:

- **The plain single-payload arms** (`Start`/`Stop`/`Note`) —
  `Command::Variant(a) => commands::handler(&mut conn, now, a),`. This
  is the shape for a `Command::Delete(args)` arm if Task 3 exposes two
  *separate* top-level handlers that each take the resolved
  `DeleteEntryArgs` directly and the dispatch itself must first branch
  on `DeleteTarget` (see below) — so `Delete` can't be *exactly* this
  one-liner shape; it needs a nested `match` first, same as `Week`
  does.
- **The nested-match arm** (`Week`) —
  `Command::Week(args) => match &args.action { Some(..) => f(..), None
  => g(..) }`. `Command::Delete(args)` is structurally the same
  situation: `DeleteArgs` carries a nested subcommand enum
  (`DeleteTarget`, per Task 2's plan §2–§3: `target: DeleteTarget`,
  itself `Note(DeleteEntryArgs) | Punch(DeleteEntryArgs)`) that must be
  matched to pick the right handler. The only structural difference
  from `Week`'s arm is that `DeleteTarget` is *not* wrapped in
  `Option` (Task 2's plan is explicit that `target` is a required
  subcommand, not `Option<WeekAction>`-shaped) — so there's no `None`
  arm to write; `Note`/`Punch` are the only two variants and both are
  required to be handled.

---

## 2. The new match arm

Insert **after the `Command::Note(a) => …` arm, before the `Week`
arm's comment block** — this mirrors Task 2's own placement rationale
for `Command::Delete` in `cli.rs` (§1 of the Task 2 plan: "after
`Note`, before `Week` — keeps the single-entry-recording commands
grouped together ahead of the view/report commands"). Matching that
same relative ordering here keeps `cli.rs`'s enum declaration order and
`main.rs`'s dispatch order visually parallel, which is how the existing
four arms already line up with the existing four `Command` variants
one-to-one, top to bottom. This is a style call (nothing in Rust or
clap depends on match-arm order), but it's the placement this plan
proposes.

```rust
        Command::Note(a) => commands::note(&mut conn, now, a),
        // `delete note`/`delete punch`: list mode (no id) or delete
        // mode (id given), per Milestone 14. Both branches share one
        // required subcommand enum (`DeleteTarget`, no `Option`
        // wrapper, unlike `Week`'s `action`) so there is no `None` arm.
        Command::Delete(args) => match &args.target {
            DeleteTarget::Note(a) => commands::delete_note(&conn, now, a),
            DeleteTarget::Punch(a) => commands::delete_punch(&conn, now, a),
        },
        // `week`/`week target` full dispatch (Milestone 11's wiring
        // pass): `action: Some(Target(..))` routes to Milestone 8's
        // `week_target::run`; `action: None` renders the week view.
        Command::Week(args) => match &args.action {
```

Full resulting `match` block (for the implementer's direct reference —
copy this whole block in, not just the inserted lines, since the
surrounding arms are unchanged and shown here only to make the
insertion point unambiguous):

```rust
    match &cli.command {
        Command::Start(a) => commands::start(&mut conn, now, a),
        Command::Stop(a) => commands::stop(&mut conn, now, a),
        Command::Note(a) => commands::note(&mut conn, now, a),
        // `delete note`/`delete punch`: list mode (no id) or delete
        // mode (id given), per Milestone 14. Both branches share one
        // required subcommand enum (`DeleteTarget`, no `Option`
        // wrapper, unlike `Week`'s `action`) so there is no `None` arm.
        Command::Delete(args) => match &args.target {
            DeleteTarget::Note(a) => commands::delete_note(&conn, now, a),
            DeleteTarget::Punch(a) => commands::delete_punch(&conn, now, a),
        },
        // `week`/`week target` full dispatch (Milestone 11's wiring
        // pass): `action: Some(Target(..))` routes to Milestone 8's
        // `week_target::run`; `action: None` renders the week view.
        Command::Week(args) => match &args.action {
            Some(WeekAction::Target(t)) => week_target::run(&conn, now.date_naive(), t),
            None => week_view::run(&conn, now, args),
        },
        Command::Status { date } => status::run(&conn, now, date.as_deref()),
    }
```

Notes on the exact code above:

- **`&conn` (not `&mut conn`)**: unlike `start`/`stop`/`note`,
  `delete_note`/`delete_punch` do their write as a single atomic
  `DELETE ... RETURNING` statement inside `storage::delete_punch`/
  `delete_note` (Task 1), with no multi-statement transaction to hold
  open across intermediate steps — Task 3's plan is explicit that its
  `delete_note`/`delete_punch` handlers therefore take `&Connection`,
  not `&mut Connection` (`status`/`week_view`/`week_target` are the
  existing precedent for `&Connection`-taking handlers in this file;
  `commands::start`/`stop`/`note` are the precedent for `&mut
  Connection` ones, needed because *they* build a punch+optional-note
  write together). `mut conn` stays declared at the top of `dispatch`
  regardless (other arms still need it), and passing `&conn` from a
  `mut` binding is always fine — this is a plain shared reborrow, not a
  declaration change.
- `commands::delete_note(&conn, now, a)` /
  `commands::delete_punch(&conn, now, a)` — **verify these two
  function names and this `&Connection` signature against the actual
  merged `commands.rs` before writing this arm for real** (see the
  header disclaimer above). The parent milestone plan's Task 3 section
  names them exactly this way ("`delete_note`/`delete_punch` matching
  the existing `punch()`/`note()` handler shape" for the argument
  order, though *not* for mutability — Task 3's plan deliberately
  diverges from `note`'s `&mut Connection` for the reason above), but
  Task 3 is upstream and unlanded as of this writing, so this is the
  one part of this plan an implementer must cross-check rather than
  transcribe blindly. (An earlier draft of this plan assumed `&mut
  Connection` by direct analogy to `note`'s signature without checking
  Task 3's actual text — corrected here after a cross-plan consistency
  review caught the mismatch; passing `&mut conn` where `&Connection`
  is expected happens to still compile via Rust's implicit reborrow,
  so this was a documentation bug, not a build-breaking one, but it's
  fixed now rather than left for an implementer to stumble on.)
- List mode vs delete mode (id present/absent) is **not** branched on
  here — that decision lives entirely inside Task 3's
  `delete_note`/`delete_punch` (they take the whole `&DeleteEntryArgs`,
  `id: Option<u32>` included, and decide internally). `main.rs` only
  routes on `DeleteTarget` (note vs. punch), exactly the same
  boundary `Week`'s arm draws today (`week_target::run` and
  `week_view::run` each receive their whole args struct and decide
  internally what to do with it; `main.rs` only routes on the
  `Option<WeekAction>` shape).

---

## 3. Exact imports to add/adjust

Current import lines (lines 1–4):

```rust
use chrono::{DateTime, Local, Timelike};
use clap::Parser;
use mlm::cli::{Cli, Command, WeekAction};
use mlm::{commands, db, status, week_target, week_view};
```

Change **only line 3**:

```rust
use mlm::cli::{Cli, Command, DeleteTarget, WeekAction};
```

Reasoning:
- `DeleteTarget` is matched by name in the new arm's nested `match`
  (`DeleteTarget::Note(a)` / `DeleteTarget::Punch(a)`), exactly the
  same reason `WeekAction` is already imported for the existing
  nested match on `Some(WeekAction::Target(t))`/`None`. Without this
  import, `DeleteTarget::Note`/`DeleteTarget::Punch` don't resolve —
  a plain compile error, not a warning.
- Alphabetical-ish ordering within the `use mlm::cli::{...}` braces:
  the existing list (`Cli, Command, WeekAction`) is already
  alphabetical; inserting `DeleteTarget` between `Command` and
  `WeekAction` keeps that ordering (`Cli` < `Command` < `DeleteTarget`
  < `WeekAction`) — not load-bearing for compilation, but matches this
  line's existing style and is what `cargo fmt`/`rustfmt` would leave
  untouched either way (rustfmt does not reorder `use` item lists
  inside one braced group unless `imports_granularity`/`reorder_imports`
  nightly options are enabled, which this project's default `rustfmt`
  config does not opt into — confirm this compiles clean under `cargo
  fmt --check` in verification regardless of the exact order chosen;
  don't rely on rustfmt to fix ordering for you here).
- **`DeleteArgs` and `DeleteEntryArgs` are not imported** — `main.rs`
  never names either type directly. `Command::Delete(args)` binds
  `args: &DeleteArgs` structurally via pattern matching (no explicit
  type annotation needed, same as `Command::Week(args)` above it never
  names `WeekArgs` either), and `DeleteEntryArgs`-typed values (`a` in
  each inner arm) are passed straight through to `commands::delete_note`/
  `commands::delete_punch` without `main.rs` ever needing to spell
  their type out. Importing either would be dead/unused and would
  trip clippy's/rustc's unused-import lint.
- **Line 4 (`use mlm::{commands, db, status, week_target, week_view};`)
  needs no change.** `commands` is already imported as a module path
  (`commands::start`/`stop`/`note` are already called via `commands::`
  prefix); `commands::delete_note`/`commands::delete_punch` are new
  *functions* inside that already-imported module, not new module
  paths — no new top-level import required for them, exactly as no
  new import was required when `note` was added to `commands.rs`
  originally alongside `start`/`stop`.

---

## 4. Whether a new unit test is warranted here

**No new unit test in `src/main.rs`'s `#[cfg(test)] mod tests` block
for the new arm itself.** Confirmed, not just deferred to the parent
plan's say-so — grounded in what's actually in this file today:

- The existing `mod tests` block (lines 72–86) contains exactly two
  tests, `ok_exits_zero` and `err_exits_nonzero`, and both test
  **only `exit_code()`** — a pure function taking an already-computed
  `&anyhow::Result<()>` and mapping it to an integer. Neither existing
  test calls `dispatch()` at all, let alone exercises a specific
  `Command` variant's routing. There is no precedent in this file for
  a per-variant dispatch-routing unit test — not for `Start`, not for
  `Stop`, not for `Note`, not for `Week`'s two-way branch, not for
  `Status`. Four existing `Command` variants, including one
  (`Week`) with the exact same nested-match shape this plan is adding,
  already went in with **zero** dedicated dispatch-arm tests. Adding
  one now for `Delete` alone would be new, unrequested test coverage
  inconsistent with every sibling arm in this file, not a gap-fill.
- Why `dispatch()` isn't unit-tested directly: it opens a real
  database connection via `db::connect()?` as its first line (line
  41) — a unit test calling `dispatch()` would need either a real
  filesystem-backed DB (via `MLM_DB_PATH`) or a way to inject a
  connection, and this file has neither a test harness for that nor
  any existing test attempting it. That's consistent with this
  project's actual layering: `commands.rs`/`status.rs`/`week_view.rs`/
  `week_target.rs` each take an already-open `Connection` and are
  unit-tested directly and thoroughly in their own files (Task 3's
  acceptance criteria list extensive `commands.rs`-level tests for
  exactly the `delete_note`/`delete_punch` logic this arm calls);
  `main.rs`'s `dispatch()` is deliberately left as thin, untested-at
  -the-unit-level wiring, with real end-to-end routing coverage
  supplied by the CI e2e smoke script instead (Task 6, which — per
  the parent milestone plan's dependency graph — explicitly waits on
  this task precisely so it can exercise the compiled binary's real
  dispatch).
- This matches the parent milestone plan's own Task 4 framing
  exactly: its acceptance criteria ask for routing to work "when run
  as the real compiled binary" and for "a manual smoke check" — never
  a unit test — and its "Global Constraints" architecture section
  calls this task "one new dispatch arm," full stop, with the e2e
  coverage explicitly assigned to Task 6, not this one.

**Conclusion: confirmed, not refuted** — no new unit test belongs in
this file for Task 4. The two existing `exit_code` tests are left
untouched (they're unrelated to this change and still pass unmodified
after the new arm is added, since `exit_code` itself doesn't change).

---

## 5. Manual smoke-check sequence

Per `AGENTS.md`'s documented convention (lines 33, 60–61, 76, 79): the
database path is overridden with the `MLM_DB_PATH` environment
variable, pointed at a scratch file, so this check never touches a
real/default `mlm` database. Run from the repo root, after the new arm
compiles:

```sh
# Build once.
cargo build

# Fresh scratch db for this smoke check only.
export MLM_DB_PATH=/tmp/mlm-task4-smoke.db
rm -f "$MLM_DB_PATH"

# 1. Seed a punch and a note for today.
cargo run -- start 09:00
cargo run -- stop 17:00
cargo run -- note "task 4 smoke test note"

# 2. List mode (no id): confirm both dispatch through and print
#    numbered entries without deleting anything.
cargo run -- delete punch
cargo run -- delete note

# 3. Confirm status shows the seeded entries before deleting.
cargo run -- status

# 4. Delete mode: delete punch id 1 and note id 1 (ids per the list
#    output from step 2 — adjust if list mode numbers them
#    differently than assumed here).
cargo run -- delete punch 1
cargo run -- delete note 1

# 5. Confirm the deleted entries are gone from status.
cargo run -- status

# 6. Alias smoke check: `del`, `n`, `p` all route identically.
cargo run -- start 09:00
cargo run -- del p   # list mode via aliases
cargo run -- del n

# Clean up the scratch db.
rm -f "$MLM_DB_PATH"
unset MLM_DB_PATH
```

Expected outcomes at each step, to actually eyeball rather than just
run:
- Step 2's list output shows the seeded punch/note, numbered from `1`.
- Step 3's `status` shows an 8h day total and the note body.
- Step 4 exits `0` for both deletes (check `echo $?` if not obvious
  from output) and, per the milestone plan's recreate-echo
  requirement, each delete additionally prints a ready-to-run
  recreate command to stdout — worth reading, not just discarding, to
  confirm the printed command looks sane (a quoted note body, a
  `--date` only if not today) even though verifying it byte-for-byte
  is Task 3's/Task 6's job, not this task's.
- Step 5's `status` no longer shows the deleted punch/note (day total
  drops to 0, note body is gone).
- Step 6 confirms `del`/`p`/`n` all parse and route without error —
  this is really re-confirming Task 2's aliases survive end-to-end
  through this task's new arm, not testing anything new in `main.rs`
  itself.

If any step fails to compile or panics, that's this task's own bug
(the new arm), not Task 3's — Task 3's handlers are assumed
independently tested and merged before this task starts (dependency
order per the milestone plan's Batch C).

---

## 6. Verification

- Full workspace build (confirms the new arm compiles against Task 3's
  actual merged handler signatures — the point where this plan's §3
  disclaimer either checks out or needs adjusting):
  ```sh
  cargo build
  ```
- Full test suite (per the milestone plan's Task 4 acceptance
  criterion, "Full test suite passes" — this file's own two
  `exit_code` tests plus every other crate module's tests, since nothing
  about this change should regress anything):
  ```sh
  cargo test
  ```
- Formatting gate (matches the pre-commit hook's own first gate,
  `.githooks/pre-commit` "0. Formatting gate" — run this before
  clippy since an unformatted new arm would otherwise also trip
  clippy's own opinions about the same code):
  ```sh
  cargo fmt --check
  ```
- Clippy, **both of this project's two separate gates**, run and
  verified independently — passing one does not imply the other (per
  the milestone plan's own "Worktree & Review Protocol" and Task 3's
  verification note, which this task's acceptance criteria explicitly
  inherit via "clippy clean"):
  1. CI's plain gate, exactly as `.github/workflows/ci.yml` runs it
     (line 50):
     ```sh
     cargo clippy --all-targets -- -D warnings
     ```
  2. The pre-commit hook's additional cognitive-complexity pass,
     exactly as `.githooks/pre-commit` runs it (lines 64–65) — the new
     arm's nested `match` is small (two inner arms, no branching logic
     of its own) and should not be anywhere near the configured
     threshold, but run this for real rather than assuming that from
     inspection alone, consistent with this project's "verify
     empirically" norm elsewhere in these plans:
     ```sh
     cargo clippy --all-targets --quiet --message-format=json \
         -- -W clippy::cognitive_complexity
     ```
     (Pipe through `jq` and compare against `clippy.toml`'s
     `cognitive-complexity-threshold` the way the hook itself does if
     the raw JSON is hard to eyeball — see `.githooks/pre-commit`
     lines 64–82 for its own exact parsing logic if reproducing that
     check by hand.)
- After all of the above are green, run the manual smoke-check
  sequence in §5, then proceed to
  `superpowers:finishing-a-development-branch` per the milestone
  plan's Worktree & Review Protocol (verify once more there, then its
  merge/PR/keep menu — never merge silently).

---

## 7. Summary of new/changed items in `src/main.rs` (for the implementer)

- Line 3: add `DeleteTarget` to the `use mlm::cli::{...}` import list
  (between `Command` and `WeekAction`, alphabetical).
- Line 4: unchanged.
- `dispatch()`'s `match &cli.command { ... }`: one new arm inserted
  after `Command::Note(a) => …`, before the `Week` arm's comment block
  (§2 above), routing `Command::Delete(args)` through a nested `match
  &args.target { DeleteTarget::Note(a) => …, DeleteTarget::Punch(a) =>
  … }` to `commands::delete_note(&conn, now, a)` /
  `commands::delete_punch(&conn, now, a)` respectively (note: `&conn`,
  not `&mut conn` — see §2's corrected reasoning) — **verify those two
  function names/signatures against the actually-merged `commands.rs`
  first**, per this plan's header disclaimer and §2's note.
- No changes to `run()`, `exit_code()`, or the existing `mod tests`
  block.
- No new tests added (§4).
