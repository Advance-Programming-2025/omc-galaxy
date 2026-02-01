use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use log::info;
use logging_utils::{
    LOG_ACTORS_ACTIVITY, LoggableActor, Sender, debug_println, log_fn_call, log_internal_op, log_message, payload, warning_payload
};
use rand::{Rng, random, seq::IndexedRandom};

use crate::{components::orchestrator::Orchestrator, utils::Status};

/// regulates the chance that the orchestrator decides to do anything
/// that might change the state of the galaxy (i.e. send either an 
/// asteroid or a sunray). Ideally this should be pretty high but not 1 
pub const RANDOM_ACTION_CHANCE: f64 = 0.8;

/// regulates the chance that the orchestrator, when asked to launch
/// a celestial body, launches either a sunray or an asteroid.
/// 
/// A lower setting skews the balance towards sunrays, which makes 
/// for longer games, while a value over 0.6 is pretty much
/// intergalactic nuclear war.
pub const SUNRAY_ASTEROID_CHANCE: f64 = 0.5;

impl Orchestrator {


    /// Removes the link between two planets if one of them explodes.
    ///
    /// Returns Err if the given indexes are out of bounds, Ok otherwise;
    /// it does NOT currently check wether the link was already set to false beforehand
    ///
    /// * `planet_one_pos` - Position of the first planet in the matrix. Must be a valid index
    /// * `planet_two_pos` - Position of the second planet in the matrix. Must be a valid index
    pub fn destroy_topology_link(
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
                    self.planets_info.update_status(planet_id, Status::Running)?;
                    count += 1;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Stops the AI of every planet.
    ///
    /// Goes through every PlanetToOrchestrator channel and sends the `StopPlanetAI`
    /// message.
    ///
    /// Returns Err if any of the communication channels are inaccessible.
    pub(crate) fn stop_all_planet_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "stop_all_planet_ais()");
        //LOG

        for (_id, (from_orch, _)) in &self.planet_channels {
            from_orch
                .try_send(OrchestratorToPlanet::StopPlanetAI)
                .map_err(|_| "Cannot send message to {_id}".to_string())?;

            //LOG
            log_message!(
                ActorType::Orchestrator, 0u32,
                ActorType::Planet, *_id,
                EventType::MessageOrchestratorToPlanet,
                "StopPlanetAI";
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
                    "action"=>"all planets stopped",
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
                PlanetToOrchestrator::StopPlanetAIResult { planet_id } => {
                    debug_println!("Stopped Planet AI: {}", planet_id);

                    //LOG
                    let event = LogEvent::new(
                        Some(Participant::new(ActorType::Planet, planet_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessagePlanetToOrchestrator,
                        LOG_ACTORS_ACTIVITY,
                        payload!(
                            "message"=>"StopPlanetAIResult",
                            "planet_id"=>planet_id,
                            "status"=>"Paused"
                        ),
                    );
                    event.emit();
                    //LOG
                    self.planets_info.update_status(planet_id, Status::Paused)?;
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

    /// Stop the AI of every explorer.
    ///
    /// Goes through every OrchestratorToExplorer channel and sends the `StopExplorerAI`
    ///
    /// Returns Err if any of the communication channels are inaccessible.
    pub(crate) fn stop_all_explorer_ais(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "stop_all_explorer_ais()");
        //LOG

        for (_id, (from_orch, _)) in &self.explorer_channels {
            from_orch
                .try_send(OrchestratorToExplorer::StopExplorerAI)
                .map_err(|_| format!("Cannot send message to explorer {}", _id))?;

            //LOG
            log_message!(
                ActorType::Orchestrator, 0u32,
                ActorType::Explorer, *_id,
                EventType::MessageOrchestratorToExplorer,
                "StopExplorerAI";
                "explorer_id"=>_id
            );
            //LOG
        }

        //TODO this is probably not needed for the stop function
        // also check the stop_all_planet_ais func
        let mut count = 0;
        loop {
            if count == self.explorer_channels.len() {
                //LOG
                log_internal_op!(
                    self,
                    "action"=>"all explorers stopped",
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
                ExplorerToOrchestrator::StopExplorerAIResult { explorer_id } => {
                    debug_println!("Stopped Explorer AI: {}", explorer_id);

                    //LOG
                    let event = LogEvent::new(
                        Some(Participant::new(ActorType::Explorer, explorer_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessageExplorerToOrchestrator,
                        LOG_ACTORS_ACTIVITY,
                        payload!(
                            "message"=>"StoppedExplorerAIResult",
                            "explorer_id"=>explorer_id,
                            "status"=>"Paused"
                        ),
                    );
                    event.emit();
                    //LOG

                    self.explorers_info.insert_status(explorer_id, Status::Paused);
                    count += 1;
                }
                _ => {
                    // ignores other events
                }
            }
        }

        Ok(())
    }

    /// Global start function, starts all of the planets'
    /// and explorer's AIs; wrapper on
    /// [`start_all_planet_ais`](`Self::start_all_planet_ais`)
    /// and on
    /// [`start_all_explorer_ais`](`Self::start_all_explorer_ais`).
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


    pub fn choose_random_action(&mut self) -> Result<(), String> {
        let mut rng = rand::rng();
        let living_things = self.planets_info.get_list_id_alive();

        let actions = random::<u32>() % 3;

        for _ in 0..actions {
            // get a random ID, from the list of planets that are still alive
            let &rand_id = match living_things.choose(&mut rng) {
                Some(num) => num,
                None => return Ok(()),
            };
            
            let mut params: Option<(u32,Sender<OrchestratorToPlanet>)> = None;

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
                if rng.random_bool(RANDOM_ACTION_CHANCE){
                    // chooses between sunray or asteroid
                    if rng.random_bool(SUNRAY_ASTEROID_CHANCE) {
                        self.send_asteroid(id, &channel)?;
                    } else {
                        self.send_sunray(id, &channel)?;
                    }
                }
            }
        }

        Ok(())
    }


    // TODO unify this function and the next one in send_celestial_from_gui
    pub fn send_sunray_from_gui(&mut self, id_list: Vec<u32>) -> Result<(),String> {
        let alive = self.planets_info.get_list_id_alive();
        
        for planet_id in id_list{
        if !alive.contains(&planet_id) {
            continue;
        }
        
        let parameters: Option<(u32, Sender<OrchestratorToPlanet>)> = self.planet_channels
            .iter()
            .find_map(|(&id, (sender, _))| {
                if id == planet_id {
                    Some((id, sender.clone()))
                } else {
                    None
                }
            });

        match parameters {
            Some(valid) => {
                self.send_sunray(valid.0, &valid.1)?
            }, 
            None => todo!()
        }
    }
    Ok(())
    }

    pub fn send_asteroid_from_gui(&mut self, id_list: Vec<u32>) -> Result<(),String> {
        for planet_id in id_list{
        if !self.planets_info.get_list_id_alive().contains(&planet_id) {
            return Err("Planet is either dead or not valid".to_string())
        }
        
        let parameters: Option<(u32, Sender<OrchestratorToPlanet>)> = self.planet_channels
            .iter()
            .find_map(|(&id, (sender, _))| {
                if id == planet_id {
                    Some((id, sender.clone()))
                } else {
                    None
                }
            });

        match parameters {
            Some(valid) => {
                self.send_asteroid(valid.0, &valid.1)?
            }, 
            None => {}
        }
    }
    Ok(())
        
    }
}
