use anyhow::{Context, Result};
use rspotify::{
    model::{
        AdditionalType, CurrentPlaybackContext, PlayableItem, RepeatState, SearchResult, SearchType,
    },
    prelude::{BaseClient, OAuthClient},
    AuthCodePkceSpotify,
};

use crate::model::{Playable, Track};

// Used to control the spotify player
pub struct SpotifyPlayer {
    client: AuthCodePkceSpotify,
}

impl SpotifyPlayer {
    pub fn new(client: AuthCodePkceSpotify) -> Self {
        Self { client }
    }

    pub async fn current_track(&self) -> Result<Option<Track>> {
        let currently_playing = self
            .client
            .current_playing(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed getting the current track")?
            .context("Current track is unknown")?;

        if !currently_playing.is_playing {
            return Ok(None);
        }

        return match currently_playing.item {
            Some(PlayableItem::Track(track)) => Ok(Some(Track {
                title: track.name,
                by: track.artists.iter().map(|a| a.name.clone()).collect(),
            })),
            Some(PlayableItem::Episode(episode)) => Ok(Some(Track {
                title: episode.name,
                by: vec![episode.show.name],
            })),
            _ => Ok(None),
        };
    }

    pub async fn playback_pause(&self) -> Result<()> {
        let current_playback = self.playback_state().await?;

        if current_playback.is_playing {
            self.client
                .pause_playback(None)
                .await
                .context("Failed pausing playback")?;
        }

        Ok(())
    }

    pub async fn playback_resume(&self) -> Result<()> {
        let current_playback = self.playback_state().await?;

        if !current_playback.is_playing {
            self.client
                .resume_playback(None, None)
                .await
                .context("Failed resuming playback")?;
        }

        Ok(())
    }

    pub async fn playback_toggle(&self) -> Result<()> {
        let current_playback = self.playback_state().await?;

        if current_playback.is_playing {
            self.playback_pause().await?;
        } else {
            self.playback_resume().await?;
        }

        Ok(())
    }

    pub async fn volume_get(&self) -> Result<u8> {
        let current_playback = self.playback_state().await?;

        Ok(current_playback
            .device
            .volume_percent
            .context("No current volume")? as u8)
    }

    pub async fn volume_set(&self, volume: u8) -> Result<()> {
        self.client
            .volume(volume.clamp(0, 100), None)
            .await
            .context("Failed setting volume")?;

        Ok(())
    }

    pub async fn volume_up(&self, up: u8) -> Result<()> {
        let volume = self.volume_get().await?;

        self.volume_set(volume + up).await?;

        Ok(())
    }

    pub async fn volume_down(&self, down: u8) -> Result<()> {
        let volume = self.volume_get().await?;

        self.volume_set(volume - down).await?;

        Ok(())
    }

    pub async fn search(
        &self,
        query: String,
        search_type: SearchType,
        limit: Option<u32>,
    ) -> Result<Vec<Box<dyn Playable + 'static>>> {
        let search = self
            .client
            .search(&query, search_type, None, None, limit, None)
            .await
            .context("Failed searching content")?;

        let mut results: Vec<Box<dyn Playable>> = Vec::new();

        fn map_playable<T: Playable + 'static>(items: Vec<T>) -> Vec<Box<dyn Playable>> {
            items
                .into_iter()
                .map(|item| Box::new(item) as Box<dyn Playable>)
                .collect()
        }

        match search {
            SearchResult::Playlists(playlists) => results.extend(map_playable(playlists.items)),
            SearchResult::Albums(albums) => results.extend(map_playable(albums.items)),
            SearchResult::Artists(artists) => results.extend(map_playable(artists.items)),
            SearchResult::Tracks(tracks) => results.extend(map_playable(tracks.items)),
            SearchResult::Shows(shows) => results.extend(map_playable(shows.items)),
            SearchResult::Episodes(episodes) => results.extend(map_playable(episodes.items)),
        }

        Ok(results)
    }

    pub async fn play(&self, item: &Box<dyn Playable>) -> Result<()> {
        item.play(&self.client)
            .await
            .context("Failed playing item")?;

        Ok(())
    }

    pub async fn song_next(&self) -> Result<()> {
        self.client
            .next_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    pub async fn song_prev(&self) -> Result<()> {
        self.client
            .previous_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    pub async fn shuffle_on(&self) -> Result<()> {
        self.client
            .shuffle(true, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    pub async fn shuffle_off(&self) -> Result<()> {
        self.client
            .shuffle(false, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn shuffle_toggle(&self) -> Result<()> {
        let current_playback = self.playback_state().await?;

        if current_playback.shuffle_state {
            self.shuffle_off().await?;
        } else {
            self.shuffle_on().await?;
        }

        Ok(())
    }

    pub async fn repeat_on(&self) -> Result<()> {
        self.client
            .repeat(RepeatState::Context, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    pub async fn repeat_off(&self) -> Result<()> {
        self.client
            .repeat(RepeatState::Off, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn repeat_track(&self) -> Result<()> {
        self.client
            .repeat(RepeatState::Track, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn repeat_toggle(&self) -> Result<()> {
        let current_playback = self.playback_state().await?;

        match current_playback.repeat_state {
            RepeatState::Off => self.repeat_on().await?,
            RepeatState::Track => self.shuffle_off().await?,
            RepeatState::Context => self.repeat_off().await?,
        }

        Ok(())
    }

    async fn playback_state(&self) -> Result<CurrentPlaybackContext> {
        let current_playback = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed getting current playback state")?
            .context("No current playback")?;

        Ok(current_playback)
    }
}
