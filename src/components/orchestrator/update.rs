use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, Sender, debug_println, log_fn_call, log_internal_op, log_message, payload, warning_payload
};
use rand::{Rng, seq::{IndexedRandom, IteratorRandom}};

use crate::{components::orchestrator::Orchestrator, utils::Status};

impl Orchestrator {
    /// Removes the link between two planets if one of them explodes.
    ///
    /// Returns Err if the given indexes are out of bounds, Ok otherwise;
    /// it does NOT currently check wether the link was already set to false beforehand
    ///
    /// * `planet_one_pos` - Position of the first planet in the matrix. Must be a valid index
    /// * `planet_two_pos` - Position of the second planet in the matrix. Must be a valid index
    pub(crate) fn destroy_topology_link(
        &mut self,
        planet_one_pos: usize,
        planet_two_pos: usize,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(
            self,
            "destroy_topology_link()",
            planet_one_pos,
            planet_two_pos,
        );
        //LOG

        match self.galaxy_topology.write() {
            Ok(mut gtop) => {
                let gtop_len = gtop.len();
                if planet_one_pos < gtop_len && planet_two_pos < gtop_len {
                    gtop[planet_one_pos][planet_two_pos] = false;
                    gtop[planet_two_pos][planet_one_pos] = false;
                    //LOG
                    log_internal_op!(
                        self,
                        "action"=>"adj link destroyed",
                        "updated topology"=>format!("{:?}",gtop),
                    );
                    //LOG
                    drop(gtop);
                    Ok(())
                } else {
                    //LOG
                    let event = LogEvent::self_directed(
                        Participant::new(ActorType::Orchestrator, 0u32),
                        EventType::InternalOrchestratorAction,
                        Channel::Warning,
                        warning_payload!(
                            format!(
                                "One of the indexes is out of bounds. upper bound: {}",
                                gtop_len - 1
                            ),
                            "_",
                            "destroy_topology_link()",
                            planet_one_pos,
                            planet_two_pos
                        ),
                    );
                    event.emit();
                    //LOG
                    Err("index out of bounds (too large)".to_string())
                }
            }
            Err(e) => {
                //LOG
                let event = LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::InternalOrchestratorAction,
                    Channel::Warning,
                    warning_payload!(
                        "ERROR galaxy topology lock failed.",
                        e,
                        "destroy_topology_link()",
                        planet_one_pos,
                        planet_two_pos
                    ),
                );
                event.emit();
                //LOG
                debug_println!("RwLock failed for destroy_topology_link");
                Err(e.to_string())
            }
        }
    }

    /// Starts the AI of every planet.
    ///
    /// Goes through every PlanetToOrchestrator channel and sends the `StartPlanetAI`
    /// message. As of version 0.1 of the project, the execution of this function is
    /// non-deterministic and might never return in case one of the channels just
    /// hangs forever.
    ///
    /// Returns Err if any of the communication channels are inaccessible.
    pub(crate) fn start_all_planet_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "start_all_planet_ais()");
        //LOG

        for (_id, (from_orch, _)) in &self.planet_channels {
            from_orch
                .try_send(OrchestratorToPlanet::StartPlanetAI)
                .map_err(|_| "Cannot send message to {_id}".to_string())?;

            //LOG
            log_message!(
                ActorType::Orchestrator, 0u32,
                ActorType::Planet, *_id,
                EventType::MessageOrchestratorToPlanet,
                "StartPlanetAI";
                "planet_id"=>_id
            );
            //LOG
        }

        let mut count = 0;
        //TODO REVIEW is it possible that this loop could block forever the game?
        loop {
            if count == self.planet_channels.len() {
                //LOG
                log_internal_op!(
                    self,
                    "action"=>"all planets started",
                    "count"=>count
                );
                //LOG
                break;
            }
            let receive_channel = self
                .receiver_orch_planet
                .recv()
                .map_err(|_| "Cannot receive message from planets".to_string())?;
            match receive_channel {
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    debug_println!("Started Planet AI: {}", planet_id);

                    //LOG
                    let event = LogEvent::new(
                        Some(Participant::new(ActorType::Planet, planet_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessagePlanetToOrchestrator,
                        LOG_ACTORS_ACTIVITY,
                        payload!(
                            "message"=>"StartPlanetAIResult",
                            "planet_id"=>planet_id,
                            "status"=>"Running"
                        ),
                    );
                    event.emit();
                    //LOG
                    //TODO handle this result
                    self.planets_info.update_status(planet_id, Status::Running);
                    count += 1;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Starts the AI of every explorer.
    ///
    /// Goes through every OrchestratorToExplorer channel and sends the `StartExplorerAI`
    ///
    /// Returns Err if any of the communication channels are inaccessible.
    pub(crate) fn start_all_explorer_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "start_all_explorer_ais()");
        //LOG

        for (_id, (from_orch, _)) in &self.explorer_channels {
            from_orch
                .try_send(OrchestratorToExplorer::StartExplorerAI)
                .map_err(|_| format!("Cannot send message to explorer {}", _id))?;

            //LOG
            log_message!(
                ActorType::Orchestrator, 0u32,
                ActorType::Explorer, *_id,
                EventType::MessageOrchestratorToExplorer,
                "StartExplorerAI";
                "explorer_id"=>_id
            );
            //LOG
        }

        let mut count = 0;
        loop {
            if count == self.explorer_channels.len() {
                //LOG
                log_internal_op!(
                    self,
                    "action"=>"all explorers started",
                    "count"=>count
                );
                //LOG
                break;
            }

            let receive_channel = self
                .receiver_orch_explorer
                .recv()
                .map_err(|_| "Cannot receive message from explorers".to_string())?;

            match receive_channel {
                ExplorerToOrchestrator::StartExplorerAIResult { explorer_id } => {
                    debug_println!("Started Explorer AI: {}", explorer_id);

                    //LOG
                    let event = LogEvent::new(
                        Some(Participant::new(ActorType::Explorer, explorer_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessageExplorerToOrchestrator,
                        LOG_ACTORS_ACTIVITY,
                        payload!(
                            "message"=>"StartExplorerAIResult",
                            "explorer_id"=>explorer_id,
                            "status"=>"Running"
                        ),
                    );
                    event.emit();
                    //LOG

                    self.explorers_info.insert_status(explorer_id, Status::Running);
                    count += 1;
                }
                _ => {
                    // ignores other events
                }
            }
        }

        Ok(())
    }

    /// Global start function, starts all of the planets' AIs; wrapper on
    /// [`start_all_planet_ais`](`Self::start_all_planet_ais`).
    ///
    /// Returns Err if any of the planets fail to start.
    pub fn start_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "start_all()");
        //LOG
        self.start_all_planet_ais()?;
        self.start_all_explorer_ais()?;
        //LOG
        log_internal_op!(
            self,
            "action"=>"all systems started",
            "status"=>"success"
        );
        //LOG
        Ok(())
    }

    /// Global stop function.
    ///
    /// The function is yet to be implemented, and WILL panic no matter what.
    pub(crate) fn stop_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "stop_all()");
        //LOG
        //TODO
        //LOG
        log_internal_op!(
            self,
            "action"=>"stop_all requested",
            "status"=>"TODO - not implemented" //TODO change this message
        );
        //LOG
        todo!();
        //Ok(())
    }

    pub fn choose_random_action(&mut self) -> Result<(), String> {
        let mut rng = rand::rng();
        let living_things = self.planets_info.get_list_id_alive();

        let &rand_id = match living_things.choose(&mut rng) {
            Some(num) => num,
            None => return Ok(()),
        };

        
        let mut params: Option<(u32,Sender<OrchestratorToPlanet>)> = None;

        for (&id, channels) in self.planet_channels.iter() {
            if id == rand_id {
                //cloning a sender is not a deep copy, safe
                params = Some((id, channels.0.clone()));
            }
        }

        if let Some((id, channel)) = params {
            // chooses whether to do anything at all (probably will)
            if rng.random_bool(0.8){
                // chooses between sunray or asteroid
                if rng.random_bool(0.5) {
                    self.send_asteroid(id, &channel)?;
                } else {
                    self.send_sunray(id, &channel)?;
                }
            }
        }

        Ok(())
    }
}
