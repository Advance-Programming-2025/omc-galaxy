use crate::components::mattia_explorer::explorer_ai::AiData;
use crate::components::mattia_explorer::helpers::gather_info_from_planet;
use crate::components::mattia_explorer::resource_management::ToGeneric;
use crate::components::mattia_explorer::states::ExplorerState;
use crate::components::mattia_explorer::{Explorer, PlanetInfo};
use common_game::components::resource::{
    BasicResource, BasicResourceType, ComplexResource, ComplexResourceType,
    GenericResource,
};
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::Sender;
use logging_utils::{log_internal_op, log_message, payload, warning_payload, LoggableActor};
use one_million_crabs::planet::ToString2;
use std::collections::HashSet;

/// this function put the explorer in the condition to receive messages (idle state),
/// it is called when the explorer receives the StartExplorerAI message
pub fn start_explorer_ai(explorer: &mut Explorer) -> Result<(), String> {
    explorer.state = ExplorerState::Idle;
    explorer.manual_mode = false;
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "explorer ai started"
    );
    log_internal_op!(explorer, "sending StartExplorerAIResult");
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::StartExplorerAIResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => Ok(()),
        Err(err) => {
            LogEvent::new( //todo forse posso togliere questo log
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "StartExplorerAIResult not sent",
                    err,
                    "start_explorer_ai()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            Err(err.to_string())
        }
    }
}

/// this function resets the topology known by the explorer and its AiData,
/// it is called when the explorer receives the ResetExplorerAI message
pub fn reset_explorer_ai(explorer: &mut Explorer) -> Result<(), String> {
    explorer.state = ExplorerState::Idle;
    explorer.topology_info.clear();
    explorer
        .topology_info
        .insert(explorer.planet_id, PlanetInfo::new(0));
    explorer.current_planet_neighbors_update = false;
    explorer.manual_mode = false;
    explorer.ai_data = AiData::new();
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "explorer ai reset"
    );
    log_internal_op!(explorer, "sending ResetExplorerAIResult");
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::ResetExplorerAIResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => Ok(()),
        Err(err) => {
            LogEvent::new( //todo forse posso togliere questo log
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
            )
            .emit();
            Err(err.to_string())
        }
    }
}

/// this function put the explorer in the condition to wait for a StartExplorerAI message (WaitingToStartExplorerAI state),
/// it is called when the explorer receives the StopExplorerAI message
pub fn stop_explorer_ai(explorer: &mut Explorer) -> Result<(), String> {
    explorer.manual_mode = true;
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "explorer ai stopped";
        "manual_mode"=>"true",
    );
    log_internal_op!(explorer, "sending StopExplorerAIResult");
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::StopExplorerAIResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => Ok(()),
        Err(err) => {
            LogEvent::new( //todo forse posso togliere questo log
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
            )
            .emit();
            Err(err.to_string())
        }
    }
}

/// this function puts the explorer in the Killed state waiting for the thread to be terminated
pub fn kill_explorer(explorer: &mut Explorer) -> Result<(), String> {
    explorer.state = ExplorerState::Killed;
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "explorer killed"
    );

    log_internal_op!(explorer, "sending KillExplorerResult");
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::KillExplorerResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => Ok(()),
        Err(err) => {
            LogEvent::new( //todo forse posso togliere questo log
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                EventType::MessageExplorerToOrchestrator,
                Channel::Error,
                warning_payload!(
                    "KillExplorerResult not sent",
                    err,
                    "kill_explorer()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            Err(err.to_string())
        }
    }
}

/// this function sets the sender_to_planet of the explorer struct
pub fn move_to_planet(
    explorer: &mut Explorer,
    sender_to_new_planet: Option<Sender<ExplorerToPlanet>>,
    planet_id: ID,
) -> Result<(), String> {
    explorer.state = ExplorerState::Idle;
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
    let mut ris;
    match sender_to_new_planet {
        //in case the planet dies there are 2 cases:
        // the orchestrator refuses the move operation
        // the orchestrator kills also the explorer if it has already accepted the move
        Some(sender) => {
            explorer.planet_channels.1 = sender;
            explorer.planet_id = planet_id;
            match explorer.topology_info.get(&planet_id) {
                Some(planet_info) => {
                    if !explorer.manual_mode {
                        explorer.state = ExplorerState::Surveying {
                            resources: planet_info.basic_resources.is_none(),
                            combinations: planet_info.complex_resources.is_none(),
                            energy_cells: true,
                            orch_resource: false,
                            orch_combination: false,
                        };
                    }

                    log_internal_op!(explorer, "sending MovedToPlanetResult");
                    match explorer.orchestrator_channels.1.send(
                        ExplorerToOrchestrator::MovedToPlanetResult {
                            explorer_id: explorer.explorer_id,
                            planet_id: planet_id,
                        },
                    ) {
                        Ok(_) => ris = Ok(()),
                        Err(err) => ris = Err(err.to_string()),
                    }
                }
                None => {
                    explorer.topology_info.insert(planet_id, PlanetInfo::new(0));
                    if !explorer.manual_mode {
                        explorer.current_planet_neighbors_update = true;
                        explorer.state = ExplorerState::Surveying {
                            resources: true,
                            combinations: true,
                            energy_cells: true,
                            orch_resource: false,
                            orch_combination: false,
                        };
                    }
                    log_internal_op!(explorer, "sending MovedToPlanetResult");
                    match explorer.orchestrator_channels.1.send(
                        ExplorerToOrchestrator::MovedToPlanetResult {
                            explorer_id: explorer.explorer_id,
                            planet_id: explorer.planet_id,
                        },
                    ) {
                        Ok(_) => ris = Ok(()),
                        Err(err) => {
                            ris = Err(err.to_string())
                        }
                    }
                }
            }
            if !explorer.manual_mode {
                gather_info_from_planet(explorer).map_err(|e| e.to_string())?;
            }
            ris
        }
        None => {
            //the explorer cannot move, but it is not a problem
            //absolute priority
            explorer.current_planet_neighbors_update = true;
            log_message!(
                ActorType::Orchestrator,
                0u32,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessageOrchestratorToExplorer,
                "move to planet failed - sender channel is None";
                "planet_id"=>planet_id.to_string()
            );
            Ok(())
        }
    }
}

/// this function sends the current planet id to the orchestrator
pub fn current_planet_request(explorer: &mut Explorer) -> Result<(), String> {
    explorer.state = ExplorerState::Idle;
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "current planet id requested";
        "planet_id"=>explorer.planet_id.to_string()
    );
    log_internal_op!(explorer, "sending CurrentPlanetResult");
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::CurrentPlanetResult {
            explorer_id: explorer.explorer_id,
            planet_id: explorer.planet_id,
        }) {
        Ok(_) => Ok(()),
        Err(err) => {
            LogEvent::new(//todo forse posso togliere questo log
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
            )
            .emit();
            Err(err.to_string())
        }
    }
}

/// this function sends the basic resources supported by the current planet to the orchestrator
/// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
/// response and then send it back to the orchestrator)
pub fn supported_resource_request(explorer: &mut Explorer) -> Result<(), String> {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "supported resource request";
        "planet_id"=>explorer.planet_id.to_string()
    );
    match explorer.topology_info.get(&explorer.planet_id) {
        Some(planet_info) => {
            match &planet_info.basic_resources {
                Some(basic_resources) => {
                    log_internal_op!(explorer, "sending SupportedResourceResult");
                    explorer.orchestrator_channels.1.send(
                        ExplorerToOrchestrator::SupportedResourceResult {
                            explorer_id: explorer.explorer_id,
                            supported_resources: basic_resources.clone(),
                        },
                    ).map_err(|err| err.to_string())?;
                }
                None => {
                    match explorer.state {
                        ExplorerState::Idle => {
                            explorer.state = ExplorerState::Surveying {
                                resources: true,
                                combinations: false,
                                energy_cells: false,
                                orch_resource: true,
                                orch_combination: false,
                            };
                            gather_info_from_planet(explorer).map_err(|e| e.to_string())?;
                        }
                        _ => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                                EventType::MessageOrchestratorToExplorer,
                                Channel::Warning,
                                warning_payload!(
                                    "Tried to survey supported_resource from planet while not in Idle state.\
                                    No reply will be sent to orchestrator.\
                                    This should never happen.",
                                    "",
                                    "supported_resource_request()";
                                    "explorer data"=>format!("{:?}", explorer)
                                )
                            ).emit();
                            return Err("Tried to survey supported_resource from planet while not in Idle state".to_string());
                        }
                    }
                }
            }
        }
        None => {
            log_internal_op!(explorer, "the explorer doesnt have the current planet in his topology\
                this isn't something that should happen, but the explorer will try to recover\
                requesting information about the planet");
            //this should not happen
            match explorer.state {
                ExplorerState::Idle => {
                    explorer.state = ExplorerState::Surveying {
                        resources: true,
                        combinations: true,
                        energy_cells: true,
                        orch_resource: true,
                        orch_combination: false,
                    };
                    gather_info_from_planet(explorer).map_err(|e| e.to_string())?;
                }
                _ => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        EventType::MessageOrchestratorToExplorer,
                        Channel::Warning,
                        warning_payload!(
                            "Tried to survey supported_resource from planet while not in Idle state.\
                            No reply will be sent to orchestrator.\
                            This should never happen.",
                            "",
                            "supported_resource_request()";
                            "explorer data"=>format!("{:?}", explorer)
                        )
                    ).emit();
                    return Err("Tried to survey supported_resource from planet while not in Idle state".to_string());
                }
            }
        }
    }
    Ok(())
}

/// this function sends the complex resources supported by the current planet to the orchestrator
/// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
/// response and then send it back to the orchestrator)
pub fn supported_combination_request(explorer: &mut Explorer) -> Result<(), String> {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "supported combination request";
        "planet_id"=>explorer.planet_id.to_string()
    );
    match explorer.topology_info.get(&explorer.planet_id) {
        Some(planet_info) => {
            match &planet_info.complex_resources {
                Some(complex_resource) => {
                    explorer.orchestrator_channels.1.send(
                        ExplorerToOrchestrator::SupportedCombinationResult {
                            explorer_id: explorer.explorer_id,
                            combination_list: complex_resource.clone(),
                        },
                    ).map_err(|err| err.to_string())?;
                }
                None => {
                    //this should not happen
                    match explorer.state {
                        ExplorerState::Idle => {
                            explorer.state = ExplorerState::Surveying {
                                resources: false,
                                combinations: true,
                                energy_cells: false,
                                orch_resource: false,
                                orch_combination: true,
                            };
                            gather_info_from_planet(explorer).map_err(|e| e.to_string())?;
                        }
                        _ => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                                EventType::MessageOrchestratorToExplorer,
                                Channel::Warning,
                                warning_payload!(
                                    "Tried to survey complex_resource from planet while not in Idle state.\
                                    No reply will be sent to orchestrator.\
                                    This should never happen.",
                                    "",
                                    "supported_combination_request()";
                                    "explorer data"=>format!("{:?}", explorer)
                                )
                            ).emit();
                            return Err("Tried to survey complex_resource from planet while not in Idle state".to_string());
                        }
                    }
                }
            }
        }
        None => {
            //this should not happen
            log_internal_op!(explorer, "the explorer doesnt have the current planet in his topology\
                this isn't something that should happen, but the explorer will try to recover\
                requesting information about the planet");
            match explorer.state {
                ExplorerState::Idle => {
                    explorer.state = ExplorerState::Surveying {
                        resources: true,
                        combinations: true,
                        energy_cells: true,
                        orch_resource: false,
                        orch_combination: true,
                    };
                    gather_info_from_planet(explorer).map_err(|e| e.to_string())?;
                }
                _ => {
                    LogEvent::new(
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                        EventType::MessageOrchestratorToExplorer,
                        Channel::Warning,
                        warning_payload!(
                            "Tried to survey complex_resource from planet while not in Idle state.\
                            No reply will be sent to orchestrator.\
                            This should never happen.",
                            "",
                            "supported_combination_request()";
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    )
                    .emit();
                    return Err("Tried to survey complex_resource from planet while not in Idle state".to_string());
                }
            }
        }
    }
    Ok(())
}

/// this function sends the GenerateResourceRequest, waits for the planet response, and,
/// if successful puts the resource in the bag
pub fn generate_resource_request(
    explorer: &mut Explorer,
    to_generate: BasicResourceType,
    to_orchestrator: bool,
) -> Result<(), String> {
    explorer.state = ExplorerState::GeneratingResource {
        orchestrator_response: to_orchestrator,
    };
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "generate resource request";
        "to_generate" => to_generate.to_string_2(),
        "to_orchestrator" => to_orchestrator,
        "planet_id"=>explorer.planet_id.to_string()
    );

    log_internal_op!(explorer, "sending GenerateResourceRequest");
    match explorer
        .planet_channels
        .1
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id: explorer.explorer_id,
            resource: to_generate,
        }) {
        Ok(_) => Ok(()),
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
                    "to_orchestrator" => to_orchestrator,
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            Err(err.to_string())
        }
    }
}

/// this function sends the CombineResourceRequest, waits for the planet response, and,
/// if successful puts the resource in the bag
pub fn combine_resource_request(
    explorer: &mut Explorer,
    to_generate: ComplexResourceType,
    to_orchestrator: bool,
) -> Result<(), String> {
    log_message!(
        ActorType::Orchestrator,
        0u32,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessageOrchestratorToExplorer,
        "combine resource request";
        "to_generate" => to_generate.to_string_2(),
        "to_orchestrator" => to_orchestrator,
        "planet_id"=>explorer.planet_id.to_string()
    );
    let complex_resource_req = match to_generate {
        //provide the requested resources from the bag for each combination
        ComplexResourceType::Diamond => explorer.bag.make_diamond_request(),
        ComplexResourceType::Water => explorer.bag.make_water_request(),
        ComplexResourceType::Life => explorer.bag.make_life_request(),
        ComplexResourceType::Robot => explorer.bag.make_robot_request(),
        ComplexResourceType::Dolphin => explorer.bag.make_dolphin_request(),
        ComplexResourceType::AIPartner => explorer.bag.make_ai_partner_request(),
    };
    let ris = match complex_resource_req {
        Ok(request) => {
            explorer.state = ExplorerState::CombiningResources {
                orchestrator_response: to_orchestrator,
            };

            log_internal_op!(explorer, "sending CombineResourceRequest");
            match explorer
                .planet_channels
                .1
                .send(ExplorerToPlanet::CombineResourceRequest {
                    explorer_id: explorer.explorer_id,
                    msg: request,
                }) {
                Ok(_) => Ok(()),
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
                            "to_orchestrator" => to_orchestrator,
                            "explorer data"=>format!("{:?}", explorer)
                        ),
                    ).emit();
                    Err(err.to_string())
                }
            }
        }
        Err(err) => {
            LogEvent::self_directed(
                Participant::new(ActorType::Explorer, explorer.explorer_id),
                EventType::InternalExplorerAction,
                Channel::Debug,
                warning_payload!(
                    format!("Cannot create complex resource request for {:?}", to_generate),
                    err,
                    "combine_resource_request()";
                    "explorer data"=>format!("{:?}", explorer)
                )
            ).emit();
            explorer.state = ExplorerState::Idle;
            explorer.orchestrator_channels.1.send(
                ExplorerToOrchestrator::CombineResourceResponse {
                    explorer_id: 0,
                    generated: Err("Not enough basic resource".to_string()),
                },
            ).map_err(|err| err.to_string())?;
            Err(err)
        }
    };
    ris
}

/// this function updates the neighbours of the current planet
pub fn neighbours_response(explorer: &mut Explorer, neighbors: Vec<ID>) {
    explorer.state = ExplorerState::Idle;
    for &neighbour in &neighbors {
        explorer
            .topology_info
            .entry(neighbour)
            .or_insert(PlanetInfo::new(explorer.time));
    }
    log_message!(
        ActorType::Planet,
        explorer.planet_id,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessagePlanetToExplorer,
        "neighbors received";
        "neighbors"=>format!("{:?}", neighbors)
    );
    match explorer.topology_info.get_mut(&explorer.planet_id) {
        Some(planet_info) => {
            explorer.current_planet_neighbors_update = false;
            //already overriding the neighbors
            planet_info.neighbors = Some(neighbors.clone().into_iter().collect());
            planet_info.timestamp_neighbors = explorer.time;
            //updating ai move_utility data
            explorer.ai_data.ai_action.move_to.clear();
            explorer.ai_data.ai_action.move_to = neighbors.into_iter().map(|x| (x, 0.0)).collect();
        }
        None => {
            explorer
                .topology_info
                .insert(explorer.planet_id, PlanetInfo::new(explorer.time));
            //this should never panic
            explorer
                .topology_info
                .get_mut(&explorer.planet_id)
                .unwrap()
                .neighbors = Some(neighbors.clone().into_iter().collect());
        }
    }
}
/// this function takes a basic resource list and updates the explorer topology data,
/// also if the orchestrator requested the supported resource this function will send it
/// to the orchestrator
pub fn manage_supported_resource_response(
    explorer: &mut Explorer,
    resource_list: HashSet<BasicResourceType>,
) -> Result<(), String> {
    log_message!(
        ActorType::Planet,
        explorer.planet_id,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessagePlanetToExplorer,
        "supported resource received";
        "supported resource"=>format!("{:?}", resource_list)
    );
    match explorer.state {
        ExplorerState::Surveying {
            resources: true,
            combinations,
            energy_cells,
            orch_resource,
            orch_combination,
        } => {
            match explorer.topology_info.get_mut(&explorer.planet_id) {
                Some(planet_info) => {
                    planet_info.basic_resources = Some(resource_list.clone());
                    if planet_info.complex_resources.is_some() {
                        planet_info.calculate_planet_type()?;
                    }
                }
                None => {
                    explorer
                        .topology_info
                        .insert(explorer.planet_id, PlanetInfo::new(explorer.time));
                    //this should never panic
                    explorer
                        .topology_info
                        .get_mut(&explorer.planet_id)
                        .unwrap()
                        .basic_resources = Some(resource_list.clone());
                }
            }
            if orch_resource {
                log_internal_op!(explorer, "sending SupportedResourceResult");
                explorer.orchestrator_channels.1.send(
                    ExplorerToOrchestrator::SupportedResourceResult {
                        explorer_id: explorer.explorer_id,
                        supported_resources: resource_list,
                    },
                ).map_err(|err| err.to_string())?;
            }

            //updating explorer state
            if !combinations && !energy_cells {
                explorer.state = ExplorerState::Idle;
            } else {
                explorer.state = ExplorerState::Surveying {
                    resources: false,
                    combinations,
                    energy_cells,
                    orch_resource: false,
                    orch_combination,
                };
            }
        }
        _ => {
            LogEvent::new(
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessageOrchestratorToExplorer,
                Channel::Warning,
                warning_payload!(
                    "tried to manage supported resource response while not in Idle state\
                    this should never happen\
                    the response will not be processed",
                    "",
                    "manage_supported_resource_response()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            return Err("tried to manage supported resource response while not in Idle state".to_string())
        }
    }
    Ok(())
}
/// this function takes a complex resource list and updates the explorer topology data,
/// also if the orchestrator requested the supported combination this function will send it
/// to the orchestrator
pub fn manage_supported_combination_response(
    explorer: &mut Explorer,
    combination_list: HashSet<ComplexResourceType>,
) -> Result<(), String> {
    log_message!(
        ActorType::Planet,
        explorer.planet_id,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessagePlanetToExplorer,
        "supported combinations received";
        "supported combinations"=>format!("{:?}", combination_list)
    );
    match explorer.state {
        ExplorerState::Surveying {
            resources,
            combinations: true,
            energy_cells,
            orch_resource,
            orch_combination,
        } => {
            match explorer.topology_info.get_mut(&explorer.planet_id) {
                Some(planet_info) => {
                    planet_info.complex_resources = Some(combination_list.clone());
                    if planet_info.basic_resources.is_some() {
                        planet_info.calculate_planet_type()?;
                    }
                }
                None => {
                    explorer
                        .topology_info
                        .insert(explorer.planet_id, PlanetInfo::new(explorer.time));
                    //this should never panic
                    explorer
                        .topology_info
                        .get_mut(&explorer.planet_id)
                        .unwrap()
                        .complex_resources = Some(combination_list.clone());
                }
            }
            if orch_combination {
                log_internal_op!(explorer, "sending SupportedCombinationResult");
                explorer.orchestrator_channels.1.send(
                    ExplorerToOrchestrator::SupportedCombinationResult {
                        explorer_id: explorer.explorer_id,
                        combination_list,
                    },
                ).map_err(|err| err.to_string())?;
            }
            if !resources && !energy_cells {
                explorer.state = ExplorerState::Idle;
            } else {
                explorer.state = ExplorerState::Surveying {
                    resources,
                    combinations: false,
                    energy_cells,
                    orch_resource,
                    orch_combination: false,
                };
            }
        }
        _ => {
            LogEvent::new(
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessageOrchestratorToExplorer,
                Channel::Warning,
                warning_payload!(
                    "tried to manage supported combination response while not in Idle state\
                    this should never happen\
                    the response will not be processed",
                    "",
                    "manage_supported_combination_response()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            return Err("tried to manage supported combination response while not in Idle state".to_string())
        }
    }
    Ok(())
}
/// this function takes the generated resource from the planet and puts it in the bag of the explorer
/// also if this action was started by the orchestrator it sends back the response
pub fn manage_generate_response(
    explorer: &mut Explorer,
    resource: Option<BasicResource>,
) -> Result<(), String> {
    log_message!(
        ActorType::Planet,
        explorer.planet_id,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessagePlanetToExplorer,
        "generated resource received";
        "resource"=>format!("{:?}", resource)
    );
    match explorer.state {
        ExplorerState::GeneratingResource {
            orchestrator_response,
        } => {
            let mut orc_res = Ok(());
            match resource {
                Some(resource) => {
                    explorer.bag.insert(resource.res_to_generic());
                    if orchestrator_response {
                        orc_res = Ok(());
                    }
                }
                None => {
                    if orchestrator_response {
                        orc_res = Err("Cannot generate resource".to_string());
                    }
                }
            }
            if orchestrator_response {
                log_internal_op!(explorer, "sending GenerateResourceResponse");
                explorer.orchestrator_channels.1.send(
                    ExplorerToOrchestrator::GenerateResourceResponse {
                        explorer_id: explorer.explorer_id,
                        generated: orc_res,
                    },
                ).map_err(|err| err.to_string())?;
            }
            explorer.state = ExplorerState::Idle;
        }
        _ => {
            LogEvent::new(
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessageOrchestratorToExplorer,
                Channel::Warning,
                warning_payload!(
                    "tried to manage generated resource response while not in Idle state\
                    this should never happen\
                    the response will not be processed",
                    "",
                    "manage_generate_response()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            return Err("tried to manage generated resource response while not in Idle state".to_string());
        }
    }
    Ok(())
}
/// this function takes the combined resource from the planet and puts it in the bag of the explorer
/// also if this action was started by the orchestrator it sends back the response
pub fn manage_combine_response(
    explorer: &mut Explorer,
    complex_response: Result<ComplexResource, (String, GenericResource, GenericResource)>,
) -> Result<(), String> {
    log_message!(
        ActorType::Planet,
        explorer.planet_id,
        ActorType::Explorer,
        explorer.explorer_id,
        EventType::MessagePlanetToExplorer,
        "combined resource received";
        "combined resource"=>format!("{:?}", complex_response)
    );
    match explorer.state {
        ExplorerState::CombiningResources {
            orchestrator_response,
        } => {
            let mut orch_res = Ok(());
            match complex_response {
                Ok(complex_resource) => {
                    explorer.bag.insert(complex_resource.res_to_generic());
                    if orchestrator_response {
                        orch_res = Ok(());
                    }
                }
                Err((err, r1, r2)) => {
                    explorer.bag.insert(r1);
                    explorer.bag.insert(r2);
                    if orchestrator_response {
                        orch_res = Err("Cannot combine resource".to_string());
                    }
                }
            }
            if orchestrator_response {
                log_internal_op!(explorer, "sending CombineResourceResponse");
                explorer.orchestrator_channels.1.send(
                    ExplorerToOrchestrator::CombineResourceResponse {
                        explorer_id: explorer.explorer_id,
                        generated: Ok(()),
                    },
                ).map_err(|err| err.to_string())?;
            }
            explorer.state = ExplorerState::Idle;
        }
        _ => {
            LogEvent::new(
                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessageOrchestratorToExplorer,
                Channel::Warning,
                warning_payload!(
                    "tried to manage complex resource response while not in Idle state\
                    this should never happen\
                    the response will not be processed",
                    "",
                    "manage_combine_response()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            )
            .emit();
            return Err("tried to manage complex resource response while not in Idle state".to_string());
        }
    }
    Ok(())
}
