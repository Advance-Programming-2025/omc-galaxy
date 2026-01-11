use common_game::components::planet::Planet;

use crate::utils::{ExplorerStatus, GalaxyTopology, PlanetStatus};

#[derive(Debug)]
pub enum GameToUi{
    GameStatusUpdate{
        galaxy_topology: GalaxyTopology,    
        planets_status: PlanetStatus,
        explorer_status: ExplorerStatus,
    }
}

#[derive(Debug)]
pub enum UiToGame{
    StartGame,
    StopGame,
    ResetGame,
    EndGame,
}
