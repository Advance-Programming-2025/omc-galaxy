use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use common_game::protocols::planet_explorer::PlanetToExplorer;

// these are the states of the explorer state machine
#[derive(PartialEq, Debug)]
pub enum ExplorerState {
    Idle,
    WaitingForNeighbours,
    Traveling,
    GeneratingResource {
        orchestrator_response: bool,
    },
    CombiningResources {
        orchestrator_response: bool,
    },
    Surveying {
        resources: bool,
        combinations: bool,
        energy_cells: bool,
        orch_resource: bool,
        orch_combination: bool,
    },
    Killed,
}

// this function checks if the orchestrator message received is the one expected (based on the explorer state)
pub fn orch_msg_match_state(explorer_state: &ExplorerState, msg: &OrchestratorToExplorer) -> bool {
    match (explorer_state, msg) {
        (ExplorerState::Idle, _) => true,
        (ExplorerState::WaitingForNeighbours, OrchestratorToExplorer::NeighborsResponse { .. }) => {
            true
        }
        (ExplorerState::Traveling, OrchestratorToExplorer::MoveToPlanet { .. }) => true,
        (_, OrchestratorToExplorer::KillExplorer) => true,
        _ => false,
    }
}

// this function checks if the planet message received is the one expected (based on the explorer state)
pub fn planet_msg_match_state(explorer_state: &ExplorerState, msg: &PlanetToExplorer) -> bool {
    match (explorer_state, msg) {
        (ExplorerState::Idle, _) => true,
        (
            ExplorerState::GeneratingResource {
                orchestrator_response: _,
            },
            PlanetToExplorer::GenerateResourceResponse { .. },
        ) => true,
        (
            ExplorerState::CombiningResources {
                orchestrator_response: _,
            },
            PlanetToExplorer::CombineResourceResponse { .. },
        ) => true,
        (
            ExplorerState::Surveying {
                resources: true, ..
            },
            PlanetToExplorer::SupportedResourceResponse { .. },
        ) => true,
        (
            ExplorerState::Surveying {
                combinations: true, ..
            },
            PlanetToExplorer::SupportedCombinationResponse { .. },
        ) => true,
        (
            ExplorerState::Surveying {
                energy_cells: true, ..
            },
            PlanetToExplorer::AvailableEnergyCellResponse { .. },
        ) => true,
        // (ExplorerState::WaitingForSupportedResources{ orchestrator_response: _ }, PlanetToExplorer::SupportedResourceResponse { .. }) => true,
        // (ExplorerState::WaitingForSupportedCombinations{ orchestrator_response: _ }, PlanetToExplorer::CombineResourceResponse { .. }) => true,
        // (ExplorerState::WaitingForAvailableEnergyCells, PlanetToExplorer::AvailableEnergyCellResponse { .. }) => true,
        _ => false,
    }
}
