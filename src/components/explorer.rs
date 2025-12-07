//use std::sync::mpsc;
use crossbeam_channel::{Sender, Receiver, unbounded};

use common_game::{components::planet::Planet, protocols::messages::{
    ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer,
    PlanetToExplorer,
}};

pub type BagType = u32;

pub struct Explorer {
    planet_id: Option<u32>, //I assume that the travel isn't instant so I put an Option we should manage the case the planet explodes
    orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<BagType>>,
    ),
    planet_channels: Option<(
        Receiver<PlanetToExplorer>,
        Sender<ExplorerToPlanet>,
    )>,
}

impl Explorer {
    //At creation, an Explorer should be connected to Orchestrator and the starting Planet
    pub fn new(
        planet_id: Option<u32>,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<BagType>>,
        ),
        explorer_to_planet_channels:(
            Receiver<PlanetToExplorer>,
            Sender<ExplorerToPlanet>,
        )
    ) -> Self {
        Self {
            planet_id: planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: Some(explorer_to_planet_channels),
        }
    }
}
