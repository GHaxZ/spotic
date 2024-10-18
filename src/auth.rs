use anyhow::{Context, Result};
use const_format::concatcp;
use core::str;
use rspotify::{
    prelude::{BaseClient, OAuthClient},
    scopes, AuthCodePkceSpotify, Config, Credentials, OAuth,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use crate::{client::SpotifyPlayer, ui};

const CALLBACK_SERVER_PORT: u32 = 8080;
const CALLBACK_URI: &'static str =
    concatcp!("http://localhost:", CALLBACK_SERVER_PORT, "/callback");

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
/// - Collect the redirect url, get the code from it
/// - Write the tokens to the cache file
pub async fn run_flow() -> Result<SpotifyPlayer> {
    let creds = ui::collect_creds(CALLBACK_URI).context("Failed collecting credentials")?;

    authorize_spotify(creds, oauth()).await
}

/// Run the authorization process for spotify
///
/// - Write the credentials to file
/// - Get the authorization URL
/// - Collect the code from the callback URL
/// using either small web server or manual user input
/// - Use the code to request authorization tokens
/// - Write the tokens to file
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

    println!("\nAuthorization link: {}\n", url);

    // Try opening the URL using a browser
    if open::that(url).is_err() {
        println!("Failed opening the link in a browser, please open it manually.\n");
    }

    // Either get the callback URL using a locally running web server, or, in case of errors,
    // let the user enter the URL manually
    let url = match run_callback_server().await {
        Ok(url) => url,
        Err(_) => ui::collect_callback_url().context("Failed reading the callback URL")?,
    };

    // Parse the code from the callback URL
    let code = spotify
        .parse_response_code(&url)
        .context("Failed reading authorization code from url")?;

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

    println!("Successfully authorized!");

    Ok(SpotifyPlayer::new(spotify))
}

/// Runs a local server which is used as the callback for the spotify API
///
/// This allows us to do two things:
/// - Collect the response URL and thus the authorization code automatically
/// - Show the user a neat "You can close this page now" message after authorizing
/// the spotify app
async fn run_callback_server() -> Result<String> {
    // Listen on the callback port
    let listener = TcpListener::bind(format!("0.0.0.0:{}", CALLBACK_SERVER_PORT))
        .await
        .context("Failed running callback server")?;

    // Accept connection
    let (mut socket, _) = listener
        .accept()
        .await
        .context("Failed accepting connection")?;

    // Create buffer for reading from socket
    let mut buffer = vec![0; 1024];

    // Read from socket into buffer, store amount of read bytes
    let n = socket
        .read(&mut buffer)
        .await
        .context("Failed reading bytes from connection to callback server")?;

    // Create string from byte array until the n (the read amount)
    let request = str::from_utf8(&buffer[..n]).context("Callback URL is malformed")?;

    // Get the first line from the buffer, that being the URL
    let url = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .context("Failed to extract URL from the request")?;

    // Create response
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <body style='font-family: sans-serif; display: flex; align-items: center; height: 100vh;'>\
        <h1 style='margin: auto;'>You can close this page now.</h1>\
        </body>";

    // Write the response
    socket
        .write_all(response)
        .await
        .context("Failed sending response from callback server")?;

    // Reconstruct the full URL
    let full_url = format!("http://localhost:{}{}", CALLBACK_SERVER_PORT, url);

    // Finally return the URL
    Ok(full_url)
}
