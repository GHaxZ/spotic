use anyhow::Result;

mod args;
mod auth;
mod client;
mod model;
mod ui;

//  TODO:
//  Add configuration support for current song formatting etc.
//  Add feature for playing users playlists (with selection or search)
//  Output current song ascii cover art

#[tokio::main]
async fn main() -> Result<()> {
    args::parse().await?;

    Ok(())
}
