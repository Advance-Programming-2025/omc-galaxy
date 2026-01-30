use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, debug_println, log_fn_call, log_internal_op, log_message,
    payload, warning_payload,
};
use rand::{Rng, seq::IteratorRandom};

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
    /// non deterministic and might never return in case one of the channels just
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

    /// Global start function, starts all of the planets' AIs; wrapper on
    /// [`start_all_planet_ais`](`Self::start_all_planet_ais`).
    ///
    /// Returns Err if any of the planets fail to start.
    pub fn start_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "start_all()");
        //LOG
        self.start_all_planet_ais()?;
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

        // Pick a random planet from the HashMap with the choose method
        let (planet_id, (orch_tx, _expl_tx)) = match self.planet_channels.iter().choose(&mut rng) {
            Some((id, chans)) => (*id, chans.clone()),
            None => return Ok(()), // REVIEW: is this correct or a silent fail?
        };

        if rng.random_bool(0.5) {
            self.send_asteroid(planet_id, &orch_tx)?;
        } else {
            self.send_sunray(planet_id, &orch_tx)?;
        }

        Ok(())
    }
}
