mod cli;
mod date;
mod db;
mod stint;
mod time;
mod week;
mod week_target;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let _conn = db::connect().expect("failed to open database");

    match cli.command {
        Command::Start { note } => {
            println!("TODO: start point today, note={note:?}");
        }
        Command::Stop { note } => {
            println!("TODO: stop point today, note={note:?}");
        }
        Command::Log { date } => {
            println!("TODO: show log for {date:?}");
        }
        // Full dispatch (including the `action: None` render arm) is
        // Milestone 7/11's wiring pass; `week_target::run` is unit-tested
        // standalone in the meantime (Milestone 8 scope).
        Command::Week(_args) => {
            todo!("`week`/`week target` dispatch is wired by a later milestone")
        }
    }
}
