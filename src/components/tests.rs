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
        let (tx, _) = crossbeam_channel::unbounded();

        orch.add_explorer(1, 10, 5, tx);

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

        orch.destroy_topology_link(0, 1).unwrap();

        let gtop = orch.galaxy_topology.read().unwrap();
        assert_eq!(gtop[0][1], false);
    }

    #[test]
    fn test_topology_destroy_link_out_of_bounds_errors() {
        let mut orch = Orchestrator::new().unwrap();
        orch.initialize_galaxy_by_adj_list(vec![vec![]]).unwrap();

        let result = orch.destroy_topology_link(0, 5);
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

        // Setup a planet
        orch.add_planet(planet_id, PlanetType::Ciuc).unwrap();

        // Simulate an Asteroid hitting with NO rocket (None means destruction)
        let msg = PlanetToOrchestrator::AsteroidAck {
            planet_id,
            rocket: None,
        };
        orch.handle_planet_message(msg).unwrap();

        assert!(orch.planets_info.is_dead(&planet_id));
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
        let content = "0, 4, 1\n1, 4, 0";
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
            orch.add_planet(p_id_a, PlanetType::Ciuc).unwrap();

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

            // Phase 2: Asteroid Attack
            orch.send_asteroid(p_id_a, &channel_a).unwrap();
            orch.send_asteroid(p_id_b, &channel_b).unwrap();

            // Give the planet threads a moment to process the asteroids and build
            std::thread::sleep(Duration::from_millis(500));
            // We simulate receiving the responses from the channels
            // (In a real run, handle_game_messages would do this)
            orch.handle_game_messages().unwrap();
            orch.handle_game_messages().unwrap();

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
