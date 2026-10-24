use std::ffi::OsString;

use chrono::{DateTime, Local, Timelike};
use mlm::cli::{self, Cli, Command, DeleteTarget, ExportTarget, WeekAction};
use mlm::{commands, db, export, status, week_target, week_view};

fn main() {
    std::process::exit(run());
}

/// Top-level control flow (PLAN.md interface contract 7). `main` itself
/// returns `()`: a `Result`-returning `main` would print errors via
/// `Debug` rather than `Display`, so the error path is explicit here.
fn run() -> i32 {
    // Help, version and usage errors go through `ParseFailure::report`
    // (exit 0 for help/version, 2 for usage errors).
    let args: Vec<OsString> = std::env::args_os().collect();
    let cli = match cli::parse(&args) {
        Ok(cli) => cli,
        Err(failure) => return failure.report(),
    };

    // --verbose raises the log level to Debug; the default is Warn
    // (SPEC §4.1). env_logger is initialized here, once, before anything
    // else runs.
    let level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
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
        Command::Export(args) => match &args.target {
            ExportTarget::Notes(a) => export::run_notes(&conn, now.date_naive(), a),
        },
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
