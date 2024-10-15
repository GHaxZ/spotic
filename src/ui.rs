use anyhow::{Context, Result};
use inquire::Select;

use crate::model::Playable;

pub fn select_playable(content_list: Vec<Box<dyn Playable>>) -> Result<Box<dyn Playable>> {
    Select::new("Select an item to play", content_list)
        .prompt()
        .context("Failed displaying content selection")
}
