# mlm

_Pronounced "mlem"._

Simple CLI time tracker. Quickly log start/stop points for the current
date plus a short note of what you did, stored in SQLite.

## Features

- `start`/`stop` — log a punch for today, with an optional note
  attached in the same call
- `note` — log a work-log entry independent of any punch
- `status` — a day's stints, notes, and day/week totals, with a live
  "how much is left today" estimate while a stint is open
- `week` — a week's per-day totals plus carry-in/fulfillment against
  target
- `week target` — override a week's target hours; any shortfall or
  surplus carries into the next week
- Everything stored locally in SQLite — no account, no external
  service
- Cross-platform: Linux (x86_64 and ARM64), Windows (MSVC), and macOS
  (Intel and Apple Silicon)

## Planned

Not implemented yet — see [`docs/dev/SPEC.md`](docs/dev/SPEC.md) §1.2
for the full list and the reasoning behind each:

- Editing or deleting a punch/note after it's entered
- Per-project tagging on notes/stints
- A terminal dashboard (the deps are already in, the UI isn't built)
- Shell prompt integration (current tracking state in PS1/starship)
- Non-ISO week conventions
- 12-hour (AM/PM) time input — 24h only for now
- `+N`/`-N` relative week notation for `week`'s week-id argument
  (`status`'s `DATE` argument already accepts `-N` — see
  [`docs/dev/specs/2026-09-13-backdated-punches.md`](docs/dev/specs/2026-09-13-backdated-punches.md))

### Known limitations

- A session spanning midnight splits into two pieces instead of one
  clean stint (pairing is strictly per calendar date)
- A `stop`/`start` typed at the exact same instant, back-to-back
  between two real stints, can mis-pair (tracked, not yet fixed —
  see `docs/dev/SPEC.md` §1.2/§4.3)

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

Every `start`/`stop`/`note`/`week target` call is silent on success —
nothing prints unless something went wrong. `status` and `week` are
the commands that produce output; `delete note`/`delete punch` are a
narrow exception too (see below) — a `status` after punching in/out is
still how you confirm things landed correctly.

All commands include 1 character aliases for quick use.
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
instead of parsing it as the date flag. See
[`docs/dev/specs/2026-09-13-backdated-punches.md`](docs/dev/specs/2026-09-13-backdated-punches.md)
§2.1.

### `mlm stop|s [TIME] [NOTE...] [-d/--date DATE]`

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
before today (e.g. `-1` = yesterday), and defaults to today.

```sh
$ mlm status
Sat 2026-09-12

Day total:     07h 25m (+ ongoing), 00h 35m left to 08h 00m daily target, est. EOD 23:39
Week 2026-37:  -04h 35m left by end of Saturday (fulfillment 44h 35m / target 40h 00m)

  09:00-13:00  (04h 00m)
  14:05-17:30  (03h 25m)
  17:45-now    (05h 19m, ongoing)

Notes:
  - reviewed open PRs
  - fixed migration runner bug
```

The daily-target pace hint and estimated-EOD line only show up when
`DATE` is today (they need "now" to mean anything). A past date's
status just shows that day's total and its week's numbers:

```sh
$ mlm status 2026-09-08
Tue 2026-09-08

Day total:     07h 35m
Week 2026-37:  -04h 35m left by end of Saturday (fulfillment 44h 35m / target 40h 00m)

  09:05-16:40  (07h 35m)
```

### `mlm week|w [WEEK_ID]`

Show a week's per-day totals plus its carry-in/worked/fulfillment/
target summary. `WEEK_ID` accepts a full id (`YYYY-WW`, e.g. `2026-37`)
or a bare week number for the current year (e.g. `37`), and defaults
to the current week.

```sh
$ mlm week
Week 2026-37 (2026-09-07 - 2026-09-13)

-04h 35m left by end of Saturday

  Mon 2026-09-07   08h 15m
  Tue 2026-09-08   07h 35m
  Wed 2026-09-09   08h 15m
  Thu 2026-09-10   07h 45m
  Fri 2026-09-11   05h 20m
  Sat 2026-09-12   07h 25m (ongoing)
  Sun 2026-09-13   00h 00m

Carry-in:      00h 00m
Worked:        44h 35m
Fulfillment:   44h 35m
Target:        40h 00m
```

### `mlm week|w target [WEEK_ID] DURATION`

Set an absolute target override for a week (default target is 40h
when no override exists). `WEEK_ID` accepts the same forms as `week`
and defaults to the current week; `DURATION` uses the human format
(`20h`, `33h30m`, `45m`), never raw minutes, and is always the last
token. Silent on success:

```sh
$ mlm week target 45h
$ mlm week
Week 2026-37 (2026-09-07 - 2026-09-13)

00h 25m left by end of Saturday

  Mon 2026-09-07   08h 15m
  Tue 2026-09-08   07h 35m
  Wed 2026-09-09   08h 15m
  Thu 2026-09-10   07h 45m
  Fri 2026-09-11   05h 20m
  Sat 2026-09-12   07h 25m (ongoing)
  Sun 2026-09-13   00h 00m

Carry-in:      00h 00m
Worked:        44h 35m
Fulfillment:   44h 35m
Target:        45h 00m
```

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

## Build (local dev)

Plain `cargo`, no `dist` needed — a debug build, for iterating on the
code itself:

```sh
cargo build
cargo run -- start "working on mlm"
```

## License

Licensed under the [EUPL v1.2](LICENSE).
