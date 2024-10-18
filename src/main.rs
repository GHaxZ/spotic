use anyhow::Result;

mod args;
mod auth;
mod client;
mod model;
mod ui;

//  TODO:
//  Make it possible to select playback device, or automatically choose one when only one is
//  currently available

#[tokio::main]
async fn main() -> Result<()> {
    args::parse().await?;

    Ok(())
}
