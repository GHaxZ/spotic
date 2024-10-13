use anyhow::Result;
use auth::Auth;
use rspotify::model::SearchType;

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
    let player = match Auth::load_cached().await? {
        Some(player) => player,
        None => Auth::run_flow().await?,
    };

    let res = player
        .search(format!("Never gonna give you up"), SearchType::Track, None)
        .await?;

    println!(
        "{}",
        res.iter()
            .map(|p| p.to_display())
            .collect::<Vec<String>>()
            .join("\n")
    );

    let first = res.get(0).unwrap();

    println!("Playing: {} [{}]", first.to_display(), first.type_string());

    player.play(first).await?;

    let cur = player.current_track().await?;

    match cur {
        Some(t) => println!("\"{}\" by {}", t.title, t.by.join(", ")),
        None => println!("Nothing playing"),
    }

    Ok(())
}
