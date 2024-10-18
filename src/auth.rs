use anyhow::{Context, Result};
use rspotify::{
    prelude::{BaseClient, OAuthClient},
    scopes, AuthCodePkceSpotify, Config, Credentials, OAuth,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf};

use crate::{client::SpotifyPlayer, ui};

const CALLBACK_URI: &'static str = "http://localhost/callback";

#[derive(Serialize, Deserialize)]
pub struct ClientCredentials {
    client_id: String,
}

/// Get the directory where data should be stored
pub fn data_dir() -> PathBuf {
    // Store in conventional place or next to binary
    let mut data_dir = dirs::data_dir().unwrap_or(PathBuf::from("./"));
    data_dir.push("spotic");
    data_dir
}

/// Get the tokens storage path
pub fn tokens_path() -> PathBuf {
    let mut credentials_path = data_dir();
    credentials_path.push("tokens.json");
    credentials_path
}

/// Get the client credentials storage path
pub fn credentials_path() -> PathBuf {
    let mut client_path = data_dir();
    client_path.push("credentials.json");
    client_path
}

/// Ensure the data directory is created
pub fn ensure_dir() -> Result<()> {
    fs::create_dir_all(data_dir()).context("Failed creating data directory")
}

/// Do saved tokens and credentials exist
pub fn saved() -> bool {
    tokens_path().exists() && credentials_path().exists()
}

/// Get the scopes required for all functionality
///
/// In case these get updated and are not granted by the current authorization, the user will
/// be asked to re-authorize
fn scopes() -> HashSet<String> {
    scopes!(
        "user-read-currently-playing",
        "user-modify-playback-state",
        "user-read-playback-state"
    )
}

/// Get the oauth settings used across the authorization code
fn oauth() -> OAuth {
    OAuth {
        redirect_uri: CALLBACK_URI.to_string(),
        scopes: scopes(),
        ..Default::default()
    }
}

/// Get the config used across the authorization code
fn config() -> Config {
    Config {
        token_cached: true,
        token_refreshing: true,
        cache_path: tokens_path(),
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
    if !saved() {
        return Ok(None);
    }

    let creds_str = fs::read_to_string(credentials_path())
        .context("Failed reading stored client credentials, try re-authorizing")?;

    let creds = serde_json::from_str::<ClientCredentials>(&creds_str)
        .context("Failed deserializing stored client credentials, try re-authorizing")?;

    let spotify = AuthCodePkceSpotify::with_config(
        Credentials::new_pkce(&creds.client_id),
        oauth(),
        config(),
    );

    match spotify.read_token_cache(true).await {
        Ok(Some(token)) => {
            *spotify.token.lock().await.unwrap() = Some(token.clone());

            if token.is_expired() {
                spotify
                    .refresh_token()
                    .await
                    .context("Failed to refresh token")?;
            }

            Ok(Some(SpotifyPlayer::new(spotify)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e).context("Failed reading cached tokens, try re-authorizing"),
    }
}

/// Run an authorization flow
///
/// - Ask the user for credentials
/// - Generate the authorization url and open it
/// - Ask the user for the redirect url, get the code from it
/// - Write the tokens to the cache file
pub async fn run_flow() -> Result<SpotifyPlayer> {
    let creds = ui::collect_creds(CALLBACK_URI).context("Failed collecting credentials")?;

    authorize_spotify(creds, oauth()).await
}

/// Run the authorization process for spotify
///
/// - Get the authorization URL
/// - If tokens can't be used, prompt the user
/// - Return a usable SpotifyPlayer if everything went well
async fn authorize_spotify(creds: Credentials, oauth: OAuth) -> Result<SpotifyPlayer> {
    ensure_dir()?;

    let mut spotify = AuthCodePkceSpotify::with_config(creds.clone(), oauth, config());

    // Serialize the client credentials
    let creds_str = serde_json::to_string(&ClientCredentials {
        client_id: creds.id,
    })
    .context("Failed serializing client credentials")?;

    // Save the client credentials
    fs::write(credentials_path(), creds_str).context("Failed saving client credentials")?;

    // Get the authorization url
    let url = spotify
        .get_authorize_url(None)
        .context("Failed getting auth URL")?;

    let url_input = ui::collect_redirect_url(&url).context("Failed collecting the redirect url")?;

    // Parse the code from the url input
    let code = spotify
        .parse_response_code(&url_input)
        .context("Failed parsing response code from url")?;

    // Request the tokens using the code
    spotify
        .request_token(&code)
        .await
        .context("Failed requesting token")?;

    // Write the token to cache file
    spotify
        .write_token_cache()
        .await
        .context("Failed caching the token")?;

    Ok(SpotifyPlayer::new(spotify))
}
