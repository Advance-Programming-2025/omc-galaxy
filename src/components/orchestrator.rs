use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::{fs, thread};

use common_game::components::forge::Forge;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};

use crate::components::explorer::{BagType, Explorer};
use one_million_crabs::planet::create_planet;

#[derive(PartialEq)]
pub enum Status {
    Running,
    Paused,
    Dead,
}
// B generic is there for representing the content type of the bag
pub struct Orchestrator {
    // Forge sunray and asteroid
    pub forge: Forge,

    //Galaxy
    pub galaxy_topology: Vec<Vec<bool>>,

    //Status for each planets and explorers
    pub planets_status: HashMap<u32, Status>,
    pub explorer_status: HashMap<u32, Status>,
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

impl Orchestrator {
    //Check and init orchestrator
    pub fn new() -> Result<Self, String> {
        let (sender_planet_orch, recevier_orch_planet) = unbounded();
        let (sender_explorer_orch, receiver_orch_explorer) = unbounded();

        let new_orch = Self {
            forge: Forge::new()?,
            galaxy_topology: Vec::new(),
            planets_status: HashMap::new(),
            explorer_status: HashMap::new(),
            planet_channels: HashMap::new(),
            explorer_channels: HashMap::new(),
            sender_planet_orch,
            recevier_orch_planet,
            sender_explorer_orch,
            receiver_orch_explorer,
        };
        Ok(new_orch)
    }
    fn init_comms_planet() -> (
        Sender<OrchestratorToPlanet>,
        Receiver<OrchestratorToPlanet>,
        Sender<ExplorerToPlanet>,
        Receiver<ExplorerToPlanet>,
    ) {
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

        (
            sender_orch,
            receiver_orch,
            sender_explorer,
            receiver_explorer,
        )
    }
    fn init_comms_explorers() -> (
        Sender<OrchestratorToExplorer>,
        Receiver<OrchestratorToExplorer>,
        Sender<PlanetToExplorer>,
        Receiver<PlanetToExplorer>,
    ) {
        let (sender_orch, receiver_orch): (
            Sender<OrchestratorToExplorer>,
            Receiver<OrchestratorToExplorer>,
        ) = unbounded();

        let (sender_planet, receiver_planet): (
            Sender<PlanetToExplorer>,
            Receiver<PlanetToExplorer>,
        ) = unbounded();

        (sender_orch, receiver_orch, sender_planet, receiver_planet)
    }
    pub fn add_planet(&mut self, id: u32) -> Result<(), String> {
        //Init comms
        let (sender_orchestrator, receiver_orchestrator, sender_explorer, receiver_explorer) =
            Orchestrator::init_comms_planet();

        let planet_to_orchestrator_channels =
            (receiver_orchestrator, self.sender_planet_orch.clone());

        //Construct crab-rave planet
        //REVIEW check if there is a better way to write it
        let mut new_planet = create_planet(
            planet_to_orchestrator_channels.0,
            planet_to_orchestrator_channels.1,
            receiver_explorer,
            id,
        )?;

        //Update HashMaps
        self.planets_status.insert(new_planet.id(), Status::Paused);
        self.planet_channels
            .insert(new_planet.id(), (sender_orchestrator, sender_explorer));

        debug_println!("Start planet{id} thread");
        thread::spawn(move || -> Result<(), String> { new_planet.run() });
        Ok(())
    }
    pub fn add_explorer(&mut self, id: u32) {
        //Create the comms for the new explorer
        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        //Construct Explorer
        let new_explorer = Explorer::new(
            id,
            None,
            (receiver_orch, self.sender_explorer_orch.clone()),
            receiver_planet,
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
        self.add_planet(0)?;
        self.add_planet(1)?;
        Ok(())
    }

    pub fn initialize_galaxy_by_file(&mut self, path: &str) -> Result<(), String> {
        //At the moment are allowed only consecutive id from 0 to MAX u32

        //Read the input file and handle it
        let input = fs::read_to_string(path)
            .map_err(|_| format!("Unable to read the input from {path}"))?;
        let input_refined: Vec<&str> = input.split('\n').collect();

        //Check the input and convert the string into u32
        let input_refined_2 = input_refined
            .iter()
            .map(|row| {
                row.split_ascii_whitespace()
                    .map(|x| {
                        x.parse::<u32>()
                            .map_err(|_| "Unable to convert value to u32".to_string())
                    })
                    .collect::<Result<Vec<u32>, String>>()
            })
            .collect::<Result<Vec<Vec<u32>>, String>>()?;

        //Initialize the orchestrator galaxy topology
        self.initialize_galaxy_by_adj_list(input_refined_2)?;

        Ok(())
    }

    pub fn initialize_galaxy_by_adj_list(&mut self, adj_list: Vec<Vec<u32>>) -> Result<(), String> {
        let num_planets = adj_list.len();
        //Print the result
        debug_println!("Init file content:");
        adj_list.iter().for_each(|row| debug_println!("{:?}", row));

        //Initialize matrix of adjecencies
        let mut new_topology: Vec<Vec<bool>> = Vec::new();
        for _ in 0..num_planets - 1 {
            let v = vec![false; num_planets - 1];
            new_topology.push(v);
        }
        debug_println!("empty adj matrix:");
        new_topology
            .iter()
            .for_each(|row| debug_println!("{:?}", row));

        for row in &adj_list {
            let planet = row[0];
            for (i, conn) in row.iter().enumerate() {
                if i != 0 {
                    new_topology[planet as usize][*conn as usize] = true;
                    new_topology[*conn as usize][planet as usize] = true;
                }
            }
        }

        debug_println!("full adj matrix:");
        new_topology
            .iter()
            .for_each(|row| debug_println!("{:?}", row));

        //Update orchestrator topology
        self.galaxy_topology = new_topology;

        //Initialize all the planets give the list of ids
        let ids_list = adj_list.iter().map(|x| x[0]).collect::<Vec<u32>>(); //Every row should have at least one ids
        self.initialize_planets_by_ids_list(ids_list.clone())?;

        Ok(())
    }

    pub fn initialize_planets_by_ids_list(&mut self, ids_list: Vec<u32>) -> Result<(), String> {
        for planet_id in ids_list {
            //TODO we need to initialize the other planets randomly or precisely
            self.add_planet(planet_id)?;
        }
        Ok(())
    }

    fn start_all_planet_ais(&mut self) -> Result<(), String> {
        for (id, (from_orch, _)) in &self.planet_channels {
            let send_channel = from_orch
                .try_send(OrchestratorToPlanet::StartPlanetAI)
                .map_err(|_| "Cannot send message to {id}".to_string())?;
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
                debug_println!("SunrayAck from: {planet_id}")
            }
            PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                debug_println!("AsteroidAck from: {planet_id}");
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
    fn send_sunray_to_all(&self) -> Result<(), String> {
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
            .map_err(|_| "Unable to send a sunray to planet: {id}".to_string())
    }
    fn send_asteroid_to_all(&self) -> Result<(), String> {
        for (id, (sender, _)) in &self.planet_channels {
            if *self.planets_status.get(id).unwrap() != Status::Dead {
                self.send_asteroid(sender)?;
            }
        }
        Ok(())
    }

    pub fn run_only_planets(&mut self) -> Result<(), String> {
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

    pub fn run_only_planet_sequence(
        &mut self,
        mut asteroid_sunray_list: String,
    ) -> Result<(), String> {
        self.start_all_planet_ais()?;

        //Game
        let start = Instant::now();
        let ticker = tick(Duration::from_millis(100));

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

                    match asteroid_sunray_list.pop(){
                        Some('A')=>self.send_asteroid_to_all()?,
                        Some('S')=>self.send_sunray_to_all()?,
                        _=>break,
                    }
                }
            }
        }

        Ok(())
    }
}
