mod cli;
mod db;
mod time;

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
    }
}
