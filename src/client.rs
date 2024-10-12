use anyhow::{Context, Result};
use rspotify::{
    model::{AdditionalType, PlayableItem},
    prelude::OAuthClient,
    AuthCodePkceSpotify,
};

use crate::model::Track;

// Used to control the spotify player
pub struct SpotifyPlayer {
    client: AuthCodePkceSpotify,
}

impl SpotifyPlayer {
    pub fn new(client: AuthCodePkceSpotify) -> Self {
        Self { client }
    }

    pub async fn current_track(&mut self) -> Result<Option<Track>> {
        let currently_playing = self
            .client
            .current_playing(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed getting the current track")?;

        if let Some(cp) = currently_playing {
            if !cp.is_playing {
                return Ok(None);
            }

            return match cp.item {
                Some(PlayableItem::Track(track)) => Ok(Some(Track {
                    id: track.id.map(|i| i.to_string()),
                    title: track.name,
                    by: track.artists.iter().map(|a| a.name.clone()).collect(),
                })),
                Some(PlayableItem::Episode(episode)) => Ok(Some(Track {
                    id: Some(episode.id.to_string()),
                    title: episode.name,
                    by: vec![episode.show.name],
                })),
                _ => Ok(None),
            };
        }

        // Return None if there is no currently playing item
        Ok(None)
    }

    pub fn play_track(&self, track: Track) {}

    pub fn set_volume(&self, volume: u8) {}

    pub fn volume_up(&self, up: u8) {}

    pub fn volume_down(&self, down: u8) {}
}
