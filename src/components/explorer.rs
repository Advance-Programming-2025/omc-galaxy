use std::sync::mpsc;

use common_game::{components::planet::Planet, protocols::messages::{
    ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer,
    PlanetToExplorer,
}};

pub type BagType = u32;

pub struct Explorer {
    planet_id: Option<u32>, //I assume that the travel isn't instant so I put an Option we should manage the case the planet explodes
    orchestrator_channels: (
        mpsc::Receiver<OrchestratorToExplorer>,
        mpsc::Sender<ExplorerToOrchestrator<BagType>>,
    ),
    planet_channels: Option<(
        mpsc::Receiver<PlanetToExplorer>,
        mpsc::Sender<ExplorerToPlanet>,
    )>,
}

impl Explorer {
    //At creation, an Explorer should be connected to Orchestrator and the starting Planet
    pub fn new(
        planet_id: Option<u32>,
        explorer_to_orchestrator_channels: (
            mpsc::Receiver<OrchestratorToExplorer>,
            mpsc::Sender<ExplorerToOrchestrator<BagType>>,
        ),
        explorer_to_planet_channels:(
            mpsc::Receiver<PlanetToExplorer>,
            mpsc::Sender<ExplorerToPlanet>,
        )
    ) -> Self {
        Self {
            planet_id: planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: Some(explorer_to_planet_channels),
        }
    }
}
