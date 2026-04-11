use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::utils::ID;
use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use crossbeam_channel::select;
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, debug_println, log_explorer_to_orch, log_fn_call,
    log_internal_op, log_message, log_planet_to_orch, payload, warning_payload,
};
use std::time::{Duration, Instant};

use crate::components::explorer::BagType;
use crate::{components::orchestrator::Orchestrator, log_orch_internal, utils::Status};
pub const TIMEOUT_DURATION: Duration = Duration::from_millis(10);

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
        log_planet_to_orch!(format!("{:?} received", msg), msg.planet_id());
        //LOG

        match msg {
            PlanetToOrchestrator::SunrayAck { planet_id } => {
                debug_println!("SunrayAck from: {}", planet_id);

                self.emit_sunray_ack(planet_id);
            }
            PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                debug_println!("AsteroidAck from: {}", planet_id);

                if let None = rocket {
                    // Skip if the planet is already dead (e.g. a previous AsteroidAck
                    // already triggered its kill before this one was processed)
                    if self.planets_info.get_status(&planet_id) == Status::Dead {
                        log_orch_internal!(format!(
                            "AsteroidAck for already-dead planet {}, skipping",
                            planet_id
                        ));
                        return Ok(());
                    }

                    //If you have the id then surely that planet exists so we can unwrap without worrying
                    let sender = &self.planet_channels.get(&planet_id).ok_or_else(
                        || format!{"No channels found in the orchestrator for planet:{}", planet_id}
                    )?.0;

                    //Send KillPlanet message, if it returns Err then the planet it's already killed
                    sender.send(OrchestratorToPlanet::KillPlanet).map_err(|_| {
                        format!("Unable to send KillPlanet to planet: {}", planet_id)
                    })?;

                    //LOG
                    log_message!(
                        ActorType::Orchestrator, 0u32,
                        ActorType::Planet, planet_id,
                        EventType::MessageOrchestratorToPlanet,
                        "KillPlanet sent",
                        planet_id;
                        "reason"=>"no rocket to deflect asteroid"
                    );
                    //LOG

                    self.destroy_topology_link(planet_id as usize)?;

                    //Update planet State
                    match self.planets_info.update_status(planet_id, Status::Dead) {
                        Ok(_) => {}
                        Err(err) => {
                            log_orch_internal!(format!("planet status not updated: {}", err));
                            return Err(err.to_string());
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
                    //sending explorer kill
                    self.send_kill_to_explorers_on_dying_planet(&planet_id)?;
                }
            }
            PlanetToOrchestrator::InternalStateResponse {
                planet_id,
                planet_state,
            } => {
                self.planets_info
                    .update_from_planet_state(planet_id, planet_state);
            }
            PlanetToOrchestrator::KillPlanetResult { planet_id } => {
                // Guard: if the planet is already dead (e.g. killed via AsteroidAck
                // before this KillPlanetResult arrived), skip redundant processing
                if self.planets_info.get_status(&planet_id) == Status::Dead {
                    log_orch_internal!(format!(
                        "KillPlanetResult for already-dead planet {}, skipping",
                        planet_id
                    ));
                    return Ok(());
                }
                self.destroy_topology_link(planet_id as usize)?;
                self.planets_info.update_status(planet_id, Status::Dead)?;
                self.emit_planet_death(planet_id);

                //LOG
                debug_println!("Planet killed: {}", planet_id);
                LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "Message"=>"Planet killed",
                        "planet_id"=>planet_id,
                    ),
                )
                .emit();
                //killing explorer just in case the KillPlanet message is manually sended
                self.send_kill_to_explorers_on_dying_planet(&planet_id)?;
                //LOG
            }
            // PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, res }=>{},
            PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                if self.planets_info.is_dead(&planet_id) {
                    log_orch_internal!(format!(
                        "planet: {} is already dead, StartPlanetAIResult is ineffective",
                        planet_id
                    ));
                    return Ok(());
                }
                self.planets_info
                    .update_status(planet_id, Status::Running)?;
                //LOG
                LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"Planet AI started",
                        "planet_id"=>planet_id
                    ),
                )
                .emit();
                //LOG
            }
            PlanetToOrchestrator::StopPlanetAIResult { planet_id } => {
                if self.planets_info.is_dead(&planet_id) {
                    log_orch_internal!(format!(
                        "planet: {} is already dead, StopPlanetAIResult is ineffective",
                        planet_id
                    ));
                    return Ok(());
                }
                //LOG
                LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"Planet AI stopped",
                        "planet_id"=>planet_id
                    ),
                )
                .emit();
                //LOG
                self.planets_info.update_status(planet_id, Status::Paused)?;
            }
            PlanetToOrchestrator::Stopped { planet_id: _ } => {}
            PlanetToOrchestrator::IncomingExplorerResponse {
                planet_id,
                explorer_id,
                res,
            } => {
                if let Ok(_) = res {
                    // Guard: if the explorer is already dead, skip processing
                    if self.explorers_info.is_dead(&explorer_id) {
                        log_orch_internal!(format!(
                            "IncomingExplorerResponse for dead explorer {}, skipping",
                            explorer_id
                        ));
                        return Ok(());
                    }

                    let current_planet_id = self
                        .explorers_info
                        .get_current_planet(&explorer_id)
                        .ok_or("could not get explorer planet".to_string())?;

                    //this is safe because we already checked it before
                    let move_to_planet_id = self
                        .explorers_info
                        .get(&explorer_id)
                        .unwrap()
                        .move_to_planet_id;

                    // If move_to_planet_id is -1, no travel was requested for this
                    // explorer (e.g. this is just the initial arrival confirmation).
                    // Skip the destination planet check entirely.
                    if move_to_planet_id >= 0
                        && !self.planets_info.is_running(&(move_to_planet_id as u32))
                    {
                        log_orch_internal!(format!(
                            "IncomingExplorerResponse: destination planet {} is dead, skipping",
                            move_to_planet_id
                        ));
                        let sender = &self
                            .explorer_channels
                            .get(&explorer_id)
                            .ok_or("could not get explorer sender".to_string())?
                            .0;
                        sender
                            .send(OrchestratorToExplorer::MoveToPlanet {
                                sender_to_new_planet: None,
                                planet_id: move_to_planet_id as ID,
                            })
                            .map_err(|err| format!("could not send MoveToPlanet: {:?}", err))?;
                        return Ok(());
                    }

                    // Guard: if the current planet is dead, we cannot send OutgoingExplorerRequest
                    if self.planets_info.is_dead(&current_planet_id) {
                        //not need to send a response to the explorer because it will be killed in moments
                        log_orch_internal!(format!(
                            "IncomingExplorerResponse: current planet {} is dead, skipping",
                            current_planet_id
                        ));
                        return Ok(());
                    }

                    let orch_current_planet_sender =
                        match self.planet_channels.get(&current_planet_id) {
                            Some(sender) => sender,
                            None => {
                                return Err(format!("Planet not found: {}", planet_id));
                            }
                        };

                    if move_to_planet_id >= 0 && (move_to_planet_id as u32) != current_planet_id {
                        match orch_current_planet_sender
                            .0
                            .send(OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id })
                        {
                            Ok(_) => {
                                log_message!(
                                    ActorType::Orchestrator,
                                    0u32,
                                    ActorType::Planet,
                                    current_planet_id,
                                    EventType::MessageOrchestratorToPlanet,
                                    "OutgoingExplorerRequest sended"
                                );
                            }
                            Err(err) => {
                                //todo possible to log this in the main loop
                                LogEvent::new(
                                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                    Some(Participant::new(ActorType::Planet, current_planet_id)),
                                    EventType::MessageOrchestratorToPlanet,
                                    Channel::Warning,
                                    warning_payload!(
                                        "Failed to send OutgoingExplorerRequest",
                                        err,
                                        "handle_planet_msg()";
                                        "msg" => format!("PlanetToOrchestrator::IncomingExplorerResponse {{ {}, {}, {:?} }}",planet_id, explorer_id, res )
                                    )
                                ).emit();

                                return Err(format!("Failed to send explorer request: {}", err));
                            }
                        }
                    } else if (move_to_planet_id as u32) == current_planet_id {
                        //if the explorer has to stay on the same planet

                        let sender = &self
                            .explorer_channels
                            .get(&explorer_id)
                            .ok_or("could not get explorer sender".to_string())?
                            .0;
                        sender
                            .send(OrchestratorToExplorer::MoveToPlanet {
                                sender_to_new_planet: None,
                                planet_id: move_to_planet_id as ID,
                            })
                            .map_err(|err| format!("could not send MoveToPlanet: {:?}", err))?;

                        return Ok(());
                    }
                }
            }
            PlanetToOrchestrator::OutgoingExplorerResponse {
                planet_id,
                explorer_id,
                res,
            } => {
                if let Ok(_) = res {
                    // Guard: if the explorer is already dead, skip sending
                    let dst_planet_id = self
                        .explorers_info
                        .get(&explorer_id)
                        .ok_or("could not get explorer info".to_string())?
                        .move_to_planet_id;
                    let explorer_alive = !self.explorers_info.is_dead(&explorer_id);
                    let dst_planet_alive = !self.planets_info.is_dead(&(dst_planet_id as u32));
                    let current_planet_alive = !self.planets_info.is_dead(&planet_id);
                    //current planet dead so the explorer will be killed
                    if !explorer_alive || !current_planet_alive {
                        log_orch_internal!(format!(
                            "OutgoingExplorerResponse for dead planet {}/explorer {}, skipping",
                            planet_id, explorer_id
                        ));
                        //this unwrap should not panic
                        let sender_dst_planet =
                            &self.planet_channels.get(&(dst_planet_id as u32)).unwrap().0;
                        sender_dst_planet.send(OrchestratorToPlanet::OutgoingExplorerRequest {explorer_id}).map_err(
                            |err| format!("could not send OutgoingExplroerRequest to planet: {}. Err: {:?}", dst_planet_id, err)
                        )?;
                        return Ok(());
                    }
                    //destination planet killed, trying to recover
                    if !dst_planet_alive {
                        self.explorers_info
                            .get_mut(&explorer_id)
                            .unwrap()
                            .move_to_planet_id = planet_id as i32; //updating dst to current planet
                        self.send_incoming_explorer_request(planet_id, explorer_id)?;
                        log_orch_internal!(format!(
                            "OutgoingExplorerResponse with destination planet killed {}, trying to recover",
                            dst_planet_id
                        ));
                        return Ok(());
                    }

                    let dst_planet_id = match self.explorers_info.get(&explorer_id) {
                        Some(explorer_info) => explorer_info.move_to_planet_id,
                        None => {
                            return Err(format!("Explorer not found: {}", explorer_id));
                        }
                    };
                    if let Err(err) = self.send_move_to_planet(explorer_id, dst_planet_id as u32) {
                        return Err(format!("Failed to send explorer request: {}", err));
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
        log_internal_op!(self, "explorer message received");
        log_explorer_to_orch!(format!("{:?} received", msg), msg.explorer_id());

        // Guard: skip messages from dead explorers (stale messages that were
        // queued before the explorer was killed). We still allow
        // KillExplorerResult through so the status update is recorded.
        let explorer_id_for_guard = msg.explorer_id();
        if !matches!(msg, ExplorerToOrchestrator::KillExplorerResult { .. }) {
            if self.explorers_info.is_dead(&explorer_id_for_guard) {
                log_orch_internal!(format!(
                    "Ignoring message from dead explorer {}, skipping",
                    explorer_id_for_guard
                ));
                return Ok(());
            }
        }

        match msg {
            ExplorerToOrchestrator::StartExplorerAIResult { explorer_id } => {
                //LOG
                LogEvent::new(
                    Some(Participant::new(ActorType::Explorer, explorer_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessageExplorerToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=> "Explorer AI started",
                        "explorer_id"=>explorer_id,
                    ),
                )
                .emit();
                //LOG

                self.explorers_info
                    .insert_status(explorer_id, Status::Running);
                if self.explorers_info.get(&explorer_id).is_none() {
                    self.send_current_planet_request(explorer_id)?; //todo is this necessary?
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

                self.explorers_info.insert_status(explorer_id, Status::Dead);

                //LOG
                LogEvent::new(
                    Some(Participant::new(ActorType::Explorer, explorer_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessageExplorerToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=> "Explorer killed",
                        "explorer_id"=>explorer_id,
                    ),
                )
                .emit();
                log_internal_op!(
                    self,
                    "action" => "explorer status updated to Dead",
                    "explorer_id" => explorer_id
                );
                //LOG
            }
            ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id } => {
                //LOG
                LogEvent::new(
                    Some(Participant::new(ActorType::Explorer, explorer_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessageExplorerToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=> "Explorer AI reset",
                        "explorer_id"=>explorer_id,
                    ),
                )
                .emit();
                //LOG
                //the ai is started if it was in manual mode
                self.explorers_info
                    .insert_status(explorer_id, Status::Running);
                if self.explorers_info.get(&explorer_id).is_none() {
                    self.send_current_planet_request(explorer_id)?; //todo is this necessary
                }

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
                LogEvent::new(
                    Some(Participant::new(ActorType::Explorer, explorer_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessageExplorerToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=> "Explorer AI stopped ",
                        "explorer_id"=>explorer_id,
                    ),
                )
                .emit();
                //LOG
                self.explorers_info
                    .insert_status(explorer_id, Status::Paused);
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

                self.explorers_info
                    .update_current_planet(explorer_id, planet_id);

                self.emit_explorer_move(explorer_id, planet_id);
            }
            ExplorerToOrchestrator::CurrentPlanetResult {
                explorer_id,
                planet_id,
            } => {
                self.explorers_info
                    .update_current_planet(explorer_id, planet_id);
            }
            ExplorerToOrchestrator::SupportedResourceResult {
                explorer_id,
                supported_resources,
            } => {
                //dobbiamo aggiornare le info dei pianeti salvarcele una volta per poterle riusare a piacimento
                let planet_id = self
                    .explorers_info
                    .get_current_planet(&explorer_id)
                    .ok_or("could not get explorer planet".to_string())?;
                self.planets_info
                    .update_supported_resources(planet_id, supported_resources)?;
            }
            ExplorerToOrchestrator::SupportedCombinationResult {
                explorer_id,
                combination_list,
            } => {
                //dobbiamo aggiornare le info dei pianeti salvarcele una volta per poterle riusare a piacimento
                let planet_id = self
                    .explorers_info
                    .get_current_planet(&explorer_id)
                    .ok_or("could not get explorer planet".to_string())?;
                self.planets_info
                    .update_supported_combination(planet_id, combination_list)?;
            }
            ExplorerToOrchestrator::GenerateResourceResponse {
                explorer_id,
                generated,
            } => {
                if generated.is_ok() {
                    self.send_bag_content_request(explorer_id)?;
                }
            }
            ExplorerToOrchestrator::CombineResourceResponse {
                explorer_id,
                generated,
            } => {
                if generated.is_ok() {
                    self.send_bag_content_request(explorer_id)?;
                }
            }
            ExplorerToOrchestrator::BagContentResponse {
                explorer_id,
                bag_content,
            } => {
                self.explorers_info.update_bag(explorer_id, bag_content);
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
                //un explorer può andare su un pianeta che è stoppato? risposta: no

                // Check if dst_planet_id exists in the galaxy at all
                if self.galaxy_lookup.get(&dst_planet_id).is_none() {
                    log_orch_internal!(format!(
                        "TravelToPlanetRequest: dst_planet_id {} does not exist, rejecting",
                        dst_planet_id
                    ));
                    if let Some(ch) = self.explorer_channels.get(&explorer_id) {
                        let _ = ch.0.send(OrchestratorToExplorer::MoveToPlanet {
                            sender_to_new_planet: None,
                            planet_id: dst_planet_id,
                        });
                    }
                    return Ok(());
                }

                // verify that the destination planet is a neighbour
                let is_neighbour = {
                    // Translate real planet_ids to matrix indices via the lookup table
                    let current_idx = self
                        .galaxy_lookup
                        .get(&current_planet_id)
                        .map(|(idx, _)| *idx as usize);
                    let dst_idx = self
                        .galaxy_lookup
                        .get(&dst_planet_id)
                        .map(|(idx, _)| *idx as usize);

                    match (current_idx, dst_idx) {
                        (Some(ci), Some(di)) => self
                            .galaxy_topology
                            .get(ci)
                            .and_then(|row| row.get(di))
                            .copied()
                            .unwrap_or(false),
                        _ => false,
                    }
                };

                // if not existing or not a neighbour of the current planet, reject
                if !is_neighbour
                    || self.planets_info.get_status(&dst_planet_id) != Status::Running
                    || self.planets_info.get_status(&current_planet_id) != Status::Running
                {
                    // Try to notify the explorer that the move was rejected.
                    // If the explorer is already dead its channel is disconnected,
                    // so we just log and move on instead of propagating the error.
                    if let Some(ch) = self.explorer_channels.get(&explorer_id) {
                        let _ = ch.0.send(OrchestratorToExplorer::MoveToPlanet {
                            sender_to_new_planet: None,
                            planet_id: dst_planet_id,
                        });
                    } else {
                        return Err(format!(
                            "could not get explorer channel for {}",
                            explorer_id
                        ));
                    }
                    return Ok(());
                }

                //updating move_to_planet_id
                if self.explorers_info.get(&explorer_id).is_some() {
                    log_internal_op!(self, "updated move_to_planet_id");
                }
                match self.explorers_info.get_mut(&explorer_id) {
                    Some(explorer_info) => {
                        explorer_info.move_to_planet_id = dst_planet_id as i32;
                    }
                    None => {
                        return Err(format!("Explorer {} not found", explorer_id));
                    }
                }
                self.send_incoming_explorer_request(dst_planet_id, explorer_id)?
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
        let deadline = Instant::now() + TIMEOUT_DURATION;
        while Instant::now() < deadline {
            select! {
                recv(self.receiver_orch_planet)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(e)=>{
                            //LOG
                            LogEvent::self_directed(
                                Participant::new(ActorType::Orchestrator, 0u32),
                                EventType::InternalOrchestratorAction,
                                Channel::Warning,
                                warning_payload!(
                                    "Cannot receive message from planets",
                                    e,
                                    "handle_game_messages()"
                                )
                            ).emit();
                            //LOG
                            return Err(format!{"Cannot receive message from planets: {}", e})
                        },
                    };
                    let msg_string=format!("{:?}", msg_unwraped);
                    if let Err(err)=self.handle_planet_message(msg_unwraped){
                            //LOG
                            LogEvent::self_directed(
                                Participant::new(ActorType::Orchestrator, 0u32),
                                EventType::InternalOrchestratorAction,
                                Channel::Warning,
                                warning_payload!(
                                    format!("A handler returned a error while handling the planet msg: {:?}", msg_string),
                                    err,
                                    "handle_game_messages()"
                                )
                            ).emit();
                            //LOG
                    }
                }
                recv(self.receiver_orch_explorer)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(e)=>{
                            return Err(format!("Cannot receive message from explorers: {}", e));
                        },
                    };
                    let msg_string=format!("{:?}", msg_unwraped);
                    if let Err(err)=self.handle_explorer_message(msg_unwraped){
                            //LOG
                            LogEvent::self_directed(
                                Participant::new(ActorType::Orchestrator, 0u32),
                                EventType::InternalOrchestratorAction,
                                Channel::Warning,
                                warning_payload!(
                                    format!("A handler returned a error while handling the explorer msg: {:?}", msg_string),
                                    err,
                                    "handle_game_messages()"
                                )
                            ).emit();
                            //LOG
                    }
                }
                default=>{

                }
            }
        }

        Ok(())
    }
    fn send_kill_to_explorers_on_dying_planet(&mut self, planet_id: &ID) -> Result<(), String> {
        log_fn_call!(self, "send_kill_to_explorers_on_dying_planet()", planet_id);
        for i in self
            .explorers_info
            .iter()
            .filter(|x| x.1.current_planet_id == *planet_id && x.1.status != Status::Dead)
        {
            match self
                .explorer_channels
                .get(i.0)
                .unwrap()
                .0
                .send(OrchestratorToExplorer::KillExplorer)
            {
                Ok(_) => {
                    log_message!(
                        ActorType::Orchestrator,
                        0u32,
                        ActorType::Explorer,
                        *i.0,
                        EventType::MessageOrchestratorToExplorer,
                        "KillExplorer sended"
                    );
                }
                Err(_err) => {
                    // The explorer's channel is already disconnected (thread
                    // exited). This is expected during race conditions when
                    // multiple kill paths converge. Just log and continue.
                    log_orch_internal!(format!(
                        "send_kill_to_explorers_on_dying_planet: explorer {} channel disconnected, skipping",
                        i.0
                    ));
                }
            }
        }
        Ok(())
    }
}
