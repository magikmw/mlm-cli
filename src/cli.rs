//! Argument parsing (clap).

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mlm", version, about = "Quick time tracking from the CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable debug-level logging (PLAN.md interface contract 12).
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Record a start punch for today.
    Start(PunchArgs),

    /// Record an end punch for today.
    Stop(PunchArgs),

    /// Record a work-log note for today.
    Note(NoteArgs),

    /// Show a week's totals, or set its target.
    Week(WeekArgs),

    /// Show a date's stints, notes and totals (defaults to today).
    Status {
        /// Date to show, YYYY-MM-DD. Defaults to today.
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
    /// Time of day (HH:MM, HHMM or HH, 24h). Defaults to now.
    #[arg(value_name = "TIME")]
    pub time: Option<String>,

    /// Optional work-log note recorded for today alongside the punch.
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
    /// Work-log note text for today.
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
        let cli = parse(&["mlm", "note", "a", "b"]).unwrap();
        match cli.command {
            Command::Note(a) => assert_eq!(a.body, vec!["a".to_string(), "b".to_string()]),
            other => panic!("expected Command::Note, got {other:?}"),
        }
    }

    // T25
    #[test]
    fn note_can_start_with_hyphen() {
        let cli = parse(&["mlm", "note", "-ish", "progress"]).unwrap();
        match cli.command {
            Command::Note(a) => {
                assert_eq!(a.body, vec!["-ish".to_string(), "progress".to_string()])
            }
            other => panic!("expected Command::Note, got {other:?}"),
        }
    }

    // T26
    #[test]
    fn log_subcommand_is_gone() {
        assert!(parse(&["mlm", "log"]).is_err());
    }
}
