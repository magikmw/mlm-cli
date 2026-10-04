# mlm

_Pronounced "mlem"._

Simple CLI time tracker. Quickly log start/stop points for the current
date plus a short note of what you did, stored in SQLite.

## Features

- `start`/`stop` — log a punch for today, with an optional note
  attached in the same call
- `note` — log a work-log entry independent of any punch
- `status` — a day's stints, notes, and day/week totals, with a
  "how much is left today" estimate that counts the running stint while
  a stint is open
- `week` — a week's per-day totals plus carry-in/fulfillment against
  target
- `week target` — override a week's target hours and print the stored
  target back; any shortfall or surplus carries into the next week
- `delete note`/`delete punch` — list a date's entries and delete one
  by number; prints a ready-to-run command to recreate what was
  deleted
- Everything stored locally in SQLite — no account, no external
  service
- Cross-platform: Linux (x86_64 and ARM64), Windows (MSVC), and macOS
  (Intel and Apple Silicon)

## Planned

Not implemented yet:

- Editing a punch/note after it's entered (deleting is implemented —
  see `delete note`/`delete punch` above; the correction path is
  delete-then-recreate, not in-place edit)
- Per-project tagging on notes/stints
- A terminal dashboard (the deps are already in, the UI isn't built)
- Shell prompt integration (current tracking state in PS1/starship)
- Non-ISO week conventions
- 12-hour (AM/PM) time input — 24h only for now
- `+N`/`-N` relative week notation for `week`'s week-id argument
  (`status`'s `DATE` argument already accepts `-N`)

### Possible issues

Shipped behavior that's rough or confusing in a way worth fixing
later:

- A stint started yesterday and still open after midnight is not on
  today's page, so it gets no live credit and no `est. EOD`. Its minutes
  leave the day total, `Worked`, `Fulfillment` and the week line until
  `stop`, and the week line jumps at 00:00. When this happens past
  Sunday midnight, the closed week's `Worked` drops by those minutes.
  Today's `status` shows an `[i]` hint for the open start that points to
  `stop`.

## Install

```sh
cargo install mlm
```

Needs a Rust toolchain (1.85+, edition 2024) — install one via
[rustup](https://rustup.rs) if you don't have one. No other system
dependency: `rusqlite`'s `bundled` feature compiles SQLite from
source, so this works the same on Linux, Windows (MSVC), and macOS.
Prebuilt binaries for all five targets (Linux x86_64/ARM64, Windows
x86_64, macOS Intel/Apple Silicon) are attached to each
[release](https://github.com/magikmw/mlm-cli/releases) — installable
directly via [`cargo binstall mlm`](https://github.com/cargo-bins/cargo-binstall)
too, signature-verified (see [`SIGNING.md`](SIGNING.md)).

### From source

To build the same optimized, thin-LTO release binary the GitHub
releases ship, rather than `cargo install`ing from crates.io:

```sh
cargo install cargo-dist
dist build --artifacts=local --target <your-triple>
```

`<your-triple>` is whichever of this project's five targets matches
your machine (e.g. `x86_64-unknown-linux-gnu`,
`x86_64-pc-windows-msvc`, `aarch64-apple-darwin`). Without `--target`,
`dist` tries to plan a build for every configured target at once and
refuses outright the moment one of them would need cross-compilation
(it will not cross-compile to macOS, in particular). The built binary
and packaged archive land under `target/<triple>/dist/mlm` and
`target/distrib/`, respectively.

## Stack

- [`clap`](https://docs.rs/clap) — argument parsing (derive API)
- [`rusqlite`](https://docs.rs/rusqlite) (bundled SQLite) — storage
- [`chrono`](https://docs.rs/chrono) — time-of-day parsing / duration math
- [`directories`](https://docs.rs/directories) — platform app-data path
  (Linux, Windows/MSVC, and macOS)
- [`ratatui`](https://docs.rs/ratatui) + [`crossterm`](https://docs.rs/crossterm) —
  terminal dashboard (bar/sparkline charts, text-cell only, no bitmap
  graphics — portable over SSH and on Windows)

## Data location

- Linux: `~/.local/share/mlm/mlm.db`
- Windows: `%APPDATA%\mlm\data\mlm.db`
- macOS: `~/Library/Application Support/mlm/mlm.db`

Override with the `MLM_DB_PATH` environment variable (mainly useful for
tests/scripts, or running against a scratch database).

## Usage

The daily loop is: `start` when you begin working, `stop` when you
break or finish, `note` for anything worth remembering that doesn't
belong on a punch, and `status` any time you want to see today's
stints and how the current week is tracking. `week` gives the same
week-level numbers on their own (handy for a past or future week too),
and `week target` adjusts a week's target when it should be something
other than the default 40h.

Every `start`/`stop`/`note` call is silent on success, and nothing
prints unless something went wrong. That holds on the first run too,
when the database is created. `week target` prints one line confirming
the stored target. `status` and `week` are the commands that produce
output. `delete note`/`delete punch` are a narrow exception too (see
below). A `status` after punching in/out is still how you confirm things
landed correctly. Add `-v`/`--verbose` to any command to see debug log
lines on stderr, including the one-time migration line.

Every command has a 1 character alias, except `delete` (`del`).
I recommend using a 1 character shell alias for `mlm` too, so it's easy to type (I like to use `m`).

### `mlm start|s [TIME] [NOTE...] [-d/--date DATE]`

Record a start punch for today. `TIME` (`HH:MM`, `HHMM` or `HH`, 24h)
defaults to now when recording for today; an optional trailing `NOTE`
also records a work-log note for the same date in the same call.

`-d`/`--date DATE` targets a different date instead of today — either
`YYYY-MM-DD` or `-N` for N days before today (e.g. `-1` = yesterday).
The date must not be in the future. When `--date` targets a day other
than today, `TIME` is required (there's no "now" to default to).

```sh
$ mlm start 09:00 "reviewed open PRs"
$ mlm start --date -1 09:00 "forgot to punch in yesterday"
```

(no output — see `status` below to confirm it landed)

**Footgun**: `--date`/`-d` must come *before* the `NOTE` text on the
command line. `NOTE` is a trailing variadic that swallows everything
after it, including a later `--date` flag — `mlm start 09:00 wrapped
up --date -1` silently records `--date -1` as part of the note text
instead of parsing it as the date flag.

After a successful write, `start` also checks yesterday and dates up to
14 days old for open starts and prints a `warning:` block per date found
to stderr, oldest first. Stdout and the exit code are unaffected either
way. The date you just wrote to draws no warning only when its single
open start is the exact punch this command just inserted. Two or more
open starts there, or a single older open start the new punch left
behind, still warn. A start still open from yesterday warns on every
`start` until you stop it.

```sh
$ mlm start 09:00
warning: unclosed start on Fri 2026-09-25 at 09:00 (5 days ago)
    fix: mlm stop HH:MM --date 2026-09-25  (a time after 09:00)
     or: mlm delete punch --date 2026-09-25  (lists punches; rerun with the start's number)
```

```sh
$ mlm start 09:00
warning: start on Fri 2026-10-02 at 18:02 still open (yesterday)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
```

When a later punch follows the warned start, the `fix:` time is a
window, `(a time between 09:00 and 10:30)`.

### `mlm stop|e [TIME] [NOTE...] [-d/--date DATE]`

Record an end punch for today, or another day with `--date`. Same
argument shape and TIME-required-when-backdated rule as `start`
(including the `--date`-before-`NOTE` footgun above).

```sh
$ mlm stop 13:00
```

### `mlm note|n NOTE... [-d/--date DATE]`

Record a work-log note for today, independent of any punch — for
end-of-day notes or anything with nothing to attach to. `-d`/`--date`
targets a different date the same way as `start`/`stop` (`YYYY-MM-DD`
or `-N`), and must likewise come before the `NOTE` text or it is
silently absorbed into it.

```sh
$ mlm note "fixed migration runner bug"
$ mlm note --date -2 "fixed a bug"
```

### `mlm status|d [DATE]`

Show a date's stints, notes, day total, and the totals for the week
that date falls in. `DATE` accepts `YYYY-MM-DD` or `-N` for N days
before today (e.g. `-1` = yesterday), and defaults to today. `DATE`
must not be after today. `status` for a future date fails with `invalid
DATE "2099-01-01": date is in the future; to view a future week, run mlm
week WEEK_ID`, and `mlm week WEEK_ID` shows a future week.

```sh
$ mlm status
Sat 2026-09-12

Day total:     12h 44m (incl. ongoing), done for today
Week 2026-37:  09h 54m over 40h 00m target (fulfillment 49h 54m / target 40h 00m)

  09:00-13:00  (04h 00m)
  14:05-17:30  (03h 25m)
  17:45-now    (05h 19m, ongoing)

Notes:
  - reviewed open PRs
  - fixed migration runner bug
```

When a date has a pairing problem, `status` prints one or more `[!]`
lines between the day/week lines and the stint list, each with a
`fix:` (and sometimes an `or:`) line underneath naming the exact
command to run — nothing is fixed automatically:

```
[!] unclosed start at 09:00, 5 days old (no stop recorded)
    fix: mlm stop HH:MM --date 2026-09-25  (a time after 09:00)
     or: mlm delete punch --date 2026-09-25  (lists punches; rerun with the start's number)
```

When a later punch follows the open start, the `fix:` time is a window
that pairs cleanly with that start:

```
    fix: mlm stop HH:MM --date 2026-09-25  (a time between 09:00 and 10:30)
```

Overlapping stints are flagged too. A day like `start 09:00, start
10:00, stop 11:00, stop 12:00` pairs into 09:00-12:00 and 10:00-11:00,
and the overlapping hour counts twice in the day total:

```
[!] overlapping stints 09:00-12:00 and 10:00-11:00 (the overlap counts twice)
    fix: mlm delete punch --date 2026-09-25  (lists punches; rerun with the number of the punch to remove)
```

The `week` row for that date gets `[!]` as well.

Today's plain `status` (no `DATE`, or today's own date given
explicitly) also checks dates 2 to 14 days old and appends one such
line for every date it finds with an unclosed start, oldest first, so a
forgotten `stop` from last week still shows up today. Yesterday gets
its own treatment, because an overnight session may still be running. A
lone start still open from yesterday prints a quiet hint, which marks no
`week` row:

```
[i] start on Fri 2026-10-02 at 18:02 still open (yesterday; not counted until stopped)
    fix: mlm stop HH:MM --date 2026-10-02  (a time after 18:02)
     or: mlm stop  (still working: ends it now, across midnight)
```

The `or:` line only shows when no punch follows the open start on
yesterday's date and today has no punches yet. A `stop` typed
as today's first punch joins the start and clears the hint. Yesterday
with two or more open starts prints the `[!]` line `2 open stints on
Fri 2026-10-02 (yesterday)` instead. If an earlier week has a flagged
date, the last line of the page names it:

```
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)
```

An overnight session (a `start` before midnight, `stop` after) joins
into one stint on the earlier date automatically, as long as that date
has exactly one open start and the next date's very first punch is the
matching `end`. Anything messier — a second stray start or end that
day — leaves both punches alone and refuses the join, whatever the
dates' age: a `[!]` line on both dates explains why, and each date's
own `[!]` line for its unclosed start or orphaned end (whichever
applies) carries the fix.

The estimated-EOD line only shows up when `DATE` is today (it needs
"now" to mean anything). The week line prints for the current week
whether or not `DATE` is today, and it always describes now. Its figure
is the point-in-time gap, `required - fulfillment`, where `required` is
the slice of the week's target due by the end of today. Monday through
Thursday that is the target divided by 5 (rounded down to the minute)
times today's weekday number, and the line reads `X left to <required>
by end of <Weekday>` or `X over <required> by end of <Weekday>`. From
Friday on the whole target is due, and the line reads `X left to
<target> target` or `X over <target> target`. Fulfillment (worked time
plus any carry-in), worked time and today's day total all count the
running stint's minutes so far, so they change minute by minute while a
stint runs. `(incl. ongoing)` flags that the day total includes the
running stint. On an earlier date's page an open stint is not counted
and the flag reads `(+ ongoing)`. `est. EOD` is the open start plus (required
figure minus fulfillment without the running stint's minutes), which
equals now plus the `left to` figure. It holds still until you add or
remove a punch. It assumes you keep working without a break. A break
moves it later by the break's length. When the running stint has closed
the gap, the line reads `done for today` instead of a clock time. When
that lands after today, `est. EOD` names the
day it falls on. It reads `(tomorrow)` one day out, `(Mon)` two to six
days out, and `(Mon 2026-10-12)` a week or more out, so a large
remaining figure can put it days away. `mlm week` for the current week
shows the same headline and the same live-inclusive figures. A stint
still open after midnight is not on the new day's page, so it gets no
live credit until you `stop` it. `mlm status --help` and `mlm week
--help` end with a short glossary of these terms. A past date's status
just shows that day's total and its week's numbers. Here `Week
2026-37` is still the current week, so its line matches the one above:

```sh
$ mlm status 2026-09-08
Tue 2026-09-08

Day total:     07h 35m
Week 2026-37:  09h 54m over 40h 00m target (fulfillment 49h 54m / target 40h 00m)

  09:05-16:40  (07h 35m)
```

Both examples above have a `Carry-in` of `00h 00m` (see the `week`
examples below), so the parenthetical only ever shows the short
`(fulfillment X / target Y)` form. The moment the current week's
carry-in isn't zero — routine once `week target`'s carry-in mechanic
has run for a week — that parenthetical expands to spell out how the
fulfillment figure was built, e.g. with a 03h 00m deficit carried in
from the previous week:

```
Week 2026-22:  12h 30m left to 32h 00m by end of Thursday (fulfillment 19h 30m = worked 22h 30m + carry-in -03h 00m / target 40h 00m)
```

The short form appears whenever carry-in is zero; this expanded
`fulfillment F = worked W + carry-in C / target T` form appears
whenever it isn't, and `fulfillment` itself is free to go negative
when a carry-in deficit outweighs what's been worked so far.

A date in an already-closed week shows the same plain total the week
line uses instead of the current week's point-in-time framing — no
fulfillment/target parenthetical either, since that only applies to
the current week:

```sh
$ mlm status 2026-08-25
Tue 2026-08-25

Day total:     07h 50m
Week 2026-35:  Total behind: 02h 10m

  09:10-17:00  (07h 50m)
```

### `mlm week|w [WEEK_ID]`

Show a week's per-day totals plus its carry-in/worked/fulfillment/
target summary. `WEEK_ID` accepts a full id (`YYYY-WW`, e.g. `2026-37`)
or a bare week number for the current year (e.g. `37`), and defaults
to the current week.

```sh
$ mlm week
Week 2026-37 (2026-09-07 - 2026-09-13)

09h 54m over 40h 00m target

  Mon 2026-09-07   08h 15m
  Tue 2026-09-08   07h 35m
  Wed 2026-09-09   08h 15m
  Thu 2026-09-10   07h 45m
  Fri 2026-09-11   05h 20m
  Sat 2026-09-12   12h 44m (incl. ongoing)
  Sun 2026-09-13   00h 00m

Carry-in:      00h 00m
Worked:        49h 54m
Fulfillment:   49h 54m
Target:        40h 00m
```

When any row in the table above carries `[!]`, one line follows after
a blank line:

```
[!] needs a look: run mlm status DATE for details and fixes
```

No footer line prints when nothing in the week is flagged.

An unfixed anomaly in a past week also throws off every later week's
`Carry-in`, since carry-in is just the prior week's fulfillment against
target. When any earlier week has a flagged date, the Carry-in figure
gets a `[!]` and a line naming how many earlier weeks are affected and
the oldest one sits above the Carry-in block. It goes directly under
the footer line when a row is flagged, and takes the footer's place
otherwise, with one blank line before the Carry-in block:

```
[!] carry-in includes anomalies from 2 earlier weeks, oldest 2026-38 (see mlm week 2026-38)

Carry-in:      -80h 00m  [!]
```

With one affected week the line reads `from 1 earlier week, 2026-38 (see
mlm week 2026-38)`. The line also prints on `status` (see above). It
stays until the old anomaly is fixed. If `mlm week` cannot read the
database, it prints an `error:` line and exits 1 instead of showing
default figures.

### `mlm week|w target [WEEK_ID] DURATION`

Set an absolute target override for a week (default target is 40h
when no override exists). `WEEK_ID` accepts the same forms as `week`
and defaults to the current week; `DURATION` uses the human format
(`20h`, `33h30m`, `45m`), never raw minutes, and is always the last
token. On success it prints one line with the stored target:

```sh
$ mlm week target 45h
Week 2026-37 target: 45h 00m
$ mlm week
Week 2026-37 (2026-09-07 - 2026-09-13)

04h 54m over 45h 00m target

  Mon 2026-09-07   08h 15m
  Tue 2026-09-08   07h 35m
  Wed 2026-09-09   08h 15m
  Thu 2026-09-10   07h 45m
  Fri 2026-09-11   05h 20m
  Sat 2026-09-12   12h 44m (incl. ongoing)
  Sun 2026-09-13   00h 00m

Carry-in:      00h 00m
Worked:        49h 54m
Fulfillment:   49h 54m
Target:        45h 00m
```

The duration echoes in the standard format, so `90m` prints as `01h 30m`.
`WEEK_ID` goes after `target`. Typing it before `target` fails with a
reminder and exit code 2:

```sh
$ mlm week 2026-41 target 20h
error: put WEEK_ID after target: mlm week target 2026-41 20h
```

`mlm week target --help` shows the arguments as `[WEEK_ID] <DURATION>`.

A past (or future) week has no "today" to frame a deadline against, so
its headline is a plain total behind or ahead instead — everything
else is the same shape:

```sh
$ mlm week 2026-35
Week 2026-35 (2026-08-24 - 2026-08-30)

Total behind: 02h 10m

  Mon 2026-08-24   08h 15m
  Tue 2026-08-25   07h 50m
  Wed 2026-08-26   08h 00m
  Thu 2026-08-27   07h 45m
  Fri 2026-08-28   06h 00m
  Sat 2026-08-29   00h 00m
  Sun 2026-08-30   00h 00m

Carry-in:      00h 00m
Worked:        37h 50m
Fulfillment:   37h 50m
Target:        40h 00m
```

### `mlm delete|del note|n [ID] [-d/--date DATE]`

List or delete today's (or another date's) notes. Run with no `ID` to
list that date's notes numbered `1..N`; run again with a number to
delete that entry — deleting prints a ready-to-run command to recreate
it. `-d`/`--date` targets a different date the same way as
`start`/`stop`/`note` (`YYYY-MM-DD` or `-N`), and defaults to today.

```sh
$ mlm delete note --date 2026-09-10
1  fixed migration runner bug
2  reviewed open PRs
$ mlm delete note 1 --date 2026-09-10
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
2  end 13:00
$ mlm delete punch 2
deleted. to recreate: mlm stop 13:00 --date 2026-09-10
```

## Build (local dev)

Plain `cargo`, no `dist` needed — a debug build, for iterating on the
code itself:

```sh
cargo build
cargo run -- start "working on mlm"
```

## License

Licensed under the [EUPL v1.2](LICENSE).
