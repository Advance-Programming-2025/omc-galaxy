//use std::sync::mpsc;
use crossbeam_channel::{Sender, Receiver, select_biased, unbounded};
use std::thread;

use common_game::components::forge::Forge;
use common_game::components::planet::Planet;
use common_game::protocols::messages::{
    ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer, OrchestratorToPlanet,
    PlanetToExplorer, PlanetToOrchestrator,
};

use crate::components::CrabRaveConstructor;
use crate::components::explorer::{BagType, Explorer};

// B generic is there for representing the content type of the bag
pub struct Orchestrator {
    forge: Forge,

    galaxy_topology: Vec<Planet>, //At the moment we need only one planet for testing
    explorers: Vec<Explorer>,     //At the momet we need only one explorer for testing

    //we can better define communication like this: galaxy_communication: Option<HashMap<id,channel>>
    planet_channels: (
        Receiver<PlanetToOrchestrator>,
        Sender<OrchestratorToPlanet>,
    ),
    explorer_channels: Option<(
        Receiver<ExplorerToOrchestrator<BagType>>,
        Sender<OrchestratorToExplorer>,
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
            Sender<PlanetToOrchestrator>,
            Receiver<PlanetToOrchestrator>,
        ) = unbounded();
        let (orch_sender, planet_receiver): (
            Sender<OrchestratorToPlanet>,
            Receiver<OrchestratorToPlanet>,
        ) = unbounded();

        let planet_to_orchestrator_channels = (planet_receiver, planet_sender);
        let orchestrator_to_planet_channels = (orch_receiver, orch_sender);

        //planet-explorer and explorer-planet
        let (planet_sender, explorer_receiver): (
            Sender<PlanetToExplorer>,
            Receiver<PlanetToExplorer>,
        ) = unbounded();
        let (explorer_sender, planet_receiver): (
            Sender<ExplorerToPlanet>,
            Receiver<ExplorerToPlanet>,
        ) = unbounded();

        let planet_to_explorer_channels = planet_receiver;
        let explorer_to_planet_channels = (explorer_receiver, explorer_sender);

        //explorer-orchestrator and orchestrator-explorer
        let (explorer_sender, orch_receiver): (
            Sender<ExplorerToOrchestrator<BagType>>,
            Receiver<ExplorerToOrchestrator<BagType>>,
        ) = unbounded();
        let (orch_sender, explorer_receiver): (
            Sender<OrchestratorToExplorer>,
            Receiver<OrchestratorToExplorer>,
        ) = unbounded();

        let explorer_to_orchestrator_channels = (explorer_receiver, explorer_sender);
        let orchestrator_to_explorer_channels = (orch_receiver, orch_sender);

        //Construct crab-rave planet
        let mut crab_rave_planet = CrabRaveConstructor::new(
            0,
            planet_to_orchestrator_channels,
            planet_to_explorer_channels,
        )?;
        // crab_rave_planet.run();
        // self.planet_channels = Some(orchestrator_to_planet_channels);
        // self.explorer_channels = Some(orchestrator_to_explorer_channels);
        //Add the constructed galaxy to our Orchestrator
        let galaxy = vec![crab_rave_planet];

        //Construct Explorer
        let explorer = Explorer::new(Some(galaxy[0].id()), explorer_to_orchestrator_channels, explorer_to_planet_channels);
        let explorers = vec![explorer];
        Ok(
            Self { 
                forge: Forge::new()?, 
                galaxy_topology: galaxy, 
                explorers, 
                planet_channels: orchestrator_to_planet_channels, 
                explorer_channels: Some(orchestrator_to_explorer_channels) 
            }
        )
    }


    pub fn run(&mut self)->Result<(),String>{
        let mut planet1 = match self.galaxy_topology.pop(){
            Some(p)=>p,
            None=>return Err("Cannot find any planet to pop".to_string()),
        };

        println!("Creating planet thread...");
        thread::spawn(move ||->Result<(), String>{
            println!("Planet running...");
            let success = planet1.run()?;
            Ok(())
        });

        println!("Start Planet...");
        let start_planet = self.planet_channels.1.send(OrchestratorToPlanet::StartPlanetAI);


        // loop{
            // println!("Receive planet messages...");
            // let planet_response = match self.planet_channels.0.try_recv(){
            //     Ok(res)=>res,
            //     Err(_)=>return Err("Planet is disconnected\n".to_string())
            // };

            println!("Send Asteroid to Planet");
            let planet_message = self.planet_channels.1.send(OrchestratorToPlanet::Asteroid(self.forge.generate_asteroid()));
            let planet_response = match self.planet_channels.0.recv(){
                Ok(res)=>res,
                Err(_)=>{
                    return Err("Planet is disconnected".to_string());
                }
            };
            println!("Planet should have finished running...");

            let planet_message = match self.planet_channels.1.send(OrchestratorToPlanet::Asteroid(self.forge.generate_asteroid())){
                Ok(_)=>println!("PLANET STILL WORKING..."),
                Err(_)=>{
                    println!("Everthing is okey...");
                    // break;
                },
            };
        // }
        // let result = spawning_thread.join().expect("Something went wrong in the thread...");
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

#[cfg(test)]
mod tests {
    use common_game::components::forge;
    use crossbeam_channel::RecvError;
    use crate::components::orchestrator;
    use super::*;
    #[test]
    fn t01_asteroid_exchange()->Result<(),String>{
        let mut orchestrator = Orchestrator::new()?;
        let mut planet1 = match orchestrator.galaxy_topology.pop(){
            Some(p)=>p,
            None=>return Err("Cannot find any planet to pop".to_string()),
        };

        println!("Creating planet thread...");
        thread::spawn(move ||->Result<(), String>{
            println!("Planet running...");
            let success = planet1.run()?;
            Ok(())
        });

        println!("Start Planet...");
        match orchestrator.planet_channels.1.send(OrchestratorToPlanet::StartPlanetAI) {
            Ok(_) => { println!("Planet AI started."); },
            Err(err)=>{ panic!("Failed to start planet AI: {}", err); },
        }

        println!("Waiting for response...");
        match orchestrator.planet_channels.0.recv(){
            Ok(res) => {
                match res {
                    PlanetToOrchestrator::StartPlanetAIResult { planet_id } => { println!("Planet {} AI started.", planet_id); },
                    _ => panic!("Unexpected response to StartPlanetAI.")
                }
            }
            Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
        }

        println!("Sending asteroid...");
        match orchestrator.planet_channels.1.send(OrchestratorToPlanet::Asteroid(orchestrator.forge.generate_asteroid())){
            Ok(_) => { println!("Asteroid sent."); },
            Err(err)=>{ panic!("Failed to send asteroid: {}.", err); }
        }

        println!("Waiting for response...");
        match orchestrator.planet_channels.0.recv(){
            Ok(res) => {
                match res {
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                        println!("Planet {} received asteroid.", planet_id);
                        if rocket.is_some() {
                            println!("Planet {} survived.", planet_id);
                            Err("Planet survived but it should have died...".to_string())
                        } else {
                            match orchestrator.planet_channels.1.send(OrchestratorToPlanet::KillPlanet) {
                                Ok(_) => {
                                    println!("Planet {} kill request sent..", planet_id);
                                    match orchestrator.planet_channels.0.recv() {
                                        Ok(res) => {
                                            match res {
                                                PlanetToOrchestrator::KillPlanetResult {
                                                    planet_id } => { println!("Planet {} killed.", planet_id);
                                                    Ok(())
                                                },
                                                _ => panic!("Unexpected response to KillPlanet.")
                                            }
                                        }
                                        Err(err)=>{
                                            panic!("Failed to receive KillPlanetResponse: {}", err);
                                        }
                                    }
                                },
                                Err(err)=>{ panic!("Failed to send KillPlanet: {}.", err); }
                            }
                        }
                    },
                    _ => panic!("Unexpected response to Asteroid.")
                }
            }
            _ => { panic!("Something went wrong in receiving the AsteroidAck."); }
        }

    }
}