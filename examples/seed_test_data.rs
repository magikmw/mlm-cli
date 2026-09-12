//! Dev-only test-data generator: populates an `mlm` database with a few
//! weeks of randomized-but-plausible punches/notes ending today, for
//! manually exercising `status`/`week` without hand-typing entries.
//!
//! NOT part of the shipped CLI (SPEC.md's command surface is fixed at
//! six commands) — this is tooling, run via `cargo run --example
//! seed_test_data`. Writes through the real `mlm::storage`/`mlm::db`
//! insert functions, so every row is exactly what a real `start`/`stop`/
//! `note`/`week target` invocation would have produced (correct
//! per-instant UTC conversion, trimming, etc.) — just backdated, which
//! the real CLI deliberately never allows (§1.2).
//!
//! Usage:
//!   cargo run --example seed_test_data -- [--weeks N] [--seed N] [--db PATH]
//!
//! Refuses to run against the real app-data database by accident: you
//! must pass `--db PATH` or set `MLM_DB_PATH` (same env var the real
//! binary honors). There is no default fallback to the real path.

use std::path::PathBuf;

use chrono::{Datelike, Duration, Local, NaiveTime};
use mlm::date::WeekId;
use mlm::storage::{self, PunchKind};
use mlm::{db, week_target};

/// Tiny dependency-free PRNG (SplitMix64) — good enough for "plausible
/// randomness," and keeps this example from needing a `rand` dependency
/// just for test-data generation.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Uniform float in `[0, 1)`.
    fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform integer in `[lo, hi]`, inclusive.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.f64() * (hi - lo + 1) as f64) as i64
    }

    fn chance(&mut self, probability: f64) -> bool {
        self.f64() < probability
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() as i64 - 1) as usize]
    }
}

const NOTE_POOL: &[&str] = &[
    "fixed a flaky test",
    "reviewed PRs",
    "pairing on the auth flow",
    "wrote docs for the new endpoint",
    "investigated the slow query",
    "onboarding sync",
    "cleaned up the CI pipeline",
    "refactored the parser",
    "debugging the deploy",
    "planning next sprint",
    "code review backlog",
    "customer bug triage",
];

const PROJECT_TAGS: &[&str] = &["api", "infra", "web", "mobile"];

fn main() -> anyhow::Result<()> {
    let mut weeks: i64 = 3;
    let mut seed: u64 = Local::now().timestamp() as u64;
    let mut db_path: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--weeks" => weeks = args.next().expect("--weeks needs a value").parse()?,
            "--seed" => seed = args.next().expect("--seed needs a value").parse()?,
            "--db" => db_path = Some(PathBuf::from(args.next().expect("--db needs a value"))),
            other => anyhow::bail!("unrecognized argument: {other}"),
        }
    }

    let path = db_path
        .or_else(|| std::env::var_os("MLM_DB_PATH").map(PathBuf::from))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "refusing to guess a database path — pass --db PATH or set MLM_DB_PATH, \
                 so this never accidentally seeds your real mlm database"
            )
        })?;

    let conn = db::connect_at(&path)?;
    let mut rng = Rng::new(seed);

    let today = Local::now().date_naive();
    let now_time = Local::now().time();
    let now_utc = Local::now().with_timezone(&chrono::Utc);
    let first_day = today - Duration::days(weeks * 7 - 1);

    println!("seeding {weeks} week(s) into {}", path.display());
    println!("seed: {seed} (pass --seed {seed} to reproduce this exact run)");
    println!("date range: {first_day} .. {today}");
    println!();

    let mut punch_count = 0u32;
    let mut note_count = 0u32;
    let mut anomaly_notes = Vec::new();

    let mut date = first_day;
    while date <= today {
        let is_today = date == today;
        let weekday = date.weekday();
        let is_weekend = matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun);

        // Roughly: weekdays are worked unless a rolled "day off", a rare
        // weekend session happens, and today is truncated at "now" if
        // it's a workday (can't punch in the future).
        let works_today = if is_weekend {
            rng.chance(0.05)
        } else {
            !rng.chance(0.08) // ~8% chance of a full day off (PTO/sick)
        };

        if works_today {
            let (stints, left_open) = plausible_stints(&mut rng, is_today, now_time);
            for (start, end) in &stints {
                storage::insert_punch(&conn, PunchKind::Start, date, *start, &Local)?;
                punch_count += 1;
                if let Some(end) = end {
                    storage::insert_punch(&conn, PunchKind::End, date, *end, &Local)?;
                    punch_count += 1;
                }
            }
            if left_open {
                println!("  {date}: left open (ongoing stint) — exercises the EOD estimate");
            }

            if rng.chance(0.75) {
                let mut body = rng.pick(NOTE_POOL).to_string();
                if rng.chance(0.3) {
                    body = format!("{}: {}", rng.pick(PROJECT_TAGS), body);
                }
                storage::insert_note(&conn, date, &body, now_utc)?;
                note_count += 1;
            }
        }

        // Small chance (once every few weeks, never on today) of a
        // deliberate anomaly, to exercise status/week's `[!]` rendering.
        // An orphaned `end` (a stop with nothing open to close) is a
        // flagged anomaly regardless of what else happened that day —
        // unlike a single stray trailing `start`, which SPEC.md treats
        // as an ordinary open stint, not an anomaly, on its own.
        if !is_today && rng.chance(0.05) {
            let stray_time = NaiveTime::from_hms_opt(rng.range(20, 22) as u32, 0, 0).unwrap();
            storage::insert_punch(&conn, PunchKind::End, date, stray_time, &Local)?;
            punch_count += 1;
            anomaly_notes.push(format!(
                "  {date}: injected an orphaned `end` at {stray_time} (flagged anomaly)"
            ));
        }

        date += Duration::days(1);
    }

    // Override one of the covered weeks' targets, so `week`'s target
    // override path (§3.7) has something to show too.
    let candidate_weeks: Vec<WeekId> = {
        let mut seen = Vec::new();
        let mut d = first_day;
        while d <= today {
            let w = WeekId::from_date(d);
            if seen.last() != Some(&w) {
                seen.push(w);
            }
            d += Duration::days(1);
        }
        seen
    };
    if let Some(overridden) = candidate_weeks.first() {
        // A plausible half-week: 20h instead of the 40h default.
        week_target::set_week_target(&conn, overridden, 20 * 60)?;
        println!(
            "  target override: week {} set to 20h 00m (half week)",
            overridden
        );
    }

    println!();
    println!("done: {punch_count} punches, {note_count} notes inserted.");
    if !anomaly_notes.is_empty() {
        println!("anomalies injected for testing:");
        for line in &anomaly_notes {
            println!("{line}");
        }
    }
    println!();
    println!("try:");
    println!("  MLM_DB_PATH={} cargo run -- status", path.display());
    println!("  MLM_DB_PATH={} cargo run -- week", path.display());

    Ok(())
}

/// One day's worth of stints: usually one continuous block, sometimes
/// split by a lunch break. `is_today` truncates everything at `now` (no
/// future punches) and may leave the day's last stint open.
fn plausible_stints(
    rng: &mut Rng,
    is_today: bool,
    now: NaiveTime,
) -> (Vec<(NaiveTime, Option<NaiveTime>)>, bool) {
    let start_hour = rng.range(7, 9);
    let start_minute = rng.range(0, 59);
    let start = NaiveTime::from_hms_opt(start_hour as u32, start_minute as u32, 0).unwrap();

    let total_minutes = rng.range(390, 540); // 6.5h .. 9h
    let split_lunch = rng.chance(0.4);

    let mut stints = Vec::new();
    let mut left_open = false;

    if split_lunch {
        let morning = rng.range(150, total_minutes - 120).max(60);
        let lunch_gap = rng.range(30, 60);
        let afternoon = (total_minutes - morning).max(60);

        let morning_end = start + Duration::minutes(morning);
        let afternoon_start = morning_end + Duration::minutes(lunch_gap);
        let afternoon_end = afternoon_start + Duration::minutes(afternoon);

        if is_today && start >= now {
            // Can't have plausibly started yet today; skip entirely.
            return (Vec::new(), false);
        }
        if is_today && morning_end > now {
            stints.push((start, None));
            left_open = true;
        } else if is_today && afternoon_start > now {
            stints.push((start, Some(morning_end)));
            // Not open — the gap itself just ends there today.
        } else if is_today && afternoon_end > now {
            stints.push((start, Some(morning_end)));
            stints.push((afternoon_start, None));
            left_open = true;
        } else {
            stints.push((start, Some(morning_end)));
            stints.push((afternoon_start, Some(afternoon_end)));
        }
    } else {
        let end = start + Duration::minutes(total_minutes);
        if is_today && start >= now {
            return (Vec::new(), false);
        }
        if is_today && end > now {
            stints.push((start, None));
            left_open = true;
        } else {
            stints.push((start, Some(end)));
        }
    }

    (stints, left_open)
}
