use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use common_game::components::planet::Planet;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use crossbeam_channel::{Receiver, Sender};

use crate::utils::Status;

pub type PlanetFactory = Box<
     dyn Fn(
            Receiver<OrchestratorToPlanet>,
            Sender<PlanetToOrchestrator>,
            Receiver<ExplorerToPlanet>,
            u32,
        ) -> Result<Planet, String>
        + Send
        + Sync
        + 'static,
>;


pub type GalaxyTopologyNotLock = Vec<Vec<bool>>;
pub type PlanetStatusNotLock = BTreeMap<u32, Status>;
pub type ExplorerStatusNotLock = BTreeMap<u32, Status>;

pub type GalaxyTopology = Arc<RwLock<GalaxyTopologyNotLock>>;
pub type PlanetStatus = Arc<RwLock<PlanetStatusNotLock>>;
pub type ExplorerStatus = Arc<RwLock<ExplorerStatusNotLock>>;

pub type GalaxySnapshot = Vec<(u32, u32)>;

