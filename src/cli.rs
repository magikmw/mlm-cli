//! Argument parsing (clap).

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mlm", version, about = "Quick time tracking from the CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable debug-level logging.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Record a start punch for today.
    #[command(visible_alias = "s")]
    Start(PunchArgs),

    /// Record an end punch for today.
    #[command(visible_alias = "e")]
    Stop(PunchArgs),

    /// Record a work-log note for today.
    #[command(visible_alias = "n")]
    Note(NoteArgs),

    /// Show a week's totals, or set its target.
    #[command(visible_alias = "w")]
    Week(WeekArgs),

    /// Show a date's stints, notes and totals (defaults to today).
    #[command(visible_alias = "d")]
    Status {
        /// Date to show: YYYY-MM-DD, or `-N` for N days before today
        /// (e.g. `-1` = yesterday). Defaults to today.
        date: Option<String>,
    },
}

/// Shared argument shape for `start` and `stop` (SPEC §3.2/§3.3 — "same
/// shape as start"). `TIME` stays a plain `String` at the clap layer:
/// parse failures surface through our own `anyhow`-based error path
/// (§3), not through clap's formatting/exit code. `NOTE` is a `Vec` of
/// trailing tokens joined with single spaces by the handler; an empty
/// `Vec` means "no note given" (§1.2/§1.3).
#[derive(Args, Debug)]
pub struct PunchArgs {
    /// Time of day (HH:MM, HHMM or HH, 24h). Defaults to now when
    /// recording for today; required when `--date` targets another day.
    #[arg(value_name = "TIME")]
    pub time: Option<String>,

    /// Date to record against: YYYY-MM-DD, or `-N` for N days before
    /// today (e.g. `-1` = yesterday). Defaults to today. Must come
    /// before NOTE text on the command line, or it is silently absorbed
    /// into the note body instead of being parsed as this flag -- see
    /// docs/dev/specs/2026-09-13-backdated-punches.md §2.1.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,

    /// Optional work-log note recorded alongside the punch, against
    /// today or, with `--date`, the resolved date.
    #[arg(
        value_name = "NOTE",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub note: Vec<String>,
}

/// `mlm note NOTE...` (SPEC §3.4 — `NOTE` is mandatory here, unlike
/// `start`/`stop`).
#[derive(Args, Debug)]
pub struct NoteArgs {
    /// Date to record against: YYYY-MM-DD, or `-N` for N days before
    /// today (e.g. `-1` = yesterday). Defaults to today. Must come
    /// before NOTE text on the command line -- see
    /// docs/dev/specs/2026-09-13-backdated-punches.md §2.1.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,

    /// Work-log note text, against today or, with `--date`, the resolved
    /// date.
    #[arg(
        value_name = "NOTE",
        required = true,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub body: Vec<String>,
}

// ---------------------------------------------------------------------------
// `week` / `week target` (Milestone 8; the `action: None` non-subcommand
// arm is Milestone 11's to render, per PLAN.md's "canonical shape" note).
// ---------------------------------------------------------------------------

/// `mlm week [WEEK_ID]` and `mlm week target [WEEK_ID] DURATION` share this
/// struct: a first token matching a known subcommand name (`target`) is
/// taken as the subcommand, otherwise it falls through to the positional
/// `WEEK_ID` (no week id can ever be the literal string `target`).
#[derive(Args, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct WeekArgs {
    #[command(subcommand)]
    pub action: Option<WeekAction>,

    /// Week to show: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7).
    /// Defaults to the current week.
    #[arg(value_name = "WEEK_ID")]
    pub week_id: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum WeekAction {
    /// Set an absolute target override for a week.
    Target(WeekTargetArgs),
}

/// `[WEEK_ID] DURATION` as a single variadic positional (SPEC.md §3.7).
///
/// A required positional cannot follow an optional one in clap, so the
/// natural `week_id: Option<String>, duration: String` spelling is
/// illegal. `num_args = 1..=2` + `required = true` instead makes zero
/// arguments a clap-level missing-argument error (E10) and three or more
/// a clap-level unexpected-argument error, with DURATION always the last
/// token. `allow_hyphen_values` is load-bearing for E9: without it,
/// `week target -5h` is parsed as an unknown flag instead of reaching
/// the DURATION parser.
#[derive(Args, Debug)]
pub struct WeekTargetArgs {
    /// [WEEK_ID] DURATION — e.g. `2026-07 33h30m`, or just `33h30m` for
    /// the current week. DURATION uses the human format (20h, 33h30m,
    /// 45m), never raw minutes, and is always the last token.
    #[arg(
        value_name = "ARGS",
        num_args = 1..=2,
        required = true,
        allow_hyphen_values = true
    )]
    pub args: Vec<String>,
}

impl WeekTargetArgs {
    /// `(week_id_token, duration_token)` — one arg means WEEK_ID was
    /// omitted. Panics only if clap's own `num_args` invariant is somehow
    /// violated, which is not reachable via normal parsing.
    pub fn split(&self) -> (Option<&str>, &str) {
        match self.args.as_slice() {
            [duration] => (None, duration.as_str()),
            [week, duration] => (Some(week.as_str()), duration.as_str()),
            _ => unreachable!("clap enforces num_args = 1..=2"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> clap::error::Result<Cli> {
        Cli::try_parse_from(args)
    }

    fn week_target_args(cli: Cli) -> WeekTargetArgs {
        match cli.command {
            Command::Week(WeekArgs {
                action: Some(WeekAction::Target(t)),
                ..
            }) => t,
            other => panic!("expected Command::Week(.. Target ..), got {other:?}"),
        }
    }

    // P1
    #[test]
    fn split_one_arg_is_duration() {
        let cli = parse(&["mlm", "week", "target", "33h30m"]).unwrap();
        assert_eq!(week_target_args(cli).split(), (None, "33h30m"));
    }

    // P2
    #[test]
    fn split_two_args_is_week_then_duration() {
        let cli = parse(&["mlm", "week", "target", "2026-07", "33h30m"]).unwrap();
        assert_eq!(week_target_args(cli).split(), (Some("2026-07"), "33h30m"));
    }

    // P3
    #[test]
    fn no_args_is_clap_error() {
        let err = parse(&["mlm", "week", "target"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    // P4
    #[test]
    fn three_args_is_clap_error() {
        let err = parse(&["mlm", "week", "target", "2026-07", "33h30m", "extra"]).unwrap_err();
        assert!(err.kind() != clap::error::ErrorKind::DisplayHelp);
    }

    // P5
    #[test]
    fn hyphen_duration_reaches_our_parser() {
        let cli = parse(&["mlm", "week", "target", "-5h"]).unwrap();
        assert_eq!(week_target_args(cli).split(), (None, "-5h"));
    }

    // P6
    #[test]
    fn week_positional_still_works() {
        let cli = parse(&["mlm", "week", "2026-07"]).unwrap();
        match cli.command {
            Command::Week(WeekArgs {
                action: None,
                week_id: Some(id),
            }) => assert_eq!(id, "2026-07"),
            _ => panic!("expected Command::Week with no action and week_id set"),
        }
    }

    // --- Milestone 7: start/stop/note parse shape -----------------------

    fn start_args(cli: Cli) -> PunchArgs {
        match cli.command {
            Command::Start(a) => a,
            other => panic!("expected Command::Start, got {other:?}"),
        }
    }

    fn note_args(cli: Cli) -> NoteArgs {
        match cli.command {
            Command::Note(a) => a,
            other => panic!("expected Command::Note, got {other:?}"),
        }
    }

    // T23
    #[test]
    fn parse_start_variants() {
        let a = start_args(parse(&["mlm", "start"]).unwrap());
        assert_eq!(a.time, None);
        assert!(a.note.is_empty());

        let a = start_args(parse(&["mlm", "start", "9:05"]).unwrap());
        assert_eq!(a.time.as_deref(), Some("9:05"));
        assert!(a.note.is_empty());

        let a = start_args(parse(&["mlm", "start", "9:05", "a", "b"]).unwrap());
        assert_eq!(a.time.as_deref(), Some("9:05"));
        assert_eq!(a.note, vec!["a".to_string(), "b".to_string()]);
    }

    // T24
    #[test]
    fn parse_note_joins_tokens() {
        let a = note_args(parse(&["mlm", "note", "a", "b"]).unwrap());
        assert_eq!(a.body, vec!["a".to_string(), "b".to_string()]);
    }

    // T25
    #[test]
    fn note_can_start_with_hyphen() {
        let a = note_args(parse(&["mlm", "note", "-ish", "progress"]).unwrap());
        assert_eq!(a.body, vec!["-ish".to_string(), "progress".to_string()]);
    }

    // T26
    #[test]
    fn log_subcommand_is_gone() {
        assert!(parse(&["mlm", "log"]).is_err());
    }

    // --- backdated-punches: --date/-d plumbing --------------------------

    #[test]
    fn start_accepts_long_and_short_date_flag_with_hyphen_value() {
        let a = start_args(parse(&["mlm", "start", "--date", "-1", "9:00"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("-1"));
        assert_eq!(a.time.as_deref(), Some("9:00"));

        let a = start_args(parse(&["mlm", "start", "-d", "-1", "9:00"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("-1"));
    }

    #[test]
    fn start_date_flag_accepts_absolute_date_before_or_after_time() {
        let a = start_args(parse(&["mlm", "start", "--date", "2026-01-05", "9:00"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("2026-01-05"));
        assert_eq!(a.time.as_deref(), Some("9:00"));

        let a = start_args(parse(&["mlm", "start", "9:00", "--date", "2026-01-05"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("2026-01-05"));
        assert_eq!(a.time.as_deref(), Some("9:00"));
    }

    #[test]
    fn start_omitted_date_flag_is_none() {
        let a = start_args(parse(&["mlm", "start", "9:00"]).unwrap());
        assert_eq!(a.date, None);
    }

    #[test]
    fn note_accepts_date_flag_before_body() {
        let a = note_args(parse(&["mlm", "note", "--date", "-2", "fixed", "a", "bug"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("-2"));
        assert_eq!(
            a.body,
            vec!["fixed".to_string(), "a".to_string(), "bug".to_string()]
        );
    }

    /// Locks down the §2.1 clap footgun: once `--date` appears after
    /// NOTE tokens have started, clap's trailing_var_arg no longer
    /// re-scans for named flags, so `--date -1` is silently absorbed
    /// into the note body instead of being parsed as the date flag.
    /// This test exists so a future clap upgrade or arg refactor that
    /// changes this behavior gets caught, not silently shipped.
    #[test]
    fn date_flag_after_note_text_is_absorbed_into_the_note_body() {
        let a = start_args(
            parse(&[
                "mlm",
                "start",
                "9:00",
                "kicked",
                "off",
                "migration",
                "--date",
                "-1",
            ])
            .unwrap(),
        );
        assert_eq!(a.date, None, "the flag was swallowed, not parsed");
        assert_eq!(
            a.note,
            vec![
                "kicked".to_string(),
                "off".to_string(),
                "migration".to_string(),
                "--date".to_string(),
                "-1".to_string(),
            ]
        );
    }

    /// Same §2.1 clap footgun as `date_flag_after_note_text_is_absorbed_into_the_note_body`,
    /// but for `note`: `NoteArgs.body` has the identical
    /// `trailing_var_arg = true, allow_hyphen_values = true` shape as
    /// `PunchArgs.note`, so it carries the identical risk and needs its
    /// own lock-down rather than relying on `start`'s test to stand in
    /// for it.
    #[test]
    fn note_date_flag_after_body_text_is_absorbed_into_the_note_body() {
        let a = note_args(parse(&["mlm", "note", "fixed", "a", "bug", "--date", "-2"]).unwrap());
        assert_eq!(a.date, None, "the flag was swallowed, not parsed");
        assert_eq!(
            a.body,
            vec![
                "fixed".to_string(),
                "a".to_string(),
                "bug".to_string(),
                "--date".to_string(),
                "-2".to_string(),
            ]
        );
    }
}
