use anyhow::Result;
use auth::AuthFlow;

mod auth;
mod client;
mod model;

//  TODO:
//  Check if auth tokens are cached, don't run auth flow
//  Make the code gathering easier
//      - Run a small web server which reads the code from the callback and displays a success
//      messagee
//      - Or simply make the url input prettier

#[tokio::main]
async fn main() -> Result<()> {
    AuthFlow::run().await
}
