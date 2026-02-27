use crate::{PlanetInfoMap, utils::registry::PlanetType};
use std::sync::Arc;

use log::info;
use rustc_hash::FxHashMap;

use crate::utils::ExplorerInfoMap;
use crate::{
    GalaxyTopology, PlanetStatus,
    components::orchestrator::{Orchestrator, OrchestratorEvent},
    utils::{ExplorerStatusNotLock, GalaxySnapshot, PlanetStatusNotLock},
};
use logging_utils::LoggableActor;
use logging_utils::log_fn_call;

impl Orchestrator {
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
        let topology = self.galaxy_topology.read().unwrap();

        let mut edges = Vec::new();
        let planet_num = topology.len();

        for i in 0..topology.len() {
            for j in (i + 1)..topology[i].len() {
                if topology[i][j] {
                    edges.push((i as u32, j as u32));
                }
            }
        }

        drop(topology);

        (edges, planet_num)
    }

    // Getter functions necessary for Ratatui-gui

    pub fn get_planets_info(&self) -> PlanetInfoMap {
        //LOG
        log_fn_call!(self, "planet_states()");
        //LOG
        self.planets_info.clone()
    }
    pub fn get_explorer_states(&self) -> ExplorerInfoMap {
        //LOG
        log_fn_call!(self, "explorer_states()");
        //LOG
        let explorer_status = self.explorers_info.clone();
        explorer_status
    }
    pub fn get_galaxy_topology(&self) -> Vec<Vec<bool>> {
        self.galaxy_topology.read().unwrap().clone()
    }

    // Bevy stuff

    /// Emits a Bevy event if a planet has died
    ///
    /// If the orchestrator's Bevy Message buffer is not None,
    /// It sends a message that signals the death of planet
    /// `planet_id`
    pub(crate) fn emit_planet_death(&mut self, planet_id: u32) {
        info!("planet-death: THIS FUNCTION IS STILL BEING BUILT");
        self.gui_messages
            .push(OrchestratorEvent::PlanetDestroyed { planet_id });
    }

    pub(crate) fn emit_sunray_ack(&mut self, planet_id: u32) {
        info!("sunray-ack: THIS FUNCTION IS STILL BEING BUILT");
        self.gui_messages
            .push(OrchestratorEvent::SunrayReceived { planet_id });
    }

    pub(crate) fn emit_sunray_send(&mut self, planet_id: u32) {
        info!("sunray-send: THIS FUNCTION IS STILL BEING BUILT");
        self.gui_messages
            .push(OrchestratorEvent::SunraySent { planet_id });
    }

    pub(crate) fn emit_asteroid_send(&mut self, planet_id: u32) {
        info!("asteroid-send: THIS FUNCTION IS STILL BEING BUILT");
        self.gui_messages
            .push(OrchestratorEvent::AsteroidSent { planet_id });
    }
}
