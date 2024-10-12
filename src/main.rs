use anyhow::Result;
use auth::Auth;

mod auth;
mod client;
mod model;

//  TODO:
//  Make the code gathering easier
//      - Run a small web server which reads the code from the callback and displays a success
//      message
//      - Or simply make the url input prettier

#[tokio::main]
async fn main() -> Result<()> {
    let mut player = match Auth::load_cached().await? {
        Some(player) => player,
        None => Auth::run_flow().await?,
    };

    match player.current_track().await? {
        Some(t) => println!("\"{}\" by {}", t.title, t.by.join(", ")),
        None => println!("Nothing playing"),
    };

    Ok(())
}
