use crate::{components::orchestrator::Orchestrator, utils::Status};
use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, Sender, debug_println, log_fn_call, log_internal_op,
    log_message, payload, warning_payload,
};
use rand::{Rng, random, seq::IndexedRandom};
use std::collections::HashSet;
use std::time::Duration;

impl Orchestrator {
    /// Removes the link between two planets if one of them explodes.
    ///
    /// Returns Err if the given indexes are out of bounds, Ok otherwise;
    /// it does NOT currently check wether the link was already set to false beforehand.
    /// The function uses CONTIGUOUS indexes; you can go from real to contiguous indexes
    /// using the galaxy lookup hashmap.
    ///
    /// * `dead_planet_pos` - Position of the dead planet in the matrix. Must be a valid index
    pub fn destroy_topology_link(&mut self, dead_planet_id: usize) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "destroy_topology_link()", dead_planet_id,);
        //LOG
        let dead_planet_pos = self
            .galaxy_lookup
            .get(&(dead_planet_id as u32))
            .map(|(i, _)| *i as usize)
            .ok_or_else(|| format!("planet {} not in lookup", dead_planet_id))?;
        let gtop_len = self.galaxy_topology.len();
        if dead_planet_pos < gtop_len {
            for i in 0..gtop_len {
                self.galaxy_topology[dead_planet_pos][i] = false;
                self.galaxy_topology[i][dead_planet_pos] = false;
            }
            //LOG
            log_internal_op!(
                self,
                "action"=>"adj link destroyed",
                "updated topology"=>format!("{:?}",self.galaxy_topology),
            );
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
                        gtop_len.saturating_sub(1)
                    ),
                    "_",
                    "destroy_topology_link()",
                    dead_planet_pos
                ),
            );
            event.emit();
            //LOG
            Err("index out of bounds (too large)".to_string())
        }
    }

    /// Starts the AI of every planet.
    ///
    /// Goes through every PlanetToOrchestrator channel and sends the `StartPlanetAI`
    /// message. Each planet has 1 second to respond. If a planet does not respond
    /// within 1 second, the message is re-sent once. If it still does not respond
    /// after the second attempt, an error is returned listing the unresponsive planets.
    ///
    /// Returns Err if any of the communication channels are inaccessible or if any
    /// planet fails to respond after a retry.
    pub(crate) fn start_all_planet_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "start_all_planet_ais()");
        //LOG

        // Collect all planet ids that we need to hear back from
        let mut pending_planets: HashSet<u32> = HashSet::new();

        for (id, (from_orch, _)) in &self.planet_channels {
            if !self.planets_info.is_dead(id) {
                from_orch
                    .try_send(OrchestratorToPlanet::StartPlanetAI)
                    .map_err(|_| format!("Cannot send message to {id}"))?;

                pending_planets.insert(*id);
            }

            //LOG
            log_message!(
                ActorType::Orchestrator, 0u32,
                ActorType::Planet, *id,
                EventType::MessageOrchestratorToPlanet,
                "StartPlanetAI";
                "planet_id"=>id
            );
            //LOG
        }

        let timeout = Duration::from_secs(1);

        // First attempt: wait for responses with a 1-second timeout
        loop {
            if pending_planets.is_empty() {
                //LOG
                log_internal_op!(
                    self,
                    "action"=>"all planets started",
                    "count"=>self.planet_channels.len()
                );
                //LOG
                return Ok(());
            }
            match self.receiver_orch_planet.recv_timeout(timeout) {
                Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => {
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
                    self.planets_info
                        .update_status(planet_id, Status::Running)?;
                    pending_planets.remove(&planet_id);
                }
                Ok(_) => {}
                Err(_) => {
                    // Timeout: some planets did not respond in time, break to retry
                    break;
                }
            }
        }

        // Retry: re-send StartPlanetAI to planets that haven't responded
        let retry_planets: Vec<u32> = pending_planets.iter().copied().collect();
        for planet_id in &retry_planets {
            if let Some((from_orch, _)) = self.planet_channels.get(planet_id) {
                let _ = from_orch.try_send(OrchestratorToPlanet::StartPlanetAI);

                //LOG
                log_message!(
                    ActorType::Orchestrator, 0u32,
                    ActorType::Planet, *planet_id,
                    EventType::MessageOrchestratorToPlanet,
                    "StartPlanetAI (retry)";
                    "planet_id"=>planet_id
                );
                //LOG
            }
        }

        // Second attempt: wait again with a 1-second timeout
        loop {
            if pending_planets.is_empty() {
                //LOG
                log_internal_op!(
                    self,
                    "action"=>"all planets started",
                    "count"=>self.planet_channels.len()
                );
                //LOG
                return Ok(());
            }
            match self.receiver_orch_planet.recv_timeout(timeout) {
                Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => {
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
                    self.planets_info
                        .update_status(planet_id, Status::Running)?;
                    pending_planets.remove(&planet_id);
                }
                Ok(_) => {}
                Err(_) => {
                    // Timeout again: these planets are unresponsive
                    let unresponsive: Vec<String> =
                        pending_planets.iter().map(|id| id.to_string()).collect();
                    return Err(format!(
                        "Planets failed to respond after retry: [{}]",
                        unresponsive.join(", ")
                    ));
                }
            }
        }
    }

    /// Stops the AI of every planet.
    ///
    /// Goes through every PlanetToOrchestrator channel and sends the `StopPlanetAI`
    /// message.
    /// Returns Err if any of the communication channels are inaccessible.
    pub(crate) fn stop_all_planet_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "stop_all_planet_ais()");
        //LOG

        for (id, (from_orch, _)) in &self.planet_channels {
            if !self.planets_info.is_dead(id) {
                from_orch
                    .try_send(OrchestratorToPlanet::StopPlanetAI)
                    .map_err(|_| format!("Cannot send message to {id}"))?;

                //LOG
                log_message!(
                    ActorType::Orchestrator, 0u32,
                    ActorType::Planet, *id,
                    EventType::MessageOrchestratorToPlanet,
                    "StopPlanetAI";
                    "planet_id"=>id
                );
                //LOG
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

        for (id, (from_orch, _)) in &self.explorer_channels {
            if !self.explorers_info.is_dead(id) {
                from_orch
                    .try_send(OrchestratorToExplorer::StartExplorerAI)
                    .map_err(|_| format!("Cannot send message to explorer {}", id))?;

                //LOG
                log_message!(
                    ActorType::Orchestrator, 0u32,
                    ActorType::Explorer, *id,
                    EventType::MessageOrchestratorToExplorer,
                    "StartExplorerAI";
                    "explorer_id"=>id
                );
            }

            //LOG
        }
        //
        // let mut count = 0;
        // loop {
        //     if count == self.explorer_channels.len() {
        //         //LOG
        //         log_internal_op!(
        //             self,
        //             "action"=>"all explorers started",
        //             "count"=>count
        //         );
        //         //LOG
        //         break;
        //     }
        //
        //     let receive_channel = self
        //         .receiver_orch_explorer
        //         .recv()
        //         .map_err(|_| "Cannot receive message from explorers".to_string())?;
        //
        //     match receive_channel {
        //         ExplorerToOrchestrator::StartExplorerAIResult { explorer_id } => {
        //             debug_println!("Started Explorer AI: {}", explorer_id);
        //             //println!("Started Explorer AI: {}", explorer_id);
        //             //LOG
        //             let event = LogEvent::new(
        //                 Some(Participant::new(ActorType::Explorer, explorer_id)),
        //                 Some(Participant::new(ActorType::Orchestrator, 0u32)),
        //                 EventType::MessageExplorerToOrchestrator,
        //                 LOG_ACTORS_ACTIVITY,
        //                 payload!(
        //                     "message"=>"StartExplorerAIResult",
        //                     "explorer_id"=>explorer_id,
        //                     "status"=>"Running"
        //                 ),
        //             );
        //             event.emit();
        //             //LOG
        //
        //             self.explorers_info
        //                 .insert_status(explorer_id, Status::Running);
        //             count += 1;
        //         }
        //         msg => {
        //             self.handle_explorer_message(msg);
        //             //println!("ignoring explorer messages");
        //             debug_println!("ignoring explorer messages")
        //             // ignores other events
        //         }
        //     }
        // }

        Ok(())
    }

    /// Stop the AI of every explorer.
    ///
    /// Goes through every OrchestratorToExplorer channel and sends the `StopExplorerAI`
    ///
    /// Returns Err if any of the communication channels are inaccessible.
    pub(crate) fn stop_all_explorer_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "stop_all_explorer_ais()");
        //LOG

        for (id, (from_orch, _)) in &self.explorer_channels {
            if !self.explorers_info.is_dead(id) {
                from_orch
                    .try_send(OrchestratorToExplorer::StopExplorerAI)
                    .map_err(|_| format!("Cannot send message to explorer {}", id))?;

                //LOG
                log_message!(
                    ActorType::Orchestrator, 0u32,
                    ActorType::Explorer, *id,
                    EventType::MessageOrchestratorToExplorer,
                    "StopExplorerAI";
                    "explorer_id"=>id
                );
                //LOG
            }
        }

        Ok(())
    }

    /// Global start function, starts all of the planets'
    /// and explorer's AIs.
    ///
    /// The function performs the following steps in order:
    /// 1. Starts all planet AIs
    /// 2. Waits 20ms for the planets to be fully ready
    /// 3. Spawns all explorers (mattia and tommy) on their respective planets
    /// 4. Starts all explorer AIs
    ///
    /// Each element of `mattia_explorers` and `tommy_explorers` is a pair
    /// `(explorer_id, planet_id)` indicating which explorer to create and
    /// on which planet it should be spawned.
    ///
    /// Returns Err if any of the planets or explorers fail to start.
    pub fn start_all(
        &mut self,
        mattia_explorers: &[(u32, u32)],
        tommy_explorers: &[(u32, u32)],
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "start_all()");
        //LOG

        // 1. Start all planet AIs
        self.start_all_planet_ais()?;

        // 2. Wait 20ms for the planets to be fully ready
        std::thread::sleep(std::time::Duration::from_millis(20));

        // 3. Spawn all explorers on their designated planets
        for &(explorer_id, planet_id) in mattia_explorers {
            self.add_mattia_explorer(explorer_id, planet_id)?;
        }
        for &(explorer_id, planet_id) in tommy_explorers {
            self.add_tommy_explorer(explorer_id, planet_id)?;
        }

        // 4. Start all explorer AIs
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

    // /// Global start function, starts all of the planets'
    // /// and explorer's AIs; wrapper on
    // /// [`start_all_planet_ais`](`Self::start_all_planet_ais`)
    // /// and on
    // /// [`start_all_explorer_ais`](`Self::start_all_explorer_ais`).
    // ///
    // /// Returns Err if any of the planets fail to start.
    // pub fn start_all(&mut self) -> Result<(), String> {
    //     //LOG
    //     log_fn_call!(self, "start_all()");
    //     //LOG
    //     self.start_all_planet_ais()?;
    //
    //     //Add explorers here otherwise if the planets don't start first the explorers won't start either and the game will be stuck in a limbo state where the user can't do anything but quit
    //     //self.add_mattia_explorer(0, 0)?;
    //     //self.add_tommy_explorer(1, 1)?;
    //     self.start_all_explorer_ais()?;
    //     //LOG
    //     log_internal_op!(
    //         self,
    //         "action"=>"all systems started",
    //         "status"=>"success"
    //     );
    //     //LOG
    //     Ok(())
    // }

    /// Global stop function, pauses all of the planets'
    /// and explorer's AIs; wrapper on
    /// [`stop_all_planet_ais`](`Self::stop_all_planet_ais`)
    /// and on
    /// [`stop_all_explorer_ais`](`Self::stop_all_explorer_ais`).
    ///
    /// Returns Err if any of the planets fail to start.
    pub fn stop_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "stop_all()");
        //LOG
        self.stop_all_explorer_ais()?;
        self.stop_all_planet_ais()?;
        //LOG
        log_internal_op!(
            self,
            "action"=>"stop_all requested",
            "status"=>"success"
        );
        //LOG
        Ok(())
    }

    pub fn restart_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "restart_all()");
        //LOG

        // 1. Start all planet AIs
        self.start_all_planet_ais()?;

        // 2. Wait 20ms for the planets to be fully ready
        std::thread::sleep(std::time::Duration::from_millis(20));

        // 3. Start all explorer AIs
        self.start_all_explorer_ais()?;

        //LOG
        log_internal_op!(
            self,
            "action"=>"all systems restarted",
            "status"=>"success"
        );
        //LOG
        Ok(())
    }

    /// Choose whether to create a celestial body (and which one).
    ///
    /// The function chooses randomly whether to do anything at all in a given
    /// turn and, if needed, also decides on whether it should send an
    /// asteroid or a sunray. The probability of these two actions are regulated
    /// by the two input variables:
    /// * `p_action` regulates the chance that the orchestrator decides to do anything
    /// that might change the state of the galaxy (i.e. send either an
    /// asteroid or a sunray). Ideally this should be pretty high but not 1;
    /// * `p_asteroid` regulates the chance that the orchestrator, when asked to launch
    /// a celestial body, launches either an asteroid or a sunray.
    /// A lower setting skews the balance towards sunrays, which makes
    /// for longer games, while a value over 0.5 is pretty much
    /// intergalactic nuclear war.

    pub fn choose_random_action(&mut self, p_action: f64, p_asteroid: f64) -> Result<(), String> {
        let mut rng = rand::rng();
        let living_things = self.planets_info.get_list_id_alive();

        let actions = random::<u32>() % 3;

        for _ in 0..actions {
            // get a random ID, from the list of planets that are still alive
            let &rand_id = match living_things.choose(&mut rng) {
                Some(num) => num,
                None => return Ok(()),
            };

            let mut params: Option<(u32, Sender<OrchestratorToPlanet>)> = None;

            // find the set of channels that correspond to the chosen ID
            for (&id, channels) in self.planet_channels.iter() {
                if id == rand_id {
                    // to_owned is not a deep copy
                    params = Some((id, channels.0.to_owned()));
                }
            }

            // if there is at least one living planet...
            if let Some((id, channel)) = params {
                // ...choose whether to do anything at all (probably will)
                if rng.random_bool(p_action) {
                    // chooses between sunray or asteroid
                    if rng.random_bool(p_asteroid) {
                        self.send_asteroid(id, &channel)?;
                    } else {
                        self.send_sunray(id, &channel)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn send_celestial_from_gui(
        &mut self,
        id_list: Vec<u32>,
        is_asteroid: bool,
    ) -> Result<(), String> {
        let alive = self.planets_info.get_list_id_alive();

        for planet_id in id_list {
            if !alive.contains(&planet_id) {
                continue;
            }

            let parameters: Option<(u32, Sender<OrchestratorToPlanet>)> =
                self.planet_channels.iter().find_map(|(&id, (sender, _))| {
                    if id == planet_id {
                        Some((id, sender.clone()))
                    } else {
                        None
                    }
                });

            match parameters {
                Some(valid) => {
                    if is_asteroid {
                        self.send_asteroid(valid.0, &valid.1)?
                    } else {
                        self.send_sunray(valid.0, &valid.1)?
                    }
                }
                None => {
                    return Err(
                        "send_celestial_from_gui: no valid planet parameters found".to_string()
                    );
                }
            }
        }
        Ok(())
    }
}
