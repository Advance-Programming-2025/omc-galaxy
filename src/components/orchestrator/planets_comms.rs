use common_game::{
    logging::{ActorType, EventType},
    protocols::orchestrator_planet::OrchestratorToPlanet,
};
use crossbeam_channel::Sender;

use crate::{
    components::orchestrator::{Orchestrator}, log_message, log_orch_fn, settings, utils::Status
};

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
                let planet_id = self.get_random_planet_id()?;
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
        log_orch_fn!(
            "send_sunray()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG
        sender
            .send(OrchestratorToPlanet::Sunray(self.forge.generate_sunray()))
            .map_err(|_| "Unable to send a sunray to planet: {id}".to_string())?;
        self.emit_sunray_send(planet_id);
        
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
        log_orch_fn!("send_sunray_to_all()");
        //LOG
        //collect all of the senders in a vector
        let senders_sunray: Vec<(u32, Sender<OrchestratorToPlanet>)> =
        self.planet_channels
            .iter()
            .filter_map(|(id, (sender, _))| {
                let status = self.planets_status.read().unwrap();
                if status.get(id) != Some(&Status::Dead) {
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
        log_orch_fn!(
            "send_asteroid()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG

        sender
            .send(OrchestratorToPlanet::Asteroid(
                self.forge.generate_asteroid(),
            ))
            .map_err(|_| "Unable to send sunray to planet: {id}".to_string())?;
        self.emit_asteroid_send(planet_id);
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
        log_orch_fn!("send_asteroid_to_all()");
        //LOG

        //TODO unwrap cannot fail because every id is contained in the map
        //collect all of the senders in a vector
        let sender_asteroid: Vec<(u32, Sender<OrchestratorToPlanet>)> =
        self.planet_channels
            .iter()
            .filter_map(|(id, (sender, _))| {
                let status = self.planets_status.read().unwrap();
                if status.get(id) != Some(&Status::Dead) {
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
        log_orch_fn!(
            "send_planet_kill()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG
        sender
            .send(OrchestratorToPlanet::KillPlanet)
            .map_err(|_| "Unable to send kill message to planet: {id}".to_string())?;

        self.emit_planet_death(planet_id);
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
        log_orch_fn!("send_planet_kill_to_all()");
        //LOG

        //collect all of the senders in a vector
        let senders_to_kill: Vec<(u32, Sender<OrchestratorToPlanet>)> =
        self.planet_channels
            .iter()
            .filter_map(|(id, (sender, _))| {
                let status = self.planets_status.read().unwrap();
                if status.get(id) != Some(&Status::Dead) {
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
}
