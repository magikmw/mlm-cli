# Plan: orphan-stop-warning

Spec: `docs/dev/specs/2026-10-24-orphan-stop-warning.md` (locked). Reviews: `docs/dev/plans/reports/orphan-stop-warning-spec-review.md`, `docs/dev/plans/reports/orphan-stop-warning-spec-round-2-review.md`. Decision log: NOTES.md entries 111-113.

**Altitude note**: scope of work only. No code, no pseudocode, no function or type names. Task plans choose the identifiers. The one file name pinned below (the new module) is pinned for ownership and for AGENTS.md.

Read for this plan: the spec in full; `src/commands.rs` (`start`/`stop` handlers, `start`'s best-effort warning path, the shared write helper); `src/anomalies.rs` (fact struct, block rendering, E8 and refused-join lines, fact builder); `src/lookback.rs` (scan, line facts, block and stderr forms, yesterday hint, overnight decision, today-page filter); `src/stint.rs` (`classify`, `classify_at`, splice gate, refused-join predicate); `src/render.rs` (stop-hint helpers, next-punch helper); `src/status.rs` (where the lookback lines and the E8 block are composed); `tests/start_hint_cli.rs`; SPEC.md section index and §1.2a; NOTES.md 111-113.

## Goal

A `stop` that leaves an orphaned end, a refused join or a splice of 12h or more on the written date `D` or on `D+1` warns on stderr, then prints today's lookback lines. The write, stdout and exit code are unchanged, and a `stop` that triggers nothing stays silent. The E8 block's `fix:` hint gains a `(a time between P and T)` window, and an orphan that shares its minute with another end gets a two-line, delete-only block. Both changes apply to `status` and to `stop`'s orphan block. The yesterday hint on today's `status` drops its `or: mlm stop` line once the start is 12h old. SPEC.md, README.md and AGENTS.md are updated to match (spec §9 and the interaction section).

## Architecture (prose)

### Current shape

- `stop` calls the shared write helper and returns. It never prints. `start` runs a best-effort lookback after its write and prints each finding's stderr form, swallowing any read failure.
- `src/anomalies.rs` builds one fact struct per date from three raw punch buckets (previous date, the date, next date). The E8 facts are only a list of orphan times. The E8 lines and the refused-join line are private renderers that hardcode the `[!]` prefix and the `(a time before T)` hint.
- `src/lookback.rs` owns the 14-day scan. Each entry carries line facts (the next punch after the open start, and whether today has punches). The yesterday hint's `or:` line keys off those two facts only. Today's `status` filters the yesterday hint through the overnight decision, which drops it when a refused join names yesterday. `start`'s stderr path applies no such filter.
- `src/stint.rs`'s splice-aware classification already puts a spliced stint into the earlier date's completed list, with its real end on the later date and its real minutes. That is enough to find a long splice without changing pairing.
- Punch instants are stored minute-granular (`src/storage.rs:102`, `:207-213`), so "same minute" means "same instant".

### Target shape

1. **E8 facts carry their window.** Each orphan on a date carries, beside its time, what its hint needs: whether another end shares its minute, and the latest earlier punch `P` (in pairing order, before `T`'s minute) together with whether `P`'s minute is inside the window. The window includes `P`'s minute when `P` closed a stint and excludes it when `P` is itself an orphan. The fact builder fills this from the date's bucket and its splice-aware classification. Nothing else in the fact struct changes meaning.
2. **One renderer for the E8 hint lines.** `src/anomalies.rs` produces the hint lines for one orphan (the three-line `fix:`/`or:` form or the two-line delete-only form) in one place. `status`'s block prefixes them with `[!] orphaned end at T (no matching start)`. `stop`'s block prefixes them with `warning: orphaned end on <weekday date> at T (no matching start)`. The refused-join sentence likewise has one source with a swappable `[!]`/`warning:` prefix. This keeps spec §4.1's "word for word" rule true by construction.
3. **Yesterday hint 12h rule.** The lookback's line facts gain the start's age at `now` (or an equivalent "under 12h" flag). The `[i]` hint's `or:` line prints only when the existing two conditions hold and the start is under 12h old. The `fix:` line and the stderr yesterday form are unchanged.
4. **`stop`'s warning pass** lives in a new module `src/stop_warnings.rs`, called from `stop` after the committed write. It does all reads first, builds every line, then prints. Any read error means no output.
   - Reads: the punches needed to classify `D` and `D+1` with their neighbours (`D-1` through `D+2`, or `D-1` through `D+1` when `D` is today, so `D+1` is never read then), plus today's lookback.
   - Date warnings: for `D`, then `D+1`, one orphan block per orphaned end ascending by instant, then the refused-join line with the previous date, then with the next date. These come from the same fact builder `status` uses.
   - Long-splice blocks: for each splice of 12h or more whose end lies on `D` or `D+1` (`D` first). The splice is the one completed stint on the earlier date `A` whose end is on `A+1`. The block's `fix:` uses the existing stop-hint helper with the first punch on `A` after the start.
   - Lookback lines: only when at least one of the above printed. Today's lookback in `start`'s stderr forms, oldest first, filtered as today's `status` filters it (the yesterday entry only when the overnight decision finds a stint). No own-punch suppression and no carry marker.
5. **Untouched.** Pairing, the splice, totals, `mlm week`, `start`'s warnings, `note`, every `--help` text, stdout and exit codes.

## Global constraints

- Verification set, all clean: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- Pre-commit hook: cognitive complexity at most 15 per function (`clippy.toml`), and line coverage at or above `coverage-baseline.json`. New code ships with tests that exercise it.
- Exact output text comes from the spec (§4.1-§4.5, §5, §6). No paraphrase.
- Durations go through the one formatter (SPEC §4.2).
- `stop` never fails, retargets or refuses because of the warning pass (spec §1.1, §7). A hard error before the write is unchanged.
- Today's `status` stays the single definition of what `stop` prints: the date warnings come from the shared fact builder, the lookback filter from the shared overnight decision.
- Docs stay self-contained (AGENTS.md "Documentation"). No pointer from SPEC/README/`--help` into `docs/dev/specs/` or `docs/dev/plans/`.
- All output ASCII (an `ascii_sweep` test exists).

## Ordering and parallelization

Two tasks, parallel from the start: Task 1 (code) and Task 2 (docs). No dependency graph beyond that.

The code stays in one task because `stop`'s orphan block consumes the E8 hint renderer. Splitting E8/yesterday-hint from `stop` forces a strict sequence (hints first, then `stop`) with no parallel gain, plus a re-alignment hop for the second task. Within Task 1 the order is the same as that sequence would be: E8 window and yesterday-hint rule first, then the `stop` pass on top. If the coordinator wants smaller dispatches, that seam (`src/anomalies.rs` + `src/lookback.rs` first, then `src/stop_warnings.rs` + `src/commands.rs`) is where to cut, sequenced, with `src/lookback.rs` owned by the first half and the stderr-filter addition done there.

Task 2 works from the locked spec text alone, so it does not wait for Task 1.

## Interface contracts to pin before worktrees open

- **Output text.** Every stderr line `stop` prints, the E8 block in both forms, and the yesterday hint are exactly the spec's §4.1-§4.5, §5 and §6 text. Task 2 copies the same examples into SPEC and README.
- **Orphan header.** `stop`'s orphan header names the weekday and date (`warning: orphaned end on Mon 2026-10-05 at 14:16 (no matching start)`). `status`'s header stays `[!] orphaned end at 14:16 (no matching start)`. The lines after the header are identical in both.
- **Thresholds.** A splice warns at 720 minutes or more (exactly 12h warns, 11h 59m is silent). The yesterday hint keeps its `or:` line only while the start is under 720 minutes old at `now`.
- **E8 window rule.** As spec §5: delete-only two-line block when another end shares `T`'s minute; otherwise `(a time before T)` with no `P`, else `(a time between P and T)`. The minute-inclusion rule is described in SPEC prose only and never printed.
- **New module.** `stop`'s warning pass lives in `src/stop_warnings.rs`, listed in `src/lib.rs`. Task 2 names it in AGENTS.md's layout list ("`stop`'s post-write warnings: orphaned ends, refused joins, long splices, then today's lookback"), and amends the `src/lookback.rs` bullet to say the stderr forms serve `start` and `stop`.
- **Flow id.** The FE5 flow is E27 in SPEC §8.2, next to E19.

## Task 1: code

**Spec**: §3, §4, §5, §6, §7, §8, §1.1. Also the `src/lookback.rs` doc-comment part of the interaction section.

**Scope**: E8 facts gain the window and the duplicate-minute flag; one shared renderer for E8 hint lines and the refused-join sentence with a swappable prefix; the yesterday hint's 12h rule; the lookback's stderr lines with today's-page filter; the `stop` warning pass with its reads-first, best-effort shape; module docs in `src/lookback.rs` and `src/commands.rs` that call the stderr forms `start`'s alone are corrected to name `stop`.

**Acceptance criteria.** Each `stop` case asserts exact stderr, empty stdout and exit `0`.

- Orphan on `D` with an empty lookback prints the three-line block with `(a time before T)` and nothing else.
- FE5 (start 2 days ago open, bare `stop` today) prints the orphan block, then the stale-start lines.
- A clean pair with a stale start in the window prints nothing.
- A displaced end (`start 09:00`, `stop 17:00`, then `stop 12:00`) prints a block for `17:00` with `(a time between 12:00 and 17:00)`.
- A pre-existing orphan plus a new clean pair on the same date prints the orphan block.
- Two orphans at different minutes print two blocks, ascending.
- A duplicate same-minute `stop` prints two-line blocks for both.
- A double-tapped `stop` after a splice prints two two-line blocks, then the refused-join line, and no yesterday warning (spec §4.5 third example, exact).
- A backdated `stop` that breaks `D+1`'s splice prints the orphan block for `D+1`.
- When `D` is today, `D+1` is not read.
- Splices of 11h 59m are silent. Exactly 12h and 29h 16m print the long-splice block (spec §4.5 second example for 29h 16m).
- A long splice with a punch after `S` on `A` names the window in `fix:`.
- A backdated `stop` that clears `D`'s E7 and creates a 12h+ splice into `D+1` prints the long-splice block for `D+1`.
- A `D+1` orphan and a long-splice block from one `stop` print in spec §4 order (date warnings for `D` then `D+1`, then long-splice blocks, then lookback).
- An orphan with yesterday's lone unrefused start open prints the orphan block, then the yesterday stderr warning.
- `--date` older than 14 days prints the orphan block and today's lookback, not the written date's.
- A read failure after the committed write prints nothing extra and exits `0`, with the punch stored.
- `status` E8 window cases: no earlier punch gives `(a time before T)`; an earlier closed stint gives the window with `P`'s minute included; an earlier orphan as `P` excludes its minute; an E14 pair as `P` works; another end at `T`'s minute gives the two-line block; an empty window prints as spec §1.2 describes.
- `status` yesterday hint: start under 12h old with today empty keeps the `or:` line; 12h or more drops it; `fix:` unchanged either way.
- Existing tests that assert the old E8 hint or the old `or:` line are updated to the new rule, not deleted.

**Out of scope**: SPEC.md, README.md, AGENTS.md; `--help` text; `start`'s warnings; `mlm week`; pairing and the splice; warnings for overlaps or E7 a `stop` creates; a `--quiet` flag.

**Owns exclusively**: `src/anomalies.rs`, `src/lookback.rs`, `src/stop_warnings.rs` (new), `src/commands.rs`, `src/lib.rs`, `src/stint.rs`, `src/render.rs`, `src/status.rs`, every file under `tests/` (including a new `stop` CLI test file), `coverage-baseline.json` (hook ratchet).

`src/render.rs` and `src/status.rs` are listed because their tests build E8 fact fixtures and assert E8 and hint text. `src/stint.rs` is listed in case the long-splice lookup needs a small predicate there. Neither is expected to change behaviour.

## Task 2: docs

**Spec**: §9 and "Interaction with the project spec"; §1.2 (known issues) for SPEC §1.2a and README's Possible issues.

**Scope and acceptance criteria**:

- SPEC §1.2a replaces "None currently." with the six spec §1.2 items.
- SPEC §3.3 points to §7.4's `stop` warnings.
- SPEC §3.7's "`start`, `stop` and `note` stay silent" names the `stop` exception.
- SPEC §6.2's "not at insert time" sentence names the `stop` warning.
- SPEC §7.1 gives the yesterday hint's `or:` condition the 12h rule, drops the `or:` line from the 15h 30m overnight example, notes the 12h warning in the "joins the start and clears the hint" sentence, and names `stop`'s warnings beside `start`'s where an old start drops out of them.
- SPEC §7.3's E8 block gains spec §5's rules (window, minute inclusion, duplicate-minute two-line form, empty window), with the displaced example. The stop-hint rule's consumer list adds `stop`'s long-splice block.
- SPEC §7.4 gains `stop`'s triggers (§3), blocks (§4.1-§4.3), order (§4), lookback rule (§4.4), examples (§4.5) and best-effort rule (§7). The "write commands print nothing" rule gains the exception.
- SPEC §8.2 gains E27 (FE5 and `stop`'s warnings) next to E19. E8 and E22 change to match.
- README: the write-commands "silent on success" line names the `start` and `stop` warnings; `mlm stop` gains the warning text and the spec §4.5 examples; the "joins the start and clears the hint" sentence notes the 12h warning; the `[i]` example beside `22:00-now (15h 30m so far ...)` drops its `or:` line; Possible issues mirrors the new SPEC §1.2a.
- AGENTS.md: the layout list gains `src/stop_warnings.rs`, and the `src/lookback.rs` bullet names `stop` beside `start`.
- No doc points a reader at `docs/dev/specs/` or `docs/dev/plans/`.

**Out of scope**: all source and tests; `--help` text (spec §9 keeps it unchanged); NOTES.md (coordinator's log); archiving the spec (a later phase).

**Owns exclusively**: `docs/dev/SPEC.md`, `README.md`, `AGENTS.md`.

## Risks and notes

1. **Refused join across `D`/`D+1` prints once.** Resolved by the coordinator (NOTES.md entry 114): spec §3.1 now says a refused join between `D` and `D+1` prints once, as `D`'s line naming `D+1`. Task 1 implements that.
2. **A CLI test breaks on the 12h rule.** `tests/start_hint_cli.rs` "status hint with or line when today is empty" seeds yesterday `18:02` against the real clock and expects the `or:` line. Under the 12h rule that holds only before 06:02 UTC. A yesterday start under 12h old exists only before 12:00 UTC, so the `or:`-present case cannot be a stable binary-level test. It moves to a clock-injected unit test, and the CLI test asserts the 12h-or-more shape (or seeds relative to now and skips when impossible). Task 1 owns this.
3. **Real-clock bare `stop` in existing CLI tests may now warn.** `stop_today_joins_and_clears_the_hint` seeds yesterday `23:00` and runs a bare `stop`. After 11:00 UTC that is a 12h+ splice, which now prints a long-splice warning. The test checks stdout only, so it passes, but any test asserting empty stderr from a bare `stop` must pin the time.
4. **"Word for word" covers the hint lines, not the header.** Resolved: spec §4.1 now says so. The contract above pins it.
5. **Complexity budget.** The ordering in spec §4 (two dates, three kinds of line, conditional lookback) is the likeliest place to exceed 15. The task plan should split classification, block building and printing.
6. **Read-failure test.** Spec §8's "read failure after the write" case needs a connection that writes and then fails to read. `start`'s existing best-effort tests in `src/commands.rs` show the pattern. A binary-level version is optional.
