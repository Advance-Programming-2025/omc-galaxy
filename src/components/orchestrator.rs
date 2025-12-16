//use std::sync::mpsc;
use crossbeam_channel::{Receiver, Sender, select_biased, unbounded};
use std::collections::HashMap;
use std::fmt::format;
use std::{fs, io, thread};

use common_game::components::forge::Forge;
use common_game::components::planet::Planet;
use common_game::protocols::messages::{
    ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer, OrchestratorToPlanet,
    PlanetToExplorer, PlanetToOrchestrator,
};

use crate::components::CrabRaveConstructor;
use crate::components::explorer::{BagType, Explorer};

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
        let (sender_orchestrator, receiver_orchestrator, sender_explorer, receiver_explorer) =
            Orchestrator::init_comms_planet();

        let planet_to_orchestrator_channels =
            (receiver_orchestrator, self.sender_planet_orch.clone());
        //Construct crab-rave planet
        let mut new_planet =
            CrabRaveConstructor::new(id, planet_to_orchestrator_channels, receiver_explorer)?;

        //Update HashMaps
        self.planets_status.insert(new_planet.id(), Status::Paused);
        self.planet_channels
            .insert(new_planet.id(), (sender_orchestrator, sender_explorer));
        //Add new planet id to the list
        // self.planets_id.push(new_planet.id());
        // //Add new planet to the list
        // self.planets.push(new_planet);

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

    pub fn initialize_galaxy(&mut self /*_path: &str*/) -> Result<(), String> {
        let _init_new_planet = self.add_planet(0)?;
        let _init_new_planet = self.add_planet(1)?;
        Ok(())
    }

    pub fn initialize_galaxy_by_file(&mut self, path: &str) -> Result<(), String> {
        //At the moment are allowed only consecutive id from 0 to MAX u32

        //Read the input file and handle it
        let input = fs::read_to_string(path).map_err(|_| format!("Unable to read the input from {path}"))?;
        let input_refined: Vec<&str> = input.split('\n').collect();
        let num_of_planets = input_refined.len();

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

        //Print the result
        input_refined_2.iter().for_each(|row|println!("{:?}", row));

        //Initialize matrix of adjecencies
        

        Ok(())
    }

    //The return is Result<(), String> because if an error occur it go back to the main that finishes
    // I don't know if there are better approach but I think it is pretty elegant
    pub fn run_example(&mut self) -> Result<(), String> {
        //Start all the planets AI
        for (id, (from_orch, _)) in &self.planet_channels {
            let send_channel = from_orch
                .try_send(OrchestratorToPlanet::StartPlanetAI)
                .map_err(|_| "Cannot send message to {id}".to_string())?;
        }
        let mut count = 0;
        //Wait all the
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
                    println!("Started PAI: {}", planet_id);
                    self.planets_status.insert(planet_id, Status::Running);
                    count += 1;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
