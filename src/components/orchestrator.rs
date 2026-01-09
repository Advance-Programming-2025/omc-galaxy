use common_game::components::forge::Forge;
use common_game::logging::Channel;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use crossbeam_channel::select_biased;
use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use std::{fs, thread};

use crate::components::explorer::{BagType, Explorer};
use crate::settings::pop_sunray_asteroid_sequence;
use crate::utils_planets::PLANET_REGISTRY;
use crate::utils_planets::registry::PlanetType;
use crate::utils_planets::registry::PlanetType::{
    BlackAdidasShoe, Ciuc, HoustonWeHaveABorrow, ImmutableCosmicBorrow, OneMillionCrabs, Rustrelli,
};

const LOG_FN_CALL_CHNL: Channel = Channel::Debug;
const LOG_FN_INT_OPERATIONS: Channel = Channel::Trace;
const LOG_ACTORS_ACTIVITY: Channel = Channel::Info;

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

#[derive(PartialEq, Debug)]
pub enum Status {
    Running,
    Paused,
    Dead,
}

pub enum GameState {
    Running,
    Paused,
    Exit,
}

pub type GalaxyTopology = Arc<RwLock<Vec<Vec<bool>>>>;

pub struct Orchestrator {
    // Forge sunray and asteroid
    pub forge: Forge,

    //Galaxy
    pub galaxy_topology: GalaxyTopology,
    pub galaxy_lookup: FxHashMap<u32, (u32, PlanetType)>,

    //Status for each planets and explorers, BTreeMaps are useful for printing
    pub planets_status: BTreeMap<u32, Status>,
    pub explorer_status: BTreeMap<u32, Status>,
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
        //TODO implement proper debug. channel: LOG_FN_CALL_CHNL

        Arc::new(RwLock::new(Vec::new()))
    }

    //Check and init orchestrator for the test, the comms with the ui are fake
    pub fn new() -> Result<Self, String> {
        //TODO implement proper debug. channel: LOG_FN_CALL_CHNL

        let (sender_planet_orch, recevier_orch_planet) = unbounded();
        let (sender_explorer_orch, receiver_orch_explorer) = unbounded();

        let new_orch = Self {
            forge: Forge::new()?,
            galaxy_topology: Self::new_gtop(),
            galaxy_lookup: FxHashMap::default(),
            planets_status: BTreeMap::new(),
            explorer_status: BTreeMap::new(),
            planet_channels: HashMap::new(),
            explorer_channels: HashMap::new(),
            sender_planet_orch,
            recevier_orch_planet,
            sender_explorer_orch,
            receiver_orch_explorer,
        };
        Ok(new_orch)
    }

    pub fn reset(&mut self) -> Result<(), String> {
        //TODO implement proper debug. channel: INFO. LOG_FN_CALL_CHNL. start

        //send a message every 2000 millis to the ticker receiver
        let timeout = tick(TIMEOUT_DURATION);
        //Kill every thread
        self.send_planet_kill_to_all()?;
        loop {
            //TODO implement proper debug. channel: LOG_FN_INT_OPERATIONS
            select! {
                recv(self.recevier_orch_planet)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(_)=>return Err("No more sender connected and no messages in the buffer".to_string()),
                    };
                    match msg_unwraped{
                        PlanetToOrchestrator::KillPlanetResult { planet_id }=>{
                            self.planets_status.insert(planet_id, Status::Dead);
                            let mut planet_alive=false;
                            for (_, state) in &self.planets_status{
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
                    for (_, state) in &self.planets_status{
                        if *state != Status::Dead{
                            return Err("Not every planet is being killed".to_string());
                        }
                    }
                    break;
                }
            }
        }

        //Reinit orchestrator
        self.galaxy_topology = Self::new_gtop();
        self.planets_status = BTreeMap::new();
        self.explorer_status = BTreeMap::new();
        self.planet_channels = HashMap::new();
        self.explorer_channels = HashMap::new();
        Ok(())
        //TODO implement proper debug. channel: LOG_FN_CALL_CHNL. finish
    }

    ///initialize communication channels for planets
    /// needed as a shorthand to initialize OrchestratorToPlanet and ExplorerToPlanet channels
    /// just tu remember: these channels are simplex
    fn init_comms_planet() -> (
        Sender<OrchestratorToPlanet>,
        Receiver<OrchestratorToPlanet>,
        Sender<ExplorerToPlanet>,
        Receiver<ExplorerToPlanet>,
    ) {
        //TODO implement proper debug. channel: LOG_FN_CALL_CHNL
        //TODO implement proper debug. channel: LOG_FN_INT_OPERATIONS
        //orch-planet
        let (sender_orch, receiver_orch): (
            Sender<OrchestratorToPlanet>,
            Receiver<OrchestratorToPlanet>,
        ) = unbounded();

        //TODO implement proper debug. channel: LOG_FN_INT_OPERATIONS
        //explorer-planet
        let (sender_explorer, receiver_explorer): (
            Sender<ExplorerToPlanet>,
            Receiver<ExplorerToPlanet>,
        ) = unbounded();

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
    fn init_comms_explorers() -> (
        Sender<OrchestratorToExplorer>,
        Receiver<OrchestratorToExplorer>,
        Sender<PlanetToExplorer>,
        Receiver<PlanetToExplorer>,
    ) {
        //TODO implement proper debug. channel: LOG_FN_CALL_CHNL

        //TODO implement proper debug. channel: LOG_FN_INT_OPERATIONS
        let (sender_orch, receiver_orch): (
            Sender<OrchestratorToExplorer>,
            Receiver<OrchestratorToExplorer>,
        ) = unbounded();

        //TODO implement proper debug. channel: LOG_FN_INT_OPERATIONS
        let (sender_planet, receiver_planet): (
            Sender<PlanetToExplorer>,
            Receiver<PlanetToExplorer>,
        ) = unbounded();

        (sender_orch, receiver_orch, sender_planet, receiver_planet)
    }
    pub fn add_planet(&mut self, id: u32, type_id: PlanetType) -> Result<(), String> {
        //TODO implement proper debug. channel: LOG_FN_CALL_CHNL
        //Init comms OrchestratorToPlanet, ExplorerToPlanet
        let (sender_orchestrator, receiver_orchestrator, sender_explorer, receiver_explorer) =
            Orchestrator::init_comms_planet();

        //Planet-end of prchestrator-planet/planet-orchestrator channels
        let planet_to_orchestrator_channels =
            (receiver_orchestrator, self.sender_planet_orch.clone());

        //TODO implement proper debug. channel: LOG_ACTORS_ACTIVITY
        //creation of the planet

        let mut new_planet = (PLANET_REGISTRY.get(&type_id).unwrap().as_ref())(
            planet_to_orchestrator_channels.0,
            planet_to_orchestrator_channels.1,
            receiver_explorer,
            id,
        )?;

        //TODO implement proper debug. channel: LOG_FN_INT_OPERATIONS
        //Update HashMaps
        self.planets_status.insert(new_planet.id(), Status::Paused);
        self.planet_channels
            .insert(new_planet.id(), (sender_orchestrator, sender_explorer));

        debug_println!("Start planet{id} thread");
        thread::spawn(move || -> Result<(), String> { new_planet.run() });
        Ok(())
    }
    pub fn add_explorer(
        &mut self,
        explorer_id: u32,
        planet_id: u32,
        free_cells: u32,
        sender_explorer: Sender<ExplorerToPlanet>,
    ) {
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

        //Update HashMaps
        self.explorer_status
            .insert(new_explorer.id(), Status::Paused);
        self.explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet));

        // self.explorers.push(explorer);
        //Spawn the corresponding thread for the explorer
        thread::spawn(|| -> Result<(), String> {
            let _ = new_explorer; //TODO implement a run function for explorer to interact with orchestrator
            Ok(())
        });
    }

    pub fn initialize_galaxy_example(&mut self /*_path: &str*/) -> Result<(), String> {
        self.add_planet(0, OneMillionCrabs)?;
        self.add_planet(1, OneMillionCrabs)?;
        Ok(())
    }
    pub fn initialize_galaxy_by_file(&mut self, path: &str) -> Result<(), String> {
        //At the moment are allowed only consecutive id from 0 to MAX u32

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

    pub fn initialize_galaxy_by_adj_list(&mut self, adj_list: Vec<Vec<u32>>) -> Result<(), String> {
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

        debug_println!("full adj matrix:");
        new_topology
            .iter()
            .for_each(|_row| debug_println!("{:?}", _row));

        //Update orchestrator topology

        let lock_try = match self.galaxy_topology.write() {
            Ok(mut gtop) => {
                *gtop = new_topology;
                //drops the lock just in case
                drop(gtop);
                Ok(())
            }
            Err(_e) => {
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
            Err("rwlock error".to_string())
        }
    }

    pub fn initialize_planets_by_ids_list(&mut self, ids_list: Vec<u32>) -> Result<(), String> {
        let mut err = false;
        for planet_id in ids_list {
            //TODO we need to initialize the other planets randomly or precisely
            match self.galaxy_lookup.get(&planet_id) {
                None => {
                    err = true;
                    break;
                }
                Some((_, typ)) => {
                    self.add_planet(planet_id, typ.clone())?;
                }
            };
        }
        match err {
            false => Ok(()),
            true => Err("no planet type found".to_string()),
        }
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
    fn destroy_topology_link(
        &mut self,
        planet_one_pos: usize,
        planet_two_pos: usize,
    ) -> Result<(), String> {
        match self.galaxy_topology.write() {
            Ok(mut gtop) => {
                if planet_one_pos < gtop.len() && planet_two_pos < gtop.len() {
                    gtop[planet_one_pos][planet_two_pos] = false;
                    gtop[planet_two_pos][planet_one_pos] = false;
                    drop(gtop);
                    Ok(())
                } else {
                    Err("index out of bounds (too large)".to_string())
                }
            }
            Err(e) => {
                debug_println!("RwLock failed for destroy_topology_link");
                Err(e.to_string())
            }
        }
    }

    fn start_all_planet_ais(&mut self) -> Result<(), String> {
        for (_id, (from_orch, _)) in &self.planet_channels {
            from_orch
                .try_send(OrchestratorToPlanet::StartPlanetAI)
                .map_err(|_| "Cannot send message to {_id}".to_string())?;
        }

        let mut count = 0;
        //REVIEW is it possible that this loop could block forevere the game?
        loop {
            if count == self.planet_channels.len() {
                break;
            }
            let receive_channel = self
                .recevier_orch_planet
                .recv()
                .map_err(|_| "Cannot receive message from planets".to_string())?;
            match receive_channel {
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    debug_println!("Started Planet AI: {}", planet_id);
                    self.planets_status.insert(planet_id, Status::Running);
                    count += 1;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_planet_message(&mut self, msg: PlanetToOrchestrator) -> Result<(), String> {
        match msg {
            PlanetToOrchestrator::SunrayAck { planet_id } => {
                debug_println!("SunrayAck from: {}", planet_id)
            }
            PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                debug_println!("AsteroidAck from: {}", planet_id);
                match rocket {
                    Some(_) => {
                        //TODO some logging function
                    }
                    None => {
                        //If you have the id then surely that planet exist so we can unwrap without worring
                        let sender = &self.planet_channels.get(&planet_id).unwrap().0;
                        sender
                            .send(OrchestratorToPlanet::KillPlanet)
                            .map_err(|_| "Unable to send to planet: {planet_id}")?;

                        //Update planet State
                        self.planets_status.insert(planet_id, Status::Dead);
                        //TODO we need to do a check if some explorer is on that planet
                    }
                }
            }
            // PlanetToOrchestrator::IncomingExplorerResponse { planet_id, res }=>{},
            PlanetToOrchestrator::InternalStateResponse {
                planet_id,
                planet_state,
            } => {}
            PlanetToOrchestrator::KillPlanetResult { planet_id } => {
                debug_println!("Planet killed: {}", planet_id);
            }
            // PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, res }=>{},
            PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {}
            PlanetToOrchestrator::StopPlanetAIResult { planet_id } => {}
            PlanetToOrchestrator::Stopped { planet_id } => {}
            _ => {}
        }
        Ok(())
    }

    fn send_sunray(&self, sender: &Sender<OrchestratorToPlanet>) -> Result<(), String> {
        sender
            .send(OrchestratorToPlanet::Sunray(self.forge.generate_sunray()))
            .map_err(|_| "Unable to send a sunray to planet: {id}".to_string())
    }
    pub(crate) fn send_sunray_to_all(&self) -> Result<(), String> {
        for (id, (sender, _)) in &self.planet_channels {
            if *self.planets_status.get(id).unwrap() != Status::Dead {
                self.send_sunray(sender)?;
            }
        }
        Ok(())
    }

    fn send_asteroid(&self, sender: &Sender<OrchestratorToPlanet>) -> Result<(), String> {
        sender
            .send(OrchestratorToPlanet::Asteroid(
                self.forge.generate_asteroid(),
            ))
            .map_err(|_| "Unable to send sunray to planet: {id}".to_string())
    }
    pub(crate) fn send_asteroid_to_all(&self) -> Result<(), String> {
        //unwrap cannot fail because every id is contained in the map
        for (id, (sender, _)) in &self.planet_channels {
            if *self.planets_status.get(id).unwrap() != Status::Dead {
                self.send_asteroid(sender)?;
            }
        }
        Ok(())
    }

    fn send_planet_kill(&self, sender: &Sender<OrchestratorToPlanet>) -> Result<(), String> {
        sender
            .send(OrchestratorToPlanet::KillPlanet)
            .map_err(|_| "Unable to send kill message to planet: {id}".to_string())
    }
    pub(crate) fn send_planet_kill_to_all(&self) -> Result<(), String> {
        for (id, (sender, _)) in &self.planet_channels {
            //unwrap cannot fail because every id is contained in the map
            if *self.planets_status.get(id).unwrap() != Status::Dead {
                self.send_planet_kill(sender)?;
            }
        }
        Ok(())
    }

    /// Run by the game loop, it should handle the messages from planets and explorers
    pub(crate) fn handle_game_messages(&mut self) -> Result<(), String> {
        select! {
            recv(self.recevier_orch_planet)->msg=>{
                let msg_unwraped = match msg{
                    Ok(res)=>res,
                    Err(_)=>return Err("Cannot receive message from planets".to_string()),
                };
                self.handle_planet_message(msg_unwraped)?;
            }
            recv(self.receiver_orch_explorer)->msg=>{
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
        self.start_all_planet_ais()?;
        Ok(())
    }
    pub(crate) fn stop_all(&mut self) -> Result<(), String> {
        todo!();
        Ok(())
    }
}

// REVIEW function used for testing or to eliminate
impl Orchestrator {
    pub fn run_test(file_path: String) -> Result<(), String> {
        //Init and check orchestrator
        let mut orchestrator = Orchestrator::new()?;

        orchestrator.initialize_galaxy_by_file(file_path.as_str().trim())?;
        // orchestrator.run_asteroid_after_five()?;

        // orchestrator.run_sequence_next_probability()?;
        Ok(())
    }

    pub(crate) fn run_asteroid_after_five(&mut self) -> Result<(), String> {
        //Loop to start all planet ais
        self.start_all_planet_ais()?;

        //Game

        /*
            v0 - totally sequencial
            Every message is responded to bloking all the other channels till it is finished
            Sunrays and asteroids are sent to all the planet after a timeout
        */
        let start = Instant::now();
        let ticker = tick(Duration::from_millis(100));
        let mut count = 0;

        loop {
            select! {
                recv(self.recevier_orch_planet)->msg=>{
                    let msg_unwraped = match msg{
                        Ok(res)=>res,
                        Err(_)=>return Err("Cannot receive message from planets".to_string()),
                    };
                    self.handle_planet_message(msg_unwraped)?;
                }
                recv(self.receiver_orch_explorer)->msg=>{
                    break;
                    todo!()
                }
                recv(ticker)->time=>{
                    debug_println!("{:?}", start.elapsed());

                    if count!=4{
                        self.send_sunray_to_all()?;
                    }else{
                        self.send_asteroid_to_all()?;
                    }
                    count+=1;
                    count%=5;

                }
            }
        }

        Ok(())
    }
}

//Debug game functions
impl Orchestrator {
    pub fn print_planets_state(&self) {
        // for (id, status) in &self.planets_status{
        //     print!("({}, {:?})",id, status);
        // }
        debug_println!("{:?}", self.planets_status);
    }
    pub fn print_galaxy_topology(&self) {
        debug_println!("{:?}", self.galaxy_topology);
    }
    pub fn print_orch(&self) {
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
    pub fn get_topology(&self) -> GalaxyTopology {
        self.galaxy_topology.clone()
    }
}
