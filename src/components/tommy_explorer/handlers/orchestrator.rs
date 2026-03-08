use crossbeam_channel::{SendError, Sender};
use std::collections::HashSet;

use super::planet;
use crate::components::tommy_explorer::actions::ActionQueue;
use crate::components::tommy_explorer::{Explorer, ExplorerState};
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResourceType};
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use logging_utils::{debug_println, log_message, warning_payload};
use one_million_crabs::planet::ToString2;
use crate::components::tommy_explorer::bag::BagType;

/// Handles all messages from the orchestrator,
/// returns Ok(true) if the explorer should terminate, Ok(false) otherwise.
pub fn handle_message(
    explorer: &mut Explorer,
    msg: OrchestratorToExplorer,
) -> Result<bool, String> {
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
        OrchestratorToExplorer::MoveToPlanet {
            sender_to_new_planet,
            planet_id,
        } => {
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
    explorer
        .send_to_orchestrator(ExplorerToOrchestrator::StartExplorerAIResult {
            explorer_id: explorer.id(),
        })
        .map_err(|e| {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "StartExplorerAIResult not sent",
                    format!("Error sending start explorer AI result: {:?}", e),
                    "start_explorer_ai()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
            format!("Error sending start explorer AI result: {:?}", e)
        })?;

    explorer.set_state(ExplorerState::Idle);
    explorer.manual_mode_off();
    // LOG
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "explorer ai started"
    );
    // LOG
    Ok(())
}

/// Resets the topology known by the explorer.
fn reset_explorer_ai(explorer: &mut Explorer) {
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::ResetExplorerAIResult {
        explorer_id: explorer.id(),
    }) {
        Ok(_) => {
            explorer.manual_mode_off();
            explorer.clear_topology();
            explorer.set_state(ExplorerState::Idle);
            log_message!(
                ActorType::Orchestrator,
                0u32,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessageOrchestratorToExplorer,
                "explorer ai reset"
            );        
        }
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "ResetExplorerAIResult not sent",
                    err,
                    "reset_explorer_ai()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
        }
    }
}

/// Puts the explorer in the condition to wait for a StartExplorerAI message.
fn stop_explorer_ai(explorer: &mut Explorer) {
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::StopExplorerAIResult {
        explorer_id: explorer.id(),
    }) {
        Ok(_) => {
            // explorer.set_state(ExplorerState::WaitingToStartExplorerAI); // TODO rimuovere il waiting to start explorer ai?
            explorer.manual_mode_on();
            log_message!(
                ActorType::Orchestrator,
                0u32,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessageOrchestratorToExplorer,
                "explorer ai stopped";
                "manual_mode"=>"true",
            );        
        }
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "StopExplorerAIResult not sent",
                    err,
                    "stop_explorer_ai()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
        }
    }
}

/// Puts the explorer in the Killed state waiting for the thread to be killed.
fn kill_explorer(explorer: &mut Explorer) -> Result<(), String> {
    explorer.send_to_orchestrator(ExplorerToOrchestrator::KillExplorerResult {
            explorer_id: explorer.id(),
        })
        .map_err(|e| {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "KillExplorerResult not sent",
                    format!("Error sending kill explorer result: {:?}", e),
                    "kill_explorer()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
            format!("Error sending kill explorer result: {:?}", e)
        })?;

    explorer.set_state(ExplorerState::Killed);
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "explorer killed"
    );
    Ok(())
}

/// Sets the sender_to_planet of the explorer struct.
fn move_to_planet(
    explorer: &mut Explorer,
    sender_to_new_planet: Option<Sender<ExplorerToPlanet>>,
    planet_id: u32,
) {
    explorer.set_state(ExplorerState::Idle);
    match sender_to_new_planet {
        Some(sender) => {
            explorer.action_queue.clear();
            explorer.action_queue.reset();

            explorer.set_planet_sender(sender);
            explorer.set_planet_id(planet_id);
            //LOG
            log_message!(
                ActorType::Orchestrator,
                0u32,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessageOrchestratorToExplorer,
                "moved to planet";
                "planet_id"=>planet_id.to_string()
            );
            //LOG
        }
        None => {
            log_message!(
                ActorType::Orchestrator,
                0u32,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessageOrchestratorToExplorer,
                "move to planet failed - sender channel is None";
                "planet_id"=>planet_id.to_string()
            );
        }
    }
}

/// Sends the current planet id to the orchestrator.
fn current_planet_request(explorer: &mut Explorer) {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "current planet id requested";
        "planet_id"=>explorer.planet_id.to_string()
    );
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::CurrentPlanetResult {
        explorer_id: explorer.id(),
        planet_id: explorer.planet_id(),
    }) {
        Ok(_) => {
            explorer.set_state(ExplorerState::Idle);
        }
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "CurrentPlanetResult not sent",
                    err,
                    "current_planet_request()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
        }
    }
}

/// Sends the basic resources supported by the current planet to the orchestrator.
fn supported_resource_request(explorer: &mut Explorer) {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "supported resource request";
        "planet_id"=>explorer.planet_id.to_string()
    );
    
    let mut supported_resources = HashSet::new();

    // check if we already have this information in the topology
    if let Some(planet_info) = explorer.get_planet_info(explorer.planet_id())
        && let Some(basic_resources) = &planet_info.basic_resources
    {
        supported_resources = basic_resources.clone();
    } else {
        // supported resource request sent to the planet
        match explorer.send_to_planet(ExplorerToPlanet::SupportedResourceRequest {
            explorer_id: explorer.id(),
        }) {
            Ok(_) => 
            log_message!(
                ActorType::Explorer,
                explorer.explorer_id,
                ActorType::Planet,
                explorer.planet_id,
                EventType::MessageExplorerToPlanet,
                "supported resource request";
                "planet_id"=>explorer.planet_id.to_string()
            ),
            Err(err) => {
                LogEvent::new(
                    Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                    Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                    EventType::MessageExplorerToPlanet,
                    Channel::Error,
                    warning_payload!(
                        "SupportedResourceRequest not sent",
                        err,
                        "supported_resource_request()";
                        "explorer data"=>format!("{:?}", explorer)
                    ),
                ).emit();
                return;
            }
        }

        // waits for the response
        match explorer.receive_from_planet() {
            Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) => {
                supported_resources = resource_list;
            }
            Ok(_) => {
                supported_resources.clear();
                log_message!(
                    ActorType::Planet,
                    explorer.planet_id,
                    ActorType::Explorer,
                    explorer.explorer_id,
                    EventType::MessagePlanetToExplorer,
                    "supported resource response";
                    "planet_id"=>explorer.planet_id.to_string()
                );
                return;
            }
            Err(err) => {
                LogEvent::new(
                Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessagePlanetToExplorer,
                Channel::Error,
                warning_payload!(
                    "SupportedResourceResponse not sent",
                    err,
                    "supported_resource_response()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
                ).emit();
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
        }
        Err(err) => {
            LogEvent::new(
            Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
            Some(Participant::new(ActorType::Orchestrator, 0u32)),
            EventType::MessageExplorerToOrchestrator,
            Channel::Error,
            warning_payload!(
                "SupportedResourceResult not sent",
                err,
                "supported_resource_request()";
                "explorer data"=>format!("{:?}", explorer)
            ),
            ).emit();
        }
    }
}

/// Sends the complex resources supported by the current planet to the orchestrator.
fn supported_combination_request(explorer: &mut Explorer) {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "supported combination request";
        "planet_id"=>explorer.planet_id.to_string()
    );
    
    let mut supported_combinations = HashSet::new();

    // check if we already have this information in the topology
    if let Some(planet_info) = explorer.get_planet_info(explorer.planet_id())
        && let Some(complex_resources) = &planet_info.complex_resources
    {
        supported_combinations = complex_resources.clone();
    } else {
        // supported combination request sent to the planet
        match explorer.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
            explorer_id: explorer.id(),
        }) {
            Ok(_) =>
                log_message!(
                    ActorType::Explorer,
                    explorer.explorer_id,
                    ActorType::Planet,
                    explorer.planet_id,
                    EventType::MessageExplorerToPlanet,
                    "supported combination request";
                    "planet_id"=>explorer.planet_id.to_string()
                ),
            Err(err) => {
                LogEvent::new(
                    Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                    Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                    EventType::MessageExplorerToPlanet,
                    Channel::Error,
                    warning_payload!(
                        "SupportedCombinationRequest not sent",
                        err,
                        "supported_combination_request()";
                        "explorer data"=>format!("{:?}", explorer)
                    ),
                ).emit();
                return;
            }
        }

        // waits for the response
        match explorer.receive_from_planet() {
            Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => {
                supported_combinations = combination_list;
            }
            Ok(_) => {
                supported_combinations.clear();
                log_message!(
                    ActorType::Planet,
                    explorer.planet_id,
                    ActorType::Explorer,
                    explorer.explorer_id,
                    EventType::MessagePlanetToExplorer,
                    "supported combination reaponse";
                    "planet_id"=>explorer.planet_id.to_string()
                );
                return;
            }
            Err(err) => {
                LogEvent::new(
                    Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                    Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                    EventType::MessagePlanetToExplorer,
                    Channel::Error,
                    warning_payload!(
                        "SupportedCombinationResponse not received",
                        err,
                        "supported_combination_request()";
                        "explorer data"=>format!("{:?}", explorer)
                    ),
                ).emit();
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
        }
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "SupportedCombinationResult not sent",
                    err,
                    "supported_combination_request()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
        }
    }
}

/// Sends the GenerateResourceRequest, waits for the planet response, and if successful puts the resource in the bag.
pub fn generate_resource_request(explorer: &mut Explorer, to_generate: BasicResourceType) {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "generate resource request";
        "to_generate" => to_generate.to_string_2(),
        "planet_id"=>explorer.planet_id.to_string()
    );
    
    match explorer.send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
        explorer_id: explorer.id(),
        resource: to_generate,
    }) {
        Ok(_) =>
            log_message!(
                ActorType::Explorer,
                explorer.explorer_id,
                ActorType::Planet,
                explorer.planet_id,
                EventType::MessageExplorerToPlanet,
                "generate resource request";
                "to_generate" => to_generate.to_string_2(),
                "planet_id"=>explorer.planet_id.to_string()
            ),
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                EventType::MessageExplorerToPlanet,
                Channel::Error,
                warning_payload!(
                    "GenerateResourceRequest not sent",
                    err,
                    "generate_resource_request()";
                    "to_generate" => to_generate.to_string_2(),
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
            
            match explorer.send_to_orchestrator(ExplorerToOrchestrator::GenerateResourceResponse { explorer_id: explorer.explorer_id, generated: Err("failed to generate resource".to_string()) }) {
                Ok(_) => {}
                Err(err) => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessageExplorerToOrchestrator,
                        Channel::Error,
                        warning_payload!(
                            "GenerateResourceResponse was not sent",
                            err,
                            "generate_resource_request()";
                            "to_generate" => to_generate.to_string_2(),
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    ).emit();
                }
            } 
            return;
        }
    }

    match explorer.receive_from_planet() {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            planet::put_basic_resource_in_bag(explorer, resource);
            log_message!(
                ActorType::Planet,
                explorer.planet_id,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessagePlanetToExplorer,
                "generate resource response";
                "to_generate" => to_generate.to_string_2(),
                "planet_id"=>explorer.planet_id.to_string()
            );
        }
        Ok(_) => 
            // shouldn't happen // TODO log  the message even if it's the wrong one?
            debug_println!("Explorer received an unexpected message from planet")
            ,
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessagePlanetToExplorer,
                Channel::Error,
                warning_payload!(
                    "GenerateResourceResponse not sent",
                    err,
                    "generate_resource_request()";
                    "to_generate" => to_generate.to_string_2(),
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();

            match explorer.send_to_orchestrator(ExplorerToOrchestrator::GenerateResourceResponse { explorer_id: explorer.explorer_id, generated: Err("failed to generate resource".to_string()) }) {
                Ok(_) => {}
                Err(err) => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessageExplorerToOrchestrator,
                        Channel::Error,
                        warning_payload!(
                            "GenerateResourceResponse was not sent",
                            err,
                            "generate_resource_request()";
                            "to_generate" => to_generate.to_string_2(),
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    ).emit();
                }
            }
            return;
        }
    }

    match explorer.send_to_orchestrator(ExplorerToOrchestrator::GenerateResourceResponse { explorer_id: explorer.explorer_id, generated: Ok(()) }) {
        Ok(_) => {}
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                            "GenerateResourceResponse was not sent",
                            err,
                            "generate_resource_request()";
                            "to_generate" => to_generate.to_string_2(),
                            "explorer data"=>format!("{:?}", explorer)
                        ),
            ).emit();
        }
    }
}

/// Sends the CombineResourceRequest, waits for the planet response, and if successful puts the resource in the bag.
pub fn combine_resource_request(explorer: &mut Explorer, to_generate: ComplexResourceType) {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "combine resource request";
        "to_generate" => to_generate.to_string_2(),
        "planet_id"=>explorer.planet_id.to_string()
    );
    
    let complex_resource_req = explorer.make_complex_request(to_generate);

    match complex_resource_req {
        Ok(complex_resource_req) => {
            match explorer.send_to_planet(ExplorerToPlanet::CombineResourceRequest {
                explorer_id: explorer.id(),
                msg: complex_resource_req,
            }) {
                Ok(_) =>
                    log_message!(
                        ActorType::Explorer,
                        explorer.explorer_id,
                        ActorType::Planet,
                        explorer.planet_id,
                        EventType::MessageExplorerToPlanet,
                        "combine resource request";
                        "to_generate" => to_generate.to_string_2(),
                        "planet_id"=>explorer.planet_id.to_string()
                    ),
                Err(err) => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                        EventType::MessageExplorerToPlanet,
                        Channel::Error,
                        warning_payload!(
                            "CombineResourceRequest not sent",
                            err,
                            "combine_resource_request()";
                            "to_generate" => to_generate.to_string_2(),
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    ).emit();

                    match explorer.send_to_orchestrator(ExplorerToOrchestrator::CombineResourceResponse { explorer_id: explorer.explorer_id, generated: Err("failed to generate resource".to_string()) }) {
                        Ok(_) => {}
                        Err(err) => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                EventType::MessageExplorerToOrchestrator,
                                Channel::Error,
                                warning_payload!(
                                    "CombineResourceResponse was not sent",
                                    err,
                                    "combine_resource_request()";
                                    "to_generate" => to_generate.to_string_2(),
                                    "explorer data"=>format!("{:?}", explorer)
                                ),
                            ).emit();
                        }
                    }
                    return;
                }
            }

            match explorer.receive_from_planet() {
                Ok(PlanetToExplorer::CombineResourceResponse { complex_response }) => {
                    planet::put_complex_resource_in_bag(explorer, complex_response);
                }
                Ok(_) => {
                    // should not happen
                    debug_println!("Explorer received an unexpected message from planet");
                }
                Err(err) => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        EventType::MessagePlanetToExplorer,
                        Channel::Error,
                        warning_payload!(
                            "CombineResourceResponse not sent",
                            err,
                            "combine_resource_request()";
                            "to_generate" => to_generate.to_string_2(),
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    ).emit();
                    return;
                }
            }

            match explorer.send_to_orchestrator(ExplorerToOrchestrator::GenerateResourceResponse { explorer_id: explorer.explorer_id, generated: Ok(()) }) {
                Ok(_) => {}
                Err(err) => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessageExplorerToOrchestrator,
                        Channel::Error,
                        warning_payload!(
                            "CombineResourceResponse was not sent",
                            err,
                            "combine_resource_request()";
                            "to_generate" => to_generate.to_string_2(),
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    ).emit();
                }
            }
        }
        Err(err) => {
            // TODO log this error
        }
    }
}

/// Sends the bag content to the orchestrator.
fn bag_content_request(explorer: &mut Explorer) {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "bag content request";
    );
    
    match explorer.send_to_orchestrator(ExplorerToOrchestrator::BagContentResponse {
        explorer_id: explorer.id(),
        bag_content: explorer.get_bag_content(),
    }) {
        Ok(_) => {}
        Err(err) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "BagContentResponse not sent",
                    err,
                    "bag_content_request()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();
        }
    }
}

/// Updates the neighbours of the current planet.
fn neighbors_response(explorer: &mut Explorer, neighbors: Vec<u32>) {
    explorer.set_state(ExplorerState::Idle);
    explorer.update_neighbors(explorer.planet_id(), neighbors.clone());

    log_message!(
        ActorType::Planet,
        explorer.planet_id,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessagePlanetToExplorer,
        "neighbors received";
        "neighbors"=>format!("{:?}", neighbors)
    );
}
