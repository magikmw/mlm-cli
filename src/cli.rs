//! Argument parsing (clap).

use std::ffi::OsString;
use std::io::Write;

use clap::error::{ContextKind, ContextValue};
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};

/// The `status`/`week` long-help glossary (SPEC §3.5).
pub const GLOSSARY: &str = "\
Glossary:
  worked
      Time in closed stints this week. In the current week it also counts
      the stint that is running now.
  carry-in
      The balance carried in from earlier weeks. A shortfall is negative
      and a surplus is positive.
  fulfillment
      Worked plus carry-in. A week's target is measured against it.
  left to / over
      The week line's figure. `X left to F` means X more reaches F, and
      `X over F` means fulfillment is X past F. Monday to Thursday F is
      the slice of the target due by end of today (one fifth per weekday
      so far), shown as `by end of <weekday>`. Friday to Sunday F is the
      full target, shown as `<target> target`.
  est. EOD
      With a stint open today, the clock time at which that stint closes
      today's gap. It reads `done for today` instead when what is due by
      end of today is already covered.
  (incl. ongoing), (+ ongoing)
      `(incl. ongoing)` means today's open stint is counted in the total.
      `(+ ongoing)` marks an earlier date's open stint, which counts for
      nothing until it is stopped (`(+ unclosed)` once it is two or more
      days old).
  Total behind, Total ahead
      The headline of a closed or not-yet-started week. `Total behind`
      is a shortfall against the target and `Total ahead` a surplus,
      with carry-in included.
  overlapping stints
      Two stints whose times cross, such as 09:00-12:00 and 10:00-11:00.
      The overlap counts twice in the totals until you delete a punch.
  orphaned end
      An end punch with no open start to pair with. It adds no minutes.
      Add a start before it, or delete it.
  so far, started yesterday
      A stint line ending `so far, started yesterday, not counted until
      stopped` marks a start still open from yesterday, listed on
      today's page but not counted until you stop it.
  spans to next day, continues previous day's stint
      A stint stopped after midnight is counted on the day it started.
      `spans to next day` ends the start date's stint line, and
      `continues previous day's stint` ends the next date's header.
  - <note>
      One of a date's notes, in the order they were added. `mlm status`
      lists them under `Notes:`, and `mlm week --notes` under the date's
      row. Notes change no total, marker or footer.
  [!], [i]
      `[!]` flags an anomaly, such as an unclosed start, an orphaned end
      or overlapping stints. In `mlm week` it marks the date's row. `[i]`
      is a hint, not an anomaly, such as a start still open from
      yesterday. It marks yesterday's row in `mlm week` until that start
      is stopped.";

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
    /// Record a start punch for today, or another day with --date.
    #[command(visible_alias = "s")]
    Start(PunchArgs),

    /// Record an end punch for today, or another day with --date.
    #[command(visible_alias = "e")]
    Stop(PunchArgs),

    /// Record a work-log note for today, or another day with --date.
    #[command(visible_alias = "n")]
    Note(NoteArgs),

    /// Delete a punch or note by its ephemeral, per-listing position for a
    /// date (run with no ID first to list and number that date's entries).
    #[command(visible_alias = "del")]
    Delete(DeleteArgs),

    /// Show a week's totals, or set its target.
    #[command(visible_alias = "w")]
    Week(WeekArgs),

    /// Show a date's stints, notes and totals (defaults to today).
    #[command(visible_alias = "d")]
    Status {
        /// Date to show: YYYY-MM-DD, or `-N` for N days before today
        /// (e.g. `-1` = yesterday). Defaults to today.
        #[arg(allow_negative_numbers = true)]
        date: Option<String>,
    },

    /// Print stored data as plain text
    #[command(visible_alias = "ex")]
    Export(ExportArgs),
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
    /// into the note body instead of being parsed as this flag.
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
    /// before NOTE text on the command line.
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
// `delete note|punch` (SPEC §3.9).
// ---------------------------------------------------------------------------

/// `mlm delete note|punch [ID] [--date DATE]`. Mirrors `WeekArgs`'
/// top-level-command-with-its-own-subcommand-enum shape, but simpler:
/// unlike `week`, there is no bare positional that could collide with
/// the subcommand name, so — unlike `WeekArgs` —
/// `args_conflicts_with_subcommands` is not needed: `note`/`punch` is
/// always a required subcommand, there is no bare `mlm delete` to
/// disambiguate against a fallback positional.
#[derive(Args, Debug)]
pub struct DeleteArgs {
    #[command(subcommand)]
    pub target: DeleteTarget,
}

/// `note` and `punch` targets for `delete`, each taking the same
/// `[ID] [--date DATE]` argument shape.
#[derive(Subcommand, Debug)]
pub enum DeleteTarget {
    /// Delete (or list) a note for a date.
    #[command(visible_alias = "n")]
    Note(DeleteEntryArgs),

    /// Delete (or list) a punch for a date.
    #[command(visible_alias = "p")]
    Punch(DeleteEntryArgs),
}

/// Shared `[ID] [--date DATE]` shape for `delete note` and
/// `delete punch`. Unlike `PunchArgs`/`NoteArgs`, there is no
/// free-text trailing positional on this struct, so the SPEC §3.4a
/// ordering footgun (`--date` must precede free note text or be
/// silently absorbed into it) does not
/// apply to parsing these arguments: once clap has consumed (at most)
/// one token into `id`, every remaining token is still scanned
/// normally for named flags, `--date`/`-d` included, regardless of
/// where it appears relative to `id`.
#[derive(Args, Debug)]
pub struct DeleteEntryArgs {
    /// Position in this date's list, numbered from 1, taken fresh when
    /// this runs (run with no ID to see it). 0 or a number past the list
    /// is rejected.
    // `allow_negative_numbers = true` is required despite the `u32`
    // value_parser: verified empirically against clap 4.6.6 that,
    // without it, a bare `-1` positional here is rejected as
    // `UnknownArgument` (clap tries it as a flag first) rather than
    // reaching the `u32` parser to fail as `ValueValidation`.
    #[arg(value_name = "ID", allow_negative_numbers = true)]
    pub id: Option<u32>,

    /// Date to operate on: YYYY-MM-DD, or `-N` for N days before today
    /// (e.g. `-1` = yesterday). Defaults to today.
    // A malformed or future date is rejected once the app resolves it
    // with `date::resolve_future_checked_date`, the same resolver
    // start/stop/note already use -- not here at the parsing level.
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,
}

// ---------------------------------------------------------------------------
// `week` / `week target` (SPEC §3.6, §3.7).
// ---------------------------------------------------------------------------

/// `mlm week [WEEK_ID] [--notes]` and `mlm week target [WEEK_ID] DURATION`
/// share this struct: a first token matching a known subcommand name
/// (`target`) is taken as the subcommand, otherwise it falls through to the
/// positional `WEEK_ID` (no week id can ever be the literal string `target`).
#[derive(Args, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct WeekArgs {
    #[command(subcommand)]
    pub action: Option<WeekAction>,

    /// Week to show: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or
    /// `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to
    /// the current week.
    #[arg(value_name = "WEEK_ID", allow_negative_numbers = true)]
    pub week_id: Option<String>,

    /// Show each date's notes under its row.
    #[arg(short, long)]
    pub notes: bool,
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
///
/// The positional is hidden from the generated help, which cannot show an
/// optional positional before a required one. `before_help` carries a
/// hand-written Arguments section instead, and `parse` rewrites the
/// `<ARGS>` name out of usage errors.
///
/// A lone token that parses as a WEEK_ID is a missing DURATION; `parse`
/// turns it into the same usage error.
#[derive(Args, Debug)]
#[command(
    override_usage = "mlm week target [OPTIONS] [WEEK_ID] <DURATION>",
    help_template = "{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}{all-args}",
    before_help = "\
Arguments:
  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7), or `-N` for N weeks before this one (e.g. `-1` = last week). Defaults to the current week
  <DURATION>  Target to set: 20h, 33h30m or 45m, up to 168h (human format, never raw minutes)"
)]
pub struct WeekTargetArgs {
    #[arg(
        value_name = "ARGS",
        num_args = 1..=2,
        required = true,
        allow_hyphen_values = true,
        hide = true
    )]
    pub args: Vec<String>,
}

// ---------------------------------------------------------------------------
// `export notes`
// ---------------------------------------------------------------------------

/// `mlm export notes [--date DATE | --week WEEK_ID | --month MONTH]`. A
/// required subcommand, as `delete` has, so bare `mlm export` is a usage
/// error that prints this command's help.
#[derive(Args, Debug)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub target: ExportTarget,
}

/// What `export` prints.
#[derive(Subcommand, Debug)]
pub enum ExportTarget {
    /// Print notes for a date, week or month, one per line (defaults to
    /// the current week)
    #[command(visible_alias = "n")]
    Notes(ExportNotesArgs),
}

/// At most one range flag (the group). Each takes hyphen values, so `-1`
/// reaches its parser, and a following flag becomes this flag's value.
#[derive(Args, Debug)]
#[group(multiple = false)]
pub struct ExportNotesArgs {
    /// Date to export: YYYY-MM-DD, or `-N` for N days before today (e.g.
    /// `-1` = yesterday)
    #[arg(short, long, value_name = "DATE", allow_hyphen_values = true)]
    pub date: Option<String>,

    /// Week to export: YYYY-WW (e.g. 2026-07), a bare week number (e.g. 7),
    /// or `-N` for N weeks before this one (e.g. `-1` = last week)
    #[arg(short, long, value_name = "WEEK_ID", allow_hyphen_values = true)]
    pub week: Option<String>,

    /// Month to export: YYYY-MM (e.g. 2026-03), a bare month number (e.g.
    /// 3), or `-N` for N months before this one (e.g. `-1` = last month)
    #[arg(short, long, value_name = "MONTH", allow_hyphen_values = true)]
    pub month: Option<String>,
}

/// The parse tree with the help surface applied.
pub fn command() -> clap::Command {
    Cli::command()
        .mut_subcommand("status", with_glossary)
        .mut_subcommand("week", with_glossary)
}

fn with_glossary(cmd: clap::Command) -> clap::Command {
    cmd.after_long_help(GLOSSARY)
}

/// Why `parse` failed and what `main` should do about it.
#[derive(Debug)]
pub enum ParseFailure {
    /// Clap's own error: help, version, or a usage error (exit 2).
    Clap(clap::Error),
    /// `week WEEK_ID target DURATION`: the full `error: ...` line (exit 2).
    SwappedWeekTarget(String),
}

impl ParseFailure {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Clap(err) => err.exit_code(),
            Self::SwappedWeekTarget(_) => 2,
        }
    }

    /// Prints the failure, then returns the exit code.
    pub fn report(&self) -> i32 {
        match self {
            Self::Clap(err) => {
                let _ = err.print();
            }
            Self::SwappedWeekTarget(message) => {
                let _ = writeln!(std::io::stderr(), "{message}");
            }
        }
        self.exit_code()
    }
}

/// `Cli` from raw arguments (argv[0] included).
pub fn parse(args: &[OsString]) -> Result<Cli, ParseFailure> {
    let cli = command()
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
        .map_err(|err| classify(args, err))?;
    let today = chrono::Local::now().date_naive();
    match lone_week_id_failure(args, &cli, today) {
        Some(failure) => Err(failure),
        None => Ok(cli),
    }
}

fn classify(args: &[OsString], mut err: clap::Error) -> ParseFailure {
    // Help and version keep clap's own exit-0 path.
    if !err.use_stderr() {
        return ParseFailure::Clap(err);
    }
    let text: Option<Vec<String>> = args.iter().map(|a| a.to_str().map(str::to_owned)).collect();
    let today = chrono::Local::now().date_naive();
    if let Some(message) = text.and_then(|a| swapped_week_target(&a, today)) {
        return ParseFailure::SwappedWeekTarget(message);
    }
    rename_args_context(&mut err);
    ParseFailure::Clap(err)
}

const ARGS_NAME: &str = "<ARGS>";

/// Replaces the hidden `<ARGS>` name in usage errors with the names the
/// user sees in the help.
fn rename_args_context(err: &mut clap::Error) {
    let renamed = match err.get(ContextKind::InvalidArg) {
        // Missing-argument list: only DURATION can be missing.
        Some(ContextValue::Strings(names)) if names.iter().any(|n| n.starts_with(ARGS_NAME)) => {
            Some(ContextValue::Strings(vec!["<DURATION>".to_string()]))
        }
        // Too many values.
        Some(ContextValue::String(name)) if name.starts_with(ARGS_NAME) => {
            Some(ContextValue::String("[WEEK_ID] <DURATION>".to_string()))
        }
        _ => None,
    };
    if let Some(value) = renamed {
        err.insert(ContextKind::InvalidArg, value);
    }
}

/// A one-token `week target` whose token parses as a WEEK_ID is missing
/// its DURATION (SPEC §3.7). Returns the error bare
/// `mlm week target` gives, by parsing `args` again without the token.
/// No WEEK_ID is a valid DURATION, which needs an `h` or `m` suffix.
fn lone_week_id_failure(
    args: &[OsString],
    cli: &Cli,
    today: chrono::NaiveDate,
) -> Option<ParseFailure> {
    let token = lone_target_token(cli).filter(|t| is_week_token(t, today))?;
    let rest = without_last(args, token);
    command()
        .try_get_matches_from(&rest)
        .err()
        .map(|err| classify(&rest, err))
}

/// The token of a one-token `week target`.
fn lone_target_token(cli: &Cli) -> Option<&str> {
    let Command::Week(WeekArgs {
        action: Some(WeekAction::Target(target)),
        ..
    }) = &cli.command
    else {
        return None;
    };
    match target.split() {
        (None, token) => Some(token),
        (Some(_), _) => None,
    }
}

/// `args` without its last element equal to `token`.
fn without_last(args: &[OsString], token: &str) -> Vec<OsString> {
    let mut rest = args.to_vec();
    if let Some(i) = rest.iter().rposition(|a| a == token) {
        rest.remove(i);
    }
    rest
}

/// Detects `week WEEK_ID target DURATION` by token shape over the raw
/// arguments (argv[0] included), ignoring `-v`/`--verbose` anywhere. Fires
/// only when the week token is a valid WEEK_ID (a bare number resolves
/// against `today`'s ISO year) and the duration token is well-shaped.
pub fn swapped_week_target(args: &[String], today: chrono::NaiveDate) -> Option<String> {
    let tokens: Vec<&str> = args
        .iter()
        .skip(1)
        .map(String::as_str)
        .filter(|t| !matches!(*t, "-v" | "--verbose"))
        .collect();
    match tokens.as_slice() {
        ["week" | "w", week, "target", duration]
            if is_week_token(week, today) && is_duration_token(duration) =>
        {
            Some(format!(
                "error: put WEEK_ID after target: mlm week target {week} {duration}"
            ))
        }
        _ => None,
    }
}

/// True when the token parses as a WEEK_ID.
fn is_week_token(token: &str, today: chrono::NaiveDate) -> bool {
    crate::date::parse_week_id(token, today).is_ok()
}

/// True when the token has the DURATION shape. A negative value counts
/// (`-5h` reaches the duration parser, E9). `--help`, `-h` and `-V` do not.
fn is_duration_token(token: &str) -> bool {
    !matches!(
        crate::time::parse_duration(token),
        Err(crate::time::DurationParseError::InvalidFormat(_))
    )
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

    fn week_args(cli: Cli) -> WeekArgs {
        match cli.command {
            Command::Week(a) => a,
            other => panic!("expected Command::Week, got {other:?}"),
        }
    }

    fn week_target_args(cli: Cli) -> WeekTargetArgs {
        let WeekAction::Target(t) = week_args(cli).action.expect("expected week target");
        t
    }

    fn week_view_args(cli: Cli) -> WeekArgs {
        let a = week_args(cli);
        assert!(a.action.is_none(), "expected the week view, got {a:?}");
        a
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
                notes: false,
            }) => assert_eq!(id, "2026-07"),
            _ => panic!("expected Command::Week with no action and week_id set"),
        }
    }

    #[test]
    fn notes_flag_parses_long_and_short_in_either_order() {
        for args in [
            &["mlm", "week", "-n", "2026-43"][..],
            &["mlm", "week", "2026-43", "-n"],
            &["mlm", "week", "--notes", "2026-43"],
            &["mlm", "week", "2026-43", "--notes"],
            &["mlm", "w", "-n", "2026-43"],
        ] {
            let a = week_view_args(parse(args).unwrap());
            assert!(a.notes, "{args:?}");
            assert_eq!(a.week_id.as_deref(), Some("2026-43"), "{args:?}");
        }
        let bare = week_view_args(parse(&["mlm", "week", "-n"]).unwrap());
        assert!(bare.notes);
        assert_eq!(bare.week_id, None);
        assert!(!week_view_args(parse(&["mlm", "week"]).unwrap()).notes);
    }

    #[test]
    fn notes_after_target_is_a_target_token() {
        let t = week_target_args(parse(&["mlm", "week", "target", "40h", "--notes"]).unwrap());
        assert_eq!(t.split(), (Some("40h"), "--notes"));
        let err = parse(&["mlm", "week", "target", "43", "40h", "--notes"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::TooManyValues);
    }

    #[test]
    fn glossary_has_the_note_line_entry_before_markers() {
        let note = GLOSSARY.find("\n  - <note>\n").expect("note entry");
        let spans = GLOSSARY.find("\n  spans to next day").unwrap();
        let markers = GLOSSARY.find("\n  [!], [i]\n").unwrap();
        assert!(spans < note && note < markers);
        let flat = GLOSSARY.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains(
            "One of a date's notes, in the order they were added. `mlm status` lists them under `Notes:`, and `mlm week --notes` under the date's row. Notes change no total, marker or footer."
        ));
    }

    // --- start/stop/note parse shape ------------------------------------

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

    /// Locks down the SPEC §3.4a clap footgun: once `--date` appears after
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

    /// Same SPEC §3.4a clap footgun as `date_flag_after_note_text_is_absorbed_into_the_note_body`,
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

    // --- delete: cli surface ---------------------------------------------

    fn delete_target(cli: Cli) -> DeleteTarget {
        match cli.command {
            Command::Delete(DeleteArgs { target }) => target,
            other => panic!("expected Command::Delete, got {other:?}"),
        }
    }

    fn delete_note_args(target: DeleteTarget) -> DeleteEntryArgs {
        match target {
            DeleteTarget::Note(a) => a,
            other => panic!("expected DeleteTarget::Note, got {other:?}"),
        }
    }

    fn delete_punch_args(target: DeleteTarget) -> DeleteEntryArgs {
        match target {
            DeleteTarget::Punch(a) => a,
            other => panic!("expected DeleteTarget::Punch, got {other:?}"),
        }
    }

    #[test]
    fn delete_note_with_no_id_parses_as_list_mode() {
        let a = delete_note_args(delete_target(parse(&["mlm", "delete", "note"]).unwrap()));
        assert_eq!(a.id, None);
    }

    #[test]
    fn delete_punch_with_no_id_parses_as_list_mode() {
        let a = delete_punch_args(delete_target(parse(&["mlm", "delete", "punch"]).unwrap()));
        assert_eq!(a.id, None);
    }

    /// Unlike `PunchArgs`/`NoteArgs`, `DeleteEntryArgs` has no free-text
    /// trailing positional for `--date` to be swallowed into, so
    /// `id`/`--date` parse identically regardless of order.
    #[test]
    fn delete_id_and_date_parse_the_same_regardless_of_order() {
        let a = delete_note_args(delete_target(
            parse(&["mlm", "delete", "note", "2", "--date", "-1"]).unwrap(),
        ));
        assert_eq!(a.id, Some(2));
        assert_eq!(a.date.as_deref(), Some("-1"));

        let b = delete_note_args(delete_target(
            parse(&["mlm", "delete", "note", "--date", "-1", "2"]).unwrap(),
        ));
        assert_eq!(b.id, Some(2));
        assert_eq!(b.date.as_deref(), Some("-1"));
    }

    #[test]
    fn delete_del_alias_parses_like_delete() {
        let cli = parse(&["mlm", "del", "note"]).unwrap();
        assert!(matches!(cli.command, Command::Delete(_)));
    }

    #[test]
    fn delete_note_n_alias_parses_like_note() {
        let cli = parse(&["mlm", "delete", "n"]).unwrap();
        assert!(matches!(delete_target(cli), DeleteTarget::Note(_)));
    }

    #[test]
    fn delete_punch_p_alias_parses_like_punch() {
        let cli = parse(&["mlm", "delete", "p"]).unwrap();
        assert!(matches!(delete_target(cli), DeleteTarget::Punch(_)));
    }

    /// Rejecting `0` is `commands.rs`'s job, not clap's -- this
    /// test only pins clap's own permissiveness for a bare `u32` field.
    #[test]
    fn delete_id_zero_is_accepted_by_clap() {
        let a = delete_note_args(delete_target(
            parse(&["mlm", "delete", "note", "0"]).unwrap(),
        ));
        assert_eq!(a.id, Some(0));
    }

    /// Negative/non-numeric/overflow id rejected at the clap level
    /// (SPEC §3.9).
    #[test]
    fn delete_negative_id_is_rejected_by_clap() {
        let err = parse(&["mlm", "delete", "note", "-1"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    /// Negative/non-numeric/overflow id rejected at the clap level
    /// (SPEC §3.9).
    #[test]
    fn delete_non_numeric_id_is_rejected_by_clap() {
        let err = parse(&["mlm", "delete", "note", "abc"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    /// Negative/non-numeric/overflow id rejected at the clap level
    /// (SPEC §3.9). `4294967296` is
    /// `u32::MAX + 1`.
    #[test]
    fn delete_id_past_u32_max_is_rejected_by_clap() {
        let err = parse(&["mlm", "delete", "note", "4294967296"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    // --- help surface, argument-order interception (T5) ------------------

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn strs(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    fn swapped(args: &[&str]) -> Option<String> {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        swapped_week_target(&strs(args), today)
    }

    #[test]
    fn swapped_needs_a_valid_week_id_and_duration() {
        for args in [
            &["mlm", "week", "foo$(id)", "target", "20h"],
            &["mlm", "week", "foo", "target", "20h"],
            &["mlm", "week", "2026-99", "target", "20h"],
            &["mlm", "week", "2026-41", "target", "soon"],
            &["mlm", "week", "2026-41", "target", "-x"],
        ] {
            assert_eq!(swapped(args), None, "{args:?}");
        }
    }

    #[test]
    fn invalid_week_token_gets_clap_error_not_the_custom_message() {
        for week in ["foo$(id)", "2026-99"] {
            let failure = super::parse(&os(&["mlm", "week", week, "target", "20h"])).unwrap_err();
            assert!(
                matches!(failure, ParseFailure::Clap(_)),
                "{week}: {failure:?}"
            );
        }
    }

    #[test]
    fn command_passes_debug_assert() {
        command().debug_assert();
    }

    #[test]
    fn glossary_is_ascii_and_ends_cleanly() {
        assert!(GLOSSARY.is_ascii());
        let flat = GLOSSARY.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains("It marks yesterday's row in `mlm week` until that start is stopped.")
        );
        assert!(flat.contains("so far, started yesterday, not counted until stopped"));
        assert!(flat.contains("`spans to next day` ends the start date's stint line"));
        assert!(flat.contains("`continues previous day's stint` ends the next date's header"));
        assert!(GLOSSARY.ends_with("stopped."));
        assert!(!GLOSSARY.ends_with('\n'));
    }

    #[test]
    fn swapped_matches_expected_shapes() {
        let want = Some("error: put WEEK_ID after target: mlm week target 2026-41 20h".to_string());
        assert_eq!(swapped(&["mlm", "week", "2026-41", "target", "20h"]), want);
        assert_eq!(swapped(&["mlm", "w", "2026-41", "target", "20h"]), want);
        assert_eq!(
            swapped(&["mlm", "-v", "week", "2026-41", "target", "20h"]),
            want
        );
        assert_eq!(
            swapped(&["mlm", "week", "--verbose", "2026-41", "target", "20h"]),
            want
        );
        assert_eq!(
            swapped(&["mlm", "week", "2026-41", "-v", "target", "20h"]),
            want
        );
        assert_eq!(
            swapped(&["mlm", "week", "2026-41", "target", "--verbose", "20h"]),
            want
        );
        assert_eq!(
            swapped(&["mlm", "week", "2026-41", "target", "20h", "-v"]),
            want
        );
        assert_eq!(
            swapped(&["mlm", "week", "2026-7", "target", "20h"]),
            Some("error: put WEEK_ID after target: mlm week target 2026-7 20h".to_string())
        );
        assert_eq!(
            swapped(&["mlm", "week", "2026-41", "target", "200h"]),
            Some("error: put WEEK_ID after target: mlm week target 2026-41 200h".to_string())
        );
        assert_eq!(
            swapped(&["mlm", "week", "7", "target", "-5h"]),
            Some("error: put WEEK_ID after target: mlm week target 7 -5h".to_string())
        );
    }

    #[test]
    fn swapped_rejects_other_shapes() {
        let none: [&[&str]; 13] = [
            &["mlm", "week", "7", "foo"],
            &["mlm", "week", "2026-41", "target"],
            &["mlm", "week", "2026-41", "target", "20h", "extra"],
            &["mlm", "week", "target", "2026-41", "20h"],
            &["mlm", "week", "2026-41"],
            &["mlm", "week", "target", "target", "20h"],
            &["mlm", "status", "7", "target", "20h"],
            &["mlm", "week", "-x", "target", "20h"],
            &["mlm", "week", "7", "target", "--help"],
            &["mlm", "week", "7", "target", "-h"],
            &["mlm", "week", "7", "target", "-V"],
            &[],
            &["mlm"],
        ];
        for args in none {
            assert_eq!(swapped(args), None, "{args:?}");
        }
    }

    #[test]
    fn parse_reports_swapped_order() {
        let failure = super::parse(&os(&["mlm", "week", "2026-41", "target", "20h"])).unwrap_err();
        match &failure {
            ParseFailure::SwappedWeekTarget(m) => {
                assert_eq!(
                    m,
                    "error: put WEEK_ID after target: mlm week target 2026-41 20h"
                )
            }
            other => panic!("expected SwappedWeekTarget, got {other:?}"),
        }
        assert_eq!(failure.exit_code(), 2);
        assert!(super::parse(&os(&["mlm", "week", "target", "20h"])).is_ok());
    }

    #[test]
    fn parse_failure_exit_codes() {
        let help = super::parse(&os(&["mlm", "--help"])).unwrap_err();
        assert_eq!(help.exit_code(), 0);
        let usage = super::parse(&os(&["mlm", "week", "7", "foo"])).unwrap_err();
        assert_eq!(usage.exit_code(), 2);
    }

    fn clap_text(failure: ParseFailure) -> (clap::error::ErrorKind, String) {
        match failure {
            ParseFailure::Clap(e) => (e.kind(), e.render().to_string()),
            other => panic!("expected Clap, got {other:?}"),
        }
    }

    #[test]
    fn missing_duration_error_hides_args_name() {
        let (kind, text) = clap_text(super::parse(&os(&["mlm", "week", "target"])).unwrap_err());
        assert_eq!(kind, clap::error::ErrorKind::MissingRequiredArgument);
        assert!(text.contains("<DURATION>"), "{text}");
        assert!(!text.contains("<ARGS>"), "{text}");
    }

    #[test]
    fn too_many_values_error_hides_args_name() {
        let (kind, text) =
            clap_text(super::parse(&os(&["mlm", "week", "target", "1", "2", "3"])).unwrap_err());
        assert_eq!(kind, clap::error::ErrorKind::TooManyValues);
        assert!(!text.contains("<ARGS>"), "{text}");
    }

    #[test]
    fn unrelated_errors_pass_through_untouched() {
        let (kind, text) = clap_text(super::parse(&os(&["mlm", "week", "7", "foo"])).unwrap_err());
        assert_eq!(kind, clap::error::ErrorKind::ArgumentConflict);
        assert!(text.contains("Usage:"), "{text}");
    }

    // --- relative WEEK_ID (SPEC §3.6) -------------------------------------

    #[test]
    fn week_positional_takes_negative_numbers_verbatim() {
        for tok in [
            "-1",
            "-01",
            "-07",
            "-2",
            "-0",
            "-1.5",
            "-1e3",
            "-99999999999999999999",
        ] {
            let a = week_view_args(parse(&["mlm", "week", tok]).unwrap());
            assert_eq!(a.week_id.as_deref(), Some(tok));
            assert!(!a.notes, "{tok}");
        }
    }

    #[test]
    fn week_minus_n_works_with_flags_in_either_order() {
        for (args, notes) in [
            (&["mlm", "week", "-1", "-n"][..], true),
            (&["mlm", "week", "-n", "-1"], true),
            (&["mlm", "w", "-1", "--notes"], true),
            (&["mlm", "week", "-v", "-1"], false),
            (&["mlm", "week", "-1", "-v"], false),
        ] {
            let a = week_view_args(parse(args).unwrap());
            assert_eq!(a.week_id.as_deref(), Some("-1"), "{args:?}");
            assert_eq!(a.notes, notes, "{args:?}");
        }
    }

    #[test]
    fn week_non_numeric_hyphen_tokens_stay_usage_errors() {
        for tok in ["-1x", "-x", "--bogus"] {
            let failure = super::parse(&os(&["mlm", "week", tok])).unwrap_err();
            assert_eq!(failure.exit_code(), 2, "{tok}");
            let (kind, _) = clap_text(failure);
            assert_eq!(kind, clap::error::ErrorKind::UnknownArgument, "{tok}");
        }
    }

    #[test]
    fn swapped_accepts_a_relative_week() {
        let args = ["mlm", "week", "-1", "target", "35h"];
        assert_eq!(
            swapped(&args),
            Some("error: put WEEK_ID after target: mlm week target -1 35h".to_string())
        );
        let failure = super::parse(&os(&args)).unwrap_err();
        assert!(
            matches!(failure, ParseFailure::SwappedWeekTarget(_)),
            "{failure:?}"
        );
        assert_eq!(failure.exit_code(), 2);
    }

    fn lone(args: &[&str], today: chrono::NaiveDate) -> Option<ParseFailure> {
        let cli = Cli::try_parse_from(args).unwrap();
        lone_week_id_failure(&os(args), &cli, today)
    }

    #[test]
    fn lone_week_id_after_target_is_a_missing_duration() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        let (_, bare) = clap_text(super::parse(&os(&["mlm", "week", "target"])).unwrap_err());
        for args in [
            &["mlm", "week", "target", "-1"][..],
            &["mlm", "week", "target", "7"],
            &["mlm", "week", "target", "2026-07"],
            &["mlm", "week", "target", "-01"],
            &["mlm", "w", "target", "-1"],
            &["mlm", "-v", "week", "target", "-1"],
            &["mlm", "week", "target", "-v", "7"],
            &["mlm", "week", "target", "--", "-1"],
        ] {
            let failure = lone(args, today).unwrap_or_else(|| panic!("{args:?}"));
            assert_eq!(failure.exit_code(), 2, "{args:?}");
            let (kind, text) = clap_text(failure);
            assert_eq!(
                kind,
                clap::error::ErrorKind::MissingRequiredArgument,
                "{args:?}"
            );
            assert_eq!(text, bare, "{args:?}");
            assert!(text.contains("<DURATION>") && !text.contains("<ARGS>"));
        }
    }

    #[test]
    fn other_lone_target_tokens_are_left_alone() {
        let today = chrono::NaiveDate::from_ymd_opt(2027, 6, 15).unwrap();
        for args in [
            &["mlm", "week", "target", "35h"][..],
            &["mlm", "week", "target", "-0"],
            &["mlm", "week", "target", "53"],
            &["mlm", "week", "target", "-99999999999999999999"],
            &["mlm", "week", "target", "2026-07", "35h"],
            &["mlm", "week", "7"],
            &["mlm", "status", "-1"],
        ] {
            assert!(lone(args, today).is_none(), "{args:?}");
        }
    }

    #[test]
    fn parse_turns_a_lone_week_id_into_the_usage_error() {
        let failure = super::parse(&os(&["mlm", "week", "target", "-1"])).unwrap_err();
        assert!(matches!(failure, ParseFailure::Clap(_)), "{failure:?}");
        assert_eq!(failure.exit_code(), 2);
        assert!(super::parse(&os(&["mlm", "week", "target", "35h"])).is_ok());
    }

    // --- export notes: cli surface ---------------------------------------

    fn export_notes_args(cli: Cli) -> ExportNotesArgs {
        let Command::Export(ExportArgs {
            target: ExportTarget::Notes(a),
        }) = cli.command
        else {
            panic!("expected export notes");
        };
        a
    }

    #[test]
    fn export_aliases_parse_like_the_long_form() {
        let a = export_notes_args(parse(&["mlm", "ex", "n", "-w", "43"]).unwrap());
        assert_eq!(a.week.as_deref(), Some("43"));
        assert_eq!(a.date, None);
        assert_eq!(a.month, None);
        let cli = parse(&["mlm", "n", "hi"]).unwrap();
        assert!(matches!(cli.command, Command::Note(_)), "{:?}", cli.command);
    }

    #[test]
    fn export_range_flags_take_hyphen_values() {
        let a = export_notes_args(parse(&["mlm", "export", "notes", "-d", "-1"]).unwrap());
        assert_eq!(a.date.as_deref(), Some("-1"));
        let a = export_notes_args(parse(&["mlm", "export", "notes", "-w", "-1"]).unwrap());
        assert_eq!(a.week.as_deref(), Some("-1"));
        let a = export_notes_args(parse(&["mlm", "export", "notes", "-m", "-1"]).unwrap());
        assert_eq!(a.month.as_deref(), Some("-1"));
        let a = export_notes_args(parse(&["mlm", "export", "notes", "--month=3"]).unwrap());
        assert_eq!(a.month.as_deref(), Some("3"));
        let a = export_notes_args(parse(&["mlm", "export", "notes", "-m", "-d"]).unwrap());
        assert_eq!(a.month.as_deref(), Some("-d"));
        assert_eq!(a.date, None);
    }

    #[test]
    fn export_with_no_flag_has_no_range() {
        let a = export_notes_args(parse(&["mlm", "export", "notes"]).unwrap());
        assert_eq!((a.date, a.week, a.month), (None, None, None));
    }

    #[test]
    fn export_usage_errors_are_clap_errors() {
        use clap::error::ErrorKind;
        let cases: [(&[&str], ErrorKind); 4] = [
            (
                &["mlm", "export"],
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
            ),
            (
                &[
                    "mlm",
                    "export",
                    "notes",
                    "-d",
                    "2026-01-05",
                    "-w",
                    "2026-02",
                ],
                ErrorKind::ArgumentConflict,
            ),
            (
                &["mlm", "export", "notes", "-m", "3", "-m", "4"],
                ErrorKind::ArgumentConflict,
            ),
            (&["mlm", "export", "notes", "-d"], ErrorKind::InvalidValue),
        ];
        for (args, want) in cases {
            let failure = super::parse(&os(args)).unwrap_err();
            assert_eq!(failure.exit_code(), 2, "{args:?}");
            let (kind, _) = clap_text(failure);
            assert_eq!(kind, want, "{args:?}");
        }
    }
}
