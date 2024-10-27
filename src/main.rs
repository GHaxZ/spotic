use anyhow::Result;

mod args;
mod auth;
mod client;
mod model;
mod ui;

//  TODO:
//  Add configuration support for current song formatting, silent mode etc.
//  Output current song ASCII cover art
//  Make "device" command work the same way as the "playlist" command

#[tokio::main]
async fn main() -> Result<()> {
    args::parse().await?;

    Ok(())
}
