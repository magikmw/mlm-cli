//! Argument parsing (clap).

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mlm", version, about = "Quick time tracking from the CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Log a start point for today.
    Start {
        /// Optional note about what you're starting.
        note: Option<String>,
    },
    /// Log a stop point for today.
    Stop {
        /// Optional note about what you finished.
        note: Option<String>,
    },
    /// Show today's (or a given date's) log.
    Log {
        /// Date to show, defaults to today. Format: YYYY-MM-DD.
        date: Option<String>,
    },
    /// Show a week's totals, or set its target.
    Week(WeekArgs),
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
}
