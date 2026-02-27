/// Explorer tests

#[cfg(test)]
mod tests {
    use crate::components::tommy_explorer::actions::*;
    use crate::components::tommy_explorer::bag::*;
    use crate::components::tommy_explorer::state::*;
    use crate::components::tommy_explorer::topology::*;
    use crate::components::tommy_explorer::*;

    use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
    use common_game::protocols::orchestrator_explorer::{
        ExplorerToOrchestrator, OrchestratorToExplorer,
    };
    use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
    use crossbeam_channel::{Receiver, Sender, unbounded};
    use std::collections::{HashSet, VecDeque};

    // ==================== Helper Functions ====================

    /// Creates a test explorer with mock channels.
    fn create_test_explorer() -> (
        Explorer,
        Receiver<ExplorerToOrchestrator<BagType>>,
        Sender<OrchestratorToExplorer>,
        Receiver<ExplorerToPlanet>,
        Sender<PlanetToExplorer>,
    ) {
        let (orch_send, orch_recv) = unbounded();
        let (explorer_send, explorer_recv) = unbounded();
        let (explorer_planet_send, planet_recv) = unbounded();
        let (planet_explorer_send, explorer_planet_recv) = unbounded();

        let explorer = Explorer::new(
            1,
            100,
            (orch_recv, explorer_send),
            (explorer_planet_recv, explorer_planet_send),
            5,
        );

        (
            explorer,
            explorer_recv,
            orch_send,
            planet_recv,
            planet_explorer_send,
        )
    }

    // ==================== Bag Tests ====================

    mod bag_tests {
        use super::*;

        #[test]
        fn test_bag_new() {
            let bag = Bag::new();
            assert_eq!(bag.to_resource_types().len(), 0);
        }

        #[test]
        fn test_bag_contains_empty() {
            let bag = Bag::new();
            let resource_type = ResourceType::Basic(BasicResourceType::Carbon);
            assert!(!bag.contains(resource_type));
        }

        #[test]
        fn test_bag_to_resource_types_empty() {
            let bag = Bag::new();
            let types = bag.to_resource_types();
            assert!(types.is_empty());
        }

        #[test]
        fn test_bag_take_resource_empty() {
            let mut bag = Bag::new();
            let result = bag.take_resource(ResourceType::Basic(BasicResourceType::Oxygen));
            assert!(result.is_none());
        }
    }

    // ==================== TopologyManager Tests ====================

    mod topology_tests {
        use super::*;

        #[test]
        fn test_topology_manager_new() {
            let topology = TopologyManager::new(100);
            assert!(topology.contains(100));
            assert_eq!(topology.known_planets().len(), 1);
        }

        #[test]
        fn test_topology_get_or_create() {
            let mut topology = TopologyManager::new(100);
            let info = topology.get_or_create(200);
            assert!(info.basic_resources.is_none());
            assert!(topology.contains(200));
        }

        #[test]
        fn test_topology_add_planets() {
            let mut topology = TopologyManager::new(100);
            topology.add_planets(&[200, 300, 400]);
            assert_eq!(topology.known_planets().len(), 4);
            assert!(topology.contains(200));
            assert!(topology.contains(300));
            assert!(topology.contains(400));
        }

        #[test]
        fn test_topology_update_neighbours() {
            let mut topology = TopologyManager::new(100);
            topology.update_neighbours(100, vec![200, 300]);

            assert!(topology.contains(200));
            assert!(topology.contains(300));

            let info = topology.get(100).unwrap();
            let neighbours = info.get_neighbours().unwrap();
            assert_eq!(neighbours.len(), 2);
            assert!(neighbours.contains(&200));
            assert!(neighbours.contains(&300));
        }

        #[test]
        fn test_topology_is_fully_discovered_empty() {
            let topology = TopologyManager::new(100);
            assert!(!topology.is_fully_discovered());
        }

        #[test]
        fn test_topology_is_fully_discovered_complete() {
            let mut topology = TopologyManager::new(100);
            let info = topology.get_or_create(100);
            info.set_basic_resources(HashSet::new());
            info.set_complex_resources(HashSet::new());
            info.set_neighbours(HashSet::new());

            assert!(topology.is_fully_discovered());
        }

        #[test]
        fn test_topology_clear() {
            let mut topology = TopologyManager::new(100);
            topology.add_planets(&[200, 300]);
            assert_eq!(topology.known_planets().len(), 3);

            topology.clear();
            assert_eq!(topology.known_planets().len(), 0);
        }
    }

    // ==================== PlanetInfo Tests ====================

    mod planet_info_tests {
        use super::*;

        #[test]
        fn test_planet_info_new() {
            let info = PlanetInfo::new();
            assert!(info.basic_resources.is_none());
            assert!(info.complex_resources.is_none());
            assert!(info.neighbours.is_none());
            assert!(!info.is_complete());
        }

        #[test]
        fn test_planet_info_with_data() {
            let basic = HashSet::new();
            let complex = HashSet::new();
            let neighbours = HashSet::new();

            let info = PlanetInfo::with_data(basic, complex, neighbours);
            assert!(info.is_complete());
        }

        #[test]
        fn test_planet_info_set_basic_resources() {
            let mut info = PlanetInfo::new();
            let mut resources = HashSet::new();
            resources.insert(BasicResourceType::Carbon);

            info.set_basic_resources(resources.clone());
            assert_eq!(info.get_basic_resources().unwrap(), &resources);
        }

        #[test]
        fn test_planet_info_set_complex_resources() {
            let mut info = PlanetInfo::new();
            let mut resources = HashSet::new();
            resources.insert(ComplexResourceType::Diamond);

            info.set_complex_resources(resources.clone());
            assert_eq!(info.get_complex_resources().unwrap(), &resources);
        }

        #[test]
        fn test_planet_info_set_neighbours() {
            let mut info = PlanetInfo::new();
            let mut neighbours = HashSet::new();
            neighbours.insert(200);
            neighbours.insert(300);

            info.set_neighbours(neighbours.clone());
            assert_eq!(info.get_neighbours().unwrap(), &neighbours);
        }
    }

    // ==================== ExplorerState Tests ====================

    mod state_tests {
        use super::*;

        #[test]
        fn test_state_matches_orchestrator_msg_idle() {
            let state = ExplorerState::Idle;
            let msg = OrchestratorToExplorer::CurrentPlanetRequest;
            assert!(state.matches_orchestrator_msg(&msg));
        }

        #[test]
        fn test_state_matches_orchestrator_msg_kill() {
            let state = ExplorerState::WaitingForNeighbours;
            let msg = OrchestratorToExplorer::KillExplorer;
            assert!(state.matches_orchestrator_msg(&msg));
        }

        #[test]
        fn test_state_matches_orchestrator_msg_waiting_to_start() {
            let state = ExplorerState::WaitingToStartExplorerAI;
            let msg = OrchestratorToExplorer::StartExplorerAI;
            assert!(state.matches_orchestrator_msg(&msg));
        }

        #[test]
        fn test_state_should_terminate() {
            assert!(ExplorerState::Killed.should_terminate());
            assert!(!ExplorerState::Idle.should_terminate());
            assert!(!ExplorerState::Traveling.should_terminate());
        }

        #[test]
        fn test_state_can_process_buffer() {
            assert!(ExplorerState::Idle.can_process_buffer());
            assert!(!ExplorerState::Traveling.can_process_buffer());
            assert!(!ExplorerState::Killed.can_process_buffer());
        }
    }

    // ==================== ActionQueue Tests ====================

    mod action_queue_tests {
        use super::*;

        #[test]
        fn test_action_queue_new() {
            let queue = ActionQueue::new();
            assert!(!queue.is_empty());
            assert_eq!(queue.len(), 6);
        }

        #[test]
        fn test_action_queue_next_action() {
            let mut queue = ActionQueue::new();
            let action = queue.next_action();
            assert_eq!(action, Some(ExplorerAction::AskNeighbours));
            assert_eq!(queue.len(), 5);
        }

        #[test]
        fn test_action_queue_push_back() {
            let mut queue = ActionQueue::new();
            let initial_len = queue.len();
            queue.push_back(ExplorerAction::Move);
            assert_eq!(queue.len(), initial_len + 1);
        }

        #[test]
        fn test_action_queue_push_front() {
            let mut queue = ActionQueue::new();
            queue.push_front(ExplorerAction::Move);
            let next = queue.next_action();
            assert_eq!(next, Some(ExplorerAction::Move));
        }

        #[test]
        fn test_action_queue_clear() {
            let mut queue = ActionQueue::new();
            queue.clear();
            assert!(queue.is_empty());
            assert_eq!(queue.len(), 0);
        }

        #[test]
        fn test_action_queue_reset() {
            let mut queue = ActionQueue::new();
            queue.clear();
            assert!(queue.is_empty());

            queue.reset();
            assert!(!queue.is_empty());
            assert_eq!(queue.len(), 6);
        }
    }

    // ==================== MoveQueue Tests ====================

    mod move_queue_tests {
        use super::*;

        #[test]
        fn test_move_queue_new() {
            let queue = MoveQueue::new();
            assert!(queue.is_empty());
        }

        #[test]
        fn test_move_queue_push_back() {
            let mut queue = MoveQueue::new();
            queue.push_back(100);
            queue.push_back(200);
            assert!(!queue.is_empty());
        }

        #[test]
        fn test_move_queue_next_move() {
            let mut queue = MoveQueue::new();
            queue.push_back(100);
            queue.push_back(200);

            assert_eq!(queue.next_move(), Some(100));
            assert_eq!(queue.next_move(), Some(200));
            assert_eq!(queue.next_move(), None);
        }

        #[test]
        fn test_move_queue_push_path() {
            let mut queue = MoveQueue::new();
            let mut path = VecDeque::new();
            path.push_back(100);
            path.push_back(200);
            path.push_back(300);

            queue.push_path(path);
            assert_eq!(queue.next_move(), Some(100));
            assert_eq!(queue.next_move(), Some(200));
            assert_eq!(queue.next_move(), Some(300));
        }

        #[test]
        fn test_move_queue_clear() {
            let mut queue = MoveQueue::new();
            queue.push_back(100);
            queue.push_back(200);

            queue.clear();
            assert!(queue.is_empty());
        }
    }

    // ==================== Explorer Tests ====================

    mod explorer_tests {
        use super::*;

        #[test]
        fn test_explorer_new() {
            let (explorer, _, _, _, _) = create_test_explorer();
            assert_eq!(explorer.id(), 1);
            assert_eq!(explorer.planet_id(), 100);
            // assert_eq!(*explorer.state(), ExplorerState::WaitingToStartExplorerAI);
            assert_eq!(*explorer.state(), ExplorerState::Idle);
        }

        #[test]
        fn test_explorer_set_state() {
            let (mut explorer, _, _, _, _) = create_test_explorer();
            explorer.set_state(ExplorerState::Idle);
            assert_eq!(*explorer.state(), ExplorerState::Idle);
        }

        #[test]
        fn test_explorer_set_planet_id() {
            let (mut explorer, _, _, _, _) = create_test_explorer();
            explorer.set_planet_id(200);
            assert_eq!(explorer.planet_id(), 200);
        }

        #[test]
        fn test_explorer_get_bag_content_empty() {
            let (explorer, _, _, _, _) = create_test_explorer();
            let bag_content = explorer.get_bag_content();
            assert!(bag_content.is_empty());
        }

        #[test]
        fn test_explorer_clear_topology() {
            let (mut explorer, _, _, _, _) = create_test_explorer();
            explorer.update_neighbors(100, vec![200, 300]);
            assert!(explorer.topology.known_planets().len() > 1);

            explorer.clear_topology();
            assert_eq!(explorer.topology.known_planets().len(), 0);
        }

        #[test]
        fn test_explorer_update_neighbors() {
            let (mut explorer, _, _, _, _) = create_test_explorer();
            explorer.update_neighbors(100, vec![200, 300]);

            let info = explorer.get_planet_info(100).unwrap();
            let neighbours = info.get_neighbours().unwrap();
            assert_eq!(neighbours.len(), 2);
            assert!(neighbours.contains(&200));
            assert!(neighbours.contains(&300));
        }

        #[test]
        fn test_explorer_send_to_orchestrator() {
            let (explorer, receiver, _, _, _) = create_test_explorer();

            let msg = ExplorerToOrchestrator::CurrentPlanetResult {
                explorer_id: 1,
                planet_id: 100,
            };

            let result = explorer.send_to_orchestrator(msg);
            assert!(result.is_ok());

            // Verify message was received within 100 ms
            let received = receiver.recv_timeout(std::time::Duration::from_millis(100));
            assert!(received.is_ok());
        }

        #[test]
        fn test_explorer_send_to_planet() {
            let (explorer, _, _, planet_recv, _) = create_test_explorer();

            let msg = ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 };

            let result = explorer.send_to_planet(msg);
            assert!(result.is_ok());

            // Verify message was received
            let received = planet_recv.recv_timeout(std::time::Duration::from_millis(100));
            assert!(received.is_ok());
        }

        #[test]
        fn test_explorer_set_energy_cells() {
            let (mut explorer, _, _, _, _) = create_test_explorer();
            explorer.set_energy_cells(10);
            assert_eq!(explorer.energy_cells, 10);
        }
    }

    // ==================== Pathfinding Tests ====================

    mod pathfinding_tests {
        use super::*;

        #[test]
        fn test_find_path_to_nearest_frontier_simple() {
            let mut topology = TopologyManager::new(100);

            // Setup: 100 -> 200 (200 not complete)
            topology.update_neighbours(100, vec![200]);
            let info = topology.get_or_create(100);
            info.set_basic_resources(HashSet::new());
            info.set_complex_resources(HashSet::new());

            let path = topology.find_path_to_nearest_frontier(100);
            assert!(path.is_some());
            let path = path.unwrap();
            assert_eq!(path.len(), 1);
            assert_eq!(path[0], 200);
        }

        #[test]
        fn test_find_path_to_nearest_frontier_fully_discovered() {
            let mut topology = TopologyManager::new(100);

            // Setup: both planets fully discovered
            let info = topology.get_or_create(100);
            info.set_basic_resources(HashSet::new());
            info.set_complex_resources(HashSet::new());
            info.set_neighbours(HashSet::from_iter(vec![200]));

            let info2 = topology.get_or_create(200);
            info2.set_basic_resources(HashSet::new());
            info2.set_complex_resources(HashSet::new());
            info2.set_neighbours(HashSet::from_iter(vec![100]));

            let path = topology.find_path_to_nearest_frontier(100);
            assert!(path.is_none());
        }

        #[test]
        fn test_find_path_to_resource_simple() {
            let mut topology = TopologyManager::new(100);

            // Setup: planet 200 has carbon
            topology.update_neighbours(100, vec![200]);
            let mut basic_resources = HashSet::new();
            basic_resources.insert(BasicResourceType::Carbon);

            let info = topology.get_or_create(200);
            info.set_basic_resources(basic_resources);
            info.set_complex_resources(HashSet::new());
            info.set_neighbours(HashSet::new());

            let target = ResourceType::Basic(BasicResourceType::Carbon);
            let path = topology.find_path_to_resource(100, target);

            assert!(path.is_some());
            let path = path.unwrap();
            assert_eq!(path.len(), 1);
            assert_eq!(path[0], 200);
        }

        #[test]
        fn test_find_path_to_resource_not_found() {
            let mut topology = TopologyManager::new(100);

            // Setup: planet 200 exists but has no carbon
            topology.update_neighbours(100, vec![200]);
            let info = topology.get_or_create(200);
            info.set_basic_resources(HashSet::new());
            info.set_complex_resources(HashSet::new());
            info.set_neighbours(HashSet::new());

            let target = ResourceType::Basic(BasicResourceType::Carbon);
            let path = topology.find_path_to_resource(100, target);

            assert!(path.is_none());
        }

        #[test]
        fn test_find_path_complex_topology() {
            let mut topology = TopologyManager::new(100);

            // Setup: 100 -> 200 -> 300 (300 has carbon)
            topology.update_neighbours(100, vec![200]);
            topology.update_neighbours(200, vec![100, 300]);

            let mut basic_resources = HashSet::new();
            basic_resources.insert(BasicResourceType::Carbon);

            let info_300 = topology.get_or_create(300);
            info_300.set_basic_resources(basic_resources);
            info_300.set_complex_resources(HashSet::new());
            info_300.set_neighbours(HashSet::new());

            // Set other planets as complete without carbon
            let info_100 = topology.get_or_create(100);
            info_100.set_basic_resources(HashSet::new());
            info_100.set_complex_resources(HashSet::new());

            let info_200 = topology.get_or_create(200);
            info_200.set_basic_resources(HashSet::new());
            info_200.set_complex_resources(HashSet::new());

            let target = ResourceType::Basic(BasicResourceType::Carbon);
            let path = topology.find_path_to_resource(100, target);

            assert!(path.is_some());
            let path = path.unwrap();
            assert_eq!(path.len(), 2);
            assert_eq!(path[0], 200);
            assert_eq!(path[1], 300);
        }
    }

    // ==================== Integration Tests ====================

    mod integration_tests {
        use super::*;

        #[test]
        fn test_explorer_topology_discovery_flow() {
            let (mut explorer, _, _, _, _) = create_test_explorer();

            // Set explorer to idle state
            explorer.set_state(ExplorerState::Idle);

            // Simulate receiving neighbors
            explorer.update_neighbors(100, vec![200, 300, 400]);

            // Verify topology
            assert!(explorer.topology.contains(100));
            assert!(explorer.topology.contains(200));
            assert!(explorer.topology.contains(300));
            assert!(explorer.topology.contains(400));

            let planet_100_info = explorer.get_planet_info(100).unwrap();
            assert_eq!(planet_100_info.get_neighbours().unwrap().len(), 3);
        }

        #[test]
        fn test_explorer_pathfinding_integration() {
            let (mut explorer, _, _, _, _) = create_test_explorer();

            // Setup: 100 -> 200 -> 300
            explorer.update_neighbors(100, vec![200]);
            explorer.update_neighbors(200, vec![100, 300]);

            // Mark planets as discovered
            let info_100 = explorer.topology.get_or_create(100);
            info_100.set_basic_resources(HashSet::new());
            info_100.set_complex_resources(HashSet::new());

            let info_200 = explorer.topology.get_or_create(200);
            info_200.set_basic_resources(HashSet::new());
            info_200.set_complex_resources(HashSet::new());

            // Planet 300 has carbon
            let mut basic = HashSet::new();
            basic.insert(BasicResourceType::Carbon);
            let info_300 = explorer.topology.get_or_create(300);
            info_300.set_basic_resources(basic);
            info_300.set_complex_resources(HashSet::new());
            info_300.set_neighbours(HashSet::new());

            // Find path to carbon
            let path = explorer
                .topology
                .find_path_to_resource(100, ResourceType::Basic(BasicResourceType::Carbon));

            assert!(path.is_some());
            let path = path.unwrap();
            assert_eq!(path.len(), 2); // 200, 300
            assert_eq!(path[0], 200);
            assert_eq!(path[1], 300);
        }

        #[test]
        fn test_explorer_state_transitions() {
            let (mut explorer, _, _, _, _) = create_test_explorer();

            // Initial state
            // assert_eq!(*explorer.state(), ExplorerState::WaitingToStartExplorerAI);
            assert_eq!(*explorer.state(), ExplorerState::Idle);

            // Transition to Idle
            explorer.set_state(ExplorerState::Idle);
            assert_eq!(*explorer.state(), ExplorerState::Idle);
            assert!(explorer.state().can_process_buffer());

            // Transition to Traveling
            explorer.set_state(ExplorerState::Traveling);
            assert_eq!(*explorer.state(), ExplorerState::Traveling);
            assert!(!explorer.state().can_process_buffer());

            // Transition to Killed
            explorer.set_state(ExplorerState::Killed);
            assert!(explorer.state().should_terminate());
        }

        #[test]
        fn test_explorer_multi_planet_pathfinding() {
            let (mut explorer, _, _, _, _) = create_test_explorer();

            // Create a more complex topology
            explorer.update_neighbors(100, vec![200, 300]);
            explorer.update_neighbors(200, vec![100, 400]);
            explorer.update_neighbors(300, vec![100, 500]);
            explorer.update_neighbors(400, vec![200]);
            explorer.update_neighbors(500, vec![300]);

            // Mark all as discovered except 500
            for id in [100, 200, 300, 400] {
                let info = explorer.topology.get_or_create(id);
                info.set_basic_resources(HashSet::new());
                info.set_complex_resources(HashSet::new());
            }

            // 500 has silicon
            let mut basic = HashSet::new();
            basic.insert(BasicResourceType::Silicon);
            let info_500 = explorer.topology.get_or_create(500);
            info_500.set_basic_resources(basic);
            info_500.set_complex_resources(HashSet::new());
            info_500.set_neighbours(HashSet::new());

            // Find path from 100 to silicon
            let path = explorer
                .topology
                .find_path_to_resource(100, ResourceType::Basic(BasicResourceType::Silicon));

            assert!(path.is_some());
            let path = path.unwrap();
            // Path should be: 300, 500
            assert_eq!(path.len(), 2);
            assert_eq!(path[0], 300);
            assert_eq!(path[1], 500);
        }
    }
}
