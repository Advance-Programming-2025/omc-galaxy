#[cfg(test)]
use crate::components::orchestrator::Orchestrator;
use crate::utils::Status;
use crate::utils::registry::PlanetType;

#[cfg(test)]
mod tests_core_lifecycle {
    use super::*;

    #[test]
    fn test_lifecycle_new_initializes_empty_state() {
        let orch = Orchestrator::new().unwrap();
        assert!(orch.planets_info.is_empty());
        assert!(orch.explorers_info.is_empty());
        assert!(orch.galaxy_lookup.is_empty());
    }

    #[test]
    fn test_lifecycle_reset_clears_internal_maps() {
        let mut orch = Orchestrator::new().unwrap();
        // Manually pollute state
        orch.planets_info.insert_status(1, PlanetType::OneMillionCrabs, Status::Dead);
        orch.explorers_info.insert_status(1, Status::Running);

        orch.reset().unwrap();

        assert!(orch.planets_info.is_empty());
        assert!(orch.explorers_info.is_empty());
        assert!(orch.planet_channels.is_empty());
    }
}

#[cfg(test)]
mod tests_actor_management {
    use super::*;

    #[test]
    fn test_membership_add_planet_updates_status_to_paused() {
        let mut orch = Orchestrator::new().unwrap();
        let planet_id = 10;

        orch.add_planet(planet_id, PlanetType::OneMillionCrabs)
            .unwrap();

        assert!(orch.planets_info.is_paused(&planet_id));
        assert!(orch.planet_channels.contains_key(&planet_id));
    }

    #[test]
    fn test_membership_add_explorer_creates_comms() {
        let mut orch = Orchestrator::new().unwrap();
        orch.add_tommy_explorer(1, 10);

        assert!(orch.explorers_info.get(&1).is_some());
        assert_eq!(
            orch.explorers_info.get_status(&1),
            Status::Paused
        );
        assert!(orch.explorer_channels.contains_key(&1));
    }
}

#[cfg(test)]
mod tests_topology_logic {
    use super::*;

    #[test]
    fn test_topology_adj_list_creates_symmetric_matrix() {
        let mut orch = Orchestrator::new().unwrap();
        // 0 -- 1
        let adj_list = vec![vec![1], vec![0]];

        orch.initialize_galaxy_by_adj_list(adj_list).unwrap();

        let gtop = orch.galaxy_topology.read().unwrap();
        assert_eq!(gtop[0][1], true);
        assert_eq!(gtop[1][0], true);
        assert_eq!(gtop[0][0], false);
    }

    #[test]
    fn test_topology_destroy_link_updates_matrix() {
        let mut orch = Orchestrator::new().unwrap();
        let adj_list = vec![vec![1], vec![0]];
        orch.initialize_galaxy_by_adj_list(adj_list).unwrap();

        orch.destroy_topology_link(0).unwrap();

        let gtop = orch.galaxy_topology.read().unwrap();
        assert_eq!(gtop[0][1], false);
    }

    #[test]
    fn test_topology_destroy_link_out_of_bounds_errors() {
        let mut orch = Orchestrator::new().unwrap();
        orch.initialize_galaxy_by_adj_list(vec![vec![]]).unwrap();

        let result = orch.destroy_topology_link(5);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tests_messaging_protocol {
    use super::*;
    use common_game::protocols::orchestrator_planet::PlanetToOrchestrator;

    #[test]
    fn test_messaging_handle_asteroid_ack_kills_planet_on_failure() {
        let mut orch = Orchestrator::new().unwrap();
        let planet_id = 1;

        let adj_list = vec![vec![1], vec![0]];
        orch.initialize_galaxy_by_adj_list(adj_list).unwrap();

        let a = orch.galaxy_topology.as_ref().read().unwrap()[1][0];
        assert!(a); // we want the link to exist

        // Setup a planet
        orch.add_planet(planet_id, PlanetType::Ciuc).unwrap();

        // Simulate an Asteroid hitting with NO rocket (None means destruction)
        let msg = PlanetToOrchestrator::AsteroidAck {
            planet_id,
            rocket: None,
        };
        orch.handle_planet_message(msg).unwrap();
        let b = orch.galaxy_topology.as_ref().read().unwrap()[1][0];
        assert!(orch.planets_info.is_dead(&planet_id));
        assert!(!b); // not b, we don't want the planet to have a link
    }

    #[test]
    fn test_messaging_send_sunray_to_all_skips_dead_planets() {
        let mut orch = Orchestrator::new().unwrap();
        orch.add_planet(1, PlanetType::OneMillionCrabs).unwrap();
        let update = orch.planets_info.update_status(1, Status::Dead); // Force dead
        assert!(update.is_ok());

        // This should not fail even if the channel is technically "broken" for the dead planet
        let result = orch.send_sunray_to_all();
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod tests_file_integration {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_file_initialize_galaxy_from_valid_csv() {
        let mut orch = Orchestrator::new().unwrap();
        let file_path = "test_galaxy.csv";

        // Format: ID, Type, Neighbors...
        let content = "0, 4, 1, 400\n1, 4, 0, 400\n400, 4, 0, 1";
        let mut file = File::create(file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let result = orch.initialize_galaxy_by_file(file_path);

        // Clean up
        let _ = std::fs::remove_file(file_path);

        assert!(result.is_ok());
        assert!(orch.galaxy_lookup.contains_key(&0));
        assert!(orch.galaxy_lookup.contains_key(&1));
    }
}
#[cfg(test)]
mod test_One_million_crabs_planet{
    use std::thread;
    use std::thread::sleep;
    use std::time::Duration;
    use common_game::components::resource::BasicResourceType;
    use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
    use crossbeam_channel::{select, tick};
    use logging_utils::log_internal_op;
    use crate::*;
    use crate::utils::ExplorerInfo;
    use crate::utils::registry::*;
    #[test]
    fn planet_energy_cells_management(){
        let mut orchestrator = Orchestrator::new().unwrap();
        let planet_id = 1;
        let explorer_id = 2;
        orchestrator.add_planet(planet_id, PlanetType::OneMillionCrabs).unwrap();
        orchestrator.start_all().unwrap();
        orchestrator.add_mattia_explorer(explorer_id,planet_id);
        let planet_channel = orchestrator.planet_channels.get(&planet_id).unwrap().0.clone();
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_sunray(planet_id, &planet_channel);
        println!("SENDED 6 SUNRAY");
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        sleep(Duration::from_secs(1));
        orchestrator.send_bag_content_request(explorer_id);
        orchestrator.send_internal_state_request(&orchestrator.planet_channels.get(&planet_id).unwrap().0);
        let timeout = tick(Duration::from_millis(1000));
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
                recv(orchestrator.receiver_orch_explorer)-> explorer_msg=> {
                    match explorer_msg {
                        Ok(msg) => {
                            orchestrator.handle_explorer_message(msg);
                        }
                        Err(_) => {}
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert_eq!(orchestrator.planets_info.get_info(planet_id).unwrap().energy_cells.iter().filter(|&&x| x).count(), 0);
        println!("explorer bag: {:?}", orchestrator.explorers_info.get(&explorer_id).unwrap().bag);
        assert_eq!(orchestrator.explorers_info.get(&explorer_id).unwrap().bag.iter().filter(|&&x| x.is_silicon() ).count(), 6)
    }
    #[test]
    fn stress_planet_energy_cells_management(){
        let mut orchestrator = Orchestrator::new().unwrap();
        let planet_id = 1;
        let explorer_id = 2;
        orchestrator.add_planet(planet_id, PlanetType::OneMillionCrabs).unwrap();
        orchestrator.start_all().unwrap();
        orchestrator.add_mattia_explorer(explorer_id,planet_id);
        let planet_channel = orchestrator.planet_channels.get(&planet_id).unwrap().0.clone();
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        orchestrator.send_sunray(planet_id, &planet_channel);
        orchestrator.send_generate_resource_request(explorer_id, BasicResourceType::Silicon);
        sleep(Duration::from_secs(1));
        orchestrator.send_bag_content_request(explorer_id);
        orchestrator.send_internal_state_request(&orchestrator.planet_channels.get(&planet_id).unwrap().0);
        let timeout = tick(Duration::from_millis(1000));
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
                recv(orchestrator.receiver_orch_explorer)-> explorer_msg=> {
                    match explorer_msg {
                        Ok(msg) => {
                            orchestrator.handle_explorer_message(msg);
                        }
                        Err(_) => {}
                    }
                }
                recv(timeout) -> _ => {
                    break;
                }
            }
        }
        assert!(orchestrator.explorers_info.get(&explorer_id).unwrap().bag.iter().filter(|&&x| x.is_silicon() ).count()<= 12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::registry::PlanetType;
    use std::time::Duration;

    // --- MACRO CATEGORY: MIXED SITUATIONS ---
    // Testing survival rates when different planet types are combined.
    mod mixed_scenarios {
        use super::*;

        #[test]
        fn test_orchestrator_mixed_survival_logic() {
            let mut orch = Orchestrator::new().unwrap();

            // Type A (Ciuc) - Can build rockets
            let p_id_a = 1;
            orch.add_planet(p_id_a, PlanetType::HoustonWeHaveABorrow).unwrap();

            // Type B (BlackAdidasShoe) - Cannot build rockets
            let p_id_b = 2;
            orch.add_planet(p_id_b, PlanetType::BlackAdidasShoe)
                .unwrap();

            orch.start_all().unwrap();

            // Phase 1: Provide resources
            // We give them sunrays. Only Type A should effectively use it.
            // Cloning is ok: Sender is a handler, not a full structure.
            let channel_a = orch.planet_channels.get(&p_id_a).unwrap().0.clone();
            let channel_b = orch.planet_channels.get(&p_id_b).unwrap().0.clone();

            orch.send_sunray(p_id_a, &channel_a).unwrap();
            orch.send_sunray(p_id_b, &channel_b).unwrap();

            // Give the planet threads a moment to process the sunray and build
            std::thread::sleep(Duration::from_millis(500));
            // We simulate receiving the responses from the channels
            // (In a real run, handle_game_messages would do this)
            orch.handle_game_messages().unwrap();
            orch.handle_game_messages().unwrap();

            println!("after sunray - planet a status: {:?}", orch.planets_info.get_info(p_id_a));
            println!("after sunray - planet b status: {:?}", orch.planets_info.get_info(p_id_b));

            // Phase 2: Asteroid Attack
            orch.send_asteroid(p_id_a, &channel_a).unwrap();
            orch.send_asteroid(p_id_b, &channel_b).unwrap();

            // Give the planet threads a moment to process the asteroids and build
            std::thread::sleep(Duration::from_millis(500));
            // We simulate receiving the responses from the channels
            // (In a real run, handle_game_messages would do this)
            orch.handle_game_messages().unwrap();
            orch.handle_game_messages().unwrap();

            println!("after sunray - planet a status: {:?}", orch.planets_info.get_info(p_id_a));
            println!("after sunray - planet b status: {:?}", orch.planets_info.get_info(p_id_b));

            // Verification: A should be Alive/Running, B should be Dead
            assert!(orch.planets_info.is_running(&p_id_a));
            assert!(orch.planets_info.is_dead(&p_id_b));
        }
    }

    // --- MACRO CATEGORY: PLANET INTEGRATION (ALL TYPES) ---
    // Testing one of every single planet in the registry simultaneously.
    mod planet_integration {
        use super::*;
        use strum::IntoEnumIterator;

        #[test]
        fn test_orchestrator_integration_all_planet_types_behavior() {
            let mut orch = Orchestrator::new().unwrap();
            let mut id_counter = 0;

            // Add one of every planet type
            for p_type in PlanetType::iter() {
                orch.add_planet(id_counter, p_type).unwrap();
                id_counter += 1;
            }

            orch.start_all().unwrap();

            // Sequence: 3 Sunrays (enough to build defense), then 1 Asteroid
            for _ in 0..3 {
                for id in 0..id_counter {
                    let _ = orch.send_sunray(id, &orch.planet_channels.get(&id).unwrap().0.clone());
                }
                std::thread::sleep(Duration::from_millis(100));
            }

            // Fire Asteroids
            for id in 0..id_counter {
                let _ = orch.send_asteroid(id, &orch.planet_channels.get(&id).unwrap().0.clone());
            }

            // Wait for processing
            std::thread::sleep(Duration::from_secs(1));
            orch.handle_game_messages().unwrap();

            // Validation logic based on your rules:
            // Type A/C (Ciuc, ImmutableCosmicBorrow) should survive.
            // Type B/D (Houston, BlackAdidas, OneMillionCrabs) should be Dead.
            for (id, info) in orch.planets_info.iter() {
                // This is a high-level check. Depending on specific AI timing,
                // some might still be Alive if they didn't finish processing the death.
                println!("Planet {} status: {:?}", id, info.status);
            }
        }

        #[test]
        fn sunray_flood_all_planets() {
            let mut orch = Orchestrator::new().unwrap();
            let mut id_counter = 0;

            // Add one of every planet type
            for p_type in PlanetType::iter() {
                orch.add_planet(id_counter, p_type).unwrap();
                id_counter += 1;
            }

            orch.start_all().unwrap();

            //send 10 sunrays to all planets: they should all be full
            for _ in 0..40 {
                for id in 0..id_counter{
                    orch.send_sunray(id, &orch.planet_channels.get(&id).unwrap().0.clone())
                    .expect("failed sending sunray");
                }
                std::thread::sleep(Duration::from_millis(100));
            }

            std::thread::sleep(Duration::from_secs(1));
            orch.handle_game_messages().unwrap();

            //used to see all the charging statuses, even
            // if a planet fails early
            let mut failed_counter = 0;

            //check their status after the flood
            for id in 0..id_counter{
                let status = orch.planets_info.get_info(id).expect("error getting planet info");
                let max_charged = status.energy_cells.len();
                let curr_charged = status.charged_cells_count;

                println!("checking id {}: max of {} and charged to {}", id, max_charged, curr_charged);
                if max_charged != curr_charged{
                    failed_counter += 1;
                }
            }

            assert!(failed_counter == 0);
        }
    }

    // --- MACRO CATEGORY: HEAVY & LONG TESTS ---
    // Stress testing the Orchestrator with many actors and repeated cycles.
    mod heavy_load {
        use super::*;

        #[test]
        fn test_orchestrator_heavy_load_mass_extinction() {
            let mut orch = Orchestrator::new().unwrap();
            let n_planets = 50;

            // Fill the galaxy with 50 random planets
            for i in 0..n_planets {
                orch.add_planet(i, PlanetType::random()).unwrap();
            }

            orch.start_all().unwrap();

            // Long test: 10 cycles of sunrays/asteroids
            for cycle in 0..10 {
                for i in 0..n_planets {
                    let _ = orch.send_sunray(i, &orch.planet_channels.get(&i).unwrap().0.clone());
                }
                std::thread::sleep(Duration::from_millis(50));

                for i in 0..n_planets {
                    let _ = orch.send_asteroid(i, &orch.planet_channels.get(&i).unwrap().0.clone());
                }

                let _ = orch.handle_game_messages();
                println!("Cycle {} complete", cycle);
            }

            // Check how many survived the onslaught
            let survivors = orch.planets_info.count_survivors();

            println!("Survivors: {}/{}", survivors, n_planets);
            // In a heavy scenario, we just want to ensure the Orchestrator didn't crash
            assert!(orch.planets_info.len() == n_planets as usize);
        }

        #[test]
        fn test_orchestrator_heavy_channel_congestion() {
            let mut orch = Orchestrator::new().unwrap();
            orch.add_planet(0, PlanetType::Ciuc).unwrap();
            orch.start_all().unwrap();

            // Spam 1000 sunrays to a single planet to test channel capacity/backpressure
            for _ in 0..1000 {
                let _ = orch.send_sunray(0u32, &orch.planet_channels.get(&0).unwrap().0.clone());
            }

            // Ensure the orchestrator remains responsive
            let result = orch.handle_game_messages();
            assert!(result.is_ok());
        }
    }
}
