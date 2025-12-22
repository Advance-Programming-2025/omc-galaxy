//use std::sync::mpsc;
use crossbeam_channel::{Receiver, Sender, unbounded};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};

pub type BagType = u32;

pub struct Explorer {
    explorer_id: u32,
    planet_id: Option<u32>, //I assume that the travel isn't instant so I put an Option we should manage the case the planet explodes
    orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<BagType>>,
    ),
    planet_channels: Receiver<PlanetToExplorer>,
}

impl Explorer {
    //At creation, an Explorer should be connected to Orchestrator and the starting Planet
    pub fn new(
        explorer_id: u32,
        planet_id: Option<u32>,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<BagType>>,
        ),
        explorer_to_planet_channels: Receiver<PlanetToExplorer>,
    ) -> Self {
        Self {
            explorer_id,
            planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: explorer_to_planet_channels,
        }
    }
    pub fn id(&self) -> u32 {
        self.explorer_id
    }
}
