use std::sync::mpsc;
use std::thread;

use common_game::components::generator::Generator;
use common_game::components::planet::Planet;
use common_game::protocols::messages::{
    ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer, OrchestratorToPlanet,
    PlanetToExplorer, PlanetToOrchestrator,
};

use crate::components::CrabRaveConstructor;
use crate::components::explorer::Explorer;

// B generic is there for representing the content type of the bag
pub struct Orchestrator {
    generator: Generator,

    galaxy_topology: Vec<Planet>, //At the moment we need only one planet for testing
    explorers: Vec<Explorer>,     //At the momet we need only one explorer for testing

    //we can better define communication like this: galaxy_communication: Option<HashMap<u32,u32>>
    planet_channels: (
        mpsc::Receiver<PlanetToOrchestrator>,
        mpsc::Sender<OrchestratorToPlanet>,
    ),
    explorer_channels: Option<(
        mpsc::Receiver<ExplorerToOrchestrator>,
        mpsc::Sender<OrchestratorToExplorer>,
    )>,
}

impl Orchestrator {
    //Check and init orchestrator
    pub fn new() -> Result<Self, String> {
        Orchestrator::initialize_galaxy()
    }
    
    //The return is Result<(), String> because if an error occur it go back to the main that finishes
    // I don't know if there are better approach but I think it is pretty elegant

    pub fn initialize_galaxy(/*_path: &str*/) -> Result<Orchestrator, String> {
        // Orchestrator know the file path where the galaxy topology is written and also the type of each planet
        /*
            Steps of initialization:
            1. read the line to make a planet (at the moment one planet so there is no loop and linear implementation)
            2. generate the id - the id generator methos should be on the orchestrator cause is the one to define everything
            3. generate all the communication channels with the planet
            3. generate the planet - if it fails then handle the error
            4. if planet is generated succefully then add it to the topology

         */
        //planet-orch and orch-planet
        let (planet_sender, orch_receiver): (
            mpsc::Sender<PlanetToOrchestrator>,
            mpsc::Receiver<PlanetToOrchestrator>,
        ) = mpsc::channel();
        let (orch_sender, planet_receiver): (
            mpsc::Sender<OrchestratorToPlanet>,
            mpsc::Receiver<OrchestratorToPlanet>,
        ) = mpsc::channel();

        let planet_to_orchestrator_channels = (planet_receiver, planet_sender);
        let orchestrator_to_planet_channels = (orch_receiver, orch_sender);

        //planet-explorer and explorer-planet
        let (planet_sender, explorer_receiver): (
            mpsc::Sender<PlanetToExplorer>,
            mpsc::Receiver<PlanetToExplorer>,
        ) = mpsc::channel();
        let (explorer_sender, planet_receiver): (
            mpsc::Sender<ExplorerToPlanet>,
            mpsc::Receiver<ExplorerToPlanet>,
        ) = mpsc::channel();

        let planet_to_explorer_channels = (planet_receiver, planet_sender);
        let explorer_to_planet_channels = (explorer_receiver, explorer_sender);

        //explorer-orchestrator and orchestrator-explorer
        let (explorer_sender, orch_receiver): (
            mpsc::Sender<ExplorerToOrchestrator>,
            mpsc::Receiver<ExplorerToOrchestrator>,
        ) = mpsc::channel();
        let (orch_sender, explorer_receiver): (
            mpsc::Sender<OrchestratorToExplorer>,
            mpsc::Receiver<OrchestratorToExplorer>,
        ) = mpsc::channel();

        let explorer_to_orchestrator_channels = (explorer_receiver, explorer_sender);
        let orchestrator_to_explorer_channels = (orch_receiver, orch_sender);

        //Construct crab-rave planet
        let mut crab_rave_planet = CrabRaveConstructor::new(
            0,
            planet_to_orchestrator_channels,
            planet_to_explorer_channels,
        )?;
        crab_rave_planet.run();
        // self.planet_channels = Some(orchestrator_to_planet_channels);
        // self.explorer_channels = Some(orchestrator_to_explorer_channels);
        //Add the constructed galaxy to our Orchestrator
        let galaxy = vec![crab_rave_planet];

        //Construct Explorer
        let explorer = Explorer::new(Some(galaxy[0].id()), explorer_to_orchestrator_channels, explorer_to_planet_channels);
        let explorers = vec![explorer];
        Ok(
            Self { 
                generator: Generator::new()?, 
                galaxy_topology: galaxy, 
                explorers, 
                planet_channels: orchestrator_to_planet_channels, 
                explorer_channels: Some(orchestrator_to_explorer_channels) 
            }
        )
    }


    pub fn run(&mut self)->Result<(),String>{
        thread::spawn(||{
            &self.galaxy_topology[0]::run();
        });
        loop{
            
        }
        Ok(())
    }

    // fn make_planet<T: PlanetAI>(&self, init_sting: String) -> Planet<T> {
    //     let gen_rules = vec![Carbon];
    //     Planet::new(0, PlanetType::C, ai, , comb_rules, orchestrator_channels, explorer_channels)
    // }
}

// struct Dummy;
// impl Dummy{
//     fn new()->Self{
//         Dummy
//     }
// }

/*
    Implementazioni presenti nelle prime versioni dell'orchestrator,
    salvate qua perchè sarebbe utile utilizzarle e avere presente quali funzioni dovremmo implementare
impl OrchestratorTrait for Orchestrator{
    fn combine_resource_request<T, E>(&self, msg: CombineResourceRequest) -> Result<T, E> {
        todo!()
    }
    fn current_planet<T, E>(&self, msg: CurrentPlanetRequest) -> Result<T, E> {
        todo!()
    }
    fn generate_resource_request<T, E>(&self, msg: GenerateResourceRequest) -> Result<T, E> {
        todo!()
    }

    fn make_explorer(&self) -> Explorer {
        todo!()
    }

    fn move_to_planet<T, E>(&self, msg: MoveToPlanet) -> Result<T, E> {
        todo!()
    }
    fn reset_explorer_ai<T, E>(&self, msg: ResetExplorerAIMsg, explorer_id: u32) -> Result<T, E> {
        todo!()
    }
    fn send_asteroid<T, E>(&self, a: Asteroid, planet_id: u32) -> Result<T, E> {
        todo!()
    }
    fn send_sunray<T, E>(&self, s: Sunray, planet_id: u32) -> Result<T, E> {
        todo!()
    }
    fn start_game(path: &str) -> Self {
        todo!()
    }
    fn start_planet_ai<T, E>(&self, msg: StartPlanetAiMsg, planet_id: u32) -> Result<T, E> {
        todo!()
    }
    fn stop_planet_ai<T, E>(&self, msg: StopPlanetAiMsg, planet_id: u32) -> Result<T, E> {
        todo!()
    }
    fn supported_combination_request<T, E>(&self, msg: SupportedCombinationRequest)
        -> Result<T, E> {
        todo!()
    }
    fn supported_resource_request<T, E>(&self, msg:SupportedResourceRequest) -> Result<T, E> {
        todo!()
    }

}
    */
