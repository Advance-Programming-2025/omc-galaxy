use std::sync::Arc;

use crate::{ExplorerStatus, GalaxyTopology, PlanetStatus, components::orchestrator::Orchestrator, log_orch_fn, utils::GalaxySnapshot};

impl Orchestrator {
    /// Get a snapshot of the current galaxy topology
    ///
    /// Returns an atomic reference of the current
    /// galaxy topology. This is made to avoid changing
    /// the topology from the GUI's side in an improper
    /// way that might misalign the internal state
    pub fn get_topology(&self) -> GalaxySnapshot {
        //LOG
        log_orch_fn!("get_topology()");
        //LOG
        let topology = self.galaxy_topology.read().unwrap();

        let mut edges = Vec::new();

        for i in 0..topology.len() {
            for j in (i + 1)..topology[i].len() {
                if topology[i][j] {
                    edges.push((i as u32, j as u32));
                }
            }
        }

        drop(topology);

        edges
    }

    /// Get the game's current state, as present in the orchestrator.
    ///
    /// Returns a tuple of 3 atomic references to objects that represent
    /// the game's state:
    /// - `GalaxyTopology`, the current structure of the galaxy
    /// - `PlanetStatus`, the status (Running, Paused, ...) of all planets
    /// - `ExplorerStatus`, the status (Running, Paused, ...) of all explorers
    pub(crate) fn get_game_status(
        &self,
    ) -> Result<(GalaxyTopology, PlanetStatus, ExplorerStatus), String> {
        //LOG
        log_orch_fn!("get_game_status()");
        //LOG
        Ok((
            Arc::clone(&self.galaxy_topology),
            Arc::clone(&self.planets_status),
            Arc::clone(&self.explorer_status),
        ))
    }
}
