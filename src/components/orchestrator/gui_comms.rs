use crate::PlanetInfoMap;
use crossbeam_channel::Sender;
use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
use log::info;

use crate::utils::ExplorerInfoMap;
use crate::{
    components::orchestrator::{Orchestrator, OrchestratorEvent},
    utils::GalaxySnapshot,
};
use logging_utils::LoggableActor;
use logging_utils::log_fn_call;

impl Orchestrator {
    // TODO unify this function and the next one in send_celestial_from_gui
    pub fn send_sunray_from_gui(&mut self, id_list: Vec<u32>) -> Result<(), String> {
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
                Some(valid) => self.send_sunray(valid.0, &valid.1)?,
                None => todo!(),
            }
        }
        Ok(())
    }

    pub fn send_asteroid_from_gui(&mut self, id_list: Vec<u32>) -> Result<(), String> {
        for planet_id in id_list {
            if !self.planets_info.get_list_id_alive().contains(&planet_id) {
                return Ok(());
                // return Err("Planet is either dead or not valid".to_string());
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
                Some(valid) => self.send_asteroid(valid.0, &valid.1)?,
                None => {}
            }
        }
        Ok(())
    }

    /// Get a snapshot of the current galaxy topology
    ///
    /// Returns an atomic reference of the current
    /// galaxy topology. This is made to avoid changing
    /// the topology from the GUI's side in an improper
    /// way that might misalign the internal state
    pub fn get_topology(&self) -> (GalaxySnapshot, usize) {
        //LOG
        log_fn_call!(self, "get_topology()");
        //LOG
        let topology = &self.galaxy_topology;

        let mut edges = Vec::new();
        let planet_num = topology.len();

        for i in 0..topology.len() {
            for j in (i + 1)..topology[i].len() {
                if topology[i][j] {
                    // Translate matrix indices back to real planet_ids
                    let planet_a = self
                        .galaxy_reverse_lookup
                        .get(&(i as u32))
                        .copied()
                        .unwrap_or(i as u32);
                    let planet_b = self
                        .galaxy_reverse_lookup
                        .get(&(j as u32))
                        .copied()
                        .unwrap_or(j as u32);
                    edges.push((planet_a, planet_b));
                }
            }
        }

        (edges, planet_num)
    }

    // Getter functions necessary for Ratatui-gui

    pub fn get_planets_info(&mut self) -> PlanetInfoMap {
        //LOG
        log_fn_call!(self, "planet_states()");
        //LOG
        self.planets_info.clone()
    }
    pub fn send_stop_explorer_from_gui(&mut self, explorer_id: u32) -> Result<(), String> {
        let explorer_channel = self
            .explorer_channels
            .get(&explorer_id)
            .ok_or_else(|| format!("Explorer {explorer_id} not found"))?;
        let from_orch = &explorer_channel.0;

        from_orch
            .try_send(OrchestratorToExplorer::StopExplorerAI)
            .map_err(|_| format!("Cannot send message to {explorer_id}"))?;

        //LOG

        //LOG

        Ok(())
    }
    pub fn send_move_explorer_from_gui(
        &mut self,
        explorer_id: u32,
        destination_planet_id: u32,
    ) -> Result<(), String> {
        self.send_stop_explorer_from_gui(explorer_id)?;
        let explorer_channel = self
            .explorer_channels
            .get(&explorer_id)
            .ok_or_else(|| format!("Explorer {explorer_id} not found"))?;
        let from_orch = &explorer_channel.0;

        let planet_channel = self
            .planet_channels
            .get(&destination_planet_id)
            .ok_or_else(|| format!("Planet {destination_planet_id} not found"))?;
        let sender_to_new_planet = planet_channel.1.clone();

        if !self.planets_info.is_dead(&destination_planet_id) {
            from_orch
                .try_send(OrchestratorToExplorer::MoveToPlanet {
                    sender_to_new_planet: Some(sender_to_new_planet),
                    planet_id: destination_planet_id,
                })
                .map_err(|_| format!("Cannot send message to {explorer_id}"))?;

            //LOG

            //LOG
        }
        Ok(())
    }
    pub fn send_bag_content_request_from_ui(&self) -> Result<(), String> {
        for explorer_id in self.explorer_channels.keys() {
            if !self.explorers_info.is_dead(explorer_id) {
                self.send_bag_content_request(*explorer_id)?;
            }
        }
        Ok(())
    }
    pub fn get_explorer_states(&self) -> ExplorerInfoMap {
        self.explorers_info.clone()
    }
    pub fn get_galaxy_topology(&self) -> Vec<Vec<bool>> {
        self.galaxy_topology.clone()
    }

    // Bevy stuff

    /// Emits a Bevy event if a planet has died
    ///
    /// If the orchestrator's Bevy Message buffer is not None,
    /// It sends a message that signals the death of planet
    /// `planet_id`
    pub(crate) fn emit_planet_death(&mut self, planet_id: u32) {
        info!("GUI event planet_death was triggered");
        self.gui_messages
            .push(OrchestratorEvent::PlanetDestroyed { planet_id });
    }

    pub(crate) fn emit_sunray_ack(&mut self, planet_id: u32) {
        info!("GUI event sunray_ack was triggered");
        self.gui_messages
            .push(OrchestratorEvent::SunrayReceived { planet_id });
    }

    pub(crate) fn emit_sunray_send(&mut self, planet_id: u32) {
        info!("GUI event sunray_send was triggered");
        self.gui_messages
            .push(OrchestratorEvent::SunraySent { planet_id });
    }

    pub(crate) fn emit_asteroid_send(&mut self, planet_id: u32) {
        info!("GUI event asteroid_send was triggered");
        self.gui_messages
            .push(OrchestratorEvent::AsteroidSent { planet_id });
    }

    pub(crate) fn emit_explorer_move(&mut self, explorer_id: u32, planet_id: u32) {
        info!("GUI event esplorer_move was triggered");
        self.gui_messages.push(OrchestratorEvent::ExplorerMoved {
            explorer_id,
            destination: planet_id,
        });
    }
}
