# mlm

_Pronounced "mlem"._

Simple CLI time tracker. Quickly log start/stop points for the current
date plus a short note of what you did, stored in SQLite.

## Install

```sh
cargo install mlm
```

Needs a Rust toolchain (1.85+, edition 2024) — install one via
[rustup](https://rustup.rs) if you don't have one. No other system
dependency: `rusqlite`'s `bundled` feature compiles SQLite from
source, so this works the same on Linux and Windows (MSVC).

## Stack

- [`clap`](https://docs.rs/clap) — argument parsing (derive API)
- [`rusqlite`](https://docs.rs/rusqlite) (bundled SQLite) — storage
- [`chrono`](https://docs.rs/chrono) — time-of-day parsing / duration math
- [`directories`](https://docs.rs/directories) — platform app-data path
  (Linux, Windows/MSVC targets supported)
- [`ratatui`](https://docs.rs/ratatui) + [`crossterm`](https://docs.rs/crossterm) —
  terminal dashboard (bar/sparkline charts, text-cell only, no bitmap
  graphics — portable over SSH and on Windows)

## Stretch goals

- Shell prompt integration (e.g. show current tracking state in
  PS1/starship).

## Data location

- Linux: `~/.local/share/mlm/mlm.db`
- Windows: `%APPDATA%\mlm\data\mlm.db`

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
the commands that produce output, so a `status` after punching in/out
is how you confirm things landed correctly.

All the examples below are real output, captured by running the
built binary against a scratch database.

### `mlm start [TIME] [NOTE...]`

Record a start punch for today. `TIME` (`HH:MM`, `HHMM` or `HH`, 24h)
defaults to now; an optional trailing `NOTE` also records a work-log
note for today in the same call.

```sh
$ mlm start 09:00 "reviewed open PRs"
```

(no output — see `status` below to confirm it landed)

### `mlm stop [TIME] [NOTE...]`

Record an end punch for today. Same argument shape as `start`.

```sh
$ mlm stop 13:00
```

### `mlm note NOTE...`

Record a work-log note for today, independent of any punch — for
end-of-day notes or anything with nothing to attach to.

```sh
$ mlm note "fixed migration runner bug"
```

### `mlm status [DATE]`

Show a date's stints, notes, day total, and the totals for the week
that date falls in. `DATE` (`YYYY-MM-DD`) defaults to today.

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

### `mlm week [WEEK_ID]`

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

### `mlm week target [WEEK_ID] DURATION`

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

## Build

```sh
cargo build
cargo run -- start "working on mlm"
```

## License

Licensed under the [EUPL v1.2](LICENSE).
