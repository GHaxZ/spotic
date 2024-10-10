use anyhow::{anyhow, Context, Result};
use inquire::{Password, PasswordDisplayMode, Text};
use rspotify::{scopes, Credentials};
use std::{collections::HashSet, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Tokens {
    refresh_token: String,
    auth_token: String,
    expires: u64,
}

impl Tokens {
    // Load saved credentials
    pub fn load() -> Result<Self> {
        let path = Self::storage_path();

        if Self::saved() {
            let cred_str =
                fs::read_to_string(&path).context("Failed reading stored auth tokens")?;

            let cred = serde_json::from_str::<Tokens>(&cred_str)
                .context("Failed deserializing auth tokens")?;

            Ok(cred)
        } else {
            Err(anyhow!("No stored auth tokens were found"))
        }
    }

    // Save this credential set to file
    pub fn save(&self) -> Result<()> {
        let path = Self::storage_path();

        let creds = serde_json::to_string(self).context("Failed serializing auth tokens")?;

        Self::ensure_dir()?;

        fs::write(&path, creds).context("Failed writing auth tokens to file")?;

        Ok(())
    }

    // Ensure the data directory is created
    pub fn ensure_dir() -> Result<()> {
        fs::create_dir_all(Self::storage_path()).context("Failed creating data directory")
    }

    // Do saved credentials exist
    pub fn saved() -> bool {
        Self::storage_path().exists()
    }

    // Get the credentials storage path
    pub fn storage_path() -> PathBuf {
        // Either store in conventional place or next to binary
        let mut data_dir = dirs::data_dir().unwrap_or(PathBuf::from("./"));

        data_dir.push("spotic");
        data_dir.push("credentials.json");

        data_dir
    }
}

// Provides auth tokens, refreshing if necessary
pub struct TokenProvider {}

impl TokenProvider {
    // Get a new token provider
    pub fn new(credentials: Tokens) -> Self {
        Self {}
    }

    // Get a token, refresh if necessary
    pub fn token() -> String {
        "".to_string()
    }
}

pub struct AuthFlow {}

impl AuthFlow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn scopes() -> HashSet<String> {
        scopes!("user-read-currently-playing")
    }

    pub fn run(&self) -> Result<()> {
        let creds = self.collect_creds()?;

        Ok(())
    }

    pub fn collect_creds(&self) -> Result<rspotify::Credentials> {
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
