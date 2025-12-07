#[cfg(test)]

use std::thread;
use common_game::protocols::messages::{ExplorerToOrchestrator, ExplorerToPlanet, OrchestratorToExplorer, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
use crate::{Orchestrator, components::{CrabRaveConstructor, explorer::BagType}};
use common_game::components::forge;
use crossbeam_channel::{Receiver, RecvError, Sender, unbounded};
use crate::components::orchestrator;
#[test]
fn t01_asteroid_exchange()->Result<(),String>{
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
    env_logger::init(); //initialize logging backend, this is only for testing purpose,
     // in the final implementation the logging backend will be initialized in the orchestrator
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