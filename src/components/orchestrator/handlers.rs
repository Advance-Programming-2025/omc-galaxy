use std::time::{Duration, Instant};

use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use common_game::logging::EventType::MessageOrchestratorToExplorer;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{select, SendError};
use log::info;
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, debug_println, log_fn_call, log_internal_op, log_message,
    payload, warning_payload,
};

use crate::{components::orchestrator::{Orchestrator}, utils::Status, ExplorerStatus};
use crate::components::explorer::BagType;
use crate::utils::ExplorerInfoMap;

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
                        match self.planets_info.update_status(planet_id, Status::Dead){
                            Ok(_) => {}
                            Err(err) => {
                                //todo logs
                                debug_println!("planet status not updated: {}", err)
                            }
                        }
                        //LOG
                        log_internal_op!(
                            self,
                            "action"=>"planet status updated to Dead",
                            "planet_id"=>planet_id,
                            "planet status"=> format!("{:?}",self.planets_info.get_status(&planet_id))
                        );
                        //LOG
                        //TODO we need to do a check if some explorer is on that planet
                    }
                }
            }
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
                //todo send kill to every explorer on the planet
                self.emit_planet_death(planet_id);
                //LOG
                debug_println!("Planet killed: {}", planet_id);
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
                //LOG
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
            PlanetToOrchestrator::IncomingExplorerResponse {planet_id, explorer_id, res }=>{
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "PlanetToOrchestrator::IncomingExplorerResponse";
                    "planet_id"=>planet_id,
                    "explorer_id" => explorer_id,
                    "Result"=> format!("{:?}", res)
                );
                match res{
                    Ok(_) => {
                        let current_planet_id=self.explorers_info.get_current_planet(&explorer_id);
                        let orch_current_planet_sender=match self.planet_channels.get(&current_planet_id){
                            Some(sender) => sender,
                            None=>{
                                return Err(format!("Planet not found: {}", planet_id));
                            }
                        };
                        //this is safe because we already checked it before
                        let move_to_planet_id=self.explorers_info.get(&explorer_id).unwrap().move_to_planet_id;
                        if move_to_planet_id >=0 {
                            match orch_current_planet_sender.0.send(OrchestratorToPlanet::OutgoingExplorerRequest {
                                explorer_id,
                            }) {
                                Ok(_) => {}
                                Err(err) => {
                                    //todo logs
                                    return Err(format!("Failed to send explorer request: {}", err));
                                }
                            }
                        }
                    }
                    Err(err) => {
                        //todo logs
                    }
                }
            }
            PlanetToOrchestrator::OutgoingExplorerResponse {planet_id, explorer_id, res}=>{
                match res {
                    Ok(_) => {
                        let dst_planet_id=match self.explorers_info.get(&explorer_id){
                            Some(explorer_info) => explorer_info.move_to_planet_id,
                            None=>{
                                //todo logs
                                return Err(format!("Planet not found: {}", planet_id));
                            }
                        };
                        match self.send_move_to_planet(explorer_id, dst_planet_id as u32){
                            Ok(_) => {}
                            Err(err)=>{
                                //todo logs
                                return Err(format!("Failed to send explorer request: {}", err));
                            }
                        }
                    }
                    Err(err) => {
                        //todo logs
                    }
                }
            }
        }
        Ok(())
    }

    pub fn handle_explorer_message(
        &mut self,
        msg: ExplorerToOrchestrator<BagType>,
    ) -> Result<(), String> {
        log_internal_op!(
            self,
            "explorer message received"
        );
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

                self.explorers_info.insert_status(explorer_id, Status::Running);
                if self.explorers_info.get(&explorer_id).is_none() {
                    self.send_current_planet_request(explorer_id)?;
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

                self.explorers_info.insert_status(explorer_id, Status::Dead);

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

                self.explorers_info.insert_status(explorer_id, Status::Paused);
                if self.explorers_info.get(&explorer_id).is_none() {
                    self.send_current_planet_request(explorer_id)?;
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

                self.explorers_info.update_current_planet(explorer_id, planet_id);
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

                self.explorers_info.update_current_planet(explorer_id, planet_id);
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

                self.send_bag_content_request(explorer_id)?;
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

                self.send_bag_content_request(explorer_id)?;
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

                self.explorers_info.update_bag(explorer_id, bag_content);
            }
            ExplorerToOrchestrator::NeighborsRequest { explorer_id, current_planet_id } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "NeighborsRequest",
                    explorer_id;
                    "current_planet_id" => current_planet_id
                );
                //LOG
                self.send_neighbours_response(explorer_id, current_planet_id)?;
            }
            ExplorerToOrchestrator::TravelToPlanetRequest { //todo nel caso non sia possibile muoversi la funzione
                explorer_id,                            //todo deve mandare un MoveToPlanet con il sender None
                current_planet_id,
                dst_planet_id,
            } => {
                //LOG
                log_message!(
                    ActorType::Explorer,
                    explorer_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessageExplorerToOrchestrator,
                    "TravelToPlanetRequest",
                    explorer_id;
                    "current_planet_id" => current_planet_id,
                    "dst_planet_id" => dst_planet_id,
                );
                //LOG
                //todo un explorer può andare su un pianeta che è stoppato?

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
                //todo questa sarebbe da rimuovere in favore di un aggiornamento corretto della topologia
                if !is_neighbour || self.planets_info.get_status(&dst_planet_id) !=Status::Running{
                    match self.explorer_channels.get(&explorer_id).unwrap().0.send(
                        OrchestratorToExplorer::MoveToPlanet {
                            sender_to_new_planet: None,
                            planet_id: dst_planet_id,
                        }
                    ){
                        Ok(_) => {}
                        Err(_) => {
                            //todo logs
                        }
                    }
                    return Err("Planet id not found".to_string());
                }
                //todo add incomingexplorerRequest and outgoingexplorerrequest
                //updating move_to_planet_id
                match self.explorers_info.get_mut(&explorer_id){
                    Some(explorer_info)=>{
                        explorer_info.move_to_planet_id=dst_planet_id as i32;
                    }
                    None=>{
                        //todo logs
                        return Err(format!("Explorer {} not found", explorer_id));
                    }
                }
                match self.send_incoming_explorer_request(
                    dst_planet_id,
                    explorer_id,
                ){
                    Ok(_) => {}
                    Err(e) => {
                        //todo logs
                    }
                }

                // else send the move to planet
                //return self.send_move_to_planet(explorer_id, dst_planet_id);
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
        let deadline = Instant::now() + Duration::from_millis(10);
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
