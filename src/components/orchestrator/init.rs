use logging_utils::LoggableActor;
use std::{
    fs,
    sync::{Arc, RwLock},
    thread,
};

use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::{
        orchestrator_explorer::OrchestratorToExplorer,
        orchestrator_planet::OrchestratorToPlanet,
        planet_explorer::{ExplorerToPlanet, PlanetToExplorer},
    },
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use rustc_hash::FxHashMap;

use super::Orchestrator;
use crate::utils::registry::PlanetType::{
    BlackAdidasShoe, Ciuc, HoustonWeHaveABorrow, ImmutableCosmicBorrow, OneMillionCrabs, Rustrelli,
};
use crate::{
    GalaxyTopology,
    components::explorer::Explorer,
    utils::{
        Status,
        registry::{PLANET_REGISTRY, PlanetType},
    },
};

use logging_utils::{debug_println, log_fn_call, log_internal_op, warning_payload};

//Initialization game functions
impl Orchestrator {
    /// Create a new Galaxy Topology.
    ///
    /// This function is used as shorthand to create a new galaxy topology instance.
    /// Returns an atomic reference containing the galaxy's structure.
    pub fn new_gtop() -> GalaxyTopology {
        //Log
        log_fn_call!(dir ActorType::Orchestrator, 0u32, "new_gtop()",);
        //LOG
        Arc::new(RwLock::new(Vec::new()))
    }

    /// Reset the orchestrator.
    ///
    /// Reset the orchestrator by killing all planets.
    /// Returns Err if any planet is still alive after one second,
    /// or if no sender is connected and the message buffer
    /// is empty.
    pub(crate) fn reset(&mut self) -> Result<(), String> {
        // //Log
        // log_orch_fn!(
        //     "reset()";
        //     "procedure"=>"started"
        // );
        // //LOG

        // //send a message every 2000 millis to the ticker receiver
        // let timeout = tick(TIMEOUT_DURATION);
        // //Kill every thread
        // self.send_planet_kill_to_all()?;
        // loop {
        //     select! {
        //         recv(self.receiver_orch_planet)->msg=>{
        //             let msg_unwrapped = match msg{
        //                 Ok(res)=>res,
        //                 Err(e)=>{
        //                     //Log
        //                     let event=LogEvent::self_directed(
        //                         Participant::new(logging::ActorType::Orchestrator, 0u32),
        //                         EventType::InternalOrchestratorAction,
        //                         Channel::Warning,
        //                         warning_payload!(
        //                             "No more sender connected and no messages in the buffer",
        //                             e,
        //                             "reset()"
        //                         )
        //                     );
        //                     event.emit();
        //                     //LOG

        //                     return Err("No more sender connected and no messages in the buffer".to_string())
        //                 },
        //             };
        //             match msg_unwrapped{
        //                 PlanetToOrchestrator::KillPlanetResult { planet_id }=>{
        //                     //Log
        //                     let event=LogEvent::new(
        //                         Some(Participant::new(logging::ActorType::Planet, planet_id)),
        //                         Some(Participant::new(logging::ActorType::Orchestrator, 0u32)),
        //                         EventType::MessagePlanetToOrchestrator,
        //                         LOG_ACTORS_ACTIVITY,
        //                         payload!(
        //                             "message"=>"KillPlanetResult",
        //                             "planet"=>planet_id,
        //                             "status"=>"Dead"
        //                         )
        //                     );
        //                     event.emit();
        //                     //LOG

        //                     self.planets_status.write().unwrap().insert(planet_id, Status::Dead);
        //                     let mut planet_alive=false;
        //                     for (_, state) in self.planets_status.read().unwrap().iter(){
        //                         if *state != Status::Dead{
        //                             planet_alive=true;
        //                             break;
        //                         }
        //                     }
        //                     if !planet_alive{
        //                         break;
        //                     }
        //                 },
        //                 _=>{}
        //             }
        //         }
        //         recv(timeout)->_msg=>{
        //             //After one second every planet should have been killed
        //             for (id, state) in self.planets_status.read().unwrap().iter(){
        //                 if *state != Status::Dead{
        //                     //Log
        //                     let event=LogEvent::new(
        //                         Some(Participant::new(logging::ActorType::Orchestrator, 0u32)),
        //                         Some(Participant::new(logging::ActorType::Planet, *id)),
        //                         EventType::MessageOrchestratorToPlanet,
        //                         Channel::Warning,
        //                         warning_payload!(
        //                             "Timeout",
        //                             "_",
        //                             "reset()";
        //                             "duration_ms"=>TIMEOUT_DURATION.as_millis()
        //                         )
        //                     );
        //                     event.emit();
        //                     //LOG

        //                     return Err("Not every planet is being killed".to_string());
        //                 }
        //             }
        //             break;
        //         }
        //     }
        // }

        // //Reinit orchestrator
        // self.galaxy_topology = Self::new_gtop();
        // self.planets_status = Arc::new(RwLock::new(BTreeMap::new()));
        // self.explorer_status = Arc::new(RwLock::new(BTreeMap::new()));
        // self.planet_channels = HashMap::new();
        // self.explorer_channels = HashMap::new();

        // //Log
        // log_orch_internal!({"orchestrator reinitialized"=>"galaxy_topology, planets_status, explorer_status, planet_channels, explorer_channels"});

        // log_orch_fn!(
        //     "reset()";
        //     "procedure"=>"Completed"
        // );
        // //LOG

        Ok(())
    }

    /// Initialize communication channels for planets.
    ///
    /// needed as a shorthand to initialize the OrchestratorToPlanet and ExplorerToPlanet channels |
    /// NOTE: these channels are simplex.
    pub(crate) fn init_comms_planet() -> (
        Sender<OrchestratorToPlanet>,
        Receiver<OrchestratorToPlanet>,
        Sender<ExplorerToPlanet>,
        Receiver<ExplorerToPlanet>,
    ) {
        //LOG
        log_fn_call!(dir ActorType::Orchestrator, 0u32, "init_comms_planet()");
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
        log_internal_op!(dir ActorType::Orchestrator, 0u32,
                "action"=>"channels initialized",
                "from"=>"orchestrator, explorer",
                "to"=>"planet"
        );
        //LOG

        (
            sender_orch,
            receiver_orch,
            sender_explorer,
            receiver_explorer,
        )
    }

    /// Initialize the communication channels for an explorer.
    ///
    /// Needed as a shorthand to initialize OrchestratorToExplorer and PlanetToExplorer.
    ///
    /// This function is NOT supposed to be used when an explorer gets moved: first the
    ///  new planet is connected to the pre-existing sender side and, only after that
    /// happens, the previous planet is disconnected from the channel. No new channel
    /// is created. See function [`add_explorer`](Self::add_explorer).
    ///
    /// NOTE: These channels are simplex.
    pub(crate) fn init_comms_explorers() -> (
        Sender<OrchestratorToExplorer>,
        Receiver<OrchestratorToExplorer>,
        Sender<PlanetToExplorer>,
        Receiver<PlanetToExplorer>,
    ) {
        //LOG
        log_fn_call!(dir ActorType::Orchestrator, 0u32, "init_comms_explorers()");
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
        log_internal_op!(dir ActorType::Orchestrator, 0u32, 
            "action"=>"channels initialized",
            "from"=>"orchestrator, planet",
            "to"=>"explorer"
        );
        //LOG

        (sender_orch, receiver_orch, sender_planet, receiver_planet)
    }

    /// Add a new planet to the orchestrator.
    ///
    /// Adds a new planet inside the orchestrator state, using the internal planet
    /// registry. It first creates the planet object, then adds it to the galaxy lookup
    /// hashmap and starts the planet thread.
    ///
    /// Returns Err if the planet registry closure fails, which means that the planet
    /// could not be instantiated.
    ///
    /// * `id` - id of the planet
    /// * `type_id` - the type of the planet (A,B,C,D)
    pub(crate) fn add_planet(&mut self, id: u32, type_id: PlanetType) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "add_planet()", id, type_id,);
        //LOG

        //Init comms OrchestratorToPlanet, ExplorerToPlanet
        let (sender_orchestrator, receiver_orchestrator, sender_explorer, receiver_explorer) =
            Orchestrator::init_comms_planet();

        //Planet-end of orchestrator-planet/planet-orchestrator channels
        let planet_to_orchestrator_channels =
            (receiver_orchestrator, self.sender_planet_orch.clone());

        //LOG
        log_internal_op!(
            self,
            "action"=>"channel initialized",
            "from"=>"planet",
            "id"=>id,
            "to"=>"orchestrator"
        );
        //LOG

        //creation of the planet

        let mut new_planet = (PLANET_REGISTRY.get(&type_id).unwrap().as_ref())(
            planet_to_orchestrator_channels.0,
            planet_to_orchestrator_channels.1,
            receiver_explorer,
            id,
        )?;

        //LOG
        log_internal_op!(
                self,
                "action"=>"planet created",
                "id"=>id
        );
        //LOG

        //Update HashMaps
        self.planets_status
            .write()
            .unwrap()
            .insert(new_planet.id(), Status::Paused);
        self.planet_channels
            .insert(new_planet.id(), (sender_orchestrator, sender_explorer));

        debug_println!("Start planet{id} thread");
        thread::spawn(move || -> Result<(), String> { new_planet.run() });

        //LOG
        log_internal_op!(
            self, 
            "action"=>"planet thread started",
            "planet_id"=>id
        );
        //LOG
        Ok(())
    }

    /// Add a new explorer to the orchestrator.
    ///
    /// Adds a new explorer inside the orchestrator state; it first creates the
    /// necessary channels and the explorer instance, then adds it to the explorer
    /// status hashmap and starts the explorer itself.
    ///
    /// * `explorer_id` - id of the new explorer
    /// * `planet_id` - id of the planet the explorer will be spawned on
    /// * `free_cells` - the amount of currently free cells in the visiting planet
    /// * `sender_explorer` - pre-existing explorer to planet channel
    pub(crate) fn add_explorer(
        &mut self,
        explorer_id: u32,
        planet_id: u32,
        free_cells: u32,
        sender_explorer: Sender<ExplorerToPlanet>,
    ) {
        log_fn_call!(
            self,
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

        log_internal_op!(
            self,
            "action"=>"explorer created",
            "explorer_id"=>explorer_id,
        );

        //Update HashMaps
        self.explorer_status
            .write()
            .unwrap()
            .insert(new_explorer.id(), Status::Paused);
        log_internal_op!(
            self,
            "action"=>"explorer_status hashmap updated",
        );
        self.explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet));
        log_internal_op!(
            self,
            "action"=>"saved channels: sender_orch, sender_planet",
        );
        // self.explorers.push(explorer);
        //Spawn the corresponding thread for the explorer
        thread::spawn(|| -> Result<(), String> {
            let _ = new_explorer; //TODO implement a run function for explorer to interact with orchestrator
            Ok(())
        });
        log_internal_op!(
            self, 
            "action"=>"explorer thread created",
            "explorer_id"=>explorer_id,
        );
    }

    /// Initialize the galaxy using a topology file.
    ///
    /// Uses the galaxy topology file (which should be based on the INPUT_FILE
    /// environment variable, set in the .env file of the project) and performs
    /// parsing operations to pass it on to
    /// [`initialize_galaxy_by_adj_list`](Self::initialize_galaxy_by_adj_list).
    ///
    /// Returns Err if the file is formatted incorrectly or if any of the following
    /// initialization functions return Err as well.
    ///
    /// * `path` - path to the galaxy initialization file
    pub fn initialize_galaxy_by_file(&mut self, path: &str) -> Result<(), String> {
        //At the moment are allowed only id from 0 to MAX u32
        log_fn_call!(self, "initialize_galaxy_by_file()", path,);

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

    /// Initialize the galaxy using an adjacency list.
    ///
    /// This function is normally called by
    /// [`initialize_galaxy_by_file`](`Self::initialize_galaxy_by_file`), who in
    /// turn hands off control to
    /// [`initialize_planets_by_ids_list`](Self::initialize_planets_by_ids_list).
    /// The function is thread safe thanks to the use of RwLock, even though no
    /// other threads should request the galaxy topology during initialization.
    ///
    /// Returns Err if RwLock fails to lock on a write or if the following function in
    /// the initialization chain fails as well.
    ///
    /// * `adj_list` - a two dimensional matrix,
    ///  parsed by `initialize_galaxy_by_file`
    pub(crate) fn initialize_galaxy_by_adj_list(
        &mut self,
        adj_list: Vec<Vec<u32>>,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "initialize_galaxy_by_adj_list()", adj_list);
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
        log_internal_op!(
            self,
            "action"=>"adj matrix created",
            "matrix"=>format!("{:?}",new_topology),
        );
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
                log_internal_op!(self, "update galaxy_topology" );
                //LOG

                //drops the lock just in case
                drop(gtop);

                Ok(())
            }
            Err(_e) => {
                //LOG
                let event = LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::InternalOrchestratorAction,
                    Channel::Warning,
                    warning_payload!(
                        "ERROR galaxy topology lock failed.",
                        _e,
                        "initialize_galaxy_by_adj_list()",
                        adj_list
                    ),
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

    /// Initialize the galaxy using a list of planet IDs.
    ///
    /// This function is normally called by
    /// [`initialize_galaxy_by_adj_list`](Self::initialize_galaxy_by_adj_list). The
    /// IDs given as input are given to [`add_planet`](Self::add_planet) to start every planet thread
    /// with the necessary information.
    ///
    /// Returns Err if the planet ID isn't valid or if the [`add planet`]
    /// function returns Err as well.
    ///
    /// * `ids_list` - list of planet IDs, parsed by `initialize_galaxy_by_adj_list`
    pub(crate) fn initialize_planets_by_ids_list(
        &mut self,
        ids_list: Vec<u32>,
    ) -> Result<(), String> {
        //LOG
        log_fn_call!(self, "initialize_planets_by_ids_list()", ids_list,);
        //LOG
        for planet_id in ids_list.iter() {
            //TODO we need to initialize the other planets randomly or precisely
            match self.galaxy_lookup.get(&planet_id) {
                None => {
                    //LOG
                    let event = LogEvent::self_directed(
                        Participant::new(ActorType::Orchestrator, 0u32),
                        EventType::InternalOrchestratorAction,
                        Channel::Warning,
                        warning_payload!(
                            format!("Planet ID '{}' not found", planet_id),
                            "_",
                            "initialize_planets_by_ids_list()",
                            ids_list
                        ),
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
