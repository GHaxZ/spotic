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

// Used to control the spotify player
pub struct SpotifyPlayer {
    client: AuthCodePkceSpotify,
    cached_device: Option<CachedDevice>,
}

impl SpotifyPlayer {
    pub fn new(client: AuthCodePkceSpotify) -> Self {
        Self {
            client,
            cached_device: None,
        }
    }

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

    pub async fn volume_get(&mut self) -> Result<u8> {
        self.ensure_device().await?;

        let current_playback = self.playback_context().await?;

        Ok(current_playback
            .device
            .volume_percent
            .context("No current volume")? as u8)
    }

    pub async fn volume_set(&mut self, volume: u8) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .volume(volume.clamp(0, 100), None)
            .await
            .context("Failed setting volume")?;

        Ok(())
    }

    pub async fn volume_up(&mut self, up: u8) -> Result<()> {
        self.ensure_device().await?;

        let volume = self.volume_get().await?;

        self.volume_set(volume + up).await?;

        Ok(())
    }

    pub async fn volume_down(&mut self, down: u8) -> Result<()> {
        self.ensure_device().await?;

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

    pub async fn play(&mut self, item: &Box<dyn Playable>) -> Result<()> {
        self.ensure_device().await?;

        item.play(&self.client)
            .await
            .context("Failed playing item")?;

        Ok(())
    }

    // Get all user playlists
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
        // for device changes, great, I know

        const MAX_WAIT_TIME: Duration = Duration::from_secs(1);
        const POLL_INTERVAL: Duration = Duration::from_millis(100);
        let start_time = Instant::now();

        // Poll until the device is confirmed or we exceed the timeout
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
        self.ensure_device().await?;

        self.client
            .next_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    pub async fn song_prev(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .previous_track(None)
            .await
            .context("Failed skipping track")?;

        Ok(())
    }

    pub async fn shuffle_on(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .shuffle(true, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    pub async fn shuffle_off(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .shuffle(false, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

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

    pub async fn repeat_on(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .repeat(RepeatState::Context, None)
            .await
            .context("Failed turning shuffle on")?;

        Ok(())
    }

    pub async fn repeat_off(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .repeat(RepeatState::Off, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

    pub async fn repeat_track(&mut self) -> Result<()> {
        self.ensure_device().await?;

        self.client
            .repeat(RepeatState::Track, None)
            .await
            .context("Failed turning shuffle off")?;

        Ok(())
    }

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

    async fn playback_context(&mut self) -> Result<CurrentPlaybackContext> {
        let current_playback = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed determining current playback state")?
            .context("No current playback device")?;

        Ok(current_playback)
    }

    async fn ensure_device(&mut self) -> Result<()> {
        // Do we have a cache
        if let Some(cached) = &self.cached_device {
            // Is the cache valid? If yes we return.
            if cached.is_valid() {
                return Ok(());
            }
        }

        // Get the new context
        let playback_context = self
            .client
            .current_playback(None, None::<Option<&AdditionalType>>)
            .await
            .context("Failed determining current playback state")?;

        // If we have a context, set it and return
        if let Some(current_playback) = playback_context {
            self.cached_device = Some(CachedDevice::new(current_playback.device));
            return Ok(());
        }

        // Otherwise select a playback device
        let devices = self
            .client
            .device()
            .await
            .context("Failed getting available playback devices")?;

        // Select a device
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
