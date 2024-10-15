use anyhow::Result;

mod args;
mod auth;
mod client;
mod model;

//  TODO:
//  Make the code gathering easier
//      - Run a small web server which reads the code from the callback and displays a success
//      message
//      - Or simply make the url input prettier
//  Implement the search display functionality

#[tokio::main]
async fn main() -> Result<()> {
    args::parse().await?;

    Ok(())
}
