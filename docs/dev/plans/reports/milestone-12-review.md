# Milestone 12 — Independent adversarial review (documentation)

> **Archived — historical review/report record.** Not authoritative. Current spec: `docs/dev/SPEC.md`.


Reviewed fresh, adversarially, against `plans/reports/milestone-12-report.md`,
PLAN.md's Milestone 12 acceptance criteria, `README.md`, `AGENTS.md`, and
`src/cli.rs`. Built the binary, ran `cargo build` / `cargo test` / `cargo
clippy --all-targets -- -D warnings` myself, and reconstructed the
implementer's README scenario (same dates — today really is Sat 2026-09-12 —
same punch times) in a scratch `MLM_DB_PATH` to compare real output against
the README byte-for-byte. Cleaned up all temp DBs/files afterward.

## Findings

- **README `status`/`week` examples verified byte-identical** (README.md:84-168
  vs. real binary output) for every value that does not depend on wall-clock
  "now" at capture time. Reconstructed the exact scenario (start 09:00 "reviewed
  open PRs" / stop 13:00 / start 14:05 / stop 17:30 / start 17:45 / note "fixed
  migration runner bug", plus Mon–Fri seeded to match the stated daily totals):
  `status 2026-09-08`, `week`, and `week target 45h` + `week` all matched the
  README text exactly, character for character (headers, column alignment,
  "Carry-in/Worked/Fulfillment/Target" block, day-total lines, notes list). Only
  the today-`status`'s ongoing-stint duration and "est. EOD" naturally differed
  (05h 26m/23:46 for me vs. 05h 19m/23:39 in the README) because that depends on
  the real instant the command was run — expected and not a discrepancy. No
  severity — this is a pass, noted for the record since it was the main thing to
  break.
- **`start`/`stop`/`note`/`week target` confirmed silent on stdout** on success,
  matching README.md:42-45. Verified directly by redirecting stdout/stderr
  separately. No finding.
- **One-time migration log line goes to stderr, not stdout** — on first use of a
  brand-new DB file, `rusqlite_migration` logs `INFO ... migrated to version 1`.
  This does not contradict the "silent on success" claim (it's stderr, and only
  fires once per fresh DB), and it isn't mentioned in the README. Severity:
  minor / informational only — not a documentation defect, just a note in case a
  future doc pass wants to mention it.
- **`AGENTS.md`'s "Verifying changes" block runs clean end-to-end**, including
  the exact `MLM_DB_PATH=/tmp/mlm-check.db cargo run -- start "9:00" "note"`
  followed by `... status` — ran it verbatim, got a real status block with the
  note attached and no errors. `cargo build`, `cargo test` (314 total: 307 + 1 +
  2 + 4, all passing, matches the report's claim exactly), and `cargo clippy
  --all-targets -- -D warnings` (clean, zero warnings) all verified myself, not
  taken on faith. No finding.
- **`mlm --help` and every `mlm <command> --help` read consistently**, no
  leftover `log`-command wording, no contradictions. `TIME`'s description
  ("Time of day (HH:MM, HHMM or HH, 24h). Defaults to now") is defined once on
  the shared `PunchArgs` struct and displays identically under `start --help`
  and `stop --help`. No finding.
- **`README.md` lists all six real commands** (`start`, `stop`, `note`,
  `status`, `week`, `week target`) with syntax matching `src/cli.rs`'s actual
  arg shapes (`mlm start [TIME] [NOTE...]`, `mlm note NOTE...` required vs.
  optional-note siblings, `mlm week target [WEEK_ID] DURATION`, etc.). No `log`
  command anywhere in README, AGENTS.md, or `cli.rs`/its `--help` output. No
  finding.
- **`src/cli.rs` parsing behavior is untouched** — `git show`/`git log -p` on
  the Milestone 12 commit (`b5fbb13`) shows exactly one line changed in
  `cli.rs`: the `Cli::verbose` doc comment (`"Enable debug-level logging (PLAN.md
  interface contract 12)."` → `"Enable debug-level logging."`). No attribute
  (`num_args`, `required`, `allow_hyphen_values`, `trailing_var_arg`,
  `args_conflicts_with_subcommands`, etc.) changed. Cross-checked that
  `--verbose` genuinely does raise the log level in `src/main.rs` (so the
  trimmed doc comment is still accurate, not just shorter). No finding.
- **README's Windows data path fix is correct and verified**, not just
  asserted. `src/db.rs`'s own doc comment (line 5) says
  `%APPDATA%\mlm\data\mlm.db`, matching the corrected README.md:27. The old
  README text (`%APPDATA%\mlm\mlm.db`, missing `\data\`) was indeed wrong pre-
  milestone. No finding — correctly fixed, and the acceptance criteria's caveat
  ("already accurate" bar for leaving Data location alone) rightly did not apply
  here.
- **"Build" section scope discipline**: diff shows only the leftover
  scaffold-era note text (`"working on scaffold"` → `"working on mlm"`) changed;
  the `cargo build` / `cargo run` shape itself is untouched, and re-running it
  works. No finding.
- **Minor pre-existing inconsistency, out of this milestone's scope**: README's
  "Stack" section (README.md:15-17) describes `ratatui`/`crossterm` as powering
  a "terminal dashboard," stated matter-of-factly alongside the other
  already-implemented dependencies, while `AGENTS.md`'s "Current state" section
  correctly lists the dashboard as a not-yet-implemented stretch item. Also,
  README's own "Stretch goals" section (README.md:19-22) only lists shell-prompt
  integration, omitting the dashboard that AGENTS.md separately calls a stretch
  item — so a reader of README alone could believe the dashboard already exists.
  This predates Milestone 12 (not touched in the M12 diff) and the AC scoped
  this milestone to Usage/Data-location/Build/AGENTS.md/cli.rs comments only, so
  it's not a scope violation by the implementer. Severity: minor — flagging for
  a future pass, not a blocker for this milestone.

## Scope discipline

Diffed `README.md` and `AGENTS.md` for the Milestone 12 commit (`b5fbb13`)
against its parent. Confirmed: "Status" section removed (accurately stale),
"Usage (planned)" replaced with full "Usage," "Data location" Windows path
fixed (was wrong, fair to fix per the report's own justification), "Build"
left alone except for the anachronistic note text, `AGENTS.md`'s "What this
is"/"Current state"/"Verifying changes" updated as described. Nothing outside
this milestone's stated scope was touched.

## Verdict

APPROVE — the documentation matches reality: every runnable example was
independently reproduced and matches byte-for-byte apart from the two values
that are expected to depend on real wall-clock time, `cargo build`/`test`/
`clippy` all confirmed clean and matching the claimed test count (314), the
`cli.rs` change is proven comment-only via diff, and `--help` output is
internally consistent with no scaffold leftovers. Only a pre-existing, out-of-
scope minor doc inconsistency (README Stack section vs. AGENTS.md on dashboard
status) is worth a future look.
