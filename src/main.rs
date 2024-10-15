use anyhow::Result;

mod args;
mod auth;
mod client;
mod model;
mod ui;

//  TODO:
//  Make the code gathering easier
//      - Run a small web server which reads the code from the callback and displays a success
//      message
//      - Or simply make the url input prettier
//  Make it possible to select playback device, or automatically choose one when only one is
//  currently available

#[tokio::main]
async fn main() -> Result<()> {
    args::parse().await?;

    Ok(())
}
