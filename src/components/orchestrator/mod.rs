
pub mod planets_comms;
pub mod debug;
pub mod gui_comms;
pub mod handlers;
pub mod init;
pub mod macros;
pub mod update;




use crate::{ExplorerStatus, log_orch_fn, log_orch_internal};
use crate::components::explorer::{BagType};
use crate::utils::registry::{PlanetType};
use crate::utils::types::GalaxyTopology;
use crate::utils::{PlanetStatus};
use common_game::components::forge::Forge;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use crossbeam_channel::{Receiver, Sender, unbounded};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;


///The core of the game.
///
/// The orchestrator's main responsibility is to handle game state, without directly
/// affecting the game timeline. The orchestrator can modify the game state via
/// automatic predefined behavior or via direct intervention through its API. Its main
/// responsibilities are:
/// - generating and eventually updating the galaxy topology
/// - handling the creation and assignment of communication channels
/// - directly overseeing the requests of the game's components
/// - creating and sending both asteroids and sun rays
/// - coordinating and overseeing the actions of explorers
/// - ensuring the state of the various elements of the game are congruent with the
/// game timeline
pub struct Orchestrator {
    // Forge sunray and asteroid
    pub forge: Forge,

    //Galaxy
    pub galaxy_topology: GalaxyTopology,
    pub galaxy_lookup: FxHashMap<u32, (u32, PlanetType)>,

    //Status for each planets and explorers, BTreeMaps are useful for printing
    pub planets_status: PlanetStatus,
    pub explorer_status: ExplorerStatus,
    //Communication channels for sending messages to planets and explorers
    pub planet_channels: HashMap<u32, (Sender<OrchestratorToPlanet>, Sender<ExplorerToPlanet>)>,
    pub explorer_channels: HashMap<u32, (Sender<OrchestratorToExplorer>, Sender<PlanetToExplorer>)>,

    //Channel to clone for the planets and for receiving Planet Messages
    pub sender_planet_orch: Sender<PlanetToOrchestrator>,
    pub receiver_orch_planet: Receiver<PlanetToOrchestrator>,

    //Channel to clone for the explorer and for receiving Explorer Messages
    pub sender_explorer_orch: Sender<ExplorerToOrchestrator<BagType>>,
    pub receiver_orch_explorer: Receiver<ExplorerToOrchestrator<BagType>>,
}
impl Orchestrator{
    /// Create a new orchestrator instance.
pub(crate) fn new() -> Result<Self, String> {
    //env_logger initialization
    env_logger::init();
    //Log
    log_orch_fn!("new()",);
    //LOG

    let (sender_planet_orch, receiver_orch_planet) = unbounded();
    let (sender_explorer_orch, receiver_orch_explorer) = unbounded();

    //Log
    log_orch_internal!({
        "action"=>"channels initialized",
        "from"=>"planet, explorer",
        "to"=>"orchestrator"
    });
    //LOG

    let new_orch = Self {
        forge: Forge::new()?,
        galaxy_topology: Self::new_gtop(),
        galaxy_lookup: FxHashMap::default(),
        planets_status: Arc::new(RwLock::new(BTreeMap::new())),
        explorer_status: Arc::new(RwLock::new(BTreeMap::new())),
        planet_channels: HashMap::new(),
        explorer_channels: HashMap::new(),
        sender_planet_orch,
        receiver_orch_planet,
        sender_explorer_orch,
        receiver_orch_explorer,
    };
    Ok(new_orch)
}

}
