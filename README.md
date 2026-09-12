# mlm

_Pronounced "mlem"._

Simple CLI time tracker. Quickly log start/stop points for the current
date plus a short note of what you did, stored in SQLite.

## Status

Scaffolding stage — deps and project structure in place, commands are
stubs. Design still being worked out (see `AGENTS.md`).

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
- Windows: `%APPDATA%\mlm\mlm.db`

## Usage (planned)

```sh
mlm start ["note"]   # log a start point for today
mlm stop ["note"]    # log a stop point for today
mlm log [date]       # show today's (or given date's) log
```

## Build

```sh
cargo build
cargo run -- start "working on scaffold"
```
