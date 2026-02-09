use crate::{Status, components::orchestrator::Orchestrator, settings};
use common_game::{
    logging::{ActorType, EventType},
    protocols::orchestrator_planet::OrchestratorToPlanet,
};
use common_game::logging::{Channel, LogEvent, Participant};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::Sender;
use log::info;
use logging_utils::{LoggableActor, log_fn_call, log_message, warning_payload, debug_println};

impl Orchestrator {
    pub fn send_sunray_or_asteroid(&mut self) -> Result<(), String> {
        // debug_println!("{:?}", self.ticker);
        match settings::pop_sunray_asteroid_sequence() {
            Some('S') => {
                self.send_sunray_to_all()?;
            }
            Some('A') => {
                self.send_asteroid_to_all()?;
            }
            msg => {
                // Probability mode

                // Get a random planet
                let planet_id= self.get_random_planet_id()?;
                
                // Get planet communication channel
                let sender = &self.planet_channels.get(&planet_id).unwrap().0.clone();

                // Decide whether to send sunray or asteroid
                if settings::does_sunray_spawn() {
                    self.send_sunray(planet_id, sender)?;
                } else {
                    self.send_asteroid(planet_id, sender)?;
                }
            }
        }
        Ok(())
    }
    /// Send a sun ray to a planet.
    ///
    /// Requests a sun ray through the `forge` and sends it to the planet.
    ///
    /// Returns Err if the planet's channel is inaccessible.
    pub fn send_sunray(
        &mut self,
        planet_id: u32,
        sender: &Sender<OrchestratorToPlanet>,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(
            self,
            "send_sunray()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG if the planet is dead we do not send the sunray
        //send sunray
        let handle_by_log = sender
            .send(OrchestratorToPlanet::Sunray(self.forge.generate_sunray()))
            .map_err(|_| "Unable to send a sunray to planet: {id}".to_string());
        self.emit_sunray_send(planet_id);

        //send update request
        self.send_internal_state_request(sender)?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Planet,
            planet_id,
            EventType::MessageOrchestratorToPlanet,
            "Sunray",
        );
        //LOG
        Ok(())
    }

    /// Sends a sun ray to all planets.
    ///
    /// See [`send_sunray`](`Self::send_sunray`) for more details on how a sunray is sent.
    pub(crate) fn send_sunray_to_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "send_sunray_to_all()");
        //LOG
        //collect all of the senders in a vector
        let senders_sunray: Vec<(u32, Sender<OrchestratorToPlanet>)> = self
            .planet_channels
            .iter()
            .filter_map(|(id, (sender, _))| {
                let status = &self.planets_info;
                if status.get_status(id) != Status::Dead {
                    Some((*id, sender.clone()))
                } else {
                    None
                }
            })
            .collect();

        // actually send the messages
        for (id, sender) in senders_sunray {
            self.send_sunray(id, &sender)?;
        }
        Ok(())
    }

    /// Send an asteroid to a planet.
    ///
    /// Requests an asteroid through the `forge` and sends it to the planet.
    ///
    /// Returns Err if the planet's channel is inaccessible.
    pub fn send_asteroid(
        &mut self,
        planet_id: u32,
        sender: &Sender<OrchestratorToPlanet>,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(
            self,
            "send_asteroid()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG
        //send asteroid LOG if the asteroid wasn't sent we still log it because the attempt was made
        let handle_by_log = sender
            .send(OrchestratorToPlanet::Asteroid(
                self.forge.generate_asteroid(),
            ))
            .map_err(|_| "Unable to send asteroid to planet: {id}".to_string());
        self.emit_asteroid_send(planet_id);
        //send update request
        self.send_internal_state_request(sender)?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Planet,
            planet_id,
            EventType::MessageOrchestratorToPlanet,
            "Asteroid",
        );
        //LOG
        Ok(())
    }

    /// Sends an asteroid to all planets.
    ///
    /// See [`send_asteroid`](`Self::send_asteroid`) for more details on how an asteroid
    /// is sent.
    pub(crate) fn send_asteroid_to_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "send_asteroid_to_all()");
        //LOG

        //TODO unwrap cannot fail because every id is contained in the map
        //collect all of the senders in a vector
        let sender_asteroid: Vec<(u32, Sender<OrchestratorToPlanet>)> = self
            .planet_channels
            .iter()
            .filter_map(|(id, (sender, _))| {
                let status = &self.planets_info;
                if status.get_status(id) != Status::Dead {
                    Some((*id, sender.clone()))
                } else {
                    None
                }
            })
            .collect();

        // actually send the messages
        for (id, sender) in sender_asteroid {
            self.send_asteroid(id, &sender)?;
        }
        Ok(())
    }

    /// Kill a specific planet.
    ///
    /// Sends a KillPlanet message to the planet, which is required to handle it.
    /// This function does not check wether the planet has actually died: it only
    /// sends the message.
    ///
    /// Returns Err if the planet's channel is inaccessible.
    pub fn send_planet_kill(
        &mut self,
        planet_id: u32,
        sender: &Sender<OrchestratorToPlanet>,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(
            self,
            "send_planet_kill()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );

        info!("killing planet {planet_id}");
        //LOG
        sender
            .send(OrchestratorToPlanet::KillPlanet)
            .map_err(|_| "Unable to send kill message to planet: {id}".to_string())?;

        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Planet,
            0u32, //TODO missing planet id
            EventType::MessageOrchestratorToPlanet,
            "KillPlanet",
        );
        Ok(())
    }

    /// Sends a Kill message to all planets.
    ///
    /// See [`send_planet_kill`](`Self::send_planet_kill`) for more details on how a
    /// planet kill message is sent.
    pub(crate) fn send_planet_kill_to_all(&mut self) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "send_planet_kill_to_all()");
        //LOG

        //collect all of the senders in a vector
        let senders_to_kill: Vec<(u32, Sender<OrchestratorToPlanet>)> = self
            .planet_channels
            .iter()
            .filter_map(|(id, (sender, _))| {
                let status = &self.planets_info;
                if status.get_status(id) != Status::Dead {
                    Some((*id, sender.clone()))
                } else {
                    None
                }
            })
            .collect();

        // actually send the messages
        for (id, sender) in senders_to_kill {
            self.send_planet_kill(id, &sender)?;
        }
        Ok(())
    }

    pub fn send_internal_state_request(
        &self,
        sender: &Sender<OrchestratorToPlanet>,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(
            self,
            "send_internal_state_request()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG if the planet is dead we do not send the request
        let handle_by_log = sender
            .send(OrchestratorToPlanet::InternalStateRequest)
            .map_err(|_| "Unable to send planet state request".to_string());

        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Planet,
            0u32, //TODO missing planet id
            EventType::MessageOrchestratorToPlanet,
            "RequestPlanetState",
        );
        Ok(())
    }

    pub fn send_incoming_explorer_request(
        &self,
        planet_id: ID,
        explorer_id: ID,
    )->Result<(), String>{
        log_fn_call!(
            self,
            "send_incoming_explorer_request()",
            planet_id,
            explorer_id,
        );
        //todo logs
        let sender = match self.planet_channels.get(&planet_id){
            Some(sender) => sender,
            None => {
                debug_println!("Unknown planet: {}", planet_id);
                return Err(format!("Unknown planet: {}", planet_id))
            },
        };

        let new_planet_to_explorer_sender=match self.explorer_channels.get(&explorer_id){
            Some(sender) => sender,
            None => {
                debug_println!("Unknown planet: {}", planet_id);
                return Err(format!("Unknown explorer: {}", explorer_id))
            },
        };

        match sender.0.send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_sender: new_planet_to_explorer_sender.1.clone(),
        }){
            Ok(_) => {
                debug_println!("IncomingExplorerRequest sent correctly")
            }
            Err(err) => {
                LogEvent::new(
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    EventType::MessageOrchestratorToPlanet,
                    Channel::Warning,
                    warning_payload!(
                        "impossible to send IncomingExplorerRequest message to planet",
                        err,
                        "send_incoming_explorer_request()";
                        "explorer_id"=>explorer_id,
                        "planet_id"=>planet_id
                    )
                ).emit();
                //todo logs
                return Err(err.to_string());
            }
        }

        Ok(())
    }
}
