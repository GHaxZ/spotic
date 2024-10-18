use anyhow::{Context, Result};
use inquire::{Password, PasswordDisplayMode, Select, Text};
use rspotify::Credentials;

use crate::model::Playable;

/// Select a playable item from a list and return it
pub fn select_playable(content_list: Vec<Box<dyn Playable>>) -> Result<Box<dyn Playable>> {
    Select::new("Select an item to play", content_list)
        .prompt()
        .context("Failed displaying content selection")
}

/// Collect client id and client secrets
pub fn collect_creds(callback_uri: &'static str) -> Result<Credentials> {
    println!(
"To authorize this tool you need to provide client credentials.

Don't worry, this is easy to do and only has to be done once.

To get these credentials go to the Spotify Developer Dashboard: https://developer.spotify.com/dashboard

1. Create a new app and give it any name and description.
2. Make sure to add the \"{}\" Redirect URI.
3. Then select the \"Web API\" option.
4. Accept the Terms of Service and finally click \"Save\".
5. Now click on the newly created app and go to the settings.
6. Here you will find the client id and the client secret.
", callback_uri
    );

    let client_id = Text::new("Enter the client id")
        .prompt()
        .context("Failed reading client id input")?;
    let client_secret = Password::new("Enter the client secret")
        .with_display_toggle_enabled()
        .without_confirmation()
        .with_display_mode(PasswordDisplayMode::Masked)
        .prompt()
        .context("Failed reading client secret input")?;

    Ok(Credentials::new(&client_id, &client_secret))
}

/// Collect the redirect url
pub fn collect_redirect_url(url: &String) -> Result<String> {
    println!("\nAuthorization link: {}\n", url);

    if open::that(url).is_ok() {
        println!("Failed opening the link in a browser, please open it manually.\n");
    }

    // Get the code from the link
    let url_input = Text::new("Please paste the url that was opened in your browser")
        .prompt()
        .context("Failed reading code input")?;

    Ok(url_input)
}
