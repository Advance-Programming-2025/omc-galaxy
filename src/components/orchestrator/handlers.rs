use std::time::{Duration, Instant};

use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::utils::ID;
use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use crossbeam_channel::select;
use log::info;
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, debug_println, log_fn_call, log_internal_op, log_message,
    payload, warning_payload,
};

use crate::components::explorer_tommy::BagType;
use crate::{ExplorerStatus, components::orchestrator::Orchestrator, utils::Status};

impl Orchestrator {
    /// Handle the planet messages that are sent through the orchestrator's
    /// communication channels.
    ///
    /// This function serves as an entry point to all the messages that originate
    /// from the planets that need the orchestrator's intervention; no logic is
    /// actually present.
    ///
    /// * `msg` - the message to pass along to other functions
    pub(crate) fn handle_planet_message(
        &mut self,
        msg: PlanetToOrchestrator,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(
            self,
            "handle_planet_message()";
            "message_type"=>format!("{:?}", msg)
        );
        //LOG

        match msg {
            PlanetToOrchestrator::SunrayAck { planet_id } => {
                debug_println!("SunrayAck from: {}", planet_id);

                self.emit_sunray_ack(planet_id);
                //LOG
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "SunrayAck",
                    planet_id
                );
                //LOG
            }
            PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                debug_println!("AsteroidAck from: {}", planet_id);
                //LOG
                log_message!(
                    ActorType::Planet, planet_id,
                    ActorType::Orchestrator, 0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "AsteroidAck",
                    planet_id;
                    "has_rocket"=>rocket.is_some()
                );

                //LOG
                match rocket {
                    Some(_) => {
                        info!("I'm planet {planet_id} and I got an asteroid. Got rocket!");
                    }
                    None => {
                        info!("I'm planet {planet_id} and I got an asteroid. NO rocket!");
                        //If you have the id then surely that planet exist so we can unwrap without worring
                        //TODO it seems fine to me but just to be more precise we could add error handling
                        let sender = &self.planet_channels.get(&planet_id).unwrap().0;

                        //Send KillPlanet message, if it returns Err then the planet it's already killed
                        //TODO we could log this too
                        let _log = sender
                            .send(OrchestratorToPlanet::KillPlanet)
                            .map_err(|_| "Unable to send to planet: {planet_id}");

                        //LOG
                        log_message!(
                            ActorType::Orchestrator, 0u32,
                            ActorType::Planet, planet_id,
                            EventType::MessageOrchestratorToPlanet,
                            "KillPlanet",
                            planet_id;
                            "reason"=>"no rocket to deflect asteroid"
                        );
                        //LOG

                        //Update planet State
                        self.planets_info.update_status(planet_id, Status::Dead);
                        //LOG
                        log_internal_op!(
                            self,
                            "action"=>"planet status updated to Dead",
                            "planet_id"=>planet_id
                        );
                        //LOG
                        //TODO we need to do a check if some explorer is on that planet
                    }
                }
            }
            // PlanetToOrchestrator::IncomingExplorerResponse { planet_id, res }=>{},
            //TODO at this point this functions don't do anything at all
            PlanetToOrchestrator::InternalStateResponse {
                planet_id,
                planet_state,
            } => {
                //LOG
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "InternalStateResponse",
                    planet_id,
                    planet_state,
                );
                //LOG
                self.planets_info
                    .update_from_planet_state(planet_id, planet_state);
            }
            PlanetToOrchestrator::KillPlanetResult { planet_id } => {
                debug_println!("Planet killed: {}", planet_id);
                self.emit_planet_death(planet_id);
                let event = LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "Message"=>"KillPlanetResult",
                        "planet_id"=>planet_id,
                    ),
                );
                event.emit();
            }
            // PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, res }=>{},
            PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                //LOG
                let event = LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"StartPlanetAIResult",
                        "planet_id"=>planet_id
                    ),
                );
                event.emit();
                //LOG
            }
            PlanetToOrchestrator::StopPlanetAIResult { planet_id } => {
                //LOG
                let event = LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"StopPlanetAIResult",
                        "planet_id"=>planet_id
                    ),
                );
                event.emit();
                //LOG
            }
            PlanetToOrchestrator::Stopped { planet_id } => {
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "Stopped",
                    planet_id
                )
            }
            _ => {
                let event = LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::MessagePlanetToOrchestrator,
                    Channel::Warning,
                    warning_payload!(
                        "unhandled planet message",
                        "_",
                        "handle_planet_message()";
                    ),
                );
                event.emit();
            }
        }
        Ok(())
    }

    pub fn handle_explorer_message(
        &mut self,
        msg: ExplorerToOrchestrator<BagType>,
    ) -> Result<(), String> {
        match msg {
            ExplorerToOrchestrator::StartExplorerAIResult { explorer_id } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "StartExplorerAIResult",
                    explorer_id
                );
                //LOG

                if let Some(mut status_map) = self.explorer_status.write().ok() {
                    status_map.insert(explorer_id, Status::Running);
                }

                //LOG
                log_internal_op!(
                    self,
                    "action" => "explorer status updated to Active",
                    "explorer_id" => explorer_id
                );
                //LOG
            }
            ExplorerToOrchestrator::KillExplorerResult { explorer_id } => {
                debug_println!("Explorer killed: {}", explorer_id);

                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "KillExplorerResult",
                    explorer_id
                );
                //LOG

                if let Some(mut status_map) = self.explorer_status.write().ok() {
                    status_map.insert(explorer_id, Status::Dead);
                }

                //LOG
                log_internal_op!(
                    self,
                    "action" => "explorer status updated to Dead",
                    "explorer_id" => explorer_id
                );
                //LOG
            }
            ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "ResetExplorerAIResult",
                    explorer_id
                );
                //LOG

                //LOG
                log_internal_op!(
                    self,
                    "action" => "explorer AI reset completed",
                    "explorer_id" => explorer_id
                );
                //LOG
            }
            ExplorerToOrchestrator::StopExplorerAIResult { explorer_id } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "StopExplorerAIResult",
                    explorer_id
                );
                //LOG

                if let Some(mut status_map) = self.explorer_status.write().ok() {
                    status_map.insert(explorer_id, Status::Paused);
                }

                //LOG
                log_internal_op!(
                    self,
                    "action" => "explorer status updated to Paused",
                    "explorer_id" => explorer_id
                );
                //LOG
            }
            ExplorerToOrchestrator::MovedToPlanetResult {
                explorer_id,
                planet_id,
            } => {
                debug_println!("Explorer {} moved to planet {}", explorer_id, planet_id);

                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "MovedToPlanetResult",
                    explorer_id;
                    "planet_id" => planet_id
                );
                //LOG
                // TODO memorize the position of the explorer? if so, where?
            }
            ExplorerToOrchestrator::CurrentPlanetResult {
                explorer_id,
                planet_id,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "CurrentPlanetResult",
                    explorer_id;
                    "planet_id" => planet_id
                );
                //LOG
            }
            ExplorerToOrchestrator::SupportedResourceResult {
                explorer_id,
                supported_resources,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "SupportedResourceResult",
                    explorer_id;
                    "resources_count" => supported_resources.len()
                );
                //LOG
            }
            ExplorerToOrchestrator::SupportedCombinationResult {
                explorer_id,
                combination_list,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "SupportedCombinationResult",
                    explorer_id;
                    "combinations_count" => combination_list.len()
                );
                //LOG
            }
            ExplorerToOrchestrator::GenerateResourceResponse {
                explorer_id,
                generated,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "GenerateResourceResponse",
                    explorer_id;
                    "success" => generated.is_ok()
                );
                //LOG
            }
            ExplorerToOrchestrator::CombineResourceResponse {
                explorer_id,
                generated,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "CombineResourceResponse",
                    explorer_id;
                    "success" => generated.is_ok()
                );
                //LOG
            }
            ExplorerToOrchestrator::BagContentResponse {
                explorer_id,
                bag_content,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "BagContentResponse",
                    explorer_id;
                    "items_count" => bag_content.len()
                );
                //LOG
            }
            ExplorerToOrchestrator::NeighborsRequest {
                explorer_id,
                current_planet_id,
            } => {
                self.send_neighbours_response(explorer_id, current_planet_id)?;
            }
            ExplorerToOrchestrator::TravelToPlanetRequest {
                explorer_id,
                current_planet_id,
                dst_planet_id,
            } => {
                // verify that the planet exists and that the destination planet is a neighbour
                let is_neighbour = {
                    let guard = self.galaxy_topology.read().unwrap();

                    guard
                        .get(current_planet_id as usize)
                        .and_then(|row| row.get(dst_planet_id as usize))
                        .copied()
                        .unwrap_or(false)
                };

                // if not existing or not a neighbour of the current planet return and Err
                if !is_neighbour {
                    return Err("Planet id not found".to_string());
                }

                // else send the move to planet
                return self.send_move_to_planet(explorer_id, dst_planet_id);
            }
        }
        Ok(())
    }

    /// Handle the planet messages that are sent through the orchestrator's
    /// communication channels.
    ///
    /// This function serves as an entry point to all the messages that need the
    /// orchestrator's intervention; no logic is actually present.
    pub fn handle_game_messages(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "handle_game_messages()");
        let deadline = Instant::now() + Duration::from_millis(100);
        while Instant::now() < deadline {
            select! {
                recv(self.receiver_orch_planet)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(e)=>{
                            //LOG
                            let event=LogEvent::self_directed(
                                Participant::new(ActorType::Orchestrator, 0u32),
                                EventType::InternalOrchestratorAction,
                                Channel::Warning,
                                warning_payload!(
                                    "Cannot receive message from planets",
                                    e,
                                    "handle_game_messages()"
                                )
                            );
                            event.emit();
                            //LOG
                            // TODO not using the "e" in Err(e), which version to use? this one or the one used below for explorer msg?
                            return Err("Cannot receive message from planets".to_string())
                        },
                    };
                    self.handle_planet_message(msg_unwraped)?;
                }
                recv(self.receiver_orch_explorer)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(e)=>{
                            //LOG
                            // TODO
                            //LOG
                            return Err(format!("Cannot receive message from explorers: {}", e));
                        },
                    };
                    self.handle_explorer_message(msg_unwraped)?;
                }
                default=>{

                }
            }
        }

        Ok(())
    }
}
