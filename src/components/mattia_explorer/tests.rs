
mod test_One_million_crabs_planet{
    use std::thread::sleep;
    use std::time::Duration;
    use common_game::components::resource::BasicResourceType;
    use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
    use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
    use common_game::protocols::planet_explorer::ExplorerToPlanet::GenerateResourceRequest;
    use crossbeam_channel::{select, tick};
    use rand::Rng;
    use crate::{Orchestrator, Status};
    use crate::utils::ExplorerInfo;
    use crate::utils::registry::PlanetType;
    use super::*;
    #[test]
    fn stress_planet_energy_cells_management_2(){
        let mut orchestrator = Orchestrator::new().unwrap();
        let planet_id = 1;
        let explorer_id = 2;
        orchestrator.add_planet(planet_id, PlanetType::OneMillionCrabs).unwrap();
        orchestrator.start_all().unwrap();

        //Create the comms for the new explorer
        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        // get the sender from explorer to planet
        let (orch_to_planet, expl_to_planet) = match orchestrator.planet_channels.get(&planet_id) {
            Some((orchestrator_sender, explorer_sender)) => (Some(orchestrator_sender.clone()),Some(explorer_sender.clone())),
            None => {
                (None,None)
            }, // sender does not exist
        };

        //Construct Explorer
        let mut new_explorer = crate::components::mattia_explorer::Explorer::new(
            explorer_id,
            planet_id,
            (receiver_orch, orchestrator.sender_explorer_orch.clone()),
            (receiver_planet, expl_to_planet.unwrap()), // TODO this unwrap is unsafe
        );


        //Update HashMaps
        orchestrator.explorers_info.insert(explorer_id, ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id));


        orchestrator.explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet.clone()));

        match orch_to_planet {
            Some(orchestrator_sender) => {
                match orchestrator_sender.send(
                    OrchestratorToPlanet::IncomingExplorerRequest {
                        explorer_id,
                        new_sender: sender_planet.clone(),
                    }
                ){
                    Ok(_) => {},
                    Err(err) => {
                    }
                }
            }
            None => {}
        }


        let planet_channel = orchestrator.planet_channels.get(&planet_id).unwrap().0.clone();
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        orchestrator.send_sunray(planet_id, &planet_channel);
        new_explorer.planet_channels.1.send(GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Silicon,
        });
        sleep(Duration::from_secs(1));
        orchestrator.send_bag_content_request(explorer_id);
        orchestrator.send_internal_state_request(&orchestrator.planet_channels.get(&planet_id).unwrap().0);
        new_explorer.planet_channels.1.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id });

        let timeout = tick(Duration::from_millis(1000));
        let mut available_energy_cells:i32=-1;
        loop{
            select! {
                recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                    match planet_msg {
                        Ok(msg) => {
                            orchestrator.handle_planet_message(msg);
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
        assert_eq!(orchestrator.planets_info.get_info(planet_id).unwrap().energy_cells.iter().filter(|&&x| x ).count(), available_energy_cells as usize);
    }
    #[test]
    fn stress_planet_energy_cells_management_3(){
        let mut orchestrator = Orchestrator::new().unwrap();
        let planet_id = 1;
        let explorer_id = 2;
        orchestrator.add_planet(planet_id, PlanetType::OneMillionCrabs).unwrap();
        orchestrator.start_all().unwrap();

        //Create the comms for the new explorer
        let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
            Orchestrator::init_comms_explorers();

        // get the sender from explorer to planet
        let (orch_to_planet, expl_to_planet) = match orchestrator.planet_channels.get(&planet_id) {
            Some((orchestrator_sender, explorer_sender)) => (Some(orchestrator_sender.clone()),Some(explorer_sender.clone())),
            None => {
                (None,None)
            }, // sender does not exist
        };

        //Construct Explorer
        let mut new_explorer = crate::components::mattia_explorer::Explorer::new(
            explorer_id,
            planet_id,
            (receiver_orch, orchestrator.sender_explorer_orch.clone()),
            (receiver_planet, expl_to_planet.unwrap()), // TODO this unwrap is unsafe
        );


        //Update HashMaps
        orchestrator.explorers_info.insert(explorer_id, ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id));


        orchestrator.explorer_channels
            .insert(new_explorer.id(), (sender_orch, sender_planet.clone()));

        match orch_to_planet {
            Some(orchestrator_sender) => {
                match orchestrator_sender.send(
                    OrchestratorToPlanet::IncomingExplorerRequest {
                        explorer_id,
                        new_sender: sender_planet.clone(),
                    }
                ){
                    Ok(_) => {},
                    Err(err) => {
                    }
                }
            }
            None => {}
        }


        let planet_channel = orchestrator.planet_channels.get(&planet_id).unwrap().0.clone();

        // max charge
        for _ in 0..5 { orchestrator.send_sunray(planet_id, &planet_channel); }

        // mixed messages
        for i in 0..200 {
            if i % 3 == 0 {
                orchestrator.send_sunray(planet_id, &planet_channel);
            }
            new_explorer.planet_channels.1.send(GenerateResourceRequest {
                explorer_id,
                resource: BasicResourceType::Silicon,
            });
        }

        sleep(Duration::from_secs(1));
        orchestrator.send_bag_content_request(explorer_id);
        orchestrator.send_internal_state_request(&orchestrator.planet_channels.get(&planet_id).unwrap().0);
        new_explorer.planet_channels.1.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id });

        let timeout = tick(Duration::from_millis(1000));
        let mut available_energy_cells:i32=-1;
        loop{
            select! {
                    recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                        match planet_msg {
                            Ok(msg) => {
                                orchestrator.handle_planet_message(msg);
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
        assert_eq!(orchestrator.planets_info.get_info(planet_id).unwrap().energy_cells.iter().filter(|&&x| x ).count(), available_energy_cells as usize);
    }

    #[test]
    #[ignore] //takes about 7/8 minutes to execute with debug logs
    fn stress_planet_energy_cells_management_4(){
        let mut orchestrator = Orchestrator::new().unwrap();
        for _ in 0..50 {
            let planet_id = 1;
            let explorer_id = 2;
            orchestrator.add_planet(planet_id, PlanetType::OneMillionCrabs).unwrap();
            orchestrator.start_all().unwrap();

            //Create the comms for the new explorer
            let (sender_orch, receiver_orch, sender_planet, receiver_planet) =
                Orchestrator::init_comms_explorers();

            // get the sender from explorer to planet
            let (orch_to_planet, expl_to_planet) = match orchestrator.planet_channels.get(&planet_id) {
                Some((orchestrator_sender, explorer_sender)) => (Some(orchestrator_sender.clone()), Some(explorer_sender.clone())),
                None => {
                    (None, None)
                }, // sender does not exist
            };

            //Construct Explorer
            let mut new_explorer = crate::components::mattia_explorer::Explorer::new(
                explorer_id,
                planet_id,
                (receiver_orch, orchestrator.sender_explorer_orch.clone()),
                (receiver_planet, expl_to_planet.unwrap()), // TODO this unwrap is unsafe
            );


            //Update HashMaps
            orchestrator.explorers_info.insert(explorer_id, ExplorerInfo::from(explorer_id, Status::Paused, Vec::new(), planet_id));


            orchestrator.explorer_channels
                .insert(new_explorer.id(), (sender_orch, sender_planet.clone()));

            match orch_to_planet {
                Some(orchestrator_sender) => {
                    match orchestrator_sender.send(
                        OrchestratorToPlanet::IncomingExplorerRequest {
                            explorer_id,
                            new_sender: sender_planet.clone(),
                        }
                    ) {
                        Ok(_) => {},
                        Err(err) => {}
                    }
                }
                None => {}
            }


            let planet_channel = orchestrator.planet_channels.get(&planet_id).unwrap().0.clone();

            // max charge
            for _ in 0..5 { orchestrator.send_sunray(planet_id, &planet_channel); }

            // mixed messages
            let mut rng = rand::rng();

            for _ in 0..200 {
                // 30% di probabilità di inviare un raggio di sole
                if rng.random_bool(0.5) {
                    orchestrator.send_sunray(planet_id, &planet_channel);
                }

                // 70% di probabilità di provare a generare una risorsa
                if rng.random_bool(0.5) {
                    let _ = new_explorer.planet_channels.1.send(GenerateResourceRequest {
                        explorer_id,
                        resource: BasicResourceType::Silicon,
                    });
                }

                if rng.random_bool(0.1) {
                    sleep(Duration::from_millis(50));
                }
            }

            sleep(Duration::from_secs(1));
            orchestrator.send_bag_content_request(explorer_id);

            let timeout = tick(Duration::from_millis(3000));

            loop {
                select! {
                        recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                            match planet_msg {
                                Ok(msg) => {
                                    orchestrator.handle_planet_message(msg);
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
            new_explorer.planet_channels.1.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id });
            orchestrator.send_internal_state_request(&orchestrator.planet_channels.get(&planet_id).unwrap().0);

            let timeout = tick(Duration::from_millis(2000));
            let mut available_energy_cells: i32 = -1;
            loop {
                select! {
                        recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                            match planet_msg {
                                Ok(msg) => {
                                    orchestrator.handle_planet_message(msg);
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
            assert_eq!(orchestrator.planets_info.get_info(planet_id).unwrap().energy_cells.iter().filter(|&&x| x ).count(), available_energy_cells as usize);

            // killing planet and explorer
            orchestrator.send_planet_kill_to_all();
            orchestrator.send_kill_explorer_ai(explorer_id);
            orchestrator.planets_info.map.clear();
            orchestrator.planet_channels.clear();
            orchestrator.explorer_channels.clear();
            orchestrator.explorers_info.map.clear();
            sleep(Duration::from_millis(100));
        }
    }
}

mod game_simulation{
    use std::thread::sleep;
    use std::time::Duration;
    use crossbeam_channel::{select, tick};
    use crate::{debug_println, Orchestrator};
    use super::*;
    #[test]
    fn simulation_25s(){
        let mut orchestrator= Orchestrator::new().unwrap();
        orchestrator.initialize_galaxy_by_file("./src/components/mattia_explorer/test_topology_files/t0.txt");
        orchestrator.start_all_planet_ais();
        // println!("galaxy topology: {:?}", orchestrator.get_topology());
        orchestrator.add_mattia_explorer(10, 0);
        orchestrator.start_all_explorer_ais();
        // println!("aaaaaaa");
        // sleep(Duration::from_secs(1));
        // orchestrator.send_supported_resource_request(10); //this breaks everything somehow
        let do_something = tick(Duration::from_millis(50));
        let mut counter =500;
        println!("aaaaaaa");
        loop {
            select! {
                recv(orchestrator.receiver_orch_planet) -> planet_msg => {
                    match planet_msg {
                        Ok(msg) => {
                            orchestrator.handle_planet_message(msg);
                        }
                        Err(_) => {}
                    }
                }
                recv(orchestrator.receiver_orch_explorer) ->explorer_msg =>{
                    match explorer_msg {
                        Ok(msg) => {
                            debug_println!("orchestrator received a message from an explorer");
                            orchestrator.handle_explorer_message(msg);
                        }
                        Err(_) => {
                            debug_println!("error receiving messages from explorer");
                        }
                    }
                }
                recv(do_something) -> _ => {
                    counter -= 1;
                    if counter == 0 {
                        orchestrator.send_planet_kill_to_all();
                    }
                    else if counter < 0 {
                        break;
                    }
                    else{
                        orchestrator.choose_random_action();
                    }
                }
            }
        }
    }
}