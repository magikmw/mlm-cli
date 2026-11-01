# Changelog

All notable changes to this project are documented in this file.

## [0.5.0] - 2026-10-31

### 🚀 Features

- *(week)* `mlm week --notes` (`-n`) lists each date's notes under its row
- *(export)* `mlm export notes` (`mlm ex n`) prints a date's, a week's or a month's notes as plain lines, e.g. `mlm export notes --month 2026-10 > october.txt`
- *(week, export)* Week and month ids take `-N` for N weeks or months back: `mlm week -1`, `mlm week target -1 35h`, `mlm ex n -w -1`, `mlm ex n -m -1`. A `+N` id gets a hint to use the absolute form
- *(stop)* `mlm stop` warns on stderr when the date it wrote to has an orphaned end, with the same hints `status` shows

### 🔄 Changed

- *(week)* `week target` rejects a target above `168h`
- *(cli)* A closed stdout pipe (`mlm week | head -1`) exits `141` quietly; any other write failure prints `error: cannot write output: <reason>` and exits `1`
- *(delete)* A deleted note whose text holds a backslash prints `deleted note (YYYY-MM-DD): <text>` instead of a recreate command
- *(cli)* `delete note|punch --help` explains how `ID` is numbered
- Minimum supported Rust is 1.95

### 🐛 Bug Fixes

- *(cli)* Commands no longer panic in the repeated hour when clocks go back
- *(cli)* A time typed in the repeated hour stores the earlier pass, and a time inside the spring-forward gap is rejected instead of stored an hour off
- *(cli)* Piping output into a command that exits early (`mlm week | true`) no longer panics

### 📚 Documentation

- README: build steps start with a clone, the dev loop uses a throwaway database, and the delete section explains recreate-line quoting across shells
- README and `--help` match what the commands print

### ⚙️ Miscellaneous Tasks

- `prep_release.sh` runs `cargo deny` (advisories and licenses) and pauses on a finding
- CI's token is read-only
- The crates.io package no longer ships the internal dev docs

## [0.4.0] - 2026-10-04

### 🚀 Features

- *(status, start)* An unclosed start up to 14 days old shows on today's `status` and as a `mlm start` warning, with `fix:` / `or:` commands to close or delete it
- *(status, week)* A start still open from yesterday gets an `[i]` hint, is listed on today's page as `started yesterday, not counted until stopped`, and marks yesterday's `week` row with `[i]`
- *(status, week)* Overlapping stints are flagged `[!]`
- *(status, week)* A carry-in skewed by an anomaly in any earlier week is flagged: `[!] carry-in includes anomalies from N earlier weeks, oldest W`
- *(cli)* `status --help` and `week --help` end with a glossary of the terms and markers on screen

### 🔄 Changed

- *(status, week)* The current week's line is point-in-time: `left to` / `over` the slice due by end of today Mon-Thu (`32h 00m by end of Thursday`), and the full target Fri-Sun
- *(status, week)* A running stint counts everywhere in the current week (day total, worked, fulfillment, week line, `week` rows), marked `(incl. ongoing)`
- *(status, week)* `[!]` anomaly lines carry `fix:` / `or:` suggestions; stop hints name a time window that pairs cleanly, e.g. `(a time between 08:00 and 09:00)`; a refused cross-midnight join is reported on both dates
- *(status)* `done for today` replaces `target already met` on the Day total line
- *(status)* `est. EOD` past tomorrow names the landing day, e.g. `(Mon)`
- *(status)* An unclosed start two or more days old prints as `09:00-?`, not `09:00-now`
- *(status)* `status` rejects a future date and points to `mlm week WEEK_ID`
- *(week)* `week target` prints the stored target, e.g. `Week 2026-41 target: 20h 00m`
- *(cli)* Default log level is `warn`, so a new database no longer logs a migration line on first run; `--verbose` shows it

### 🐛 Bug Fixes

- *(status)* `est. EOD` counts the running stint, so it no longer drifts later while you work
- *(week)* A storage read failure is an error (exit 1) instead of silently falling back to the default target
- *(week)* Viewing a far-future week is fast (one ranged read instead of one per week)
- *(cli)* `mlm week 2026-41 target 20h` explains the argument order; `week target --help` names `[WEEK_ID] <DURATION>`

### 📚 Documentation

- README and SPEC.md: the known-issues list is resolved; each accepted behavior is described where it belongs

### ⚙️ Miscellaneous Tasks

- The pre-commit hook blocks commits when tests fail
- Move aarch64 macOS runners off retiring macos-14

## [0.3.6] - 2026-09-23

### 🚀 Features

- *(status)* Closed week deficit now reads `Total behind`, pairing `Total ahead`
- *(status)* `est. EOD` gains a `(tomorrow)` suffix when it crosses midnight
- *(status)* Day-total reads `required today` on Fri/Sat/Sun instead of repeating the week line

### 📚 Documentation

- README: closed-period `status`/`week` worked examples
- README/SPEC.md: document the `fulfillment = worked + carry-in` expanded format

## [0.3.5] - 2026-09-22

### ⚙️ Miscellaneous Tasks

- Fix a stale test assertion that was failing CI for correct output
- Releases now require a passing CI run on the tagged commit before publishing

## [0.3.4] - 2026-09-22

### 🚀 Features

- *(status)* A cross-midnight stint now shows a visible span cue: `23:30-00:45  (01h 15m, spans to next day)`
- *(status)* The date that received the tail end of a cross-midnight stint now shows where that time came from: `Fri 2026-02-13  (00:45 continues previous day's stint)`
- *(status)* An open stint's duration caption now reflects how stale it is, dropping the number entirely past a day old: `09:00-now  (unclosed)`
- *(status)* The week line now breaks a nonzero carry-in out of the fulfillment figure: `(fulfillment 29h 15m = worked 31h 25m + carry-in -02h 10m / target 40h 00m)`

### 📚 Documentation

- Document the `--date`/`-d` backdating flag for `start`/`stop`/`note`, shipped since 0.2.0 but missing from the command reference until now
- General cleanup: stale terminology, drift between docs, leftover claudish

## [0.3.3] - 2026-09-19

### 🐛 Bug Fixes

- A `stop`/`start` typed at the exact same instant, back-to-back between two real stints, no longer zero-pairs and silently drops the earlier stint's time — it now closes the stint that was already open, as expected
- A session spanning midnight (`start` before, `stop` after) now merges into one clean stint on the day it started, instead of splitting into a dangling open stint and an unrelated flagged anomaly the next day

### 📚 Documentation

- `README.md` gains a "Known issues" section, split out from "Planned" (non-goals), listing rough edges in current behavior that aren't yet fixed

## [0.3.2] - 2026-09-16

### 🐛 Bug Fixes

- `status`'s daily pace hint (and its estimated-EOD projection) now includes carry-in from previous weeks, comparing fulfillment against a per-weekday-prorated slice of the target instead of a flat carry-free daily target
- Pace hint reads "X over" instead of a bare negative number once fulfillment is past the required-by-day figure

## [0.3.1] - 2026-09-14

### 📚 Documentation

- Fix stale 'editing or deleting' claims now that delete is implemented
- Amend NOTES.md decision 5, delete shipped in 0.3.0

## [0.3.0] - 2026-09-14

### 🚀 Features

- *(cli)* Add `delete note`/`delete punch` (aliases `del`, `n`, `p`) — list a date's entries with `-d/--date`, delete by number, prints a ready-to-run command to recreate what was deleted

### 🐛 Bug Fixes

- Collapse embedded `\r`/`\n` in note bodies to a single space, keeping every stored note on one line

## [0.2.0] - 2026-09-13

### 🚀 Features

- *(feat)* Enable backdating punches and notes
- *(cli)* Add -d/--date to start/stop/note, including -N shorthand and 'yesterday'
- *(cli)* Accept -N shorthand with status

## [0.1.5] - 2026-09-13

### 💼 Other Changes

- (release) strip debug symbols from release binaries
- (ci) filter Release commits and fix changelog spacing in git-cliff
- (ci) bump actions/checkout to v6 in ci.yml
- (docs) add SECURITY.md

## [0.1.4] - 2026-09-13

### 💼 Other Changes

- (ci) split release script for changelog review
 
## [0.1.3] - 2026-09-13

### 🚀 Features

- (ux) command aliases

### 💼 Other Changes

- (docs) clean impl plans from project's root
- (ci) prepend, don't overwrite the changelog but actually

## [0.1.2] - 2026-09-13

### 🐛 Bug Fixes

- *(ci)* fix github releases

## [0.1.1] - 2026-09-13

### 🐛 Bug Fixes

- *(ci)* Install minisign from upstream binary, not apt
- *(ci)* Pin sha256 of downloaded minisign binary

## [0.1.0] - 2026-09-12

Initial release.

### 🚀 Features

- `start`/`stop`: log a time punch for today, with an optional note in the same call
- `note`: log a work-log entry independent of any punch
- `status`: today's (or a given date's) stints, notes, and day/week totals — including an estimated end-of-day time while a stint is open
- `week`: a week's per-day totals, carried-over balance, and running total against target
- `week target`: override a week's target hours (defaults to 40h/week, with shortfall/surplus carried into the next week)
- All data stored locally in SQLite (via an embedded, migration-managed schema) — no external service, no account
