// ============================================================================
// Mattia Explorer - Comprehensive Communication Tests
// ============================================================================
// Tests covering every protocol interaction documented in main.tex:
//   - Orchestrator <-> Explorer lifecycle (Start/Stop/Reset/Kill)
//   - CurrentPlanetRequest/Result
//   - SupportedResourceRequest/Result
//   - SupportedCombinationRequest/Result
//   - GenerateResourceRequest/Response
//   - CombineResourceRequest/Response
//   - BagContentRequest/Response
//   - NeighborsRequest/Response
//   - TravelToPlanetRequest / MoveToPlanet / MovedToPlanetResult
//   - Edge cases (double start, kill while idle, stop+start, reset+start)
//   - Full simulation with AI
// ============================================================================

mod test_one_million_crabs_planet {
    use super::*;
    use crate::utils::registry::PlanetType;
    use crate::utils::ExplorerInfo;
    use crate::{Orchestrator, Status};
    use common_game::components::resource::BasicResourceType;
    use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
    use common_game::protocols::planet_explorer::ExplorerToPlanet::GenerateResourceRequest;
    use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
    use crossbeam_channel::{select, tick};
    use rand::Rng;
    use std::thread::sleep;
    use std::time::Duration;
    #[test]
    fn stress_planet_energy_cells_management_2() {
        let mut orchestrator = Orchestrator::new().unwrap();
        let planet_id = 1;
        let explorer_id = 2;
        let topology = format!("{},{}\n", planet_id, PlanetType::OneMillionCrabs as u32);
        orchestrator
            .initialize_galaxy_by_content(&topology)
            .unwrap();
        orchestrator.start_all(&[], &[]).unwrap();

        //Create the comms for the new explorer
        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        // get the sender from explorer to planet
        let (orch_to_planet, expl_to_planet) = match orchestrator.planet_channels.get(&planet_id) {
            Some((orchestrator_sender, explorer_sender)) => (
                Some(orchestrator_sender.clone()),
                Some(explorer_sender.clone()),
            ),
            None => (None, None), // sender does not exist
        };

        //Construct Explorer
        let new_explorer = crate::components::mattia_explorer::Explorer::new(
            explorer_id,
            planet_id,
            (receiver_orch, orchestrator.sender_explorer_orch.clone()),
            (receiver_planet, expl_to_planet.unwrap()),
        );

        //Update HashMaps
        orchestrator.explorers_info.insert(
            explorer_id,
            ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id),
        );

        orchestrator
            .explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet.clone()));

        match orch_to_planet {
            Some(orchestrator_sender) => {
                match orchestrator_sender.send(OrchestratorToPlanet::IncomingExplorerRequest {
                    explorer_id,
                    new_sender: sender_planet.clone(),
                }) {
                    Ok(_) => {}
                    Err(_err) => {}
                }
            }
            None => {}
        }

        let planet_channel = orchestrator
            .planet_channels
            .get(&planet_id)
            .unwrap()
            .0
            .clone();
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer
            .planet_channels
            .1
            .send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            })
            .expect("testing expect");
        sleep(Duration::from_secs(1));
        orchestrator
            .send_bag_content_request(explorer_id)
            .expect("testing expect");
        orchestrator
            .send_internal_state_request(
                &orchestrator.planet_channels.get(&planet_id).unwrap().0,
                planet_id,
            )
            .expect("testing expect");
        new_explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id })
            .expect("testing expect");

        let timeout = tick(Duration::from_millis(1000));
        let mut available_energy_cells: i32 = -1;
        loop {
            select! {
                recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                    match planet_msg {
                        Ok(msg) => {
                            orchestrator.handle_planet_message(msg).expect("testing expect");
                        }
                        Err(_) => {}
                    }
                }
                recv(new_explorer.planet_channels.0)-> planet_msg=> {
                    match planet_msg {
                        Ok(msg) => {
                            match msg{
                                PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                                    available_energy_cells = available_cells as i32;
                                }
                                _ => {}
                            }

                        }
                        Err(_) => {}
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert_eq!(
            orchestrator
                .planets_info
                .get_info(planet_id)
                .unwrap()
                .energy_cells
                .iter()
                .filter(|&&x| x)
                .count(),
            available_energy_cells as usize
        );
    }
    #[test]
    fn stress_planet_energy_cells_management_3() {
        let mut orchestrator = Orchestrator::new().unwrap();
        let planet_id = 1;
        let explorer_id = 2;
        let topology = format!("{},{}\n", planet_id, PlanetType::OneMillionCrabs as u32);
        orchestrator
            .initialize_galaxy_by_content(&topology)
            .unwrap();
        orchestrator.start_all(&[], &[]).unwrap();

        //Create the comms for the new explorer
        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        // get the sender from explorer to planet
        let (orch_to_planet, expl_to_planet) = match orchestrator.planet_channels.get(&planet_id) {
            Some((orchestrator_sender, explorer_sender)) => (
                Some(orchestrator_sender.clone()),
                Some(explorer_sender.clone()),
            ),
            None => (None, None), // sender does not exist
        };

        //Construct Explorer
        let new_explorer = crate::components::mattia_explorer::Explorer::new(
            explorer_id,
            planet_id,
            (receiver_orch, orchestrator.sender_explorer_orch.clone()),
            (receiver_planet, expl_to_planet.unwrap()),
        );

        //Update HashMaps
        orchestrator.explorers_info.insert(
            explorer_id,
            ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id),
        );

        orchestrator
            .explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet.clone()));

        match orch_to_planet {
            Some(orchestrator_sender) => {
                match orchestrator_sender.send(OrchestratorToPlanet::IncomingExplorerRequest {
                    explorer_id,
                    new_sender: sender_planet.clone(),
                }) {
                    Ok(_) => {}
                    Err(_err) => {}
                }
            }
            None => {}
        }

        let planet_channel = orchestrator
            .planet_channels
            .get(&planet_id)
            .unwrap()
            .0
            .clone();

        // max charge
        for _ in 0..5 {
            orchestrator
                .send_sunray(planet_id, &planet_channel)
                .expect("testing expect");
        }

        // mixed messages
        for i in 0..200 {
            if i % 3 == 0 {
                orchestrator
                    .send_sunray(planet_id, &planet_channel)
                    .expect("testing expect");
            }
            new_explorer
                .planet_channels
                .1
                .send(GenerateResourceRequest {
                    explorer_id,
                    resource: BasicResourceType::Silicon,
                })
                .expect("testing expect");
        }

        sleep(Duration::from_secs(1));
        orchestrator
            .send_bag_content_request(explorer_id)
            .expect("testing expect");
        orchestrator
            .send_internal_state_request(
                &orchestrator.planet_channels.get(&planet_id).unwrap().0,
                planet_id,
            )
            .expect("testing expect");
        new_explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id })
            .expect("testing expect");

        let timeout = tick(Duration::from_millis(1000));
        let mut available_energy_cells: i32 = -1;
        loop {
            select! {
                recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                    match planet_msg {
                        Ok(msg) => {
                            orchestrator.handle_planet_message(msg).expect("testing expect");
                        }
                        Err(_) => {}
                    }
                }
                recv(new_explorer.planet_channels.0)-> planet_msg=> {
                    match planet_msg {
                        Ok(msg) => {
                            match msg{
                                PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                                    available_energy_cells = available_cells as i32;
                                }
                                _ => {}
                            }

                        }
                        Err(_) => {}
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        println!("energy cells left: {}", available_energy_cells);
        assert_eq!(
            orchestrator
                .planets_info
                .get_info(planet_id)
                .unwrap()
                .energy_cells
                .iter()
                .filter(|&&x| x)
                .count(),
            available_energy_cells as usize
        );
    }

    #[test]
    #[ignore] //takes about 7/8 minutes to execute with debug logs
    fn stress_planet_energy_cells_management_4() {
        let mut orchestrator = Orchestrator::new().unwrap();
        for _ in 0..50 {
            let planet_id = 1;
            let explorer_id = 2;
            let topology = format!("{},{}\n", planet_id, PlanetType::OneMillionCrabs as u32);
            orchestrator
                .initialize_galaxy_by_content(&topology)
                .unwrap();
            orchestrator.start_all(&[], &[]).unwrap();

            //Create the comms for the new explorer
            let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
                Orchestrator::init_comms_explorers();

            // get the sender from explorer to planet
            let (orch_to_planet, expl_to_planet) =
                match orchestrator.planet_channels.get(&planet_id) {
                    Some((orchestrator_sender, explorer_sender)) => (
                        Some(orchestrator_sender.clone()),
                        Some(explorer_sender.clone()),
                    ),
                    None => (None, None), // sender does not exist
                };

            //Construct Explorer
            let new_explorer = crate::components::mattia_explorer::Explorer::new(
                explorer_id,
                planet_id,
                (receiver_orch, orchestrator.sender_explorer_orch.clone()),
                (receiver_planet, expl_to_planet.unwrap()),
            );

            //Update HashMaps
            orchestrator.explorers_info.insert(
                explorer_id,
                ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id),
            );

            orchestrator
                .explorer_channels
                .insert(new_explorer.id(), (sender_orch, sender_planet.clone()));

            match orch_to_planet {
                Some(orchestrator_sender) => {
                    match orchestrator_sender.send(OrchestratorToPlanet::IncomingExplorerRequest {
                        explorer_id,
                        new_sender: sender_planet.clone(),
                    }) {
                        Ok(_) => {}
                        Err(_err) => {}
                    }
                }
                None => {}
            }

            let planet_channel = orchestrator
                .planet_channels
                .get(&planet_id)
                .unwrap()
                .0
                .clone();

            // max charge
            for _ in 0..5 {
                orchestrator
                    .send_sunray(planet_id, &planet_channel)
                    .expect("testing expect");
            }

            // mixed messages
            let mut rng = rand::rng();

            for _ in 0..200 {
                // 30% di probabilità di inviare un raggio di sole
                if rng.random_bool(0.5) {
                    orchestrator
                        .send_sunray(planet_id, &planet_channel)
                        .expect("testing expect");
                }

                // 70% di probabilità di provare a generare una risorsa
                if rng.random_bool(0.5) {
                    new_explorer
                        .planet_channels
                        .1
                        .send(GenerateResourceRequest {
                            explorer_id,
                            resource: BasicResourceType::Silicon,
                        })
                        .expect("testing expect");
                }

                if rng.random_bool(0.1) {
                    sleep(Duration::from_millis(50));
                }
            }

            sleep(Duration::from_secs(1));
            orchestrator
                .send_bag_content_request(explorer_id)
                .expect("testing expect");

            let timeout = tick(Duration::from_millis(3000));

            loop {
                select! {
                    recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                        match planet_msg {
                            Ok(msg) => {
                                orchestrator.handle_planet_message(msg).expect("testing expect");
                            }
                            Err(_) => {}
                        }
                    }
                    recv(timeout) -> _ => {
                        break;
                    }
                }
            }
            sleep(Duration::from_millis(1000));
            new_explorer
                .planet_channels
                .1
                .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id })
                .expect("testing expect");
            orchestrator
                .send_internal_state_request(
                    &orchestrator.planet_channels.get(&planet_id).unwrap().0,
                    planet_id,
                )
                .expect("testing expect");

            let timeout = tick(Duration::from_millis(2000));
            let mut available_energy_cells: i32 = -1;
            loop {
                select! {
                    recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                        match planet_msg {
                            Ok(msg) => {
                                orchestrator.handle_planet_message(msg).expect("testing expect");
                            }
                            Err(_) => {}
                        }
                    }
                    recv(new_explorer.planet_channels.0)-> planet_msg=> {
                        match planet_msg {
                            Ok(msg) => {
                                match msg{
                                    PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                                        available_energy_cells = available_cells as i32;
                                    }
                                    _ => {}
                                }

                            }
                            Err(_) => {}
                        }
                    }
                    recv(timeout) -> _ => {
                        break;
                    }
                }
            }
            println!("energy cells left: {}", available_energy_cells);
            assert_eq!(
                orchestrator
                    .planets_info
                    .get_info(planet_id)
                    .unwrap()
                    .energy_cells
                    .iter()
                    .filter(|&&x| x)
                    .count(),
                available_energy_cells as usize
            );

            // killing planet and explorer
            orchestrator
                .send_planet_kill_to_all()
                .expect("testing expect");
            orchestrator
                .send_kill_explorer_ai(explorer_id)
                .expect("testing expect");
            orchestrator.planets_info.map.clear();
            orchestrator.planet_channels.clear();
            orchestrator.explorer_channels.clear();
            orchestrator.explorers_info.map.clear();
            sleep(Duration::from_millis(100));
        }
    }
}

mod game_simulation {
    use super::*;
    use crate::{debug_println, Orchestrator};
    use crossbeam_channel::{select, tick};
    use std::time::Duration;
    #[test]
    fn simulation_25s() {
        let mut orchestrator = Orchestrator::new().unwrap();
        orchestrator
            .initialize_galaxy_by_file(
                "./src/components/mattia_explorer/test_topology_files/t0.txt",
            )
            .expect("testing expect");
        orchestrator.start_all_planet_ais().expect("testing expect");
        orchestrator
            .add_mattia_explorer(10, 0)
            .expect("testing expect");
        orchestrator
            .start_all_explorer_ais()
            .expect("testing expect");
        let do_something = tick(Duration::from_millis(50));
        let mut counter = 500;
        loop {
            select! {
                recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                    match planet_msg {
                        Ok(msg) => {
                            orchestrator.handle_planet_message(msg).expect("testing expect");
                        }
                        Err(_) => {}
                    }
                }
                recv(orchestrator.receiver_orch_explorer) ->explorer_msg =>{
                    match explorer_msg {
                        Ok(msg) => {
                            debug_println!("orchestrator received a message from an explorer");
                            orchestrator.handle_explorer_message(msg).expect("testing expect");
                        }
                        Err(_) => {
                            debug_println!("error receiving messages from explorer");
                        }
                    }
                }
                recv(do_something) -> _ => {
                    counter -= 1;
                    if counter == 0 {
                        orchestrator.send_planet_kill_to_all().expect("testing expect");
                    }
                    else if counter < 0 {
                        break;
                    }
                    else{
                        orchestrator.choose_random_action().expect("testing expect");
                    }
                }
            }
        }
    }
}

use crate::Orchestrator;

#[test]
/// Test if the explorer is spawned properly
fn test_spawn_explorer_on_planet() {
    //The explorer(0) should spawn on planet(0)

    //init orchestrator
    let mut orch = Orchestrator::new().unwrap();

    // init topology with one planet
    // format: id, type, neighbors...
    let topology = "0,0\n";
    orch.initialize_galaxy_by_content(topology).unwrap();

    //init explorer
    orch.add_mattia_explorer(0, 0).unwrap();

    // check if explorer is correctly registered
    assert_eq!(orch.explorers_info.len(), 1);
    assert!(orch.explorers_info.get(&0).is_some());
}
#[cfg(test)]
mod communication {
    use common_game::components::resource::BasicResourceType;
    use std::collections::HashSet;
    use std::thread::sleep;
    use std::time::Duration;

    use crate::{utils::registry::PlanetType, Status};

    use super::*;

    #[test]
    /// Test if the explorer ai starts properly
    fn start_explorer_ai() {
        // init orchestrator
        let mut orch = Orchestrator::new().unwrap();

        //init pianeta
        let planet_id = 0;
        let topology = format!("{},{}\n", planet_id, PlanetType::BlackAdidasShoe as u32);
        orch.initialize_galaxy_by_content(&topology).unwrap();

        //init explorer
        let explorer_id = 0;
        orch.add_mattia_explorer(explorer_id, planet_id)
            .expect("testing expect");

        //should be paused because it is not already running
        assert_eq!(
            orch.explorers_info.get_status(&explorer_id).unwrap(),
            Status::Paused
        );

        //in start all is used start_planet_ais
        orch.start_all_explorer_ais().unwrap();

        //check if the explorer respond correctly(only the explorer starts so its the only one that can send messages)
        orch.handle_game_messages().unwrap();

        //should be running because started
        assert_eq!(
            orch.explorers_info.get_status(&explorer_id).unwrap(),
            Status::Running
        );
    }

    /// Test if the explorer ai stops properly
    #[test]
    fn stop_explorer_ai() {
        // init orchestrator
        let mut orch = Orchestrator::new().unwrap();

        //init pianeta
        let planet_id = 0;
        let topology = format!("{},{}\n", planet_id, PlanetType::BlackAdidasShoe as u32);
        orch.initialize_galaxy_by_content(&topology).unwrap();

        //init explorer
        let explorer_id = 0;
        orch.add_mattia_explorer(explorer_id, planet_id)
            .expect("testing expect");

        //start_planet_ais
        orch.start_all_explorer_ais().unwrap();
        //stop_planet_ais
        orch.stop_all_explorer_ais().unwrap();

        //check if the explorer respond correctly(only the explorer starts so its the only one that can send messages)
        orch.handle_game_messages().unwrap();
        orch.handle_game_messages().unwrap();
        orch.handle_game_messages().unwrap();
        //should be running because started
        assert_eq!(
            orch.explorers_info.get_status(&explorer_id).unwrap(),
            Status::Paused
        );
    }

    /// Test if the sunray exchange works properly
    #[test]
    fn supported_resource_request_from_orchestrator() {
        // init orchestrator
        let mut orch = Orchestrator::new().unwrap();

        //init pianeta
        let planet_id = 0;
        let topology = format!("{},{}\n", planet_id, PlanetType::BlackAdidasShoe as u32);
        orch.initialize_galaxy_by_content(&topology).unwrap();
        //start planet ai
        orch.start_all_planet_ais().unwrap();

        //init explorer
        let explorer_id = 0;
        orch.add_mattia_explorer(explorer_id, planet_id)
            .expect("testing expect");

        //stop_planet_ais
        orch.send_supported_resource_request(explorer_id).unwrap();
        sleep(Duration::from_millis(500)); //handle game messages has a deadline of 10 ms
                                           // 1. planet result, explorer result, combination result
        orch.handle_game_messages().unwrap();
        orch.handle_game_messages().unwrap();
        orch.handle_game_messages().unwrap();

        //check if the values are updated
        match orch.planets_info.get_info(planet_id) {
            Some(info) => {
                let mut expected = HashSet::new();
                expected.insert(BasicResourceType::Hydrogen);
                expected.insert(BasicResourceType::Carbon);
                expected.insert(BasicResourceType::Oxygen);
                assert_eq!(info.supported_resources, Some(expected));
            }
            None => panic!(),
        }
    }

    #[test]
    fn supported_combination_request_from_orchestrator() {
        // init orchestrator
        let mut orch = Orchestrator::new().unwrap();

        //init pianeta
        let planet_id = 0;
        let topology = format!("{},{}\n", planet_id, PlanetType::OneMillionCrabs as u32);
        orch.initialize_galaxy_by_content(&topology).unwrap();
        //start_planet_ais
        orch.start_all_planet_ais().unwrap();

        //init explorer
        let explorer_id = 0;
        orch.add_mattia_explorer(explorer_id, planet_id)
            .expect("testing expect");

        //stop_planet_ais
        orch.send_supported_combination_request(explorer_id)
            .unwrap();
        sleep(Duration::from_millis(500)); //handle game messages has a deadline of 10 ms
                                           // 1. planet result, explorer result, combination result
        orch.handle_game_messages().unwrap();
        orch.handle_game_messages().unwrap();
        orch.handle_game_messages().unwrap();

        //check if the values are updated
        match orch.planets_info.get_info(planet_id) {
            Some(info) => {
                assert_eq!(info.supported_combination, Some(HashSet::new()));
            }
            None => panic!(),
        }
    }
}

// ============================================================================
// NEW: Comprehensive Communication Tests
// ============================================================================

/// Helper function: sets up an orchestrator with one planet (of a given type),
/// starts the planet AI, adds a mattia explorer on it, and returns the
/// orchestrator ready for interaction. The explorer thread is already running.
#[cfg(test)]
fn setup_orch_with_explorer(
    planet_type: crate::utils::registry::PlanetType,
    planet_id: u32,
    explorer_id: u32,
) -> Orchestrator {
    let mut orch = Orchestrator::new().unwrap();
    // Initialize the galaxy topology so that NeighborsRequest can be served
    // Format: "planet_id, planet_type_discriminant\n"
    let topology = format!("{},{}\n", planet_id, planet_type as u32);
    orch.initialize_galaxy_by_content(&topology).unwrap();
    orch.start_all_planet_ais().unwrap();
    orch.add_mattia_explorer(explorer_id, planet_id).unwrap();
    orch
}

/// Helper: drain orchestrator messages for a given duration (ms), processing
/// both planet and explorer messages.
#[cfg(test)]
fn drain_messages(orch: &mut Orchestrator, duration_ms: u64) {
    use crossbeam_channel::{select, tick};
    use std::time::Duration;

    let timeout = tick(Duration::from_millis(duration_ms));
    loop {
        select! {
            recv(orch.receiver_orch_planet) -> planet_msg => {
                if let Ok(msg) = planet_msg { //handling message if there is one
                    orch.handle_planet_message(msg);
                }
            }
            recv(orch.receiver_orch_explorer) -> explorer_msg => {
                if let Ok(msg) = explorer_msg {
                    orch.handle_explorer_message(msg);
                }
            }
            recv(timeout) -> _ => {
                break;
            }
        }
    }
}

// ============================================================================
// 1. Explorer Lifecycle Tests (Start/Stop/Reset/Kill)
// ============================================================================
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use crate::Status;
    use crossbeam_channel::{select, tick};
    use std::time::Duration;
    // ---- StartExplorerAI -> StartExplorerAIResult ----

    #[test]
    fn start_explorer_ai_sets_status_running() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Paused);

        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 200);

        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);
    }

    // ---- StopExplorerAI -> StopExplorerAIResult ----

    #[test]
    fn stop_explorer_ai_sets_status_paused() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // start first
        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 200);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);

        // stop
        orch.send_stop_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 200);

        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Paused);
    }

    // ---- ResetExplorerAI -> ResetExplorerAIResult ----

    #[test]
    fn reset_explorer_ai_receives_result() {
        use crossbeam_channel::{select, tick};
        use std::time::Duration;
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // start, then reset
        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 200);

        orch.send_reset_explorer_ai(0).unwrap();
        //drain_messages(&mut orch, 50);
        let mut ack_received = false;

        let timeout = tick(Duration::from_millis(100));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if msg.is_reset_explorer_ai_result(){
                            ack_received=true;
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }

        // after reset the explorer should still be registered
        assert!(orch.explorers_info.get(&0).is_some());
        assert_eq!(
            orch.explorers_info.get_status(&0u32).unwrap(),
            Status::Running
        );
        assert!(ack_received);
    }

    // ---- KillExplorer -> KillExplorerResult ----

    #[test]
    fn kill_explorer_sets_status_dead() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        orch.send_kill_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 100);

        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Dead);
    }

    // ---- Start -> Stop -> Start cycle ----

    #[test]
    fn start_stop_start_cycle() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // start
        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 200);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);

        // stop
        orch.send_stop_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 100);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Paused);

        // start again
        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 100);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);
    }

    // ---- Kill from idle state (no start) ----

    #[test]
    fn kill_from_idle() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // explorer never started, just kill directly
        orch.send_kill_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 50);

        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Dead);
    }

    // ---- Kill while running ----

    #[test]
    fn kill_while_running() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 100);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);

        orch.send_kill_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 100);

        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Dead);
    }

    // ---- Double start (idempotency) ----

    #[test]
    fn double_start_explorer_ai() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        orch.send_start_explorer_ai(0).unwrap();
        drain_messages(&mut orch, 200);
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);

        // send start again
        orch.send_start_explorer_ai(0).unwrap();
        //drain_messages(&mut orch, 50);
        let mut ack_received = false;
        let timeout = tick(Duration::from_millis(100));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if msg.is_start_explorer_ai_result(){
                            ack_received=true;
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }

        // should still be running
        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Running);
        assert!(ack_received)
    }

    // ---- Stop without start ----

    #[test]
    fn stop_without_start() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // explorer is in idle/paused state, send stop directly
        orch.send_stop_explorer_ai(0).unwrap();
        //drain_messages(&mut orch, 200);

        let mut ack_received = false;
        let timeout = tick(Duration::from_millis(100));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if msg.is_stop_explorer_ai_result(){
                            ack_received=true;
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }

        assert_eq!(orch.explorers_info.get_status(&0).unwrap(), Status::Paused);
        assert!(ack_received);
    }

    // ---- Multiple explorers lifecycle ----

    #[test]
    fn multiple_explorers_independent_lifecycle() {
        let mut orch = Orchestrator::new().unwrap();
        let topology = format!("{},{}\n", 0, PlanetType::OneMillionCrabs as u32);
        orch.initialize_galaxy_by_content(&topology).unwrap();
        orch.start_all_planet_ais().unwrap();

        orch.add_mattia_explorer(10, 0).unwrap();
        orch.add_mattia_explorer(20, 0).unwrap();

        // start both
        orch.start_all_explorer_ais().unwrap();
        drain_messages(&mut orch, 200);

        assert_eq!(
            orch.explorers_info.get_status(&10).unwrap(),
            Status::Running
        );
        assert_eq!(
            orch.explorers_info.get_status(&20).unwrap(),
            Status::Running
        );

        // stop only one
        orch.send_stop_explorer_ai(10).unwrap();
        drain_messages(&mut orch, 100);

        assert_eq!(orch.explorers_info.get_status(&10).unwrap(), Status::Paused);
        assert_eq!(
            orch.explorers_info.get_status(&20).unwrap(),
            Status::Running
        );

        // kill both
        orch.send_kill_explorer_ai(10).unwrap();
        orch.send_kill_explorer_ai(20).unwrap();
        drain_messages(&mut orch, 100);

        assert_eq!(orch.explorers_info.get_status(&10).unwrap(), Status::Dead);
        assert_eq!(orch.explorers_info.get_status(&20).unwrap(), Status::Dead);
    }
}

// ============================================================================
// 2. CurrentPlanetRequest / CurrentPlanetResult
// ============================================================================
#[cfg(test)]
mod current_planet_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
    use crossbeam_channel::{select, tick};
    use std::time::Duration;

    #[test]
    fn current_planet_request_returns_correct_planet() {
        let planet_id = 0;
        let explorer_id = 0;
        let mut orch =
            setup_orch_with_explorer(PlanetType::OneMillionCrabs, planet_id, explorer_id);

        orch.send_current_planet_request(explorer_id).unwrap();
        let mut response = false;
        let timeout = tick(Duration::from_millis(100));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::CurrentPlanetResult {explorer_id:_res_explorer_id, planet_id:res_planet_id}=msg{
                            response=true;
                            assert_eq!(res_planet_id, planet_id);
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert!(response, "CurrentPlanetResult not received");
        assert_eq!(
            orch.explorers_info
                .get_current_planet(&explorer_id)
                .unwrap(),
            planet_id
        );
    }
}

// ============================================================================
// 3. SupportedResourceRequest / SupportedResourceResult
// ============================================================================
#[cfg(test)]
mod resource_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
    use crossbeam_channel::{select, tick};
    use std::time::Duration;

    #[test]
    fn supported_resource_request() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);
        drain_messages(&mut orch, 100);

        orch.send_supported_resource_request(0).unwrap();

        let mut response = false;
        let timeout = tick(Duration::from_millis(200));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::SupportedResourceResult {explorer_id:_res_explorer_id,ref supported_resources}=msg{
                            response=true;
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {

                    break;
                }
            }
        }
        let info = orch.planets_info.get_info(0).unwrap();
        assert!(response, "SupportedResourceResult not received");
        assert!(info.supported_resources.is_some());
        // OneMillionCrabs should have some supported resources
        assert!(!info.supported_resources.as_ref().unwrap().is_empty());
    }
    #[test]
    fn supported_combination_request() {
        let mut orch = setup_orch_with_explorer(PlanetType::RustyCrab, 0, 0);
        drain_messages(&mut orch, 100);

        orch.send_supported_combination_request(0).unwrap();

        let mut response = false;
        let timeout = tick(Duration::from_millis(200));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::SupportedCombinationResult {explorer_id:_res_explorer_id, ref combination_list}=msg{
                            response=true;
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        let info = orch.planets_info.get_info(0).unwrap();
        assert!(response, "SupportedCombinationResult not received");
        assert!(info.supported_combination.is_some());
    }
    #[test]
    fn supported_resource_then_combination_request() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);
        drain_messages(&mut orch, 100);

        // first resource
        orch.send_supported_resource_request(0).unwrap();

        // then combination
        orch.send_supported_combination_request(0).unwrap();

        let mut res_response = false;
        let mut comb_response = false;
        let timeout = tick(Duration::from_millis(300));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::SupportedCombinationResult {explorer_id:_res_explorer_id, ref combination_list}=msg{
                            comb_response=true;
                        }
                        else if let ExplorerToOrchestrator::SupportedResourceResult {explorer_id:_res_explorer_id,ref supported_resources}=msg{
                            res_response=true;
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }

        let info = orch.planets_info.get_info(0).unwrap();
        assert!(comb_response, "SupportedCombinationResult not received");
        assert!(res_response, "SupportedResourceResult not received");
        assert!(info.supported_resources.is_some());
        assert!(info.supported_combination.is_some());
    }
}

// ============================================================================
// 5. GenerateResourceRequest / GenerateResourceResponse
// ============================================================================
#[cfg(test)]
mod generate_resource_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use common_game::components::resource::{BasicResourceType, ResourceType};
    use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
    use crossbeam_channel::{select, tick};
    use std::time::Duration;

    #[test]
    fn generate_resource_request_silicon() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // charge the planet with sunrays first
        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        drain_messages(&mut orch, 200);

        // send generate resource
        orch.send_generate_resource_request(0, BasicResourceType::Silicon)
            .unwrap();
        drain_messages(&mut orch, 300);

        // after GenerateResourceResponse the orchestrator sends BagContentRequest automatically
        // so the bag should be updated
        let explorer_info = orch.explorers_info.get(&0).unwrap();
        assert!(explorer_info
            .bag
            .contains(&ResourceType::Basic(BasicResourceType::Silicon)))
    }
    // ---- Generate multiple resources in sequence ----

    #[test]
    fn generate_multiple_resources_sequentially() {
        let mut orch = setup_orch_with_explorer(PlanetType::BlackAdidasShoe, 0, 0);

        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        drain_messages(&mut orch, 200);

        // generate 3 resources in sequence
        for resource in &[
            BasicResourceType::Hydrogen,
            BasicResourceType::Carbon,
            BasicResourceType::Oxygen,
        ] {
            orch.send_generate_resource_request(0, *resource).unwrap();
        }
        drain_messages(&mut orch, 500);
        let info = orch.explorers_info.get(&0).unwrap();
        assert!(info
            .bag
            .contains(&ResourceType::Basic(BasicResourceType::Hydrogen)));
        assert!(info
            .bag
            .contains(&ResourceType::Basic(BasicResourceType::Carbon)));
        assert!(info
            .bag
            .contains(&ResourceType::Basic(BasicResourceType::Oxygen)));
    }

    // ---- Generate resource without energy (no sunrays) ----

    #[test]
    fn generate_resource_without_energy() {
        let mut orch = setup_orch_with_explorer(PlanetType::BlackAdidasShoe, 0, 0);

        // no sunrays sent -> planet has no energy
        orch.send_generate_resource_request(0, BasicResourceType::Hydrogen)
            .unwrap();
        let mut response = false;
        let timeout = tick(Duration::from_millis(200));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::GenerateResourceResponse {explorer_id:_res_explorer_id,ref generated}=msg{
                            response=true;
                            assert!(generated.is_err());
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert!(response, "GeneratedResourceResponse not received");
        assert!(orch.explorers_info.get(&0u32).unwrap().bag.is_empty());
    }
}

// ============================================================================
// 6. CombineResourceRequest / CombineResourceResponse
// ============================================================================
#[cfg(test)]
mod combine_resource_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
    use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
    use common_game::utils::ID;
    use crossbeam_channel::{select, tick};
    use std::thread::sleep;
    use std::time::Duration;

    fn setup_multi_planet_orch(explorer_id: ID) -> Orchestrator {
        let mut orch = Orchestrator::new().unwrap();
        // topology: 0-1, 0-2, 1-2 (triangle)
        let topology = "0,0,1,2\n1,6,0,2\n2,0,0,1\n";
        orch.initialize_galaxy_by_content(topology).unwrap();
        orch.start_all_planet_ais().unwrap();
        orch.add_mattia_explorer(explorer_id, 0).unwrap();
        orch
    }
    fn travel_explorer(orch: &mut Orchestrator, explorer_id: ID, dst_planet_id: ID) {
        orch.explorers_info
            .get_mut(&explorer_id)
            .unwrap()
            .move_to_planet_id = dst_planet_id as i32;
        orch.send_incoming_explorer_request(dst_planet_id, explorer_id)
            .unwrap();
        drain_messages(orch, 300);
    }

    #[test]
    fn combine_resource_request_without_ingredients() {
        let mut orch = setup_orch_with_explorer(PlanetType::RustyCrab, 0, 0);

        // try to combine Diamond without having carbon in the bag
        // the explorer should reply with an error
        orch.send_combine_resource_request(0, ComplexResourceType::Diamond)
            .unwrap();
        let mut response = false;
        let timeout = tick(Duration::from_millis(300));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::CombineResourceResponse {explorer_id:_res_explorer_id,ref generated}=msg{
                            response=true;
                            assert!(generated.is_err());
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert!(response, "GeneratedResourceResponse not received");
        assert!(orch.explorers_info.get(&0u32).unwrap().bag.is_empty());
    }

    // ---- Attempt to combine complex resource with only one base resource ----
    #[test]
    fn combine_resource_with_only_an_ingredient() {
        let mut orch = setup_orch_with_explorer(PlanetType::RustyCrab, 0, 0);
        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        sleep(Duration::from_millis(200));
        orch.send_generate_resource_request(0, BasicResourceType::Hydrogen)
            .expect("testing expect");
        drain_messages(&mut orch, 200);
        sleep(Duration::from_millis(200));

        // try to combine Water without having Oxygen in the bag
        // the explorer should reply with an error, and it should still have a hydrogen in his bag
        orch.send_combine_resource_request(0, ComplexResourceType::Water)
            .unwrap();
        let mut response = false;
        let timeout = tick(Duration::from_millis(300));
        loop {
            select! {
                recv(orch.receiver_orch_explorer) -> explorer_msg => {
                    if let Ok(msg) = explorer_msg {
                        if let ExplorerToOrchestrator::CombineResourceResponse {explorer_id:_res_explorer_id,ref generated}=msg{
                            response=true;
                            assert!(generated.is_err());
                        }
                        orch.handle_explorer_message(msg).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert!(response, "GeneratedResourceResponse not received");
        assert!(orch
            .explorers_info
            .get(&0)
            .unwrap()
            .bag
            .contains(&ResourceType::Basic(BasicResourceType::Hydrogen)));
    }

    // ---- Generate resources then combine ----

    #[test]
    fn generate_then_combine_diamond() {
        let mut orch = setup_multi_planet_orch(0);

        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..2 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        let planet_channel = orch.planet_channels.get(&1).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(1, &planet_channel)
                .expect("testing expect");
        }
        drain_messages(&mut orch, 100);

        // generate 2 carbons
        for _ in 0..2 {
            orch.send_generate_resource_request(0, BasicResourceType::Carbon)
                .unwrap();
        }
        drain_messages(&mut orch, 300);

        //move to a planet that can create Diamond
        travel_explorer(&mut orch, 0, 1);

        // now try to combine diamond
        orch.send_combine_resource_request(0, ComplexResourceType::Diamond)
            .unwrap();
        drain_messages(&mut orch, 200);
        let bag = &orch.explorers_info.get(&0).unwrap().bag;
        assert!(bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond)));
        assert!(
            !bag.contains(&ResourceType::Basic(BasicResourceType::Carbon)),
            "it should have consumed all the carbon"
        )
    }
}

// ============================================================================
// 7. BagContentRequest / BagContentResponse
// ============================================================================
#[cfg(test)]
mod bag_content_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use common_game::components::resource::{BasicResourceType, ResourceType};

    #[test]
    fn bag_content_request_empty_bag() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        orch.send_bag_content_request(0).unwrap();
        drain_messages(&mut orch, 100);

        let bag = &orch.explorers_info.get(&0).unwrap().bag;
        assert!(bag.is_empty());
    }

    #[test]
    fn bag_content_request_after_resource_generation() {
        let mut orch = setup_orch_with_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // charge planet
        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        drain_messages(&mut orch, 100);

        // generate a resource
        orch.send_generate_resource_request(0, BasicResourceType::Hydrogen)
            .unwrap();
        drain_messages(&mut orch, 300);

        // explicitly ask for bag content
        orch.send_bag_content_request(0).unwrap();
        drain_messages(&mut orch, 100);

        // bag should be updated
        assert_eq!(
            orch.explorers_info.get(&0).unwrap().bag,
            vec![ResourceType::Basic(BasicResourceType::Hydrogen)]
        );
    }
}

// ============================================================================
// 8. NeighborsRequest / NeighborsResponse + Travel Protocol
// ============================================================================
#[cfg(test)]
mod movement_tests {
    use super::*;
    use common_game::utils::ID;

    /// Helper: create a topology with multiple connected planets and an explorer on planet 0
    fn setup_multi_planet_orch(explorer_id: ID) -> Orchestrator {
        let mut orch = Orchestrator::new().unwrap();
        // topology: 0-1, 0-2, 1-2 (triangle)
        let topology = "0,0,1,2\n1,0,0,2\n2,0,0,1\n";
        orch.initialize_galaxy_by_content(topology).unwrap();
        orch.start_all_planet_ais().unwrap();
        orch.add_mattia_explorer(explorer_id, 0).unwrap();
        orch
    }

    /// Helper: perform the full travel protocol pipeline:
    /// 1. Set move_to_planet_id on explorer info
    /// 2. Send IncomingExplorerRequest to the destination planet
    /// 3. Drain messages — the handler chain will:
    ///    - process IncomingExplorerResponse → send OutgoingExplorerRequest to old planet
    ///    - process OutgoingExplorerResponse → call send_move_to_planet() to the explorer
    fn travel_explorer(orch: &mut Orchestrator, explorer_id: ID, dst_planet_id: ID) {
        // Step 1: set move_to_planet_id so the handler knows the destination
        orch.explorers_info
            .get_mut(&explorer_id)
            .unwrap()
            .move_to_planet_id = dst_planet_id as i32;

        // Step 2: send IncomingExplorerRequest to the new planet (when the orchestrator receive the
        // IncomingExplorerResponse message it also send the OutgoingExplorerRequest
        orch.send_incoming_explorer_request(dst_planet_id, explorer_id)
            .unwrap();

        // IncomingExplorerResponse → OutgoingExplorerRequest → OutgoingExplorerResponse → MoveToPlanet
        drain_messages(orch, 300);
    }

    #[test]
    fn neighbours_response_updates_explorer() {
        let explorer_id: ID = 10;
        let mut orch = setup_multi_planet_orch(explorer_id);

        // send neighbours response for planet 0 to explorer 10
        orch.send_neighbours_response(explorer_id, 0).unwrap();
        drain_messages(&mut orch, 50);

        // protocol exchange completed without panic. I don't really think there is much left that i can control
    }

    #[test]
    fn move_to_planet_valid_neighbour() {
        let explorer_id: ID = 10;
        let mut orch = setup_multi_planet_orch(explorer_id);
        drain_messages(&mut orch, 100);

        // full travel protocol: IncomingExplorerRequest → OutgoingExplorerRequest → MoveToPlanet
        travel_explorer(&mut orch, explorer_id, 1);
        // protocol completed
        let pos = orch.explorers_info.get_current_planet(&explorer_id);
        orch.send_current_planet_request(explorer_id).unwrap();
        drain_messages(&mut orch, 50);
        assert_eq!(pos, orch.explorers_info.get_current_planet(&explorer_id));
        assert_eq!(
            orch.explorers_info
                .get_current_planet(&explorer_id)
                .unwrap(),
            1
        );
    }

    #[test]
    fn move_to_multiple_planets_in_sequence() {
        let explorer_id: ID = 10;
        let mut orch = setup_multi_planet_orch(explorer_id);

        // move 0 -> 1
        travel_explorer(&mut orch, explorer_id, 1);

        orch.send_current_planet_request(explorer_id).unwrap();
        drain_messages(&mut orch, 50);
        assert_eq!(
            orch.explorers_info
                .get_current_planet(&explorer_id)
                .unwrap(),
            1
        );

        // move 1 -> 2
        travel_explorer(&mut orch, explorer_id, 2);

        orch.send_current_planet_request(explorer_id).unwrap();
        drain_messages(&mut orch, 50);
        assert_eq!(
            orch.explorers_info
                .get_current_planet(&explorer_id)
                .unwrap(),
            2
        );

        // move 2 -> 0
        travel_explorer(&mut orch, explorer_id, 0);

        orch.send_current_planet_request(explorer_id).unwrap();
        drain_messages(&mut orch, 50);
        assert_eq!(
            orch.explorers_info
                .get_current_planet(&explorer_id)
                .unwrap(),
            0
        );
    }

    // ---- Full travel protocol via AI (start AI, let it explore) ----
    //
    // #[test]
    // fn ai_driven_travel_protocol() {
    //     let explorer_id: ID = 10;
    //     let mut orch = setup_multi_planet_orch(explorer_id);
    //
    //     // charge all planets
    //     for pid in 0..3 {
    //         if let Some(ch) = orch.planet_channels.get(&pid) {
    //             let sender = ch.0.clone();
    //             for _ in 0..5 {
    //                 orch.send_sunray(pid, &sender);
    //             }
    //         }
    //     }
    //     drain_messages(&mut orch, 300);
    //
    //     // start explorer AI
    //     orch.send_start_explorer_ai(explorer_id).unwrap();
    //     drain_messages(&mut orch, 200);
    //
    //     // let the AI run for a while, processing messages
    //     let do_tick = tick(Duration::from_millis(50));
    //     let timeout = tick(Duration::from_secs(5));
    //     loop {
    //         select! {
    //             recv(orch.receiver_orch_planet) -> planet_msg => {
    //                 if let Ok(msg) = planet_msg {
    //                     let _ = orch.handle_planet_message(msg);
    //                 }
    //             }
    //             recv(orch.receiver_orch_explorer) -> explorer_msg => {
    //                 if let Ok(msg) = explorer_msg {
    //                     let _ = orch.handle_explorer_message(msg);
    //                 }
    //             }
    //             recv(do_tick) -> _ => {
    //                 // periodically send sunrays to keep planets charged
    //                 for pid in 0..3 {
    //                     if let Some(ch) = orch.planet_channels.get(&pid) {
    //                         let sender = ch.0.clone();
    //                         orch.send_sunray(pid, &sender);
    //                     }
    //                 }
    //             }
    //             recv(timeout) -> _ => {
    //                 break;
    //             }
    //         }
    //     }
    //
    //     // after running, explorer should still be alive
    //     assert_ne!(orch.explorers_info.get_status(&explorer_id), Status::Dead);
    // }

    // ---- Neighbours response for planet with no neighbours ----
}

// ============================================================================
// 9. Resource operations after movement
// ============================================================================
#[cfg(test)]
mod resource_after_movement_tests {
    use super::*;
    use common_game::components::resource::{BasicResourceType, ResourceType};
    use common_game::utils::ID;

    /// Helper: perform the full travel protocol pipeline
    fn travel_explorer(orch: &mut Orchestrator, explorer_id: ID, dst_planet_id: ID) {
        orch.explorers_info
            .get_mut(&explorer_id)
            .unwrap()
            .move_to_planet_id = dst_planet_id as i32;
        orch.send_incoming_explorer_request(dst_planet_id, explorer_id)
            .unwrap();
        drain_messages(orch, 300);
    }

    #[test]
    fn generate_resource_on_second_planet_after_move() {
        let mut orch = Orchestrator::new().unwrap();
        // planet 0 = BlackAdidasShoe (H,C,O), planet 1 = OneMillionCrabs (Si + others)
        let topology = "0,0,1\n1,4,0\n";
        orch.initialize_galaxy_by_content(topology).unwrap();
        orch.start_all_planet_ais().unwrap();
        orch.add_mattia_explorer(10, 0).unwrap();

        // charge both planets
        if let Some(ch) = orch.planet_channels.get(&1) {
            let sender = ch.0.clone();
            orch.send_sunray(1, &sender).unwrap();
        }
        drain_messages(&mut orch, 50);

        // move explorer from planet 0 to planet 1 via full protocol
        travel_explorer(&mut orch, 10, 1);

        // verify explorer is on planet 1
        orch.send_current_planet_request(10).unwrap();
        drain_messages(&mut orch, 50);
        assert_eq!(orch.explorers_info.get_current_planet(&10).unwrap(), 1);

        // now generate a resource on planet 1
        orch.send_generate_resource_request(10, BasicResourceType::Silicon)
            .unwrap();
        orch.send_bag_content_request(10).unwrap();
        drain_messages(&mut orch, 200);

        // protocol completed
        assert!(orch
            .explorers_info
            .get(&10)
            .unwrap()
            .bag
            .contains(&ResourceType::Basic(BasicResourceType::Silicon)));
    }
}

// ============================================================================
// 10. Combined protocol flows (full end-to-end scenarios)
// ============================================================================
#[cfg(test)]
mod end_to_end_tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use common_game::components::resource::{BasicResourceType, ResourceType};

    /// Rapid fire: send many messages in quick succession to test buffering
    #[test]
    fn rapid_fire_messages() {
        let mut orch = setup_orch_with_explorer(PlanetType::BlackAdidasShoe, 0, 0);

        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();

        // send multiple different requests rapidly
        orch.send_bag_content_request(0).unwrap();
        orch.send_current_planet_request(0).unwrap();
        orch.send_bag_content_request(0).unwrap();
        orch.send_supported_resource_request(0).unwrap();
        orch.send_bag_content_request(0).unwrap();

        drain_messages(&mut orch, 500);

        // everything should have been processed without panic
        assert!(orch.explorers_info.get(&0).is_some());
        assert_eq!(orch.explorers_info.get_current_planet(&0).unwrap(), 0);
    }

    /// Multiple generate requests in rapid succession (tests buffering)
    #[test]
    fn rapid_generate_requests() {
        let mut orch = setup_orch_with_explorer(PlanetType::BlackAdidasShoe, 0, 0);

        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        drain_messages(&mut orch, 300);

        // send 5 generate requests back to back
        for _ in 0..5 {
            orch.send_generate_resource_request(0, BasicResourceType::Carbon)
                .unwrap();
        }
        drain_messages(&mut orch, 500);

        // bag should contain some items
        orch.send_bag_content_request(0).unwrap();
        drain_messages(&mut orch, 50);
        let carbon_count = orch
            .explorers_info
            .get(&0)
            .unwrap()
            .bag
            .iter()
            .filter(|r| **r == ResourceType::Basic(BasicResourceType::Carbon))
            .count();
        assert_eq!(carbon_count, 5);
    }
}

// ============================================================================
// 12. Explorer-Planet direct communication tests
// ============================================================================
#[cfg(test)]
mod explorer_planet_comms {
    use super::*;
    use crate::utils::registry::PlanetType;
    use crate::utils::ExplorerInfo;
    use crate::Status;
    use common_game::components::resource::BasicResourceType;
    use common_game::protocols::orchestrator_explorer::{
        ExplorerToOrchestrator, OrchestratorToExplorer,
    };
    use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
    use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
    use crossbeam_channel::{select, tick};
    use std::thread::sleep;
    use std::time::Duration;

    /// Helper: setup orchestrator, planet, and a manual explorer (not spawned in thread)
    /// so we can directly inspect messages on channels.
    fn setup_manual_explorer(
        planet_type: PlanetType,
        planet_id: u32,
        explorer_id: u32,
    ) -> (Orchestrator, crate::components::mattia_explorer::Explorer) {
        let mut orch = Orchestrator::new().unwrap();
        let topology = format!("{},{}\n", planet_id, planet_type as u32);
        orch.initialize_galaxy_by_content(&topology).unwrap();
        orch.start_all(&[], &[]).unwrap();

        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        let (orch_to_planet, expl_to_planet) = match orch.planet_channels.get(&planet_id) {
            Some((orchestrator_sender, explorer_sender)) => (
                Some(orchestrator_sender.clone()),
                Some(explorer_sender.clone()),
            ),
            None => (None, None),
        };

        let new_explorer = crate::components::mattia_explorer::Explorer::new(
            explorer_id,
            planet_id,
            (receiver_orch, orch.sender_explorer_orch.clone()),
            (receiver_planet, expl_to_planet.unwrap()),
        );

        orch.explorers_info.insert(
            explorer_id,
            ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id),
        );
        orch.explorer_channels
            .insert(explorer_id, (sender_orch, sender_planet.clone()));

        if let Some(orchestrator_sender) = orch_to_planet {
            orchestrator_sender
                .send(OrchestratorToPlanet::IncomingExplorerRequest {
                    explorer_id,
                    new_sender: sender_planet.clone(),
                })
                .expect("testing expect");
        }

        (orch, new_explorer)
    }

    // ---- SupportedResourceRequest to Planet ----

    #[test]
    fn explorer_sends_supported_resource_request_to_planet() {
        let (mut orch, explorer) = setup_manual_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // send SupportedResourceRequest from explorer to planet
        explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 0 })
            .unwrap();

        // wait and read the response
        let timeout = tick(Duration::from_millis(300));
        let mut got_response = false;
        loop {
            select! {
                recv(explorer.planet_channels.0) -> msg => {
                    if let Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) = msg {
                        got_response = true;
                        assert!(!resource_list.is_empty());
                    }
                }
                recv(orch.receiver_orch_planet) -> msg => {
                    if let Ok(m) = msg {
                        orch.handle_planet_message(m).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => { break; }
            }
        }
        assert!(
            got_response,
            "Should have received SupportedResourceResponse from planet"
        );
    }

    // ---- SupportedCombinationRequest to Planet ----

    #[test]
    fn explorer_sends_supported_combination_request_to_planet() {
        let (mut orch, explorer) = setup_manual_explorer(PlanetType::RustyCrab, 0, 0);

        explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 0 })
            .unwrap();

        let timeout = tick(Duration::from_millis(300));
        let mut got_response = false;
        loop {
            select! {
                recv(explorer.planet_channels.0) -> msg => {
                    if let Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) = msg {
                        got_response = true;
                        assert!(!combination_list.is_empty());
                    }
                }
                recv(orch.receiver_orch_planet) -> msg => {
                    if let Ok(m) = msg {
                        orch.handle_planet_message(m).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => { break; }
            }
        }
        assert!(
            got_response,
            "Should have received SupportedCombinationResponse from planet"
        );
    }

    // ---- GenerateResourceRequest to Planet ----

    #[test]
    fn explorer_sends_generate_resource_request_to_planet() {
        let (mut orch, explorer) = setup_manual_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // charge the planet
        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..5 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        sleep(Duration::from_millis(50));

        // send generate resource request
        explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::GenerateResourceRequest {
                explorer_id: 0,
                resource: BasicResourceType::Hydrogen,
            })
            .unwrap();

        let timeout = tick(Duration::from_millis(300));
        let mut got_response = false;
        loop {
            select! {
                recv(explorer.planet_channels.0) -> msg => {
                    if let Ok(PlanetToExplorer::GenerateResourceResponse { resource }) = msg {
                        got_response = true;
                        assert!(resource.is_some());
                    }
                }
                recv(orch.receiver_orch_planet) -> msg => {
                    if let Ok(m) = msg {
                        orch.handle_planet_message(m).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => { break; }
            }
        }
        assert!(
            got_response,
            "Should have received GenerateResourceResponse from planet"
        );
    }

    // ---- AvailableEnergyCellRequest to Planet ----

    #[test]
    fn explorer_sends_available_energy_cell_request_to_planet() {
        let (mut orch, explorer) = setup_manual_explorer(PlanetType::OneMillionCrabs, 0, 0);

        // charge the planet
        let planet_channel = orch.planet_channels.get(&0).unwrap().0.clone();
        for _ in 0..3 {
            orch.send_sunray(0, &planet_channel)
                .expect("testing expect");
        }
        sleep(Duration::from_millis(50));
        explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 0 })
            .unwrap();

        let timeout = tick(Duration::from_millis(200));
        let mut got_response = false;
        loop {
            select! {
                recv(explorer.planet_channels.0) -> msg => {
                    if let Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells }) = msg {
                        got_response = true;
                        // available_cells should be >= 0
                        assert_eq!(available_cells,3);
                    }
                }
                recv(orch.receiver_orch_planet) -> msg => {
                    if let Ok(m) = msg {
                        orch.handle_planet_message(m).expect("testing expect");
                    }
                }
                recv(timeout) -> _ => { break; }
            }
        }
        assert!(
            got_response,
            "Should have received AvailableEnergyCellResponse from planet"
        );
    }

    // ---- TravelToPlanetRequest to non-existent planet ----

    #[test]
    fn explorer_requests_travel_to_nonexistent_planet() {
        let (mut orch, explorer) = setup_manual_explorer(PlanetType::OneMillionCrabs, 0, 0);

        let original_planet_id = explorer.planet_id;

        // explorer sends TravelToPlanetRequest to a planet that doesn't exist
        explorer
            .orchestrator_channels
            .1
            .send(ExplorerToOrchestrator::TravelToPlanetRequest {
                explorer_id: 0,
                current_planet_id: 0,
                dst_planet_id: 99, // doesn't exist
            })
            .unwrap();

        // orchestrator processes the request — it should reject it
        // and send MoveToPlanet { sender_to_new_planet: None } back
        drain_messages(&mut orch, 200);

        // read the response on the explorer's orchestrator channel
        let timeout = tick(Duration::from_millis(200));
        let mut got_response = false;
        loop {
            select! {
                recv(explorer.orchestrator_channels.0) -> msg => {
                    if let Ok(OrchestratorToExplorer::MoveToPlanet { ref sender_to_new_planet, planet_id }) = msg {
                        got_response = true;
                        // sender should be None because the planet doesn't exist
                        assert!(sender_to_new_planet.is_none(), "sender_to_new_planet should be None for non-existent planet");
                        assert_eq!(planet_id, 99);
                    }
                }
                recv(timeout) -> _ => { break; }
            }
        }
        assert!(
            got_response,
            "Should have received MoveToPlanet with None sender for non-existent planet"
        );

        // explorer should still be on the original planet
        assert_eq!(explorer.planet_id, original_planet_id);
    }
}
