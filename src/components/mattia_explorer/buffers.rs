use crate::components::mattia_explorer::handlers::{
    combine_resource_request, current_planet_request, generate_resource_request, kill_explorer,
    manage_combine_response, manage_generate_response, manage_supported_combination_response,
    manage_supported_resource_response, move_to_planet, neighbours_response, reset_explorer_ai,
    start_explorer_ai, stop_explorer_ai, supported_combination_request, supported_resource_request,
};
use crate::components::mattia_explorer::states::{
    orch_msg_match_state, planet_msg_match_state, ExplorerState,
};
use crate::components::mattia_explorer::Explorer;
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::PlanetToExplorer;
use logging_utils::log_fn_call;
use logging_utils::{warning_payload, LoggableActor};

/// this function manages all the messages that were put in the buffers
/// (in the same way the explorer usually manages them)
pub fn manage_buffer_msg(explorer: &mut Explorer) -> Result<(), String> {
    //LOG
    log_fn_call!(explorer, "manage_buffer_msg",);
    //LOG
    if !explorer.buffer_orchestrator_msg.is_empty() {
        //this should never panic
        if orch_msg_match_state(
            &explorer.state,
            explorer.buffer_orchestrator_msg.front().unwrap(),
        ) {
            let msg = explorer.buffer_orchestrator_msg.pop_front().unwrap();
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
                    combine_resource_request(explorer, to_generate, true)?;
                }
                OrchestratorToExplorer::BagContentRequest => {
                    // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                    explorer
                        .orchestrator_channels
                        .1
                        .send(ExplorerToOrchestrator::BagContentResponse {
                            explorer_id: explorer.explorer_id,
                            bag_content: explorer.bag.to_resource_types(),
                        })
                        .map_err(|e| e.to_string())?;
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
            let msg = explorer.buffer_planet_msg.pop_front().unwrap();
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
                    match explorer.state {
                        ExplorerState::Surveying {
                            resources,
                            combinations,
                            energy_cells: true,
                            orch_resource,
                            orch_combination,
                        } => {
                            if let Some(planet_info) =
                                explorer.topology_info.get_mut(&explorer.planet_id)
                            {
                                planet_info.update_charge_rate(
                                    available_cells,
                                    explorer.time,
                                    explorer.ai_data.params.charge_rate_alpha,
                                );
                            }
                            if !resources && !combinations {
                                explorer.state = ExplorerState::Idle;
                            } else {
                                explorer.state = ExplorerState::Surveying {
                                    resources,
                                    combinations,
                                    energy_cells: false,
                                    orch_resource,
                                    orch_combination,
                                };
                            }
                        }
                        _ => LogEvent::new(
                            Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                            Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                            EventType::MessagePlanetToExplorer,
                            Channel::Warning,
                            warning_payload!(
                                "received AvailableEnergyCellResponse while not in Surveying state\
                                this should not happen",
                                "",
                                "Explorer::run()";
                                "explorer state"=>format!("{:?}", explorer.state)
                            ),
                        )
                        .emit(),
                    }
                }
                PlanetToExplorer::Stopped => {
                    explorer.state = ExplorerState::Idle;
                }
            }
        }
    }
    Ok(())
}
