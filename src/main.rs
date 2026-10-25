use std::ffi::OsString;
use std::io::Write;

use chrono::{DateTime, Local, TimeZone, Timelike, Utc};
use mlm::OutputError;
use mlm::cli::{self, Cli, Command, DeleteTarget, ExportTarget, WeekAction};
use mlm::{commands, db, export, status, week_target, week_view};

/// What a shell reports for a process killed by `SIGPIPE` (spec §5).
const EXIT_BROKEN_PIPE: i32 = 141;

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
    // date) sees the same minute-granular instant. The truncation runs
    // on the UTC instant, so a repeated local hour cannot make it fail.
    let now = minute_floor(Utc::now(), &Local);

    let mut stdout = std::io::stdout().lock();
    let result = dispatch(&cli, now, &mut stdout);
    exit_code(&result, &mut std::io::stderr())
}

/// `instant` truncated to the whole minute, in `tz`. The truncation runs
/// in UTC, which has one instant per wall time, so it never fails.
fn minute_floor<Tz: TimeZone>(instant: DateTime<Utc>, tz: &Tz) -> DateTime<Tz> {
    instant
        .with_second(0)
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(instant)
        .with_timezone(tz)
}

fn dispatch<W: Write>(cli: &Cli, now: DateTime<Local>, out: &mut W) -> anyhow::Result<()> {
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
            DeleteTarget::Note(a) => commands::delete_note(&conn, now, a, out),
            DeleteTarget::Punch(a) => commands::delete_punch(&conn, now, a, out),
        },
        // `week`/`week target` full dispatch (Milestone 11's wiring
        // pass): `action: Some(Target(..))` routes to Milestone 8's
        // `week_target::run`; `action: None` renders the week view.
        Command::Week(args) => match &args.action {
            Some(WeekAction::Target(t)) => week_target::run(&conn, now.date_naive(), t, out),
            None => week_view::run(&conn, now, args, out),
        },
        Command::Status { date } => status::run(&conn, now, date.as_deref(), out),
        Command::Export(args) => match &args.target {
            ExportTarget::Notes(a) => export::run_notes(&conn, now.date_naive(), a, out),
        },
    }
}

/// Maps the command's result to the process exit code. `Ok` is 0. A
/// broken stdout pipe is 141 with nothing printed. Any other error
/// prints one `error: ` line on `stderr`, ignoring a failed write, and
/// is 1 (§6.3 only requires "nonzero").
fn exit_code<W: Write>(result: &anyhow::Result<()>, stderr: &mut W) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) if is_broken_pipe(e) => EXIT_BROKEN_PIPE,
        Err(e) => {
            let _ = writeln!(stderr, "error: {e}");
            1
        }
    }
}

/// True only when the top-level error is an `OutputError` for a broken
/// pipe. A storage-side I/O error is never an output error.
fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.downcast_ref::<OutputError>()
        .is_some_and(OutputError::is_broken_pipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Offset;
    use chrono_tz::Europe::Warsaw;
    use std::io::{self, ErrorKind};

    fn utc(s: &str) -> DateTime<Utc> {
        s.parse().expect("test fixture is an RFC 3339 instant")
    }

    #[test]
    fn ok_exits_zero() {
        let mut err = Vec::new();
        assert_eq!(exit_code(&Ok(()), &mut err), 0);
        assert!(err.is_empty());
    }

    #[test]
    fn err_exits_nonzero() {
        let mut err = Vec::new();
        let result: anyhow::Result<()> = Err(anyhow::anyhow!("boom"));
        assert_eq!(exit_code(&result, &mut err), 1);
        assert_eq!(String::from_utf8(err).unwrap(), "error: boom\n");
    }

    #[test]
    fn broken_pipe_output_error_exits_141_silently() {
        let mut err = Vec::new();
        let result: anyhow::Result<()> =
            Err(OutputError(io::Error::from(ErrorKind::BrokenPipe)).into());
        assert_eq!(exit_code(&result, &mut err), 141);
        assert!(err.is_empty());
    }

    #[test]
    fn other_output_error_prints_cannot_write_output() {
        let mut err = Vec::new();
        let result: anyhow::Result<()> = Err(OutputError(io::Error::other("disk on fire")).into());
        assert_eq!(exit_code(&result, &mut err), 1);
        assert_eq!(
            String::from_utf8(err).unwrap(),
            "error: cannot write output: disk on fire\n"
        );
    }

    #[test]
    fn bare_io_error_is_not_an_output_error() {
        let mut err = Vec::new();
        let result: anyhow::Result<()> =
            Err(anyhow::Error::from(io::Error::from(ErrorKind::BrokenPipe)));
        assert_eq!(exit_code(&result, &mut err), 1);
        let text = String::from_utf8(err).unwrap();
        assert!(text.starts_with("error: "), "{text}");
        assert!(!text.contains("cannot write output"), "{text}");
    }

    #[test]
    fn stderr_write_failure_is_ignored() {
        let mut full: &mut [u8] = &mut [];
        let result: anyhow::Result<()> = Err(anyhow::anyhow!("boom"));
        assert_eq!(exit_code(&result, &mut full), 1);
    }

    // 2026-10-25 in Warsaw: 00:00-00:59 UTC is the first pass (CEST,
    // +02:00), 01:00-01:59 UTC the second pass (CET, +01:00).

    #[test]
    fn minute_floor_keeps_a_second_pass_instant() {
        let instant = utc("2026-10-25T01:22:37.5Z");
        assert!(instant.with_timezone(&Warsaw).with_second(0).is_none());

        let floored = minute_floor(instant, &Warsaw);
        assert_eq!(floored.with_timezone(&Utc), utc("2026-10-25T01:22:00Z"));
        assert_ne!(floored.with_timezone(&Utc), utc("2026-10-25T00:22:00Z"));
        assert_eq!(floored.naive_local().to_string(), "2026-10-25 02:22:00");
        assert_eq!(floored.offset().fix().local_minus_utc(), 3600);
    }

    #[test]
    fn minute_floor_keeps_a_first_pass_instant() {
        let floored = minute_floor(utc("2026-10-25T00:22:37Z"), &Warsaw);
        assert_eq!(floored.with_timezone(&Utc), utc("2026-10-25T00:22:00Z"));
        assert_eq!(floored.naive_local().to_string(), "2026-10-25 02:22:00");
        assert_eq!(floored.offset().fix().local_minus_utc(), 7200);
    }
}
