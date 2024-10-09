use crate::model::Track;

// Used to control the spotify player
pub struct SpotifyPlayer {}

impl SpotifyPlayer {
    pub fn current_track() -> Option<Track> {
        None
    }

    pub fn play_track(track: Track) {}

    pub fn set_volume(volume: u8) {}

    pub fn volume_up(up: u8) {}

    pub fn volume_down(down: u8) {}
}
