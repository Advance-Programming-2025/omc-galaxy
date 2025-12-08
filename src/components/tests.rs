#[cfg(test)]
use crate::{components::{explorer::BagType, CrabRaveConstructor}};
use crate::Orchestrator;
use common_game::components::sunray::Sunray;
use common_game::protocols::messages::{ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
use crossbeam_channel::{unbounded, Receiver, RecvError, Sender};
use std::thread;
use common_game::components::resource::BasicResourceType;
use common_game::components::rocket::Rocket;
use crate::components::energy_stacks::N_CELLS;
use crate::components::energy_stacks::stacks::{get_free_cell_index, push_free_cell};
use crate::components::explorer::Explorer;

fn sending_sunray(orchestrator: &Orchestrator) -> Result<(), String> {
    println!("Sending sunray...");
    match orchestrator.planet_channels.1.send(OrchestratorToPlanet::Sunray(Sunray::default())) {
        Ok(_) => { println!("Sunray sent."); },
        Err(err)=>{ panic!("Failed to send Sunray: {}", err); }
    }
    if let Some(idx) = get_free_cell_index() {
        push_free_cell(idx);
        println!("Waiting for response...");
        match orchestrator.planet_channels.0.recv() {
            Ok(res) => {
                match res {
                    PlanetToOrchestrator::SunrayAck { planet_id } => {
                        println!("Planet {} Sunray acknowledged.", planet_id);
                    }
                    _ => panic!("Unexpected response to Sunray.")
                }
            }
            Err(err)=>{ panic!("Failed to receive SunrayAck: {}", err); }
        };
        Ok(())
    } else {
        Err(String::from("No energy cell available."))
    }
}
fn sending_asteroid(orchestrator: &Orchestrator) -> Result<Option<Rocket>, String> {
    println!("Sending asteroid...");
    match orchestrator.planet_channels.1.send(OrchestratorToPlanet::Asteroid(orchestrator.forge.generate_asteroid())){
        Ok(_) => { println!("Asteroid sent."); },
        Err(err)=>{ panic!("Failed to send asteroid: {}.", err); }
    }

    println!("Waiting for response...");
    match orchestrator.planet_channels.0.recv() {
        Ok(res) => {
            match res {
                PlanetToOrchestrator::AsteroidAck { planet_id, rocket} => {
                    println!("Planet {} Asteroid acknowledged.", planet_id);
                    Ok(rocket)
                }
                _ => panic!("Unexpected response to AsteroidAck.")
            }
        }
        Err(err)=>{ panic!("Failed to send asteroid: {}.", err); }
    }
}

fn sending_available_cell_request(orchestrator: &Orchestrator) -> Result<(), String> {
    println!("Sending available cell request...");
    match &orchestrator.explorers[0] {
        Explorer{ planet_id, orchestrator_channels, planet_channels} => {
            match planet_channels {
                Some(channels) => {
                    match channels.1.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 0 }) {
                        Ok(_) => { println!("Available EnergyCellRequest sent.") },
                        Err(err)=>{ panic!("Failed to send AvailableEnergyCellRequest: {}", err); }
                    }
                }
                None => { panic!("Planet-explorer channels not provided."); }
            }
        }
    }

    // TODO: explorer should receive the response with the index of the cell (Some(idx) or None)
    // this has to be done when the explorer is implemented, (the return type of this function could be changed to Result<Option<u32>, String>)

    Ok(())
}

fn killing_planet(orchestrator: &Orchestrator) -> Result<(), String> {
    println!("Sending KillPlanet...");
    orchestrator.planet_channels.1
        .send(OrchestratorToPlanet::KillPlanet)
        .map_err(|_| "Failed to send KillPlanet")?;

    println!("Waiting for KillPlanet response...");
    match orchestrator.planet_channels.0.recv() {
        Ok(PlanetToOrchestrator::KillPlanetResult { planet_id }) => {
            println!("Planet {} killed.", planet_id);
        }
        Ok(_) => return Err("Unexpected response to KillPlanet".to_string()),
        Err(err) => return Err(format!("Failed to receive KillPlanet response: {}", err)),
    };
    Ok(())
}

#[test]
fn t02_single_sunray_exchange() -> Result<(), String> {
    println!("+++++ Test single sunray +++++");
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    sending_sunray(&orchestrator)?;

    let result = killing_planet(&orchestrator);

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}

#[test]
fn t03_correct_resource_request() -> Result<(), String> {
    println!("+++++ Correct resource request +++++");
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    sending_sunray(&orchestrator)?;

    match &orchestrator.explorers[0] {
        Explorer { planet_id, orchestrator_channels, planet_channels } => {

            println!("Accessing planet-explorer channels...");
            match planet_channels {
                None => { panic!("Planet channels is None."); }
                Some(channels) => {

                    println!("Sending resource request...");
                    match channels.1.send(ExplorerToPlanet::GenerateResourceRequest { explorer_id: 0, resource: BasicResourceType::Carbon}) {
                        Ok(_) => { println!("Planet generated resource request."); },
                        Err(err)=>{ panic!("Failed to generate resource request: {}", err); }
                    }

                    // println!("Responding to resource request...");
                    // match channels.0.recv() {
                    //     Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
                    //         if resource.is_some() {
                    //             println!("Explorer received the resource request response correctly.");
                    //         } else {
                    //             println!("Planet gave back None resource.");
                    //         }
                    //     }
                    //     Ok(_) => { panic!("Unexpected resource request response."); }
                    //     Err(err)=>{ panic!("Failed to respond to resource request: {}", err); }
                    // }
                }
            }
        }
    }

    let result = killing_planet(&orchestrator);

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }

}

#[test]
fn t04_failure_resource_request() -> Result<(), String> {
    println!("+++++ Failure resource request +++++");
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    match &orchestrator.explorers[0] {
        Explorer { planet_id, orchestrator_channels, planet_channels } => {

            println!("Accessing planet-explorer channels...");
            match planet_channels {
                None => { panic!("Planet channels is None."); }
                Some(channels) => {

                    println!("Sending resource request...");
                    match channels.1.send(ExplorerToPlanet::GenerateResourceRequest { explorer_id: 0, resource: BasicResourceType::Carbon}) {
                        Ok(_) => { println!("Planet generated resource request."); },
                        Err(err)=>{ panic!("Failed to generate resource request: {}", err); }
                    }

                    // println!("Responding to resource request...");
                    // match channels.0.recv() {
                    //     Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
                    //         if resource.is_some() {
                    //             println!("Explorer received the resource request response correctly.");
                    //         } else {
                    //             println!("Planet gave back None resource.");
                    //         }
                    //     }
                    //     Ok(_) => { panic!("Unexpected resource request response."); }
                    //     Err(err)=>{ panic!("Failed to respond to resource request: {}", err); }
                    // }
                }
            }
        }
    }

    let result = killing_planet(&orchestrator);

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}

#[test]
fn t06_asteroid_exchange_without_rocket()->Result<(),String>{
    println!("+++++ Test asteroid without rocket +++++");
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
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
    let result = match orchestrator.planet_channels.0.recv(){
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
                                                planet_id } => {
                                                println!("Planet {} killed.", planet_id);
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
        Err(_) => { panic!("Something went wrong in receiving the AsteroidAck."); }
    };

    // aspetta che il thread del pianeta finisca
    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}

#[test]
fn t01_planet_initialization() -> Result <(),String >{
    println!("+++++ Test planet initialization +++++");
    let(planet_sender,orch_receiver):(
    Sender < PlanetToOrchestrator >,
    Receiver < PlanetToOrchestrator >,
    )= unbounded();
    let(orch_sender,planet_receiver):(
    Sender < OrchestratorToPlanet >,
    Receiver < OrchestratorToPlanet >,
    )= unbounded();

    let planet_to_orchestrator_channels =(planet_receiver,planet_sender);
    let orchestrator_to_planet_channels =(orch_receiver,orch_sender);

     //planet-explorer and explorer-planet
    let(planet_sender,explorer_receiver):(
    Sender < PlanetToExplorer >,
    Receiver < PlanetToExplorer >,
    )= unbounded();
    let(explorer_sender,planet_receiver):(
    Sender < ExplorerToPlanet >,
    Receiver < ExplorerToPlanet >,
    )= unbounded();

    let planet_to_explorer_channels = planet_receiver;
    let explorer_to_planet_channels =(explorer_receiver,explorer_sender);

     //explorer-orchestrator and orchestrator-explorer
    let(explorer_sender,orch_receiver):(
    Sender < ExplorerToOrchestrator < BagType > >,
    Receiver < ExplorerToOrchestrator < BagType > >,
    )= unbounded();
    let(orch_sender,explorer_receiver):(
    Sender < OrchestratorToExplorer >,
    Receiver < OrchestratorToExplorer >,
    )= unbounded();

    let explorer_to_orchestrator_channels =(explorer_receiver,explorer_sender);
    let orchestrator_to_explorer_channels =(orch_receiver,orch_sender);
     //Construct crab-rave planet
    let mut crab_rave_planet = CrabRaveConstructor::new(
    0,
    planet_to_orchestrator_channels,
    planet_to_explorer_channels,
    )?;
    Ok(())
}

#[test]
fn t05_asteroid_success()->Result<(),String>{
    println!("+++++ Test asteroid success +++++");
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    println!("Sending sunray...");
    match orchestrator.planet_channels.1.send(OrchestratorToPlanet::Sunray(orchestrator.forge.generate_sunray())) {
        Ok(_) => {println!("Sunray sent.")},
        Err(err)=>{ panic!("Failed to send sunray: {}.", err); }
    }

    println!("Waiting for response...");
    let result_sunray: Result<(), RecvError> = match orchestrator.planet_channels.0.recv(){
        Ok(res) => {
            match res {
                PlanetToOrchestrator::SunrayAck { planet_id } => {
                    println!("Planet {} received sunray.", planet_id);
                    Ok(())
                },
                _ => panic!("Unexpected response to Sunray.")
            }
        }
        Err(_) => { 
            panic!("Something went wrong in receiving the SunrayAck."); }
    };

    println!("Sending asteroid...");
    match orchestrator.planet_channels.1.send(OrchestratorToPlanet::Asteroid(orchestrator.forge.generate_asteroid())){
        Ok(_) => { println!("Asteroid sent."); },
        Err(err)=>{ panic!("Failed to send asteroid: {}.", err); }
    }

    println!("Waiting for response...");
    let result = match orchestrator.planet_channels.0.recv(){
        Ok(res) => {
            match res {
                PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                    println!("Planet {} received asteroid.", planet_id);
                    if rocket.is_some() {
                        println!("Planet {} survived.", planet_id);
                        Ok(())
                    } else {
                       Err("Planet died but it should have survived...".to_string())
                    }
                },
                _ => panic!("Unexpected response to Asteroid.")
            }
        }
        Err(e) => { 
            println!("Recv Error: {}", e);
            panic!("Something went wrong in receiving the AsteroidAck."); }
    };
    result
}

#[test]
fn t07_more_sunray_than_cells()->Result<(),String>{
    println!("+++++ Test more sunray than cells +++++");
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    for _ in 0..N_CELLS {
        sending_sunray(&orchestrator)?
    }

    assert!(sending_sunray(&orchestrator).is_err());

    let result = killing_planet(&orchestrator)?;

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            Ok(result)
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}

#[test]
fn t08_available_resources_request()->Result<(),String> {
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    match &orchestrator.explorers[0] {
        Explorer { planet_id, orchestrator_channels, planet_channels } => {

            println!("Accessing planet-explorer channels...");
            match planet_channels {
                None => { panic!("Planet channels is None."); }
                Some(channels) => {

                    println!("Sending resource request...");
                    match channels.1.send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 0 }) {
                        Ok(_) => { println!("Planet supported resource request sent."); },
                        Err(err)=>{ panic!("Failed to generate resource request: {}", err); }
                    }

                    // println!("Responding to available resource request...");
                    // match channels.0.recv() {
                    //     Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) => {
                    //         println!("Planet supported resource list: {:?}", resource_list);
                    //     }
                    //     Ok(_) => { panic!("Unexpected available resource request response."); }
                    //     Err(err)=>{ panic!("Failed to respond to available resource request: {}", err); }
                    // }
                }
            }
        }
    }

    let result = killing_planet(&orchestrator);

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}

#[test]
fn t11_charged_cell_request_with_one_charged_cell()->Result<(),String>{
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    sending_sunray(&orchestrator)?;

    sending_available_cell_request(&orchestrator)?;

    let result = killing_planet(&orchestrator);

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}

#[test]
fn t12_charged_cell_request_with_no_charged_cell()->Result<(),String> {
    let mut orchestrator = Orchestrator::new()?;
    let mut planet1 = match orchestrator.galaxy_topology.pop(){
        Some(p)=>p,
        None=>return Err("Cannot find any planet to pop".to_string()),
    };

    println!("Creating planet thread...");
    let handle = thread::spawn(move ||->Result<(), String>{
        println!("Planet running...");
        planet1.run()
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
                PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                    println!("Planet {} AI started.", planet_id);
                },
                _ => panic!("Unexpected response to StartPlanetAI.")
            }
        }
        Err(err)=>{ panic!("Failed to start planet AI: {}.", err); }
    }

    sending_available_cell_request(&orchestrator)?;

    let result = killing_planet(&orchestrator);

    match handle.join() {
        Ok(Ok(_)) => {
            println!("Planet thread completed successfully");
            result
        }
        Ok(Err(e)) => Err(format!("Planet thread returned error: {}", e)),
        Err(_) => Err("Planet thread panicked".to_string()),
    }
}