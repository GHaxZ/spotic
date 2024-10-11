use rspotify::AuthCodePkceSpotify;

use crate::model::Track;

// Used to control the spotify player
pub struct SpotifyPlayer {
    client: AuthCodePkceSpotify,
}

impl SpotifyPlayer {
    pub fn new(client: AuthCodePkceSpotify) -> Self {
        Self { client }
    }

    pub fn current_track() -> Option<Track> {
        None
    }

    pub fn play_track(track: Track) {}

    pub fn set_volume(volume: u8) {}

    pub fn volume_up(up: u8) {}

    pub fn volume_down(down: u8) {}
}
