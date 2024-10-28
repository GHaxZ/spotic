use std::time::{Duration, Instant};

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

const DEVICE_CACHE_VALIDITY: Duration = Duration::from_secs(3);

// Struct for caching the current playback device
struct CachedDevice {
    _device: Device, // We currently don't need the device, but no reason to not save it
    last_checked: Instant,
    valid_for: Duration,
}

impl CachedDevice {
    fn new(device: Device) -> Self {
        Self {
            _device: device,
            last_checked: Instant::now(),
            valid_for: DEVICE_CACHE_VALIDITY,
        }
    }

    fn is_valid(&self) -> bool {
        self.last_checked.elapsed() < self.valid_for
    }
}

/// Used to control the spotify player
pub struct SpotifyPlayer {
    client: AuthCodePkceSpotify,
    cached_device: Option<CachedDevice>,
}

impl SpotifyPlayer {
    /// Create a new SpotifyPlayer instance
    pub fn new(client: AuthCodePkceSpotify) -> Self {
        Self {
            client,
            cached_device: None,
        }
    }

    /// Get the currently playing track
    pub async fn current_track(&mut self) -> Result<Option<Track>> {
        self.ensure_device().await?;

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

    /// Pause the playback
    pub async fn playback_pause(&mut self) -> Result<()> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        if current_playback.is_playing {
            self.client
                .pause_playback(None)
                .await
                .context("Failed pausing playback")?;
        }

        Ok(())
    }

    /// Resume the playback
    pub async fn playback_resume(&mut self) -> Result<()> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        if !current_playback.is_playing {
            self.client
                .resume_playback(None, None)
                .await
                .context("Failed resuming playback")?;
        }

        Ok(())
    }

    /// Toggle between resume/pause playback state
    pub async fn playback_toggle(&mut self) -> Result<()> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        match current_playback.is_playing {
            true => self.client.pause_playback(None).await,
            false => self.client.resume_playback(None, None).await,
        }
        .context("Failed toggling playback")?;

        Ok(())
    }

    /// Get the current volume in percent
    pub async fn volume_get(&mut self) -> Result<u8> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        Ok(current_playback
            .device
            .volume_percent
            .context("No current volume")? as u8)
    }

    /// Set the current volume in percent
    pub async fn volume_set(&mut self, volume: u8) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .volume(volume.clamp(0, 100), None)
            .await
            .context("Failed setting volume")?;

        Ok(())
    }

    /// Increase volume by given percentage
    pub async fn volume_up(&mut self, up: u8) -> Result<()> {
        self.ensure_device().await?;

        let volume = self.volume_get().await?;

        self.volume_set(volume + up.min(100 - volume)).await?;

        Ok(())
    }

    /// Decrease volume by given percentage
    pub async fn volume_down(&mut self, down: u8) -> Result<()> {
        self.ensure_device().await?;

        let volume = self.volume_get().await?;

        self.volume_set(volume - down.min(volume)).await?;

        Ok(())
    }

    /// Search for content by using a search query and specifying the search type
    pub async fn search(
        &mut self,
        query: String,
        search_type: SearchType,
        limit: Option<u32>,
    ) -> Result<Vec<Box<dyn Playable + 'static>>> {
        self.ensure_device().await?;

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

    /// Play a Playable item using the client
    pub async fn play(&mut self, item: &Box<dyn Playable>) -> Result<()> {
        self.ensure_device().await?;

        item.play(&self.client)
            .await
            .context("Failed playing item")?;

        Ok(())
    }

    /// Get all playlists in users library
    pub async fn playlists(&mut self) -> Result<Vec<Box<dyn Playable + 'static>>> {
        let playlists = self
            .client
            .current_user_playlists_manual(None, None)
            .await
            .context("Failed getting users playlists")?
            .items;

        let playables = playlists
            .into_iter()
            .map(|item| Box::new(item) as Box<dyn Playable>)
            .collect();

        Ok(playables)
    }

    /// Set the current playback device
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

        // Unfortunately the spotify API does not tell us,
        // when the device has finished updating. That means, we have to poll
        // for device changes

        const MAX_WAIT_TIME: Duration = Duration::from_secs(1);
        const POLL_INTERVAL: Duration = Duration::from_millis(100);
        let start_time = Instant::now();

        loop {
            if start_time.elapsed() < MAX_WAIT_TIME {
                if let Ok(Some(current_playback)) = self
                    .client
                    .current_playback(None, None::<Option<&AdditionalType>>)
                    .await
                {
                    if current_playback.device.name == device.name {
                        self.cached_device = Some(CachedDevice::new(device));
                        break;
                    }
                }

                tokio::time::sleep(POLL_INTERVAL).await;
            } else {
                return Err(anyhow!("Timed out while setting a playback device"));
            }
        }

        Ok(())
    }

    /// Get all available playback devices
    pub async fn devices(&self) -> Result<Vec<Device>> {
        let devices = self
            .client
            .device()
            .await
            .context("Failed getting available playback devices")?;

        Ok(devices)
    }

    /// Skip the current track
    pub async fn track_next(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .next_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    /// Go to previous track
    pub async fn track_prev(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .previous_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    /// Set shuffle mode to on
    pub async fn shuffle_on(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .shuffle(true, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    /// Set shuffle mode to off
    pub async fn shuffle_off(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .shuffle(false, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    /// Toggle between on/off shuffle state
    pub async fn shuffle_toggle(&mut self) -> Result<()> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        if current_playback.shuffle_state {
            self.shuffle_off().await?;
        } else {
            self.shuffle_on().await?;
        }

        Ok(())
    }

    /// Set repeat mode to on
    pub async fn repeat_on(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .repeat(RepeatState::Context, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    /// Set repeat mode to off
    pub async fn repeat_off(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .repeat(RepeatState::Off, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    /// Set repeat mode to track
    pub async fn repeat_track(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .repeat(RepeatState::Track, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    /// Toggle between on/off repeat state
    pub async fn repeat_toggle(&mut self) -> Result<()> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        match current_playback.repeat_state {
            RepeatState::Off => self.repeat_on().await?,
            RepeatState::Track => self.shuffle_off().await?,
            RepeatState::Context => self.repeat_off().await?,
        }

        Ok(())
    }

    /// Get the current playback context
    async fn playback_context(&mut self) -> Result<CurrentPlaybackContext> {
        let current_playback = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed determining current playback state")?
            .context("No current playback device")?;

        Ok(current_playback)
    }

    /// Ensure that there is an active playback device
    async fn ensure_device(&mut self) -> Result<()> {
        if let Some(cached) = &self.cached_device {
            if cached.is_valid() {
                return Ok(());
            }
        }

        let playback_context = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed determining current playback state")?;

        if let Some(current_playback) = playback_context {
            self.cached_device = Some(CachedDevice::new(current_playback.device));
            return Ok(());
        }

        let devices = self.devices().await?;

        let device = match devices.len() {
            1 => devices.into_iter().next().unwrap(),
            _ => ui::select_device(devices)?,
        };

        self.set_device(device).await?;

        Ok(())
    }
}
