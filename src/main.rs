mod cli;
mod commands;
mod date;
mod db;
mod render;
mod stint;
mod storage;
mod time;
mod week;
mod week_target;

use chrono::{DateTime, Local, Timelike};
use clap::Parser;
use cli::{Cli, Command};

fn main() {
    std::process::exit(run());
}

/// Top-level control flow (PLAN.md interface contract 7). `main` itself
/// returns `()`: a `Result`-returning `main` would print errors via
/// `Debug` rather than `Display`, so the error path is explicit here.
fn run() -> i32 {
    // clap handles --help/--version/missing-arg itself and exits(2) on error.
    let cli = Cli::parse();

    // PLAN.md contract 12: --verbose raises the log level; env_logger is
    // initialized here, once, before anything else runs.
    let level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    env_logger::Builder::new().filter_level(level).init();

    // Seconds and nanoseconds are truncated to zero at the point of
    // capture (§4.1: "no seconds precision anywhere"), so every
    // downstream consumer (punch instant, note created_at_utc, today's
    // date) sees the same minute-granular instant.
    let now = Local::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    exit_code(&dispatch(&cli, now))
}

fn dispatch(cli: &Cli, now: DateTime<Local>) -> anyhow::Result<()> {
    // E6: `db::connect()` returns `Err`, never panics.
    let mut conn = db::connect()?;
    log::debug!("db path: {:?}", db::default_db_path());

    match &cli.command {
        Command::Start(a) => commands::start(&mut conn, now, a),
        Command::Stop(a) => commands::stop(&mut conn, now, a),
        Command::Note(a) => commands::note(&mut conn, now, a),
        // `week`/`week target` full dispatch (including the `action:
        // None` render arm) is Milestone 11's wiring pass; `week_target`
        // itself is already unit-tested standalone (Milestone 8 scope).
        // Left as a stub here per Milestone 8's report — not in scope
        // for Milestone 7 (start/stop/note only).
        Command::Week(_args) => {
            todo!("`week`/`week target` dispatch is wired by a later milestone")
        }
    }
}

/// One nonzero exit code for every hard error (§6.3 only requires
/// "nonzero"); a single code keeps the surface small. `Ok` is always 0.
/// Errors print once, via `Display`, on stderr, prefixed with `error: `.
fn exit_code(result: &anyhow::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_exits_zero() {
        assert_eq!(exit_code(&Ok(())), 0);
    }

    #[test]
    fn err_exits_nonzero() {
        let err: anyhow::Result<()> = Err(anyhow::anyhow!("boom"));
        assert_eq!(exit_code(&err), 1);
    }
}
