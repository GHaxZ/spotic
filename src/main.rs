use auth::AuthFlow;

mod auth;
mod client;
mod model;

fn main() {
    AuthFlow::new().run();
}
