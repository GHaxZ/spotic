use std::{
    fmt::{Display, Formatter},
    future::Future,
    pin::Pin,
};

use anyhow::{Context, Result};
use rspotify::{
    model::{
        Device, FullArtist, FullTrack, PlayContextId, PlayableId, SimplifiedAlbum,
        SimplifiedEpisode, SimplifiedPlaylist, SimplifiedShow,
    },
    prelude::OAuthClient,
    AuthCodePkceSpotify,
};

#[derive(Debug)]
pub struct Track {
    pub title: String,
    pub by: Vec<String>,
}

pub struct DisplayableDevice {
    pub device: Device,
}

impl Display for DisplayableDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.device.name)
    }
}

pub trait Playable {
    fn to_display(&self) -> String;

    fn type_string(&self) -> String;

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

impl Display for dyn Playable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.to_display(), self.type_string())
    }
}

impl Playable for FullTrack {
    fn to_display(&self) -> String {
        format!(
            "\"{}\" by {}",
            self.name,
            self.artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    fn type_string(&self) -> String {
        "Track".to_string()
    }

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = self
                .clone()
                .id
                .context("This song can't be played, since it lacks an ID. May be a local song.")?;
            client
                .start_uris_playback(vec![PlayableId::from(id)], None, None, None)
                .await
                .context("Failed to play track")?;
            Ok(())
        })
    }
}

// Implement Playable for SimplifiedPlaylist
impl Playable for SimplifiedPlaylist {
    fn to_display(&self) -> String {
        format!("{}", self.name.clone())
    }

    fn type_string(&self) -> String {
        "Playlist".to_string()
    }

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = self.clone().id;
            client
                .start_context_playback(PlayContextId::Playlist(id), None, None, None)
                .await
                .context("Failed to play playlist")?;
            Ok(())
        })
    }
}

// Implement Playable for other types (albums, artists, etc.)
impl Playable for SimplifiedAlbum {
    fn to_display(&self) -> String {
        format!(
            "\"{}\" by {}",
            self.name,
            self.artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    fn type_string(&self) -> String {
        "Album".to_string()
    }

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = self
                .clone()
                .id
                .context("This album can't be played, since it lacks an ID")?;
            client
                .start_context_playback(PlayContextId::Album(id), None, None, None)
                .await
                .context("Failed to play album")?;
            Ok(())
        })
    }
}

// Implement Playable for FullArtist
impl Playable for FullArtist {
    fn to_display(&self) -> String {
        format!("{}", self.name)
    }

    fn type_string(&self) -> String {
        "Artist".to_string()
    }

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = self.clone().id;
            client
                .start_context_playback(PlayContextId::Artist(id), None, None, None)
                .await
                .context("Failed to play artist")?;
            Ok(())
        })
    }
}

impl Playable for SimplifiedShow {
    fn to_display(&self) -> String {
        format!("{}", self.name)
    }

    fn type_string(&self) -> String {
        "Show".to_string()
    }

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = self.clone().id;
            client
                .start_context_playback(PlayContextId::Show(id), None, None, None)
                .await
                .context("Failed to play show")?;
            Ok(())
        })
    }
}

impl Playable for SimplifiedEpisode {
    fn to_display(&self) -> String {
        format!("{}", self.name)
    }

    fn type_string(&self) -> String {
        "Episode".to_string()
    }

    fn play<'a>(
        &'a self,
        client: &'a AuthCodePkceSpotify,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = self.clone().id;
            client
                .start_uris_playback(vec![PlayableId::from(id)], None, None, None)
                .await
                .context("Failed to play episode")?;
            Ok(())
        })
    }
}
