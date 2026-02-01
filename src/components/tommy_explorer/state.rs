use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use common_game::protocols::planet_explorer::PlanetToExplorer;

/// These are the states of the explorer state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum ExplorerState {
    Idle,
    WaitingToStartExplorerAI,
    WaitingForNeighbours,
    Traveling,
    GeneratingResource,
    CombiningResources,
    WaitingForSupportedResources,
    WaitingForSupportedCombinations,
    WaitingForAvailableEnergyCells,
    Killed,
}

impl ExplorerState {
    /// Checks if the orchestrator message received is the one expected (based on the explorer state).
    pub fn matches_orchestrator_msg(&self, msg: &OrchestratorToExplorer) -> bool {
        match (self, msg) {
            (ExplorerState::Idle, _) => true,
            (_, OrchestratorToExplorer::KillExplorer) => true,
            (ExplorerState::WaitingToStartExplorerAI, OrchestratorToExplorer::StartExplorerAI) => true,
            (ExplorerState::WaitingForNeighbours, OrchestratorToExplorer::NeighborsResponse { .. }) => true,
            (ExplorerState::Traveling, OrchestratorToExplorer::MoveToPlanet { .. }) => true,
            _ => false
        }
    }

    /// Checks if the planet message received is the one expected (based on the explorer state).
    pub fn matches_planet_msg(&self, msg: &PlanetToExplorer) -> bool {
        match (self, msg) {
            (ExplorerState::Idle, _) => true,
            (ExplorerState::GeneratingResource, PlanetToExplorer::GenerateResourceResponse { .. }) => true,
            (ExplorerState::CombiningResources, PlanetToExplorer::CombineResourceResponse { .. }) => true,
            (ExplorerState::WaitingForSupportedResources, PlanetToExplorer::SupportedResourceResponse { .. }) => true,
            (ExplorerState::WaitingForSupportedCombinations, PlanetToExplorer::CombineResourceResponse { .. }) => true,
            (ExplorerState::WaitingForAvailableEnergyCells, PlanetToExplorer::AvailableEnergyCellResponse { .. }) => true,
            _ => false
        }
    }

    /// Tells if the explorer is in the killed state.
    pub fn should_terminate(&self) -> bool {
        matches!(self, ExplorerState::Killed)
    }

    /// Tells if the explorer can process buffered messages.
    pub fn can_process_buffer(&self) -> bool {
        matches!(self, ExplorerState::Idle)
    }
}
