use anyhow::{Context, Result};
use inquire::{Password, PasswordDisplayMode, Text};
use rspotify::{prelude::OAuthClient, scopes, AuthCodePkceSpotify, Config, Credentials, OAuth};
use std::{collections::HashSet, fs, path::PathBuf};

use crate::client::SpotifyPlayer;

const CALLBACK_URI: &'static str = "http://localhost/callback";

pub struct Auth {}

impl Auth {
    /// Get the directory where data should be stored
    pub fn data_dir() -> PathBuf {
        // Store in conventional place or next to binary
        let mut data_dir = dirs::data_dir().unwrap_or(PathBuf::from("./"));
        data_dir.push("spotic");
        data_dir
    }

    /// Get the credentials storage path
    pub fn credentials_path() -> PathBuf {
        let mut credentials_path = Self::data_dir();
        credentials_path.push("credentials.json");
        credentials_path
    }

    /// Ensure the data directory is created
    pub fn ensure_dir() -> Result<()> {
        fs::create_dir_all(Self::data_dir()).context("Failed creating data directory")
    }

    /// Do saved credentials exist
    pub fn saved() -> bool {
        Self::credentials_path().exists()
    }

    /// Get the scopes required for all functionality
    ///
    /// In case these get updated and are not granted by the current authorization, the user will
    /// be asked to re-authorize
    fn scopes() -> HashSet<String> {
        scopes!("user-read-currently-playing")
    }

    /// Get the oauth settings used across the authorization code
    fn oauth() -> OAuth {
        OAuth {
            redirect_uri: CALLBACK_URI.to_string(),
            scopes: Self::scopes(),
            ..Default::default()
        }
    }

    /// Get the config used across the authorization code
    fn config() -> Config {
        Config {
            token_cached: true,
            token_refreshing: true,
            cache_path: Self::credentials_path(),
            ..Default::default()
        }
    }

    /// Try to load authorization tokens from cache
    ///
    /// Returns Ok(None) in case the scope does not match with the clients or we don't have any tokens
    /// cached
    /// - Or token caching is disabled (it is not)
    /// - Or token is expired (we still load it, so we can refresh)
    /// So basically, every time we need to re-authorize we return Ok(None)
    ///
    /// Returns an Err() in case tokens are cached, but can't be loaded
    pub async fn load_cached() -> Result<Option<SpotifyPlayer>> {
        if !Self::saved() {
            return Ok(None);
        }

        let spotify =
            AuthCodePkceSpotify::with_config(Credentials::default(), Self::oauth(), Self::config());

        match spotify.read_token_cache(true).await {
            Ok(Some(_)) => Ok(Some(SpotifyPlayer::new(spotify))),
            Ok(None) => Ok(None),
            Err(e) => Err(e).context("Failed reading cached tokens, try re-authorizing"),
        }
    }

    /// Run an authorization flow
    ///
    /// 1. Ask the user for credentials
    /// 2. Either load the tokens from cache or, if for any reason they can't be used, prompt the
    ///    user again to get new tokens
    ///
    ///  TODO: Don't load tokens from cache, because this will only be run when either no cached tokens
    ///  are usable or the user specifically requests it
    pub async fn run_flow() -> Result<()> {
        let creds = Self::collect_creds()?;

        Self::authorize_spotify(creds, Self::oauth()).await?;

        Ok(())
    }

    /// Run the authorization process for spotify
    ///
    /// 1. Get the authorization URL
    /// 2. If tokens can't be used, prompt the user
    /// 3. Return a usable SpotifyPlayer if everything went well
    async fn authorize_spotify(creds: Credentials, oauth: OAuth) -> Result<SpotifyPlayer> {
        Self::ensure_dir()?;

        let mut spotify = AuthCodePkceSpotify::with_config(creds, oauth, Self::config());

        let url = spotify
            .get_authorize_url(None)
            .context("Failed getting auth URL")?;

        spotify
            .prompt_for_token(&url)
            .await
            .context("Failed getting auth tokens")?;

        Ok(SpotifyPlayer::new(spotify))
    }

    /// Collect client id and client secrets
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
