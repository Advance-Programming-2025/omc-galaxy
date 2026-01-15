use crate::components::explorer::{BagType, Explorer};
use crate::utils::{ExplorerStatus, PlanetStatus};
use crate::utils::registry::PlanetType::{
    BlackAdidasShoe, Ciuc, HoustonWeHaveABorrow, ImmutableCosmicBorrow, OneMillionCrabs, Rustrelli,
};
use crate::utils::registry::{PLANET_REGISTRY, PlanetType};
use crate::utils::state_enums::Status;
use crate::utils::types::GalaxyTopology;
use common_game::components::forge::Forge;
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration};
use std::{fs, thread};
use common_game::logging;
use common_game::logging::Channel::Error;

const LOG_FN_CALL_CHNL: Channel = Channel::Debug;
const LOG_FN_INT_OPERATIONS: Channel = Channel::Trace;
const LOG_ACTORS_ACTIVITY: Channel = Channel::Info;
/// LOG macros
/// needed to reduce code duplication when writing log code
#[macro_export] //make this macro visible outside
macro_rules! payload {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut p = std::collections::BTreeMap::new();
        $(
            p.insert($key.to_string(), $val.to_string());
        )*
        p
    }};
}
#[macro_export]
macro_rules! warning_payload {
    ($warn:expr, $err:expr, $func:expr $(,$param:ident )*$(; $($key:expr => $val:expr),*)?) => {{
        let mut p = std::collections::BTreeMap::new();

        p.insert("Warning".to_string(), $warn.to_string());
        p.insert("returned error".to_string(), $err.to_string());
        p.insert("fn".to_string(), $func.to_string());

        // adds every argument
        $(
            p.insert(
                stringify!($param).to_string(),
                format!("{:?}", $param)
            );
        )*
        // generic key-value
        $($(
            p.insert($key.to_string(), $val.to_string());
        )*)?

        p
    }};
}

#[macro_export]
macro_rules! log_orch_internal {
    ({ $($key:expr => $val:expr),* $(,)? }) => {{
        use common_game::logging::{LogEvent, Participant, ActorType, EventType};

        LogEvent::self_directed(
            Participant::new(ActorType::Orchestrator, 0u32),
            EventType::InternalOrchestratorAction,
            LOG_FN_INT_OPERATIONS,
            $crate::payload!( $($key => $val),* )
        ).emit();
    }};
    // for easily write one element in the payload
    ($msg:expr) => {
        $crate::log_orch_internal!({ "action" => $msg });
    };
}
#[macro_export]
macro_rules! log_orch_fn {
    (
        $fn_name:expr
        // section that accept the function arguments
        $(, $param:ident)* // section for generic key-value elements (introduced by ';')
        $(; $($key:expr => $val:expr),*)?
        $(,)?
    ) => {{
        use common_game::logging::{LogEvent, Participant, ActorType, EventType};

        let mut p = std::collections::BTreeMap::new();
        p.insert("fn".to_string(), $fn_name.to_string());

        // adding function arguments
        $(
            p.insert(
                stringify!($param).to_string(),
                format!("{:?}", $param)
            );
        )*

        // generic key-value
        $($(
            p.insert($key.to_string(), $val.to_string());
        )*)?

        LogEvent::self_directed(
            Participant::new(ActorType::Orchestrator, 0u32),
            EventType::InternalOrchestratorAction,
            LOG_FN_CALL_CHNL,
            p
        ).emit();
    }};
}

#[macro_export]
macro_rules! log_message {
    (
        $from_actor:expr, $from_id:expr,
        $to_actor:expr, $to_id:expr,
        $event_type:expr,
        $message:expr
        $(, $param:ident)*
        $(; $($key:expr => $val:expr),*)?
        $(,)?
    ) => {{
        use common_game::logging::{LogEvent, Participant};

        let mut p = std::collections::BTreeMap::new();
        p.insert("message".to_string(), $message.to_string());

        // adding parameters
        $(
            p.insert(
                stringify!($param).to_string(),
                format!("{:?}", $param)
            );
        )*

        // generic key-value pairs
        $($(
            p.insert($key.to_string(), $val.to_string());
        )*)?

        let event = LogEvent::new(
            Some(Participant::new($from_actor, $from_id)),
            Some(Participant::new($to_actor, $to_id)),
            $event_type,
            Channel::Debug,
            p
        );
        event.emit();
    }};
}

const TIMEOUT_DURATION: Duration = Duration::from_millis(2000);

#[cfg(feature = "debug-prints")]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(feature = "debug-prints"))]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        ()
    };
}

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
    pub recevier_orch_planet: Receiver<PlanetToOrchestrator>,

    //Channel to clone for the explorer and for receiving Explorer Messages
    pub sender_explorer_orch: Sender<ExplorerToOrchestrator<BagType>>,
    pub receiver_orch_explorer: Receiver<ExplorerToOrchestrator<BagType>>,
}

//Initialization game functions
impl Orchestrator {
    /// Create a new Galaxy Topology
    /// ` `
    /// Function used as shorthand to create a new
    /// galaxy topology instance
    fn new_gtop() -> GalaxyTopology {
        //Log
        log_orch_fn!("new_gtop()",);
        //LOG


        Arc::new(RwLock::new(Vec::new()))
    }

    //Check and init orchestrator for the test, the comms with the ui are fake
    pub(crate) fn new() -> Result<Self, String> {
        //env_logger initialization
        env_logger::init();
        //Log
        log_orch_fn!("new()",);
        //LOG


        let (sender_planet_orch, recevier_orch_planet) = unbounded();
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
            recevier_orch_planet,
            sender_explorer_orch,
            receiver_orch_explorer,
        };
        Ok(new_orch)
    }

    pub(crate) fn reset(&mut self) -> Result<(), String> {
        //Log
        log_orch_fn!(
            "reset()";
            "procedure"=>"started"
        );
        //LOG

        //send a message every 2000 millis to the ticker receiver
        let timeout = tick(TIMEOUT_DURATION);
        //Kill every thread
        self.send_planet_kill_to_all()?;
        loop {
            select! {
                recv(self.recevier_orch_planet)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(e)=>{
                            //Log
                            let event=LogEvent::self_directed(
                                Participant::new(logging::ActorType::Orchestrator, 0u32),
                                EventType::InternalOrchestratorAction,
                                Channel::Warning,
                                warning_payload!(
                                    "No more sender connected and no messages in the buffer",
                                    e,
                                    "reset()"
                                )
                            );
                            event.emit();
                            //LOG

                            return Err("No more sender connected and no messages in the buffer".to_string())
                        },
                    };
                    match msg_unwraped{
                        PlanetToOrchestrator::KillPlanetResult { planet_id }=>{
                            //Log
                            let event=LogEvent::new(
                                Some(Participant::new(logging::ActorType::Planet, planet_id)),
                                Some(Participant::new(logging::ActorType::Orchestrator, 0u32)),
                                EventType::MessagePlanetToOrchestrator,
                                LOG_ACTORS_ACTIVITY,
                                payload!(
                                    "message"=>"KillPlanetResult",
                                    "planet"=>planet_id,
                                    "status"=>"Dead"
                                )
                            );
                            event.emit();
                            //LOG

                            self.planets_status.write().unwrap().insert(planet_id, Status::Dead);
                            let mut planet_alive=false;
                            for (_, state) in self.planets_status.read().unwrap().iter(){
                                if *state != Status::Dead{
                                    planet_alive=true;
                                    break;
                                }
                            }
                            if !planet_alive{
                                break;
                            }
                        },
                        _=>{}
                    }
                }
                recv(timeout)->_msg=>{
                    //After one second every planet should have been killed
                    for (id, state) in self.planets_status.read().unwrap().iter(){
                        if *state != Status::Dead{
                            //Log
                            let event=LogEvent::new(
                                Some(Participant::new(logging::ActorType::Orchestrator, 0u32)),
                                Some(Participant::new(logging::ActorType::Planet, *id)),
                                EventType::MessageOrchestratorToPlanet,
                                Channel::Warning,
                                warning_payload!(
                                    "Timeout",
                                    "_",
                                    "reset()";
                                    "duration_ms"=>TIMEOUT_DURATION.as_millis()
                                )
                            );
                            event.emit();
                            //LOG

                            return Err("Not every planet is being killed".to_string());
                        }
                    }
                    break;
                }
            }
        }

        //Reinit orchestrator
        self.galaxy_topology = Self::new_gtop();
        self.planets_status = Arc::new(RwLock::new(BTreeMap::new()));
        self.explorer_status = Arc::new(RwLock::new(BTreeMap::new()));
        self.planet_channels = HashMap::new();
        self.explorer_channels = HashMap::new();

        //Log
        log_orch_internal!({"orchestrator reinitialized"=>"galaxy_topology, planets_status, explorer_status, planet_channels, explorer_channels"});

        log_orch_fn!(
            "reset()";
            "procedure"=>"Completed"
        );
        //LOG

        Ok(())
    }

    ///initialize communication channels for planets
    /// needed as a shorthand to initialize OrchestratorToPlanet and ExplorerToPlanet channels
    /// just tu remember: these channels are simplex
    pub(crate) fn init_comms_planet() -> (
        Sender<OrchestratorToPlanet>,
        Receiver<OrchestratorToPlanet>,
        Sender<ExplorerToPlanet>,
        Receiver<ExplorerToPlanet>,
    ) {
        //LOG
        log_orch_fn!("init_comms_planet()");
        //LOG

        //orch-planet
        let (sender_orch, receiver_orch): (
            Sender<OrchestratorToPlanet>,
            Receiver<OrchestratorToPlanet>,
        ) = unbounded();

        //explorer-planet
        let (sender_explorer, receiver_explorer): (
            Sender<ExplorerToPlanet>,
            Receiver<ExplorerToPlanet>,
        ) = unbounded();

        //Log
        log_orch_internal!({
                "action"=>"channels initialized",
                "from"=>"orchestrator, explorer",
                "to"=>"planet"
        });
        //LOG

        (
            sender_orch,
            receiver_orch,
            sender_explorer,
            receiver_explorer,
        )
    }

    ///initialize communication channels for explorer.
    ///
    /// needed as a shorthand to initialize OrchestratorToExplorer and PlanetToExplorer
    ///
    /// Remember that when an explorer goes from a planet to another first the new planet is connected
    /// to the sender side and only after the previous planet is disconnected from the channel. No new channel is created
    ///
    /// just tu remember: these channels are simplex
    ///
    pub(crate) fn init_comms_explorers() -> (
        Sender<OrchestratorToExplorer>,
        Receiver<OrchestratorToExplorer>,
        Sender<PlanetToExplorer>,
        Receiver<PlanetToExplorer>,
    ) {

        //LOG
        log_orch_fn!("init_comms_explorers()");
        //LOG

        let (sender_orch, receiver_orch): (
            Sender<OrchestratorToExplorer>,
            Receiver<OrchestratorToExplorer>,
        ) = unbounded();

        let (sender_planet, receiver_planet): (
            Sender<PlanetToExplorer>,
            Receiver<PlanetToExplorer>,
        ) = unbounded();

        //Log
        log_orch_internal!({
            "action"=>"channels initialized",
            "from"=>"orchestrator, planet",
            "to"=>"explorer"
        });
        //LOG

        (sender_orch, receiver_orch, sender_planet, receiver_planet)
    }
    pub(crate) fn add_planet(&mut self, id: u32, type_id: PlanetType) -> Result<(), String> {

        //LOG
        log_orch_fn!(
            "add_planet()",
            id,
            type_id,
        );
        //LOG

        //Init comms OrchestratorToPlanet, ExplorerToPlanet
        let (sender_orchestrator, receiver_orchestrator, sender_explorer, receiver_explorer) =
            Orchestrator::init_comms_planet();

        //Planet-end of prchestrator-planet/planet-orchestrator channels
        let planet_to_orchestrator_channels =
            (receiver_orchestrator, self.sender_planet_orch.clone());

        //LOG
        log_orch_internal!({
            "action"=>"channel initialized",
            "from"=>"planet",
            "id"=>id,
            "to"=>"orchestrator"
        });
        //LOG

        //creation of the planet

        let mut new_planet = (PLANET_REGISTRY.get(&type_id).unwrap().as_ref())(
            planet_to_orchestrator_channels.0,
            planet_to_orchestrator_channels.1,
            receiver_explorer,
            id,
        )?;

        //LOG
        log_orch_internal!({
                "action"=>"planet created",
                "id"=>id
        });
        //LOG

        //Update HashMaps
        self.planets_status.write().unwrap().insert(new_planet.id(), Status::Paused);
        self.planet_channels
            .insert(new_planet.id(), (sender_orchestrator, sender_explorer));

        debug_println!("Start planet{id} thread");
        thread::spawn(move || -> Result<(), String> { new_planet.run() });

        //LOG
        log_orch_internal!({
            "action"=>"planet thread started",
            "planet_id"=>id
        });
        //LOG
        Ok(())
    }
    pub(crate) fn add_explorer(
        &mut self,
        explorer_id: u32,
        planet_id: u32,
        free_cells: u32,
        sender_explorer: Sender<ExplorerToPlanet>,
    ) {
        log_orch_fn!(
            "add_explorer()",
            explorer_id,
            planet_id,
            free_cells;
            "sender_explorer"=>"Sender<ExplorerToPlanet>"
        );
        //Create the comms for the new explorer
        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        //Construct Explorer
        let new_explorer = Explorer::new(
            explorer_id,
            planet_id,
            (receiver_orch, self.sender_explorer_orch.clone()),
            (receiver_planet, sender_explorer),
            free_cells,
        );

        log_orch_internal!({
            "action"=>"explorer created",
            "explorer_id"=>explorer_id,
        });

        //Update HashMaps
        self.explorer_status
            .write()
            .unwrap()
            .insert(new_explorer.id(), Status::Paused);
        log_orch_internal!({
            "action"=>"explorer_status hashmap updated",
        });
        self.explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet));
        log_orch_internal!({
            "action"=>"saved channels: sender_orch, sender_planet",
        });
        // self.explorers.push(explorer);
        //Spawn the corresponding thread for the explorer
        thread::spawn(|| -> Result<(), String> {
            let _ = new_explorer; //TODO implement a run function for explorer to interact with orchestrator
            Ok(())
        });
        log_orch_internal!({
            "action"=>"explorer thread created",
            "explorer_id"=>explorer_id,
        });
    }
    pub(crate) fn initialize_galaxy_by_file(&mut self, path: &str) -> Result<(), String> {
        //At the moment are allowed only id from 0 to MAX u32
        log_orch_fn!(
            "initialize_galaxy_by_file()",
            path,
        );

        //Read the input file and handle it
        let input = fs::read_to_string(path)
            .map_err(|_| format!("Unable to read the input from {path}"))?;

        let mut adj_list_for_topology = Vec::new();

        let mut new_lookup: FxHashMap<u32, (u32, PlanetType)> = FxHashMap::default();

        for (line_num, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split at comma and u32 conversion
            let values: Vec<u32> = line
                .split(',')
                .map(|s| {
                    s.trim().parse::<u32>().map_err(|_| {
                        format!("Error row {}: value '{}' is not a u32", line_num + 1, s)
                    })
                })
                .collect::<Result<Vec<u32>, String>>()?;

            if values.len() < 2 {
                return Err(format!("Row {}: ID or Type missing", line_num + 1));
            }

            let node_id = values[0];
            let node_type = values[1];
            let neighbors = &values[2..];

            //saving id-index to lookup table
            new_lookup.insert(
                node_id,
                (
                    line_num as u32,
                    match node_type {
                        0 => BlackAdidasShoe,
                        1 => Ciuc,
                        2 => HoustonWeHaveABorrow,
                        3 => ImmutableCosmicBorrow,
                        4 => OneMillionCrabs,
                        5 => Rustrelli,
                        6 => Rustrelli,
                        _ => PlanetType::random(),
                    },
                ),
            );

            let mut adj_row = vec![];
            adj_row.extend_from_slice(neighbors);

            adj_list_for_topology.push(adj_row);
        }
        for row in &mut adj_list_for_topology {
            for node in row {
                if let Some(&(new_idx, _)) = new_lookup.get(node) {
                    *node = new_idx;
                }
            }
        }
        self.galaxy_lookup = new_lookup;
        //Initialize the orchestrator galaxy topology
        self.initialize_galaxy_by_adj_list(adj_list_for_topology)?;

        Ok(())
    }

    pub(crate) fn initialize_galaxy_by_adj_list(
        &mut self,
        adj_list: Vec<Vec<u32>>,
    ) -> Result<(), String> {
        //LOG
        log_orch_fn!(
            "initialize_galaxy_by_adj_list()",
            adj_list
        );
        //LOG
        let num_planets = adj_list.len();
        //Print the result
        debug_println!("Init file content:");
        adj_list
            .iter()
            .for_each(|_row| debug_println!("{:?}", _row));

        //Initialize matrix of adjecencies
        let mut new_topology: Vec<Vec<bool>> = Vec::new();

        for _ in 0..num_planets {
            let v = vec![false; num_planets];
            new_topology.push(v);
        }
        debug_println!("empty adj matrix:");
        new_topology
            .iter()
            .for_each(|_row| debug_println!("{:?}", _row));

        for (idx, row) in adj_list.iter().enumerate() {
            for conn in row.iter() {
                new_topology[idx][*conn as usize] = true;
                new_topology[*conn as usize][idx] = true;
            }
        }

        //LOG
        log_orch_internal!({
            "action"=>"adj matrix created",
            "matrix"=>format!("{:?}",new_topology),
        });
        //LOG

        debug_println!("full adj matrix:");
        new_topology
            .iter()
            .for_each(|_row| debug_println!("{:?}", _row));

        //Update orchestrator topology

        let lock_try = match self.galaxy_topology.write() {
            Ok(mut gtop) => {
                *gtop = new_topology;

                //LOG
                log_orch_internal!({"update galaxy_topology"});
                //LOG

                //drops the lock just in case
                drop(gtop);

                Ok(())
            }
            Err(_e) => {
                //LOG
                let event=LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::InternalOrchestratorAction,
                    Channel::Warning,
                    warning_payload!(
                        "ERROR galaxy topology lock failed.",
                        _e,
                        "initialize_galaxy_by_adj_list()",
                        adj_list
                    )

                );
                event.emit();
                //LOG
                debug_println!("ERROR galaxy topology lock failed.");
                Err(())
            }
        };

        if lock_try.is_ok() {
            //Initialize all the planets give the list of ids
            let ids_list: Vec<u32> = self.galaxy_lookup.keys().map(|x| x.clone()).collect(); //Every row should have at least one ids
            self.initialize_planets_by_ids_list(ids_list.clone())?;
            Ok(())
        } else {
            Err("ERROR galaxy topology lock failed.".to_string())
        }
    }

    pub(crate) fn initialize_planets_by_ids_list(
        &mut self,
        ids_list: Vec<u32>,
    ) -> Result<(), String> {
        //LOG
        log_orch_fn!(
            "initialize_planets_by_ids_list()",
            ids_list,
        );
        //LOG
        for planet_id in ids_list.iter() {
            //TODO we need to initialize the other planets randomly or precisely
            match self.galaxy_lookup.get(&planet_id) {
                None => {
                    //LOG
                    let event=LogEvent::self_directed(
                        Participant::new(ActorType::Orchestrator, 0u32),
                        EventType::InternalOrchestratorAction,
                        Channel::Warning,
                        warning_payload!(
                            format!("Planet ID '{}' not found", planet_id),
                            "_",
                            "initialize_planets_by_ids_list()",
                            ids_list
                        )
                    );
                    event.emit();
                    //LOG
                    return Err(format!("Planet ID '{}' not found", planet_id));
                }
                Some((_, typ)) => {
                    self.add_planet(*planet_id, typ.clone())?;
                }
            };
        }
        Ok(())
    }
}

//Game functions
impl Orchestrator {
    /// Removes the link between two planets if one of them explodes.
    /// ``
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
        log_orch_fn!(
            "destroy_topology_link()",
            planet_one_pos,
            planet_two_pos,
        );
        //LOG

        match self.galaxy_topology.write() {
            Ok(mut gtop) => {
                let gtop_len=gtop.len();
                if planet_one_pos < gtop_len && planet_two_pos < gtop_len {
                    gtop[planet_one_pos][planet_two_pos] = false;
                    gtop[planet_two_pos][planet_one_pos] = false;
                    //LOG
                    log_orch_internal!({
                        "action"=>"adj link destroyed",
                        "updated topology"=>format!("{:?}",gtop),
                    });
                    //LOG
                    drop(gtop);
                    Ok(())
                } else {
                    //LOG
                    let event=LogEvent::self_directed(
                        Participant::new(ActorType::Orchestrator, 0u32),
                        EventType::InternalOrchestratorAction,
                        Channel::Warning,
                        warning_payload!(
                            format!("One of the indexes is out of bounds. upper bound: {}", gtop_len-1),
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
                let event=LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::InternalOrchestratorAction,
                    Channel::Warning,
                    warning_payload!(
                        "ERROR galaxy topology lock failed.",
                        e,
                        "destroy_topology_link()",
                        planet_one_pos,
                        planet_two_pos
                    )

                );
                event.emit();
                //LOG
                debug_println!("RwLock failed for destroy_topology_link");
                Err(e.to_string())
            }
        }
    }

    pub(crate) fn start_all_planet_ais(&mut self) -> Result<(), String> {
        //LOG
        log_orch_fn!("start_all_planet_ais()");
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
        //TODO REVIEW is it possible that this loop could block forevere the game?
        loop {
            if count == self.planet_channels.len() {
                //LOG
                log_orch_internal!({
                    "action"=>"all planets started",
                    "count"=>count
                });
                //LOG
                break;
            }
            let receive_channel = self
                .recevier_orch_planet
                .recv()
                .map_err(|_| "Cannot receive message from planets".to_string())?;
            match receive_channel {
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    debug_println!("Started Planet AI: {}", planet_id);

                    //LOG
                    let event=LogEvent::new(
                        Some(Participant::new(ActorType::Planet, planet_id)),
                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                        EventType::MessagePlanetToOrchestrator,
                        LOG_ACTORS_ACTIVITY,
                        payload!(
                            "message"=>"StartPlanetAIResult",
                            "planet_id"=>planet_id,
                            "status"=>"Running"
                        )
                    );
                    event.emit();
                    //LOG
                    //TODO unwrap così potrebbe panicare
                    self.planets_status.write().unwrap().insert(planet_id, Status::Running);
                    count += 1;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn handle_planet_message(
        &mut self,
        msg: PlanetToOrchestrator,
    ) -> Result<(), String> {
        //LOG
        log_orch_fn!(
            "handle_planet_message()";
            "message_type"=>format!("{:?}", msg)
        );
        //LOG

        match msg {
            PlanetToOrchestrator::SunrayAck { planet_id } => {
                debug_println!("SunrayAck from: {}", planet_id);

                //LOG
                log_message!(
                    ActorType::Planet, planet_id,
                    ActorType::Orchestrator, 0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "SunrayAck",
                    planet_id
                );
                //LOG
            }
            PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                debug_println!("AsteroidAck from: {}", planet_id);
                //LOG
                log_message!(
                    ActorType::Planet, planet_id,
                    ActorType::Orchestrator, 0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "AsteroidAck",
                    planet_id;
                    "has_rocket"=>rocket.is_some()
                );
                //LOG
                match rocket {
                    Some(_) => {
                    }
                    None => {
                        //If you have the id then surely that planet exist so we can unwrap without worring
                        //TODO it seems fine to me but just to be more precise we could add error handling
                        let sender = &self.planet_channels.get(&planet_id).unwrap().0;
                        sender
                            .send(OrchestratorToPlanet::KillPlanet)
                            .map_err(|_| "Unable to send to planet: {planet_id}")?;

                        //LOG
                        log_message!(
                            ActorType::Orchestrator, 0u32,
                            ActorType::Planet, planet_id,
                            EventType::MessageOrchestratorToPlanet,
                            "KillPlanet",
                            planet_id;
                            "reason"=>"no rocket to deflect asteroid"
                        );
                        //LOG

                        //Update planet State
                        self.planets_status.write().unwrap().insert(planet_id, Status::Dead);
                        //LOG
                        log_orch_internal!({
                            "action"=>"planet status updated to Dead",
                            "planet_id"=>planet_id
                        });
                        //LOG
                        //TODO we need to do a check if some explorer is on that planet
                    }
                }
            }
            // PlanetToOrchestrator::IncomingExplorerResponse { planet_id, res }=>{},
            //TODO at this point this functions don't do anything at all
            PlanetToOrchestrator::InternalStateResponse {
                planet_id,
                planet_state,
            } => {
                //LOG
                log_message!(
                    ActorType::Planet, planet_id,
                    ActorType::Orchestrator, 0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "InternalStateResponse",
                    planet_id,
                    planet_state,
                );
                //LOG
            }
            PlanetToOrchestrator::KillPlanetResult { planet_id } => {
                debug_println!("Planet killed: {}", planet_id);
                let event=LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "Message"=>"KillPlanetResult",
                        "planet_id"=>planet_id,
                    )
                );
                event.emit();
            }
            // PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, res }=>{},
            PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                //LOG
                let event=LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"StartPlanetAIResult",
                        "planet_id"=>planet_id
                    )
                );
                event.emit();
                //LOG
            }
            PlanetToOrchestrator::StopPlanetAIResult { planet_id } => {
                //LOG
                let event=LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"StopPlanetAIResult",
                        "planet_id"=>planet_id
                    )
                );
                event.emit();
                //LOG
            }
            PlanetToOrchestrator::Stopped { planet_id } => {
                log_message!(
                    ActorType::Planet, planet_id,
                    ActorType::Orchestrator, 0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "Stopped",
                    planet_id
                )
            }
            _ => {
                let event=LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::MessagePlanetToOrchestrator,
                    Channel::Warning,
                    warning_payload!(
                        "unhandled planet message",
                        "_",
                        "handle_planet_message()";
                    )
                );
                event.emit();
            }
        }
        Ok(())
    }

    //TODO missing planet id in this function, maybe is useful for the logs
    pub(crate) fn send_sunray(&self, sender: &Sender<OrchestratorToPlanet>) -> Result<(), String> {
        //LOG
        log_orch_fn!(
            "send_sunray()";
            "sender"=>"Sender<OrchestratorToPlanet>"
        );
        //LOG
        sender
            .send(OrchestratorToPlanet::Sunray(self.forge.generate_sunray()))
            .map_err(|_| "Unable to send a sunray to planet: {id}".to_string())?;

        //LOG
        log_message!(
            ActorType::Orchestrator, 0u32,
            ActorType::Planet, 0u32, //TODO missing planet id
            EventType::MessageOrchestratorToPlanet,
            "Sunray",

        );
        //LOG
        Ok(())

    }
    pub(crate) fn send_sunray_to_all(&self) -> Result<(), String> {
        //LOG
        log_orch_fn!("send_sunray_to_all()");
        //LOG
        for (id, (sender, _)) in &self.planet_channels {
            if *self.planets_status.read().unwrap().get(id).unwrap() != Status::Dead {
                self.send_sunray(sender)?;
            }
        }
        Ok(())
    }

    pub(crate) fn send_asteroid(
        &self,
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

        //LOG
        log_message!(
            ActorType::Orchestrator, 0u32,
            ActorType::Planet, 0u32, //TODO missing planet id
            EventType::MessageOrchestratorToPlanet,
            "Asteroid",

        );
        //LOG
        Ok(())
    }
    pub(crate) fn send_asteroid_to_all(&self) -> Result<(), String> {
        //LOG
        log_orch_fn!("send_asteroid_to_all()");
        //LOG

        //TODO unwrap cannot fail because every id is contained in the map
        for (id, (sender, _)) in &self.planet_channels {
            if *self.planets_status.read().unwrap().get(id).unwrap() != Status::Dead {
                self.send_asteroid(sender)?;
            }
        }
        Ok(())
    }

    pub(crate) fn send_planet_kill(
        &self,
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

        log_message!(
            ActorType::Orchestrator, 0u32,
            ActorType::Planet, 0u32, //TODO missing planet id
            EventType::MessageOrchestratorToPlanet,
            "KillPlanet",
        );
        Ok(())
    }
    pub(crate) fn send_planet_kill_to_all(&self) -> Result<(), String> {
        //LOG
        log_orch_fn!("send_planet_kill_to_all()");
        //LOG
        for (id, (sender, _)) in &self.planet_channels {
            //unwrap cannot fail because every id is contained in the map
            if *self.planets_status.read().unwrap().get(id).unwrap() != Status::Dead {
                self.send_planet_kill(sender)?;
            }
        }
        Ok(())
    }

    /// Run by the game loop, it should handle the messages from planets and explorers
    pub(crate) fn handle_game_messages(&mut self) -> Result<(), String> {
        //LOG
        log_orch_fn!("handle_game_messages()");
        //LOG
        select! {
            recv(self.recevier_orch_planet)->msg=>{
                let msg_unwraped = match msg{
                    Ok(res)=>res,
                    Err(e)=>{
                        //LOG
                        let event=LogEvent::self_directed(
                            Participant::new(ActorType::Orchestrator, 0u32),
                            EventType::InternalOrchestratorAction,
                            Channel::Warning,
                            warning_payload!(
                                "Cannot receive message from planets",
                                e,
                                "handle_game_messages()"
                            )
                        );
                        event.emit();
                        //LOG
                        return Err("Cannot receive message from planets".to_string())
                    },
                };
                self.handle_planet_message(msg_unwraped)?;
            }
            recv(self.receiver_orch_explorer)->msg=>{
                //TODO to finish this function
                todo!()
            }
            default=>{}
        }

        Ok(())
    }
}
//Functions used by the game
impl Orchestrator {
    pub(crate) fn start_all(&mut self) -> Result<(), String> {
        //LOG
        log_orch_fn!("start_all()");
        //LOG
        self.start_all_planet_ais()?;
        //LOG
        log_orch_internal!({
            "action"=>"all systems started",
            "status"=>"success"
        });
        //LOG
        Ok(())
    }
    pub(crate) fn stop_all(&mut self) -> Result<(), String> {
        //LOG
        log_orch_fn!("stop_all()");
        //LOG
        //TODO
        //LOG
        log_orch_internal!({
            "action"=>"stop_all requested",
            "status"=>"TODO - not implemented" //TODO change thi message
        });
        //LOG
        todo!();
        Ok(())
    }
}

// REVIEW function used for testing or to eliminate
impl Orchestrator {
    pub(crate) fn run_test(file_path: String) -> Result<(), String> {
        //Init and check orchestrator
        let mut orchestrator = Orchestrator::new()?;

        orchestrator.initialize_galaxy_by_file(file_path.as_str().trim())?;
        // orchestrator.run_asteroid_after_five()?;

        // orchestrator.run_sequence_next_probability()?;
        Ok(())
    }

}

//Debug game functions
impl Orchestrator {
    pub(crate) fn print_planets_state(&self) {
        // for (id, status) in &self.planets_status{
        //     print!("({}, {:?})",id, status);
        // }
        debug_println!("{:?}", self.planets_status);
    }
    pub(crate) fn print_galaxy_topology(&self) {
        debug_println!("{:?}", self.galaxy_topology);
    }
    pub(crate) fn print_orch(&self) {
        debug_println!("Orchestrator running");
    }
}

//GUI communication functions
impl Orchestrator {
    /// Get a snapshot of the current galaxy topology
    ///
    /// Returns an atomic reference of the current
    /// galaxy topology. This is made to avoid changing
    /// the topology from the GUI's side in an improper
    /// way that might misalign the internal state
    pub(crate) fn get_topology(&self) -> GalaxyTopology {
        //LOG
        log_orch_fn!("get_topology()");
        //LOG
        self.galaxy_topology.clone()
    }

    pub(crate) fn get_game_status(&self) -> Result<(GalaxyTopology, PlanetStatus, ExplorerStatus), String> {
        //LOG
        log_orch_fn!("get_game_status()");
        //LOG
        Ok((Arc::clone(&self.galaxy_topology), Arc::clone(&self.planets_status), Arc::clone(&self.explorer_status)))
    }
}