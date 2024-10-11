use anyhow::{Context, Result};
use inquire::{Password, PasswordDisplayMode, Text};
use rspotify::{prelude::OAuthClient, scopes, AuthCodePkceSpotify, Config, Credentials, OAuth};
use std::collections::HashSet;

use crate::client::SpotifyPlayer;

const CALLBACK_URI: &'static str = "http://localhost/callback";
pub struct AuthFlow {}

impl AuthFlow {
    pub fn scopes() -> HashSet<String> {
        scopes!("user-read-currently-playing")
    }

    pub async fn run() -> Result<()> {
        let creds = Self::collect_creds()?;

        let oauth = OAuth {
            redirect_uri: CALLBACK_URI.to_string(),
            scopes: Self::scopes(),
            ..Default::default()
        };

        Self::authorize_spotify(creds, oauth).await?;

        Ok(())
    }

    async fn authorize_spotify(creds: Credentials, oauth: OAuth) -> Result<SpotifyPlayer> {
        let config = Config {
            token_cached: true,
            token_refreshing: true,
            ..Default::default()
        };

        let mut spotify = AuthCodePkceSpotify::with_config(creds, oauth, config);

        let url = spotify
            .get_authorize_url(None)
            .context("Failed getting auth URL")?;

        spotify
            .prompt_for_token(&url)
            .await
            .context("Failed getting auth tokens")?;

        Ok(SpotifyPlayer::new(spotify))
    }

    fn collect_creds() -> Result<rspotify::Credentials> {
        println!(
"To authorize this tool you need to provide client credentials.

Don't worry, this is easy to do and only has to be done once.

To get these credentials go to the Spotify Developer Dashboard: https://developer.spotify.com/dashboard

1. Create a new app and give it any name and description.
2. Make sure to add the \"http://localhost/callback\" Redirect URI.
3. Then select the \"Web API\" option.
4. Accept the Terms of Service and finally click \"Save\".
5. Now click on the newly created app and go to the settings.
6. Here you will find the client id and the client secret.
"
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
}
