use anyhow::Result;
use clap::{value_parser, Arg, ArgAction, ArgGroup, ArgMatches, Command};
use rspotify::model::SearchType;

use crate::{auth, ui};

/// Describes a volume operation either increase, decrease or set.
#[derive(Clone)]
enum VolumeOperation {
    Increase(u8),
    Decrease(u8),
    Set(u8),
}

/// Describes a shuffle operation either on or off.
#[derive(Clone)]
enum ShuffleOperation {
    On,
    Off,
}

/// Describes a repeat operation either on, off or track.
#[derive(Clone)]
enum RepeatOperation {
    On,
    Off,
    Track,
}

/// Parse the command line arguments
pub async fn parse() -> Result<()> {
    let matches = command().get_matches();

    if matches.get_flag("authorize") {
        auth::run_flow().await?;
        return Ok(());
    }

    // Get SpotifyPlayer instance, run auth flow if user is unauthorized
    let mut player = match auth::load_cached().await? {
        Some(player) => player,
        None => auth::run_flow().await?,
    };

    if let Some(_) = matches.subcommand_matches("current") {
        let track = player.current_track().await?;

        match track {
            Some(t) => println!("\"{}\" by {}", t.title, t.by.join(", ")),
            None => println!("Nothing playing"),
        }

        return Ok(());
    }

    if let Some(_) = matches.subcommand_matches("pause") {
        return player.playback_pause().await;
    }

    if let Some(_) = matches.subcommand_matches("resume") {
        return player.playback_resume().await;
    }

    if let Some(_) = matches.subcommand_matches("toggle") {
        return player.playback_toggle().await;
    }

    if let Some(vol) = matches.subcommand_matches("volume") {
        if let Some(op) = vol.get_one::<VolumeOperation>("amount") {
            return match op.clone() {
                VolumeOperation::Increase(i) => player.volume_up(i).await,
                VolumeOperation::Decrease(d) => player.volume_down(d).await,
                VolumeOperation::Set(s) => player.volume_set(s).await,
            };
        }
    }

    if let Some(play) = matches.subcommand_matches("play") {
        if let Some(play_type) = type_matches(play) {
            if let Some(query) = play.get_one::<String>("content") {
                let res = player.search(query.clone(), play_type, Some(1)).await?;

                if let Some(first) = res.get(0) {
                    player.play(first).await?;
                } else {
                    println!("No matches found")
                }
            }
        }

        return Ok(());
    }

    if let Some(search) = matches.subcommand_matches("search") {
        if let Some(search_type) = type_matches(search) {
            if let Some(query) = search.get_one::<String>("content") {
                let count = search.get_one::<u32>("count").unwrap_or_else(|| &10);

                let res = player
                    .search(query.clone(), search_type, Some(*count))
                    .await?;

                let selected = ui::select_playable(res)?;

                return player.play(&selected).await;
            }
        }
    }

    if let Some(library) = matches.subcommand_matches("library") {
        let playlists = player.playlists().await?;

        let selected_playlist = match library.get_one::<String>("name") {
            Some(filter) => playlists.into_iter().find(|p| {
                p.to_display()
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }),
            None => Some(ui::select_playable(playlists)?),
        };

        match selected_playlist {
            Some(p) => player.play(&p).await?,
            None => println!("No matching library playlist found"),
        }

        return Ok(());
    }

    if let Some(device) = matches.subcommand_matches("device") {
        let devices = player.devices().await?;

        let selected_device = match device.get_one::<String>("name") {
            Some(filter) => devices
                .into_iter()
                .find(|d| d.name.to_lowercase().contains(&filter.to_lowercase())),
            None => Some(ui::select_device(devices)?),
        };

        match selected_device {
            Some(d) => player.set_device(d).await?,
            None => println!("No matching playback device found"),
        }

        return Ok(());
    }

    if let Some(_) = matches.subcommand_matches("next") {
        return player.track_next().await;
    }

    if let Some(_) = matches.subcommand_matches("prev") {
        return player.track_prev().await;
    }

    if let Some(shuffle) = matches.subcommand_matches("shuffle") {
        return match shuffle.get_one::<ShuffleOperation>("mode") {
            Some(mode) => match mode {
                ShuffleOperation::On => player.shuffle_on().await,
                ShuffleOperation::Off => player.shuffle_off().await,
            },
            None => player.shuffle_toggle().await,
        };
    }

    if let Some(repeat) = matches.subcommand_matches("repeat") {
        return match repeat.get_one::<RepeatOperation>("mode") {
            Some(mode) => match mode {
                RepeatOperation::On => player.repeat_on().await,
                RepeatOperation::Off => player.repeat_off().await,
                RepeatOperation::Track => player.repeat_track().await,
            },
            None => player.repeat_toggle().await,
        };
    }

    Ok(())
}

/// Get the settings for command parsing
fn command() -> Command {
    Command::new("sc")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Spotify CLI controller")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("current")
                .about("Output current track")
                .alias("cu"),
        )
        .subcommand(Command::new("pause").about("Pause playback").alias("pa"))
        .subcommand(Command::new("resume").about("Resume playback").alias("re"))
        .subcommand(
            Command::new("toggle")
                .about("Toggle resume/pause")
                .alias("to"),
        )
        .subcommand(
            Command::new("volume")
                .about("Control volume")
                .alias("vo")
                .arg(
                    Arg::new("amount")
                        .help("Set or change volume in percent [50 | +5 | -5]")
                        .allow_hyphen_values(true)
                        .action(ArgAction::Set)
                        .value_parser(volume_parser),
                )
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("play")
                .about("Play first matching content")
                .alias("pl")
                .group(ArgGroup::new("type").required(true).multiple(false))
                .args([
                    Arg::new("track")
                        .help("Play tracks")
                        .group("type")
                        .long("track")
                        .short('t')
                        .action(ArgAction::SetTrue),
                    Arg::new("playlist")
                        .help("Play playlists")
                        .group("type")
                        .long("playlist")
                        .short('p')
                        .action(ArgAction::SetTrue),
                    Arg::new("album")
                        .help("Play albums")
                        .group("type")
                        .long("album")
                        .short('a')
                        .action(ArgAction::SetTrue),
                    Arg::new("artist")
                        .help("Play artists")
                        .group("type")
                        .long("artist")
                        .short('A')
                        .action(ArgAction::SetTrue),
                    Arg::new("show")
                        .help("Play shows")
                        .group("type")
                        .long("show")
                        .short('s')
                        .action(ArgAction::SetTrue),
                    Arg::new("episode")
                        .help("Play episode")
                        .group("type")
                        .long("episode")
                        .short('e')
                        .action(ArgAction::SetTrue),
                    Arg::new("content")
                        .help("Content to play")
                        .required(true)
                        .action(ArgAction::Set),
                ])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("search")
                .about("Search content")
                .alias("se")
                .group(ArgGroup::new("type").required(true).multiple(false))
                .args([
                    Arg::new("count")
                        .help("The amount of items to search")
                        .long("count")
                        .short('c')
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(u32)),
                    Arg::new("track")
                        .help("Search for tracks")
                        .group("type")
                        .long("track")
                        .short('t')
                        .action(ArgAction::SetTrue),
                    Arg::new("playlist")
                        .help("Search for playlists")
                        .group("type")
                        .long("playlist")
                        .short('p')
                        .action(ArgAction::SetTrue),
                    Arg::new("album")
                        .help("Search for albums")
                        .group("type")
                        .long("album")
                        .short('a')
                        .action(ArgAction::SetTrue),
                    Arg::new("artist")
                        .help("Search for artists")
                        .group("type")
                        .long("artist")
                        .short('A')
                        .action(ArgAction::SetTrue),
                    Arg::new("show")
                        .help("Search for shows")
                        .group("type")
                        .long("show")
                        .short('s')
                        .action(ArgAction::SetTrue),
                    Arg::new("episode")
                        .help("Search for episodes")
                        .group("type")
                        .long("episode")
                        .short('e')
                        .action(ArgAction::SetTrue),
                    Arg::new("content")
                        .help("Content to search for")
                        .required(true)
                        .action(ArgAction::Set),
                ])
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("library")
                .about("Play playlist from users library")
                .alias("li")
                .after_help(
                    "Displays selection from all playlists from library, if no name is specified",
                )
                .args([Arg::new("name")
                    .help("Play first playlist from library matching this name (optional)")
                    .required(false)
                    .action(ArgAction::Set)]),
        )
        .subcommand(
            Command::new("device")
                .about("Select a playback device")
                .alias("de")
                .after_help(
                    "Displays selection from all available playback devices, if no name is specified",
                )
                .args([Arg::new("name")
                    .help("Selects first available playback device matching this name (optional)")
                    .required(false)
                    .action(ArgAction::Set)]),
        )
        .subcommand(Command::new("next").about("Skip current track").alias("ne"))
        .subcommand(
            Command::new("prev")
                .about("Play previous track")
                .alias("pr"),
        )
        .subcommand(
            Command::new("shuffle")
                .about("Control shuffle mode")
                .alias("sh")
                .after_help("Toggles between on/off if no mode is supplied")
                .args([Arg::new("mode")
                    .help("The shuffle mode [on | off] (optional)")
                    .required(false)
                    .action(ArgAction::Set)
                    .value_parser(shuffle_parser)]),
        )
        .subcommand(
            Command::new("repeat")
                .about("Control repeat mode")
                .alias("rp")
                .after_help("Toggles between on/off if no mode is supplied")
                .args([Arg::new("mode")
                    .help("The repeat mode [on | off | track] (optional)")
                    .required(false)
                    .action(ArgAction::Set)
                    .value_parser(repeat_parser)]),
        )
        .next_help_heading("Settings")
        .args([Arg::new("authorize")
            .long("authorize")
            .help("Run the authorization process")
            .exclusive(true)
            .action(ArgAction::SetTrue)])
}

/// A custom parser for volume arguments
fn volume_parser(arg: &str) -> Result<VolumeOperation, String> {
    fn parse_num(str: &str) -> Result<u8, String> {
        let num = str
            .parse::<u8>()
            .map_err(|_| format!("\"{}\" is not a valid number value", str))?;

        match num {
            0..=100 => Ok(num),
            _ => Err(format!("Please provide a volume value between 0 and 100")),
        }
    }

    if arg.starts_with("+") {
        if arg.len() < 2 {
            return Err(format!("Please provide a value to increase the volume by"));
        }

        let arg: String = arg.chars().skip(1).collect();

        return Ok(VolumeOperation::Increase(parse_num(&arg)?));
    }

    if arg.starts_with("-") {
        if arg.len() < 2 {
            return Err(format!("Please provide a value to decrease the volume by"));
        }

        let arg: String = arg.chars().skip(1).collect();

        return Ok(VolumeOperation::Decrease(parse_num(&arg)?));
    }

    return Ok(VolumeOperation::Set(parse_num(arg)?));
}

/// A custon parser for shuffle arguments
fn shuffle_parser(arg: &str) -> Result<ShuffleOperation, String> {
    match arg.to_lowercase().as_str() {
        "on" => Ok(ShuffleOperation::On),
        "off" => Ok(ShuffleOperation::Off),
        _ => Err(format!("Not a valid shuffle mode")),
    }
}

/// A custon parser for repeat arguments
fn repeat_parser(arg: &str) -> Result<RepeatOperation, String> {
    match arg.to_lowercase().as_str() {
        "on" => Ok(RepeatOperation::On),
        "off" => Ok(RepeatOperation::Off),
        "track" => Ok(RepeatOperation::Track),
        _ => Err(format!("Not a valid repeat mode")),
    }
}

/// Get the SearchType from argument matches
fn type_matches(matches: &ArgMatches) -> Option<SearchType> {
    if matches.get_flag("track") {
        Some(SearchType::Track)
    } else if matches.get_flag("playlist") {
        Some(SearchType::Playlist)
    } else if matches.get_flag("album") {
        Some(SearchType::Album)
    } else if matches.get_flag("artist") {
        Some(SearchType::Artist)
    } else if matches.get_flag("show") {
        Some(SearchType::Show)
    } else if matches.get_flag("episode") {
        Some(SearchType::Episode)
    } else {
        None
    }
}
