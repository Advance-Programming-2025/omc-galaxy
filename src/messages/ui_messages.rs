use std::{collections::BTreeMap, sync::{Arc, RwLock}};

use crate::utils::Status;

#[derive(Debug)]
pub enum GameToUi{
    GameStatePointers{
        galaxy_topology: Arc<RwLock<Vec<Vec<bool>>>>,
        planets_status: BTreeMap<u32, Status>,
        explorer_status: BTreeMap<u32, Status>,
    }
}

#[derive(Debug)]
pub enum UiToGame{
    StartGame,
    StopGame,
    ResetGame,
    EndGame,
}
