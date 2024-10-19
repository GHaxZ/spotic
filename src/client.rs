use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rspotify::{
    model::{
        AdditionalType, CurrentPlaybackContext, Device, PlayableItem, RepeatState, SearchResult,
        SearchType,
    },
    prelude::{BaseClient, OAuthClient},
    AuthCodePkceSpotify,
};

use crate::{
    model::{Playable, Track},
    ui,
};

// Used to control the spotify player
pub struct SpotifyPlayer {
    client: AuthCodePkceSpotify,
    playback_device: Option<Device>,
}

impl SpotifyPlayer {
    pub fn new(client: AuthCodePkceSpotify) -> Self {
        Self {
            client,
            playback_device: None,
        }
    }

    pub async fn current_track(&mut self) -> Result<Option<Track>> {
        self.ensure_playback_device().await?;

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

    pub async fn playback_pause(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        let current_playback = self.playback_state().await?;

        if current_playback.is_playing {
            self.client
                .pause_playback(None)
                .await
                .context("Failed pausing playback")?;
        }

        Ok(())
    }

    pub async fn playback_resume(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        let current_playback = self.playback_state().await?;

        if !current_playback.is_playing {
            self.client
                .resume_playback(None, None)
                .await
                .context("Failed resuming playback")?;
        }

        Ok(())
    }

    pub async fn playback_toggle(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        let current_playback = self.playback_state().await?;

        if current_playback.is_playing {
            self.playback_pause().await?;
        } else {
            self.playback_resume().await?;
        }

        Ok(())
    }

    pub async fn volume_get(&mut self) -> Result<u8> {
        self.ensure_playback_device().await?;

        let current_playback = self.playback_state().await?;

        Ok(current_playback
            .device
            .volume_percent
            .context("No current volume")? as u8)
    }

    pub async fn volume_set(&mut self, volume: u8) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .volume(volume.clamp(0, 100), None)
            .await
            .context("Failed setting volume")?;

        Ok(())
    }

    pub async fn volume_up(&mut self, up: u8) -> Result<()> {
        self.ensure_playback_device().await?;

        let volume = self.volume_get().await?;

        self.volume_set(volume + up).await?;

        Ok(())
    }

    pub async fn volume_down(&mut self, down: u8) -> Result<()> {
        self.ensure_playback_device().await?;

        let volume = self.volume_get().await?;

        self.volume_set(volume - down).await?;

        Ok(())
    }

    pub async fn search(
        &mut self,
        query: String,
        search_type: SearchType,
        limit: Option<u32>,
    ) -> Result<Vec<Box<dyn Playable + 'static>>> {
        self.ensure_playback_device().await?;

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

    pub async fn play(&mut self, item: &Box<dyn Playable>) -> Result<()> {
        self.ensure_playback_device().await?;

        item.play(&self.client)
            .await
            .context("Failed playing item")?;

        Ok(())
    }

    pub async fn set_device(&mut self, device: Device) -> Result<()> {
        self.client
            .transfer_playback(
                device
                    .id
                    .clone()
                    .context("Playback device is missing ID")?
                    .as_str(),
                None,
            )
            .await
            .context("Failed setting playback device")?;

        //  FIXME: Because of the nature of async, the runtime executes the next task
        //  while the transfer_playback() function is waiting for a response from the
        //  spotify API, confirming the playback device to be updated. This causes code
        //  to be executed, before the playback device was set. To avoid this, we wait
        //  for a second to give the spotify API enough time to update the playback device.
        //  This is a horrible, temporary solution, but I couldn't figure out how to fix
        //  this properly. And stop myself from going insane, I will leave it like this
        //  for the time being.
        tokio::time::sleep(Duration::from_secs(1)).await;

        self.playback_device = Some(device);

        Ok(())
    }

    pub async fn select_device(&mut self, devices: Option<Vec<Device>>) -> Result<()> {
        let devices = match devices {
            Some(d) => d,
            None => self
                .client
                .device()
                .await
                .context("Failed getting available playback devices")?,
        };

        match devices.len() {
            0 => Err(anyhow!("No devices are available")
                .context("Please make sure you are running a Spotify client")),
            _ => self.set_device(ui::select_device(devices)?).await,
        }
    }

    pub async fn song_next(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .next_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    pub async fn song_prev(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .previous_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    pub async fn shuffle_on(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .shuffle(true, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    pub async fn shuffle_off(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .shuffle(false, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn shuffle_toggle(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        let current_playback = self.playback_state().await?;

        if current_playback.shuffle_state {
            self.shuffle_off().await?;
        } else {
            self.shuffle_on().await?;
        }

        Ok(())
    }

    pub async fn repeat_on(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .repeat(RepeatState::Context, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    pub async fn repeat_off(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .repeat(RepeatState::Off, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn repeat_track(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        self.client
            .repeat(RepeatState::Track, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn repeat_toggle(&mut self) -> Result<()> {
        self.ensure_playback_device().await?;

        let current_playback = self.playback_state().await?;

        match current_playback.repeat_state {
            RepeatState::Off => self.repeat_on().await?,
            RepeatState::Track => self.shuffle_off().await?,
            RepeatState::Context => self.repeat_off().await?,
        }

        Ok(())
    }

    async fn playback_state(&mut self) -> Result<CurrentPlaybackContext> {
        let current_playback = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed determining current playback state")?
            .context("No current playback device")?;

        Ok(current_playback)
    }

    async fn ensure_playback_device(&mut self) -> Result<()> {
        let playback_context = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed determining current playback state")?;

        if let Some(playback) = playback_context {
            return self.set_device(playback.device).await;
        }

        let devices = self
            .client
            .device()
            .await
            .context("Failed getting available playback devices")?;

        if devices.len() == 1 {
            if let Some(device) = devices.get(0) {
                self.set_device(device.clone()).await?;
            }
        } else {
            self.select_device(Some(devices)).await?;
        }

        Ok(())
    }
}
