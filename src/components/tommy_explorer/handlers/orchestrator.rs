use crossbeam_channel::Sender;
use std::collections::HashSet;

use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use crate::components::tommy_explorer::{Explorer, ExplorerState};
use super::planet;

/// Handles all messages from the orchestrator,
/// returns Ok(true) if the explorer should terminate, Ok(false) otherwise.
pub fn handle_message(explorer: &mut Explorer, msg: OrchestratorToExplorer) -> Result<bool, String> {
    match msg {
        OrchestratorToExplorer::StartExplorerAI => {
            start_explorer_ai(explorer)?;
            Ok(false)
        }
        OrchestratorToExplorer::ResetExplorerAI => {
            reset_explorer_ai(explorer);
            Ok(false)
        }
        OrchestratorToExplorer::StopExplorerAI => {
            stop_explorer_ai(explorer);
            Ok(false)
        }
        OrchestratorToExplorer::KillExplorer => {
            kill_explorer(explorer)?;
            Ok(true)
        }
        OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id } => {
            move_to_planet(explorer, sender_to_new_planet, planet_id);
            Ok(false)
        }
        OrchestratorToExplorer::CurrentPlanetRequest => {
            current_planet_request(explorer);
            Ok(false)
        }
        OrchestratorToExplorer::SupportedResourceRequest => {
            supported_resource_request(explorer);
            Ok(false)
        }
        OrchestratorToExplorer::SupportedCombinationRequest => {
            supported_combination_request(explorer);
            Ok(false)
        }
        OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
            generate_resource_request(explorer, to_generate);
            Ok(false)
        }
        OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
            combine_resource_request(explorer, to_generate);
            Ok(false)
        }
        OrchestratorToExplorer::BagContentRequest => {
            bag_content_request(explorer);
            Ok(false)
        }
        OrchestratorToExplorer::NeighborsResponse { neighbors } => {
            neighbors_response(explorer, neighbors);
            Ok(false)
        }
    }
}

/// Puts the explorer in the condition to receive messages (idle state).
fn start_explorer_ai(explorer: &mut Explorer) -> Result<(), String> {
    explorer.send_to_orchestrator(ExplorerToOrchestrator::StartExplorerAIResult {
            explorer_id: explorer.id(),
        })
        .map_err(|e| format!("Error sending start explorer AI result: {:?}", e))?;

    explorer.set_state(ExplorerState::Idle);
    println!("[EXPLORER DEBUG] Start explorer AI result sent correctly.");
    Ok(())
}

/// Resets the topology known by the explorer.
fn reset_explorer_ai(explorer: &mut Explorer) {
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::ResetExplorerAIResult {
        explorer_id: explorer.id(),
    }) {
        Ok(_) => {
            explorer.clear_topology();
            explorer.set_state(ExplorerState::Idle);
            println!("[EXPLORER DEBUG] Reset explorer AI result sent correctly.");
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending reset explorer AI result: {:?}", err);
        }
    }
}

/// Puts the explorer in the condition to wait for a StartExplorerAI message.
fn stop_explorer_ai(explorer: &mut Explorer) {
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::StopExplorerAIResult {
        explorer_id: explorer.id(),
    }) {
        Ok(_) => {
            explorer.set_state(ExplorerState::WaitingToStartExplorerAI);
            println!("[EXPLORER DEBUG] Stop explorer AI result sent correctly.");
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending stop explorer AI result: {:?}", err);
        }
    }
}

/// Puts the explorer in the Killed state waiting for the thread to be killed.
fn kill_explorer(explorer: &mut Explorer) -> Result<(), String> {
    explorer
        .send_to_orchestrator(ExplorerToOrchestrator::KillExplorerResult {
            explorer_id: explorer.id(),
        })
        .map_err(|e| format!("Error sending kill explorer result: {:?}", e))?;

    explorer.set_state(ExplorerState::Killed);
    println!("[EXPLORER DEBUG] Kill explorer result sent correctly.");
    Ok(())
}

/// Sets the sender_to_planet of the explorer struct.
fn move_to_planet(
    explorer: &mut Explorer,
    sender_to_new_planet: Option<Sender<ExplorerToPlanet>>,
    planet_id: u32,
) {
    explorer.set_state(ExplorerState::Idle);
    explorer.action_queue.clear();
    match sender_to_new_planet {
        Some(sender) => {
            explorer.set_planet_sender(sender);
            explorer.set_planet_id(planet_id);
            println!("[EXPLORER DEBUG] Sender channel set correctly");
        }
        None => {
            println!("[EXPLORER DEBUG] Sender channel is None.");
        }
    }
}

/// Sends the current planet id to the orchestrator.
fn current_planet_request(explorer: &mut Explorer) {
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::CurrentPlanetResult {
        explorer_id: explorer.id(),
        planet_id: explorer.planet_id(),
    }) {
        Ok(_) => {
            explorer.set_state(ExplorerState::Idle);
            println!("[EXPLORER DEBUG] Current planet result sent correctly.");
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending current planet result: {:?}", err);
        }
    }
}

/// Sends the basic resources supported by the current planet to the orchestrator.
fn supported_resource_request(explorer: &mut Explorer) {
    let mut supported_resources = HashSet::new();

    // check if we already have this information in the topology
    if let Some(planet_info) = explorer.get_planet_info(explorer.planet_id()) 
        && let Some(basic_resources) = &planet_info.basic_resources {
        supported_resources = basic_resources.clone();
    } else {
        // supported resource request sent to the planet
        match explorer.send_to_planet(ExplorerToPlanet::SupportedResourceRequest {
            explorer_id: explorer.id(),
        }) {
            Ok(_) => println!("[EXPLORER DEBUG] Supported resource request sent correctly from explorer."),
            Err(err) => {
                println!("[EXPLORER DEBUG] Error sending supported resource request from explorer: {:?}", err);
                return;
            }
        }
        
        // waits for the response 
        match explorer.receive_from_planet() {
            Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) => {
                supported_resources = resource_list;
            }
            Ok(_) => {
                println!("[EXPLORER DEBUG] Unexpected response to SupportedResourceRequest.");
                supported_resources.clear();
                return;
            }
            Err(err) => {
                println!("[EXPLORER DEBUG] Error receiving supported resources from planet: {:?}", err);
                return;
            }
        }
    }

    // sends the result to the orchestrator
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::SupportedResourceResult {
        explorer_id: explorer.id(),
        supported_resources,
    }) {
        Ok(_) => {
            explorer.set_state(ExplorerState::Idle);
            println!("[EXPLORER DEBUG] Supported resource result sent correctly from explorer to orchestrator.");
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending supported resource result from explorer to orchestrator: {:?}", err);
        }
    }
}

/// Sends the complex resources supported by the current planet to the orchestrator.
fn supported_combination_request(explorer: &mut Explorer) {
    let mut supported_combinations = HashSet::new();

    // check if we already have this information in the topology
    if let Some(planet_info) = explorer.get_planet_info(explorer.planet_id()) 
        && let Some(complex_resources) = &planet_info.complex_resources{
        supported_combinations = complex_resources.clone();
    } else {
        // supported combination request sent to the planet
        match explorer.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
            explorer_id: explorer.id(),
        }) {
            Ok(_) => println!("[EXPLORER DEBUG] Supported combination request sent correctly from explorer."),
            Err(err) => {
                println!("[EXPLORER DEBUG] Error sending supported combination request from explorer: {:?}", err);
                return;
            }
        }

        // waits for the response
        match explorer.receive_from_planet() {
            Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => {
                supported_combinations = combination_list;
            }
            Ok(_) => {
                println!("[EXPLORER DEBUG] Unexpected response to SupportedCombinationRequest.");
                supported_combinations.clear();
                return;
            }
            Err(err) => {
                println!("[EXPLORER DEBUG] Error receiving supported combinations from planet: {:?}", err);
                return;
            }
        }
    }

    // sends the result to the orchestrator
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::SupportedCombinationResult {
        explorer_id: explorer.id(),
        combination_list: supported_combinations,
    }) {
        Ok(_) => {
            explorer.set_state(ExplorerState::Idle);
            println!("[EXPLORER DEBUG] Supported combination result sent correctly from explorer to orchestrator.");
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending supported combination result from explorer to orchestrator: {:?}", err);
        }
    }
}

/// Sends the GenerateResourceRequest, waits for the planet response, and if successful puts the resource in the bag.
pub fn generate_resource_request(explorer: &mut Explorer, to_generate: BasicResourceType) {
    match explorer.send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
        explorer_id: explorer.id(),
        resource: to_generate,
    }) {
        Ok(_) => println!("[EXPLORER DEBUG] Generate resource request sent correctly"),
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending generate resource request {}", err);
            return;
        }
    }

    match explorer.receive_from_planet() {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            planet::put_basic_resource_in_bag(explorer, resource);
        }
        Ok(_) => println!("[EXPLORER DEBUG] Unexpected response to generate resource request"),
        Err(err) => {
            println!("[EXPLORER DEBUG] Error receiving generate resource response {}", err);
        }
    }
}

/// Sends the CombineResourceRequest, waits for the planet response, and if successful puts the resource in the bag.
pub fn combine_resource_request(explorer: &mut Explorer, to_generate: ComplexResourceType) {
    let complex_resource_req = explorer.make_complex_request(to_generate);

    match complex_resource_req {
        Ok(complex_resource_req) => {
            match explorer.send_to_planet(ExplorerToPlanet::CombineResourceRequest {
                explorer_id: explorer.id(),
                msg: complex_resource_req,
            }) {
                Ok(_) => println!("[EXPLORER DEBUG] Combine resource request sent correctly"),
                Err(err) => {
                    println!("[EXPLORER DEBUG] Error sending combine resource request {}", err);
                    return;
                }
            }

            match explorer.receive_from_planet() {
                Ok(PlanetToExplorer::CombineResourceResponse { complex_response }) => {
                    planet::put_complex_resource_in_bag(explorer, complex_response);
                }
                Ok(_) => println!("[EXPLORER DEBUG] Unexpected response to combine resource request"),
                Err(err) => {
                    println!("[EXPLORER DEBUG] Error receiving combine resource response {}", err);
                }
            }
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error generating complex resource request {}", err);
        }
    }
}

/// Sends the bag content to the orchestrator.
fn bag_content_request(explorer: &mut Explorer) {
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::BagContentResponse {
        explorer_id: explorer.id(),
        bag_content: explorer.get_bag_content(),
    }) {
        Ok(_) => {
            println!("[EXPLORER DEBUG] BagContent response sent correctly");
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending bag content response: {}", err);
        }
    }
}

/// Updates the neighbours of the current planet.
fn neighbors_response(explorer: &mut Explorer, neighbors: Vec<u32>) {
    explorer.set_state(ExplorerState::Idle);
    explorer.update_neighbors(explorer.planet_id(), neighbors);
}
