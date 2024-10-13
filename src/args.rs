use anyhow::Result;
use clap::{Arg, ArgAction, Command};

pub async fn parse() -> Result<()> {
    let matches = command().get_matches();

    Ok(())
}

fn command() -> Command {
    Command::new("sc")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Spotify CLI controller")
        .arg_required_else_help(true)
        .next_help_heading("Control spotify")
        .subcommand(Command::new("current").about("Output current track"))
        .subcommand(Command::new("pause").about("Pause playback"))
        .subcommand(Command::new("resume").about("Resume playback"))
        .subcommand(Command::new("toggle").about("Toggle resume/pause"))
        .subcommand(Command::new("volume").about("Control volume"))
        .subcommand(Command::new("play").about("Play first matching content"))
        .subcommand(Command::new("search").about("Search content"))
        .subcommand(Command::new("next").about("Skip current track"))
        .subcommand(Command::new("prev").about("Play previous track"))
        .subcommand(Command::new("shuffle").about("Control shuffle mode"))
        .subcommand(Command::new("repeat").about("Control repeat mode"))
        .next_help_heading("Settings")
        .args([Arg::new("authorize")
            .long("authorize")
            .help("Run the authorization process")
            .action(ArgAction::SetTrue)])
}
