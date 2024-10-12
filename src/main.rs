use anyhow::Result;
use auth::Auth;

mod auth;
mod client;
mod model;

//  TODO:
//  Check if auth tokens are cached, don't run auth flow
//  Make the code gathering easier
//      - Run a small web server which reads the code from the callback and displays a success
//      message
//      - Or simply make the url input prettier

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(s) = Auth::load_cached().await? {
        println!("Loaded cached");
        return Ok(());
    }

    Auth::run_flow().await?;
    println!("Finished flow");

    Ok(())
}
