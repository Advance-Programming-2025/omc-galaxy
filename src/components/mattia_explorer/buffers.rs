use crate::components::mattia_explorer::handlers::{combine_resource_request, current_planet_request, generate_resource_request, kill_explorer, manage_combine_response, manage_generate_response, manage_supported_combination_response, manage_supported_resource_response, move_to_planet, neighbours_response, reset_explorer_ai, start_explorer_ai, stop_explorer_ai, supported_combination_request, supported_resource_request};
use crate::components::mattia_explorer::states::{orch_msg_match_state, planet_msg_match_state, ExplorerState};
use crate::components::mattia_explorer::Explorer;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::PlanetToExplorer;

// this function manages all the messages that were put in the buffers
// (in the same way the explorer usually manages them)
pub fn manage_buffer_msg(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    if !explorer.buffer_orchestrator_msg.is_empty() {
        //this should never panic
        if orch_msg_match_state(&explorer.state, explorer.buffer_orchestrator_msg.front().unwrap()) {
            let msg=explorer.buffer_orchestrator_msg.pop_front().unwrap();
            match msg {
                OrchestratorToExplorer::StartExplorerAI => {
                    start_explorer_ai(explorer)?;
                }
                OrchestratorToExplorer::ResetExplorerAI => {
                    reset_explorer_ai(explorer)?;
                }
                OrchestratorToExplorer::StopExplorerAI => {
                    stop_explorer_ai(explorer)?;
                }
                OrchestratorToExplorer::KillExplorer => {
                    // I don't think it is possible to arrive here
                    kill_explorer(explorer)?;
                    return Ok(()) //todo gestire questo caso nel loop principale
                }
                OrchestratorToExplorer::MoveToPlanet {
                    sender_to_new_planet,
                    planet_id,
                } => {
                    move_to_planet(explorer, sender_to_new_planet, planet_id)?;
                }
                OrchestratorToExplorer::CurrentPlanetRequest => {
                    current_planet_request(explorer)?;
                }
                OrchestratorToExplorer::SupportedResourceRequest => {
                    supported_resource_request(explorer)?;
                }
                OrchestratorToExplorer::SupportedCombinationRequest => {
                    supported_combination_request(explorer)?;
                }
                OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                    generate_resource_request(explorer, to_generate, true)?;
                }
                OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                    combine_resource_request(explorer, to_generate)?;
                }
                OrchestratorToExplorer::BagContentRequest => {
                    // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                    explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: explorer.explorer_id, bag_content: explorer.bag.to_resource_types()})?;
                }
                OrchestratorToExplorer::NeighborsResponse { neighbors } => {
                    neighbours_response(explorer, neighbors);
                }
            }
        }
    }
    if !explorer.buffer_planet_msg.is_empty() {
        //this should not panic
        if planet_msg_match_state(&explorer.state, explorer.buffer_planet_msg.front().unwrap()) {
            let msg=explorer.buffer_planet_msg.pop_front().unwrap();
            match msg {
                PlanetToExplorer::SupportedResourceResponse { resource_list } => {
                    manage_supported_resource_response(explorer, resource_list)?;
                }
                PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
                    manage_supported_combination_response(explorer, combination_list)?;
                }
                PlanetToExplorer::GenerateResourceResponse { resource } => {
                    manage_generate_response(explorer, resource)?;
                }
                PlanetToExplorer::CombineResourceResponse { complex_response } => {
                    manage_combine_response(explorer, complex_response)?;
                }
                PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                    match explorer.state{
                        ExplorerState::Surveying {resources,combinations,energy_cells:true,orch_resource,orch_combination}=>{
                            match explorer.topology_info.get_mut(&explorer.explorer_id){
                                Some(planet_info) => {
                                    planet_info.update_charge_rate(available_cells, explorer.time);
                                }
                                None => {
                                    //this should not happen
                                }
                            }
                            if !resources && !combinations{
                                explorer.state = ExplorerState::Idle;
                            }
                            else{
                                explorer.state = ExplorerState::Surveying {
                                    resources,
                                    combinations,
                                    energy_cells:false,
                                    orch_resource,
                                    orch_combination,
                                };
                            }
                        }
                        _ => {
                            //todo logs this should not happen
                        }
                    }
                }
                PlanetToExplorer::Stopped => {
                    // TODO gestire in base all'ai dell'explorer
                    explorer.state = ExplorerState::Idle;
                }
            }
        }
    }
    Ok(())
}
