//! CLI surface end-to-end (possible-issues-triage T5): default log level
//! (§4.1), `week target` echo (§3.5), help glossary (§4.2), `week target`
//! usage text and argument-order interception (§4.3).

use std::path::Path;
use std::process::{Command, Output};

use mlm::cli::GLOSSARY;
use tempfile::TempDir;

fn mlm(db: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlm"))
        .args(args)
        .env("MLM_DB_PATH", db)
        .env("TZ", "UTC")
        .env_remove("RUST_LOG")
        .output()
        .expect("failed to run mlm binary")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn fresh() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let db = dir.path().join("mlm.db");
    (dir, db)
}

fn run(args: &[&str]) -> Output {
    let (_dir, db) = fresh();
    mlm(&db, args)
}

// ---------------------------------------------------------------- §4.1

#[test]
fn default_log_level_is_silent_on_fresh_db() {
    let o = run(&["start"]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(err(&o), "");
    assert_eq!(out(&o), "");
}

#[test]
fn verbose_long_shows_info_and_debug() {
    let o = run(&["--verbose", "start"]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    let e = err(&o);
    assert!(e.contains("Database migrated"), "{e}");
    assert!(e.contains("DEBUG"), "{e}");
}

#[test]
fn verbose_short_shows_info_and_debug() {
    let o = run(&["-v", "start"]);
    let e = err(&o);
    assert!(e.contains("Database migrated"), "{e}");
    assert!(e.contains("DEBUG"), "{e}");
}

// ---------------------------------------------------------------- §3.5

#[test]
fn echo_current_week() {
    let id = mlm::date::WeekId::current(chrono::Utc::now().date_naive());
    let o = run(&["week", "target", "20h"]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(out(&o), format!("Week {id} target: 20h 00m\n"));
    assert_eq!(err(&o), "");
}

#[test]
fn echo_explicit_week() {
    let o = run(&["week", "target", "2026-41", "20h"]);
    assert_eq!(out(&o), "Week 2026-41 target: 20h 00m\n");
}

#[test]
fn echo_normalizes_week_id() {
    let o = run(&["week", "target", "2026-7", "33h30m"]);
    assert_eq!(out(&o), "Week 2026-07 target: 33h 30m\n");
}

#[test]
fn echo_zero_targets() {
    for d in ["0h", "0m"] {
        let o = run(&["week", "target", "2026-41", d]);
        assert_eq!(out(&o), "Week 2026-41 target: 00h 00m\n", "{d}");
    }
}

#[test]
fn echo_uses_standard_formatter() {
    let o = run(&["week", "target", "2026-41", "90m"]);
    assert_eq!(out(&o), "Week 2026-41 target: 01h 30m\n");
}

#[test]
fn echo_via_alias() {
    let o = run(&["w", "target", "2026-41", "20h"]);
    assert_eq!(out(&o), "Week 2026-41 target: 20h 00m\n");
}

#[test]
fn echo_with_verbose_keeps_stdout_clean() {
    let o = run(&["--verbose", "week", "target", "2026-41", "20h"]);
    assert_eq!(out(&o), "Week 2026-41 target: 20h 00m\n");
}

#[test]
fn failed_target_prints_nothing_on_stdout() {
    for args in [
        &["week", "target", "2026-41", "abc"][..],
        &["week", "target", "2027-53", "20h"],
        &["week", "target", "abcd", "20h"],
        &["week", "target", "-5h"],
    ] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(1), "{args:?}: {o:?}");
        assert_eq!(out(&o), "", "{args:?}");
        assert!(err(&o).starts_with("error:"), "{args:?}: {}", err(&o));
    }
}

#[test]
fn other_writers_stay_silent() {
    for args in [&["start"][..], &["stop"], &["note", "hello"]] {
        let (_d, db) = fresh();
        if args[0] == "stop" {
            mlm(&db, &["start", "9:00"]);
        }
        let o = mlm(&db, args);
        assert_eq!(out(&o), "", "{args:?}");
    }
}

// ---------------------------------------------------------------- §4.2

const GLOSSARY_TERMS: [&str; 18] = [
    "worked",
    "carry-in",
    "fulfillment",
    "left to",
    "over",
    "by end of <weekday>",
    "<target> target",
    "est. EOD",
    "done for today",
    "(incl. ongoing)",
    "(+ ongoing)",
    "Total behind, Total ahead",
    "\n  overlapping stints\n",
    "\n  orphaned end\n",
    "\n  so far, started yesterday\n",
    "\n  spans to next day, continues previous day's stint\n",
    "[!]",
    "[i]",
];

#[test]
fn long_help_carries_glossary() {
    let variants: [&[&str]; 6] = [
        &["status", "--help"],
        &["week", "--help"],
        &["d", "--help"],
        &["w", "--help"],
        &["help", "status"],
        &["help", "week"],
    ];
    for args in variants {
        let o = run(args);
        assert_eq!(o.status.code(), Some(0), "{args:?}");
        let s = out(&o);
        for term in GLOSSARY_TERMS {
            assert!(s.contains(term), "{args:?} lacks {term:?}:\n{s}");
        }
        assert!(s.contains("shortfall is negative"), "{args:?}");
        assert!(s.contains("surplus is positive"), "{args:?}");
        assert!(!s.contains("target already met"), "{args:?}");
        let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.ends_with("It marks yesterday's row in `mlm week` until that start is stopped."),
            "{args:?}"
        );
        assert!(!flat.contains("It never marks a row."), "{args:?}");
        assert!(
            flat.contains("a start still open from yesterday, listed on today's page but not counted until you stop it"),
            "{args:?}"
        );
        assert!(flat.contains("counted on the day it started"), "{args:?}");
    }
}

#[test]
fn glossary_is_ascii() {
    assert!(GLOSSARY.is_ascii());
}

const STATUS_SHORT: &str = "\
Show a date's stints, notes and totals (defaults to today)

Usage: mlm status [OPTIONS] [DATE]

Arguments:
  [DATE]  Date to show: YYYY-MM-DD, or `-N` for N days before today (e.g. `-1` = yesterday). Defaults to today

Options:
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help (see more with '--help')
";

const WEEK_SHORT: &str = "\
Show a week's totals, or set its target

Usage: mlm week [OPTIONS] [WEEK_ID]
       mlm week <COMMAND>

Commands:
  target  Set an absolute target override for a week
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [WEEK_ID]  Week to show: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7). Defaults to the current week

Options:
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help (see more with '--help')
";

#[test]
fn short_help_goldens() {
    let s = run(&["status", "-h"]);
    assert_eq!(out(&s), STATUS_SHORT);
    let w = run(&["week", "-h"]);
    assert_eq!(out(&w), WEEK_SHORT);
    assert!(!out(&s).contains("Glossary"));
    assert!(!out(&w).contains("Glossary"));
}

// ---------------------------------------------------------------- §4.3 help

#[test]
fn week_target_help_text() {
    let long = run(&["week", "target", "--help"]);
    let short = run(&["week", "target", "-h"]);
    assert_eq!(long.status.code(), Some(0));
    assert_eq!(out(&long), out(&short));
    let expected = "\
Set an absolute target override for a week

Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>

Arguments:
  [WEEK_ID]   Week to set: YYYY-WW (e.g. 2026-07) or a bare week number (e.g. 7). Defaults to the current week
  <DURATION>  Target to set: 20h, 33h30m or 45m (human format, never raw minutes)

Options:
  -v, --verbose  Enable debug-level logging
  -h, --help     Print help
";
    assert_eq!(out(&long), expected);
}

#[test]
fn week_target_missing_duration_names_duration() {
    let o = run(&["week", "target"]);
    assert_eq!(o.status.code(), Some(2));
    let e = err(&o);
    assert!(e.contains("<DURATION>"), "{e}");
    assert!(
        e.contains("Usage: mlm week target [OPTIONS] [WEEK_ID] <DURATION>"),
        "{e}"
    );
    assert!(!e.contains("<ARGS>"), "{e}");
    assert_eq!(out(&o), "");
}

#[test]
fn week_target_too_many_values_hides_args() {
    let o = run(&["week", "target", "2026-41", "20h", "extra"]);
    assert_eq!(o.status.code(), Some(2));
    let e = err(&o);
    assert!(e.contains("unexpected value 'extra'"), "{e}");
    assert!(!e.contains("<ARGS>"), "{e}");
    let o = run(&["week", "target", "1", "2", "3"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(!err(&o).contains("<ARGS>"), "{}", err(&o));
}

#[test]
fn hyphen_tokens_after_week_target() {
    let o = run(&["week", "target", "3", "-h"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("invalid duration \"-h\""), "{}", err(&o));
    let o = run(&["week", "target", "-h", "3"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(out(&o).contains("Usage: mlm week target"));
}

// ---------------------------------------------------------------- §4.3 order

const SWAP_LINE: &str = "error: put WEEK_ID after target: mlm week target 2026-41 20h\n";

#[test]
fn swapped_order_is_intercepted() {
    for args in [
        &["week", "2026-41", "target", "20h"][..],
        &["w", "2026-41", "target", "20h"],
    ] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert_eq!(err(&o), SWAP_LINE, "{args:?}");
        assert_eq!(out(&o), "");
    }
}

#[test]
fn swapped_order_ignores_verbose_position() {
    for args in [
        &["--verbose", "week", "7", "target", "33h30m"][..],
        &["week", "--verbose", "7", "target", "33h30m"],
        &["week", "7", "--verbose", "target", "33h30m"],
        &["week", "7", "target", "--verbose", "33h30m"],
        &["week", "7", "target", "33h30m", "-v"],
    ] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert_eq!(
            err(&o),
            "error: put WEEK_ID after target: mlm week target 7 33h30m\n",
            "{args:?}"
        );
    }
}

#[test]
fn swapped_order_echoes_tokens_as_typed() {
    let o = run(&["week", "2026-7", "target", "20h"]);
    assert_eq!(
        err(&o),
        "error: put WEEK_ID after target: mlm week target 2026-7 20h\n"
    );
    let o = run(&["week", "2026-41", "target", "-5h"]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        err(&o),
        "error: put WEEK_ID after target: mlm week target 2026-41 -5h\n"
    );
}

#[test]
fn other_week_errors_keep_clap_text() {
    for args in [
        &["week", "7", "foo"][..],
        &["week", "2026-41", "target"],
        &["week", "2026-41", "target", "20h", "extra"],
        &["week", "7", "target", "--help"],
        &["week", "foo$(id)", "target", "20h"],
        &["week", "2026-99", "target", "20h"],
    ] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        let e = err(&o);
        assert!(e.contains("Usage:"), "{args:?}: {e}");
        assert!(!e.contains("put WEEK_ID"), "{args:?}: {e}");
        assert_eq!(out(&o), "");
    }
}

#[test]
fn correct_order_and_views_still_work() {
    for args in [
        &["week", "target", "2026-41", "20h"][..],
        &["week", "2026-41"],
        &["week"],
    ] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(0), "{args:?}: {o:?}");
    }
}

#[test]
fn help_and_version_exit_zero() {
    for args in [&["--help"][..], &["week", "--help"], &["--version"]] {
        let o = run(args);
        assert_eq!(o.status.code(), Some(0), "{args:?}");
        assert_eq!(err(&o), "");
        assert!(!out(&o).is_empty());
    }
}
