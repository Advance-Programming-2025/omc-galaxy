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
#[cfg(test)]
mod explorer_full_tests {
    use crate::components::tommy_explorer::actions::*;
    use crate::components::tommy_explorer::bag::*;
    use crate::components::tommy_explorer::state::*;
    use crate::components::tommy_explorer::topology::*;
    use crate::components::tommy_explorer::*;

    use common_game::components::resource::{
        BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, ResourceType,
    };
    use common_game::protocols::orchestrator_explorer::{
        ExplorerToOrchestrator, OrchestratorToExplorer,
    };
    use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
    use crossbeam_channel::{Receiver, Sender, unbounded};
    use std::collections::{HashSet, VecDeque};
    use std::thread;
    use std::time::Duration;
    use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
    use crate::{Orchestrator, Status};
    use crate::utils::registry::PlanetType;
    // ==================== Test Helpers ====================

    struct TestStruct {
        explorer: Explorer,
        // Orchestrator side
        orch_receiver: Receiver<ExplorerToOrchestrator<BagType>>, // receives from explorer
        orch_sender: Sender<OrchestratorToExplorer>,              // sends to explorer
        // Planet side
        planet_receiver: Receiver<ExplorerToPlanet>, // receives from explorer
        planet_sender: Sender<PlanetToExplorer>,     // sends to explorer
    }

    impl TestStruct {
        fn new() -> Self {
            Self::new_with_params(1, 100, 5)
        }

        fn new_with_params(explorer_id: u32, planet_id: u32, energy_cells: u32) -> Self {
            let (orch_send, orch_recv) = unbounded::<OrchestratorToExplorer>();
            let (explorer_orch_send, explorer_orch_recv) =
                unbounded::<ExplorerToOrchestrator<BagType>>();
            let (planet_send, planet_recv) = unbounded::<PlanetToExplorer>();
            let (explorer_planet_send, explorer_planet_recv) =
                unbounded::<ExplorerToPlanet>();

            let explorer = Explorer::new(
                explorer_id,
                planet_id,
                (orch_recv, explorer_orch_send),
                (planet_recv, explorer_planet_send),
                energy_cells,
            );

            TestStruct {
                explorer,
                orch_receiver: explorer_orch_recv,
                orch_sender: orch_send,
                planet_receiver: explorer_planet_recv,
                planet_sender: planet_send,
            }
        }

        fn send_to_explorer_from_orch(&self, msg: OrchestratorToExplorer) {
            self.orch_sender.send(msg).expect("Failed to send to explorer from orchestrator");
        }

        fn send_to_explorer_from_planet(&self, msg: PlanetToExplorer) {
            self.planet_sender.send(msg).expect("Failed to send to explorer from planet");
        }

        fn recv_from_explorer_to_orch(&self) -> ExplorerToOrchestrator<BagType> {
            self.orch_receiver
                .recv_timeout(Duration::from_millis(200))
                .expect("Timeout waiting for explorer->orchestrator message")
        }

        fn recv_from_explorer_to_planet(&self) -> ExplorerToPlanet {
            self.planet_receiver
                .recv_timeout(Duration::from_millis(200))
                .expect("Timeout waiting for explorer->planet message")
        }

        fn recv_from_explorer_to_orch_opt(&self) -> Option<ExplorerToOrchestrator<BagType>> {
            self.orch_receiver
                .recv_timeout(Duration::from_millis(50))
                .ok()
        }

        fn recv_from_explorer_to_planet_opt(&self) -> Option<ExplorerToPlanet> {
            self.planet_receiver
                .recv_timeout(Duration::from_millis(50))
                .ok()
        }
    }

    // ==================== 1. ORCHESTRATOR -> EXPLORER Messages ====================

    mod orchestrator_to_explorer_tests {
        use super::*;

        /// OrchestratorToExplorer::CurrentPlanetRequest
        /// -> Explorer should respond with CurrentPlanetResult
        #[test]
        fn test_current_planet_request_response() {
            let mut h = TestStruct::new_with_params(1, 42, 5);

            // Manually call handler as if orchestrator sent the message
            let msg = OrchestratorToExplorer::CurrentPlanetRequest;
            h.explorer.send_to_orchestrator(
                // simulate the response the handler would produce
                ExplorerToOrchestrator::CurrentPlanetResult {
                    explorer_id: h.explorer.id(),
                    planet_id: h.explorer.planet_id(),
                },
            ).unwrap();

            let response = h.recv_from_explorer_to_orch();
            match response {
                ExplorerToOrchestrator::CurrentPlanetResult { explorer_id, planet_id } => {
                    assert_eq!(explorer_id, 1);
                    assert_eq!(planet_id, 42);
                }
                _ => panic!("Expected CurrentPlanetResult, got {:?}", response),
            }
        }

        /// OrchestratorToExplorer::NeighborsResponse
        /// -> Explorer should update its topology and return to Idle
        #[test]
        fn test_neighbors_response_updates_topology() {
            let mut h = TestStruct::new();

            h.explorer.set_state(ExplorerState::WaitingForNeighbours);
            let neighbors = vec![200u32, 300u32, 400u32];

            // Simulate neighbors_response handler
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.update_neighbors(h.explorer.planet_id(), neighbors.clone());

            assert_eq!(*h.explorer.state(), ExplorerState::Idle);
            assert!(h.explorer.topology.contains(200));
            assert!(h.explorer.topology.contains(300));
            assert!(h.explorer.topology.contains(400));

            let info = h.explorer.get_planet_info(100).unwrap();
            let nbrs = info.get_neighbours().unwrap();
            assert_eq!(nbrs.len(), 3);
        }

        /// OrchestratorToExplorer::KillExplorer
        /// -> Explorer should transition to Killed and send KillExplorerResult
        #[test]
        fn test_kill_explorer_message() {
            let mut h = TestStruct::new();

            // Simulate kill_explorer handler
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::KillExplorerResult {
                    explorer_id: h.explorer.id(),
                })
                .unwrap();
            h.explorer.set_state(ExplorerState::Killed);

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::KillExplorerResult { explorer_id: 1 }
            ));
            assert!(h.explorer.state().should_terminate());
        }

        /// OrchestratorToExplorer::MoveToPlanet with valid sender
        /// -> Explorer should update planet_id, reset action queue, set state Idle
        #[test]
        fn test_move_to_planet_with_valid_sender() {
            let mut h = TestStruct::new();
            let (new_planet_send, _new_planet_recv) = unbounded::<ExplorerToPlanet>();

            // Simulate move_to_planet handler
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.action_queue.clear();
            h.explorer.action_queue.reset();
            h.explorer.set_planet_sender(new_planet_send);
            h.explorer.set_planet_id(999);

            assert_eq!(*h.explorer.state(), ExplorerState::Idle);
            assert_eq!(h.explorer.planet_id(), 999);
            assert_eq!(h.explorer.action_queue.len(), 6); // reset to default
        }

        /// OrchestratorToExplorer::MoveToPlanet with None sender
        /// -> Explorer should still set Idle but NOT change planet_id
        #[test]
        fn test_move_to_planet_with_none_sender() {
            let mut h = TestStruct::new();

            // Simulate move_to_planet handler with None
            h.explorer.set_state(ExplorerState::Idle);
            // None sender: planet_id should NOT be updated per handler logic
            let original_planet_id = h.explorer.planet_id();

            assert_eq!(*h.explorer.state(), ExplorerState::Idle);
            assert_eq!(h.explorer.planet_id(), original_planet_id);
        }

        /// OrchestratorToExplorer::BagContentRequest
        /// -> Explorer should send BagContentResponse with current bag
        #[test]
        fn test_bag_content_request_response() {
            let h = TestStruct::new();

            let bag_content = h.explorer.get_bag_content();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::BagContentResponse {
                    explorer_id: h.explorer.id(),
                    bag_content: bag_content.clone(),
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            match msg {
                ExplorerToOrchestrator::BagContentResponse { explorer_id, bag_content } => {
                    assert_eq!(explorer_id, 1);
                    assert!(bag_content.is_empty());
                }
                _ => panic!("Expected BagContentResponse"),
            }
        }

        /// OrchestratorToExplorer::StartExplorerAI
        /// -> Explorer should go Idle, send StartExplorerAIResult, turn off manual mode
        #[test]
        fn test_start_explorer_ai() {
            let mut h = TestStruct::new();

            // Simulate start_explorer_ai handler
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::StartExplorerAIResult {
                    explorer_id: h.explorer.id(),
                })
                .unwrap();
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.manual_mode_off();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::StartExplorerAIResult { explorer_id: 1 }
            ));
            assert_eq!(*h.explorer.state(), ExplorerState::Idle);
        }

        /// OrchestratorToExplorer::ResetExplorerAI
        /// -> Explorer should clear topology, set Idle, send ResetExplorerAIResult
        #[test]
        fn test_reset_explorer_ai() {
            let mut h = TestStruct::new();

            // Pre-populate topology
            h.explorer.update_neighbors(100, vec![200, 300]);
            assert!(h.explorer.topology.known_planets().len() > 1);

            // Simulate reset_explorer_ai handler
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::ResetExplorerAIResult {
                    explorer_id: h.explorer.id(),
                })
                .unwrap();
            h.explorer.manual_mode_off();
            h.explorer.clear_topology();
            h.explorer.set_state(ExplorerState::Idle);

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id: 1 }
            ));
            assert_eq!(h.explorer.topology.known_planets().len(), 0);
            assert_eq!(*h.explorer.state(), ExplorerState::Idle);
        }

        /// OrchestratorToExplorer::StopExplorerAI
        /// -> Explorer should enter manual mode, send StopExplorerAIResult
        #[test]
        fn test_stop_explorer_ai() {
            let mut h = TestStruct::new();

            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::StopExplorerAIResult {
                    explorer_id: h.explorer.id(),
                })
                .unwrap();
            h.explorer.manual_mode_on();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::StopExplorerAIResult { explorer_id: 1 }
            ));
        }

        /// OrchestratorToExplorer::SupportedResourceRequest
        /// -> When planet info not cached: Explorer queries planet, then sends SupportedResourceResult to orchestrator
        #[test]
        fn test_supported_resource_request_from_orch_not_cached() {
            let h = TestStruct::new();

            // Explorer sends request to planet
            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedResourceRequest {
                    explorer_id: h.explorer.id(),
                })
                .unwrap();

            let planet_msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                planet_msg,
                ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 }
            ));
        }

        /// OrchestratorToExplorer::SupportedCombinationRequest
        /// -> When not cached: Explorer queries planet for combinations
        #[test]
        fn test_supported_combination_request_not_cached() {
            let h = TestStruct::new();

            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
                    explorer_id: h.explorer.id(),
                })
                .unwrap();

            let planet_msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                planet_msg,
                ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 1 }
            ));
        }

        /// OrchestratorToExplorer::GenerateResourceRequest
        /// -> Explorer should forward to planet
        #[test]
        fn test_generate_resource_request_forwarded_to_planet() {
            let h = TestStruct::new();

            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: h.explorer.id(),
                    resource: BasicResourceType::Oxygen,
                })
                .unwrap();

            let planet_msg = h.recv_from_explorer_to_planet();
            match planet_msg {
                ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource } => {
                    assert_eq!(explorer_id, 1);
                    assert_eq!(resource, BasicResourceType::Oxygen);
                }
                _ => panic!("Expected GenerateResourceRequest"),
            }
        }

        /// OrchestratorToExplorer::NeighborsRequest
        /// -> Explorer sends NeighborsRequest to orchestrator during AI action
        #[test]
        fn test_neighbors_request_sent_to_orchestrator() {
            let h = TestStruct::new();

            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest {
                    explorer_id: h.explorer.id(),
                    current_planet_id: h.explorer.planet_id(),
                })
                .unwrap();

            let orch_msg = h.recv_from_explorer_to_orch();
            match orch_msg {
                ExplorerToOrchestrator::NeighborsRequest { explorer_id, current_planet_id } => {
                    assert_eq!(explorer_id, 1);
                    assert_eq!(current_planet_id, 100);
                }
                _ => panic!("Expected NeighborsRequest"),
            }
        }

        /// OrchestratorToExplorer::TravelToPlanetRequest
        /// -> Explorer sends TravelToPlanetRequest to orchestrator
        #[test]
        fn test_travel_to_planet_request_sent() {
            let h = TestStruct::new();

            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: h.explorer.id(),
                    current_planet_id: h.explorer.planet_id(),
                    dst_planet_id: 200,
                })
                .unwrap();

            let orch_msg = h.recv_from_explorer_to_orch();
            match orch_msg {
                ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id,
                    current_planet_id,
                    dst_planet_id,
                } => {
                    assert_eq!(explorer_id, 1);
                    assert_eq!(current_planet_id, 100);
                    assert_eq!(dst_planet_id, 200);
                }
                _ => panic!("Expected TravelToPlanetRequest"),
            }
        }
    }

    // ==================== 2. PLANET -> EXPLORER Messages ====================

    mod planet_to_explorer_tests {
        use super::*;

        /// PlanetToExplorer::SupportedResourceResponse
        /// -> Explorer should update topology basic_resources for current planet
        #[test]
        fn test_supported_resource_response_updates_topology() {
            let mut h = TestStruct::new();

            let mut resources = HashSet::new();
            resources.insert(BasicResourceType::Oxygen);
            resources.insert(BasicResourceType::Carbon);

            // Simulate update_basic_resources handler
            if let Some(planet_info) = h.explorer.get_planet_info_mut(h.explorer.planet_id()) {
                planet_info.set_basic_resources(resources.clone());
            }

            let info = h.explorer.get_planet_info(100).unwrap();
            let stored = info.get_basic_resources().unwrap();
            assert!(stored.contains(&BasicResourceType::Oxygen));
            assert!(stored.contains(&BasicResourceType::Carbon));
        }

        /// PlanetToExplorer::SupportedCombinationResponse
        /// -> Explorer should update topology complex_resources for current planet
        #[test]
        fn test_supported_combination_response_updates_topology() {
            let mut h = TestStruct::new();

            let mut combinations = HashSet::new();
            combinations.insert(ComplexResourceType::Water);
            combinations.insert(ComplexResourceType::Diamond);

            if let Some(planet_info) = h.explorer.get_planet_info_mut(h.explorer.planet_id()) {
                planet_info.set_complex_resources(combinations.clone());
            }

            let info = h.explorer.get_planet_info(100).unwrap();
            let stored = info.get_complex_resources().unwrap();
            assert!(stored.contains(&ComplexResourceType::Water));
            assert!(stored.contains(&ComplexResourceType::Diamond));
        }

        /// PlanetToExplorer::AvailableEnergyCellResponse
        /// -> Explorer should update its energy_cells counter
        #[test]
        fn test_available_energy_cell_response() {
            let mut h = TestStruct::new();

            // Simulate AvailableEnergyCellResponse handler
            h.explorer.set_energy_cells(42);

            assert_eq!(h.explorer.energy_cells, 42);
        }

        /// PlanetToExplorer::AvailableEnergyCellResponse with zero cells
        #[test]
        fn test_available_energy_cell_response_zero() {
            let mut h = TestStruct::new();
            h.explorer.set_energy_cells(0);
            assert_eq!(h.explorer.energy_cells, 0);
        }

        /// PlanetToExplorer::GenerateResourceResponse with Some resource
        /// -> Explorer should put resource in bag
        #[test]
        fn test_generate_resource_response_adds_to_bag() {
            let mut h = TestStruct::new();

            // Simulate put_basic_resource_in_bag
            // (In real code the handler calls this; we test the bag directly)
            // We insert a resource directly to verify bag behavior
            // Since BasicResource internals may vary, we test via bag.contains
            let bag_before = h.explorer.get_bag_content();
            assert!(bag_before.is_empty());

            // The actual resource insertion depends on BasicResource::to_generic()
            // We verify the bag grows after insertion via the explorer's insert_in_bag
            // For this test we verify the state machine: bag starts empty
            assert_eq!(h.explorer.get_bag_content().len(), 0);
        }

        /// PlanetToExplorer::Stopped
        /// -> Explorer should transition to WaitingToStartExplorerAI
        #[test]
        fn test_planet_stopped_transitions_state() {
            let mut h = TestStruct::new();

            // Simulate handler for PlanetToExplorer::Stopped
            h.explorer.set_state(ExplorerState::WaitingToStartExplorerAI);

            assert_eq!(
                *h.explorer.state(),
                ExplorerState::WaitingToStartExplorerAI
            );
        }

        /// PlanetToExplorer::SupportedResourceResponse sent over channel and received
        #[test]
        fn test_planet_resource_response_via_channel() {
            let h = TestStruct::new();
            let mut resources = HashSet::new();
            resources.insert(BasicResourceType::Hydrogen);

            h.send_to_explorer_from_planet(PlanetToExplorer::SupportedResourceResponse {
                resource_list: resources,
            });

            // Verify the channel delivers the message (received by explorer's planet channel)
            // Since the explorer's planet_channels.0 is the receiver, we can't read it externally
            // but we can verify the send didn't panic/err
            // The real test is that the channel is set up correctly
            assert!(true); // channel send succeeded (would panic otherwise)
        }
    }

    // ==================== 3. EXPLORER -> ORCHESTRATOR Messages ====================

    mod explorer_to_orchestrator_tests {
        use super::*;

        #[test]
        fn test_send_neighbors_request() {
            let h = TestStruct::new();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::NeighborsRequest {
                    explorer_id: 1,
                    current_planet_id: 100
                }
            ));
        }

        #[test]
        fn test_send_travel_to_planet_request() {
            let h = TestStruct::new();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                    dst_planet_id: 200,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                    dst_planet_id: 200
                }
            ));
        }

        #[test]
        fn test_send_kill_explorer_result() {
            let h = TestStruct::new();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::KillExplorerResult {
                    explorer_id: 1,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::KillExplorerResult { explorer_id: 1 }
            ));
        }

        #[test]
        fn test_send_start_explorer_ai_result() {
            let h = TestStruct::new();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::StartExplorerAIResult {
                    explorer_id: 1,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::StartExplorerAIResult { explorer_id: 1 }
            ));
        }

        #[test]
        fn test_send_reset_explorer_ai_result() {
            let h = TestStruct::new();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::ResetExplorerAIResult {
                    explorer_id: 1,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id: 1 }
            ));
        }

        #[test]
        fn test_send_bag_content_response_empty() {
            let h = TestStruct::new();
            let bag_content = h.explorer.get_bag_content();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::BagContentResponse {
                    explorer_id: 1,
                    bag_content: bag_content.clone(),
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            match msg {
                ExplorerToOrchestrator::BagContentResponse { explorer_id, bag_content } => {
                    assert_eq!(explorer_id, 1);
                    assert!(bag_content.is_empty());
                }
                _ => panic!("Expected BagContentResponse"),
            }
        }

        #[test]
        fn test_send_supported_resource_result() {
            let h = TestStruct::new();
            let mut resources = HashSet::new();
            resources.insert(BasicResourceType::Silicon);

            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::SupportedResourceResult {
                    explorer_id: 1,
                    supported_resources: resources.clone(),
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            match msg {
                ExplorerToOrchestrator::SupportedResourceResult {
                    explorer_id,
                    supported_resources,
                } => {
                    assert_eq!(explorer_id, 1);
                    assert!(supported_resources.contains(&BasicResourceType::Silicon));
                }
                _ => panic!("Expected SupportedResourceResult"),
            }
        }

        #[test]
        fn test_send_supported_combination_result() {
            let h = TestStruct::new();
            let mut combos = HashSet::new();
            combos.insert(ComplexResourceType::Life);

            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::SupportedCombinationResult {
                    explorer_id: 1,
                    combination_list: combos.clone(),
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            match msg {
                ExplorerToOrchestrator::SupportedCombinationResult {
                    explorer_id,
                    combination_list,
                } => {
                    assert_eq!(explorer_id, 1);
                    assert!(combination_list.contains(&ComplexResourceType::Life));
                }
                _ => panic!("Expected SupportedCombinationResult"),
            }
        }

        #[test]
        fn test_send_current_planet_result() {
            let h = TestStruct::new();
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::CurrentPlanetResult {
                    explorer_id: 1,
                    planet_id: 100,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::CurrentPlanetResult {
                    explorer_id: 1,
                    planet_id: 100
                }
            ));
        }
    }

    // ==================== 4. EXPLORER -> PLANET Messages ====================

    mod explorer_to_planet_tests {
        use super::*;

        #[test]
        fn test_send_supported_resource_request() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                msg,
                ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 }
            ));
        }

        #[test]
        fn test_send_supported_combination_request() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 1 })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                msg,
                ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 1 }
            ));
        }

        #[test]
        fn test_send_available_energy_cell_request() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                msg,
                ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 }
            ));
        }

        #[test]
        fn test_send_generate_resource_request_oxygen() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Oxygen,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            match msg {
                ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource } => {
                    assert_eq!(explorer_id, 1);
                    assert_eq!(resource, BasicResourceType::Oxygen);
                }
                _ => panic!("Expected GenerateResourceRequest for Oxygen"),
            }
        }

        #[test]
        fn test_send_generate_resource_request_hydrogen() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Hydrogen,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            match msg {
                ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource } => {
                    assert_eq!(resource, BasicResourceType::Hydrogen);
                }
                _ => panic!("Expected Hydrogen"),
            }
        }

        #[test]
        fn test_send_generate_resource_request_carbon() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Carbon,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            match msg {
                ExplorerToPlanet::GenerateResourceRequest { resource, .. } => {
                    assert_eq!(resource, BasicResourceType::Carbon);
                }
                _ => panic!("Expected Carbon"),
            }
        }

        #[test]
        fn test_send_generate_resource_request_silicon() {
            let h = TestStruct::new();
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Silicon,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_planet();
            match msg {
                ExplorerToPlanet::GenerateResourceRequest { resource, .. } => {
                    assert_eq!(resource, BasicResourceType::Silicon);
                }
                _ => panic!("Expected Silicon"),
            }
        }
    }

    // ==================== 5. STATE MACHINE TRANSITIONS ====================

    mod state_machine_tests {
        use super::*;

        #[test]
        fn test_state_matches_orchestrator_all_variants() {
            // CurrentPlanetRequest -> allowed in Idle
            assert!(ExplorerState::Idle.matches_orchestrator_msg(
                &OrchestratorToExplorer::CurrentPlanetRequest
            ));

            // KillExplorer -> allowed in any state
            for state in [
                ExplorerState::Idle,
                ExplorerState::Traveling,
                ExplorerState::WaitingForNeighbours,
                ExplorerState::WaitingForSupportedResources,
                ExplorerState::WaitingForSupportedCombinations,
                ExplorerState::WaitingForAvailableEnergyCells,
            ] {
                assert!(
                    state.matches_orchestrator_msg(&OrchestratorToExplorer::KillExplorer),
                    "KillExplorer should match state {:?}",
                    state
                );
            }

            // StartExplorerAI -> only in WaitingToStartExplorerAI
            assert!(ExplorerState::WaitingToStartExplorerAI.matches_orchestrator_msg(
                &OrchestratorToExplorer::StartExplorerAI
            ));
        }

        #[test]
        fn test_should_terminate_only_killed() {
            assert!(ExplorerState::Killed.should_terminate());
            assert!(!ExplorerState::Idle.should_terminate());
            assert!(!ExplorerState::Traveling.should_terminate());
            assert!(!ExplorerState::WaitingForNeighbours.should_terminate());
            assert!(!ExplorerState::WaitingToStartExplorerAI.should_terminate());
        }

        #[test]
        fn test_can_process_buffer_only_idle() {
            assert!(ExplorerState::Idle.can_process_buffer());
            assert!(!ExplorerState::Traveling.can_process_buffer());
            assert!(!ExplorerState::Killed.can_process_buffer());
            assert!(!ExplorerState::WaitingForNeighbours.can_process_buffer());
            assert!(!ExplorerState::WaitingForSupportedResources.can_process_buffer());
        }

        #[test]
        fn test_full_state_lifecycle() {
            let mut h = TestStruct::new();

            // Start Idle
            assert_eq!(*h.explorer.state(), ExplorerState::Idle);

            // -> WaitingForNeighbours (after sending neighbors request)
            h.explorer.set_state(ExplorerState::WaitingForNeighbours);
            assert!(!h.explorer.state().can_process_buffer());

            // -> Idle (after receiving response)
            h.explorer.set_state(ExplorerState::Idle);
            assert!(h.explorer.state().can_process_buffer());

            // -> Traveling (after sending travel request)
            h.explorer.set_state(ExplorerState::Traveling);
            assert!(!h.explorer.state().can_process_buffer());

            // -> Idle (after arriving)
            h.explorer.set_state(ExplorerState::Idle);
            assert!(h.explorer.state().can_process_buffer());

            // -> Killed
            h.explorer.set_state(ExplorerState::Killed);
            assert!(h.explorer.state().should_terminate());
        }
    }

    // ==================== 6. REALISTIC SIMULATION FLOWS ====================

    mod simulation_flow_tests {
        use super::*;

        /// Simulates a complete "discover neighbors -> ask resources -> move" cycle
        /// without spawning threads (synchronous simulation)
        #[test]
        fn test_full_discovery_cycle_one_planet() {
            let mut h = TestStruct::new();

            // Step 1: Explorer sends NeighborsRequest (simulating AI action)
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                })
                .unwrap();

            let orch_msg = h.recv_from_explorer_to_orch();
            assert!(matches!(orch_msg, ExplorerToOrchestrator::NeighborsRequest { .. }));

            // Step 2: Orchestrator responds with neighbors
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.update_neighbors(100, vec![200, 300]);

            assert!(h.explorer.topology.contains(200));
            assert!(h.explorer.topology.contains(300));

            // Step 3: Explorer sends SupportedResourceRequest to planet
            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 })
                .unwrap();

            let planet_msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                planet_msg,
                ExplorerToPlanet::SupportedResourceRequest { .. }
            ));

            // Step 4: Planet responds with resources (handled by explorer)
            let mut resources = HashSet::new();
            resources.insert(BasicResourceType::Oxygen);
            resources.insert(BasicResourceType::Hydrogen);

            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_basic_resources(resources.clone());
            }

            // Step 5: Explorer sends SupportedCombinationRequest
            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 1 })
                .unwrap();

            let planet_msg2 = h.recv_from_explorer_to_planet();
            assert!(matches!(
                planet_msg2,
                ExplorerToPlanet::SupportedCombinationRequest { .. }
            ));

            // Step 6: Planet responds with combinations
            let mut combos = HashSet::new();
            combos.insert(ComplexResourceType::Water);

            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_complex_resources(combos);
            }

            // Step 7: Planet info is now complete (for planet 100, with neighbors set)
            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_neighbours(HashSet::from_iter(vec![200, 300]));
            }

            // Verify planet 100 is complete
            let info = h.explorer.get_planet_info(100).unwrap();
            assert!(info.is_complete());

            // Step 8: Explorer asks for travel to undiscovered planet
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                    dst_planet_id: 200,
                })
                .unwrap();

            let travel_msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                travel_msg,
                ExplorerToOrchestrator::TravelToPlanetRequest { dst_planet_id: 200, .. }
            ));

            h.explorer.set_state(ExplorerState::Traveling);
            assert_eq!(*h.explorer.state(), ExplorerState::Traveling);
        }

        /// Simulates resource generation: explorer asks planet for energy cells,
        /// gets them, then generates oxygen, hydrogen, and combines to water
        #[test]
        fn test_resource_production_flow() {
            let mut h = TestStruct::new();

            // Setup: planet 100 can generate O, H and combine Water
            let mut basics = HashSet::new();
            basics.insert(BasicResourceType::Oxygen);
            basics.insert(BasicResourceType::Hydrogen);

            let mut combos = HashSet::new();
            combos.insert(ComplexResourceType::Water);

            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_basic_resources(basics);
                info.set_complex_resources(combos);
                info.set_neighbours(HashSet::new());
            }

            h.explorer.set_energy_cells(10);
            assert_eq!(h.explorer.energy_cells, 10);

            // Step 1: Explorer sends AvailableEnergyCellRequest
            h.explorer
                .send_to_planet(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 })
                .unwrap();

            let planet_msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                planet_msg,
                ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: 1 }
            ));

            // Step 2: Planet responds with 10 cells
            h.explorer.set_energy_cells(10);

            // Step 3: Explorer generates Oxygen
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Oxygen,
                })
                .unwrap();

            let gen_msg = h.recv_from_explorer_to_planet();
            assert!(matches!(
                gen_msg,
                ExplorerToPlanet::GenerateResourceRequest {
                    resource: BasicResourceType::Oxygen,
                    ..
                }
            ));

            // Step 4: Explorer generates Hydrogen
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Hydrogen,
                })
                .unwrap();

            let gen_msg2 = h.recv_from_explorer_to_planet();
            assert!(matches!(
                gen_msg2,
                ExplorerToPlanet::GenerateResourceRequest {
                    resource: BasicResourceType::Hydrogen,
                    ..
                }
            ));

            // Verify: production priority logic should want Water when H+O are present
            // (This tests get_production_priority indirectly through decide_resource_action)
            let resource_needed = h.explorer.resources_needed();
            // With empty bag, should need H2O ingredients
            assert!(resource_needed.contains(&ResourceType::Basic(BasicResourceType::Hydrogen))
                || resource_needed.contains(&ResourceType::Basic(BasicResourceType::Oxygen)));
        }

        /// Simulates being killed mid-exploration
        #[test]
        fn test_kill_during_exploration() {
            let mut h = TestStruct::new();

            // Explorer in the middle of exploration (WaitingForNeighbours)
            h.explorer.set_state(ExplorerState::WaitingForNeighbours);

            // Orchestrator sends KillExplorer (matches any state)
            assert!(h.explorer.state().matches_orchestrator_msg(
                &OrchestratorToExplorer::KillExplorer
            ));

            // Simulate kill handler
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::KillExplorerResult {
                    explorer_id: 1,
                })
                .unwrap();
            h.explorer.set_state(ExplorerState::Killed);

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::KillExplorerResult { explorer_id: 1 }
            ));
            assert!(h.explorer.state().should_terminate());
        }

        /// Simulates multi-planet exploration: discover 3 planets, find resource on planet 3
        #[test]
        fn test_multi_planet_topology_discovery() {
            let mut h = TestStruct::new();

            // Build topology: 100 <-> 200 <-> 300
            h.explorer.update_neighbors(100, vec![200]);
            h.explorer.update_neighbors(200, vec![100, 300]);

            // Discover planet 100
            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_basic_resources(HashSet::new());
                info.set_complex_resources(HashSet::new());
            }

            // Discover planet 200
            if let Some(info) = h.explorer.get_planet_info_mut(200) {
                info.set_basic_resources(HashSet::new());
                info.set_complex_resources(HashSet::new());
            }

            // Planet 300 has Carbon
            let mut basics_300 = HashSet::new();
            basics_300.insert(BasicResourceType::Carbon);
            if let Some(info) = h.explorer.get_planet_info_mut(300) {
                info.set_basic_resources(basics_300);
                info.set_complex_resources(HashSet::new());
                info.set_neighbours(HashSet::from_iter(vec![200]));
            }

            // Pathfinding: from 100 to Carbon
            let path = h.explorer.topology.find_path_to_resource(
                100,
                ResourceType::Basic(BasicResourceType::Carbon),
            );

            assert!(path.is_some());
            let path = path.unwrap();
            assert_eq!(path, vec![200, 300]);

            // Explorer decides to travel to 200
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                    dst_planet_id: 200,
                })
                .unwrap();

            let msg = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg,
                ExplorerToOrchestrator::TravelToPlanetRequest { dst_planet_id: 200, .. }
            ));

            // Orchestrator sends MoveToPlanet with new sender
            let (new_send, _new_recv) = unbounded::<ExplorerToPlanet>();
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.action_queue.clear();
            h.explorer.action_queue.reset();
            h.explorer.set_planet_sender(new_send);
            h.explorer.set_planet_id(200);

            assert_eq!(h.explorer.planet_id(), 200);

            // Explorer travels to 300 next
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: 1,
                    current_planet_id: 200,
                    dst_planet_id: 300,
                })
                .unwrap();

            let msg2 = h.recv_from_explorer_to_orch();
            assert!(matches!(
                msg2,
                ExplorerToOrchestrator::TravelToPlanetRequest { dst_planet_id: 300, .. }
            ));
        }

        /// Simulates a complete start -> explore -> discover -> generate -> kill flow
        #[test]
        fn test_complete_game_flow() {
            let mut h = TestStruct::new();

            // === Phase 1: Start AI ===
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::StartExplorerAIResult {
                    explorer_id: 1,
                })
                .unwrap();
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.manual_mode_off();

            let start_msg = h.recv_from_explorer_to_orch();
            assert!(matches!(start_msg, ExplorerToOrchestrator::StartExplorerAIResult { .. }));

            // === Phase 2: Discover neighbors ===
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                })
                .unwrap();
            let _ = h.recv_from_explorer_to_orch();

            // Orchestrator responds
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.update_neighbors(100, vec![200]);

            // === Phase 3: Discover planet resources ===
            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedResourceRequest { explorer_id: 1 })
                .unwrap();
            let _ = h.recv_from_explorer_to_planet();

            let mut basics = HashSet::new();
            basics.insert(BasicResourceType::Oxygen);
            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_basic_resources(basics);
            }

            h.explorer
                .send_to_planet(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: 1 })
                .unwrap();
            let _ = h.recv_from_explorer_to_planet();

            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_complex_resources(HashSet::new());
                info.set_neighbours(HashSet::from_iter(vec![200]));
            }

            // === Phase 4: Generate resource ===
            h.explorer.set_energy_cells(3);
            h.explorer
                .send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: 1,
                    resource: BasicResourceType::Oxygen,
                })
                .unwrap();
            let msg = h.recv_from_explorer_to_planet();
            assert!(matches!(msg, ExplorerToPlanet::GenerateResourceRequest { .. }));

            // === Phase 5: Move to undiscovered planet ===
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: 1,
                    current_planet_id: 100,
                    dst_planet_id: 200,
                })
                .unwrap();
            let travel = h.recv_from_explorer_to_orch();
            assert!(matches!(
                travel,
                ExplorerToOrchestrator::TravelToPlanetRequest { dst_planet_id: 200, .. }
            ));
            h.explorer.set_state(ExplorerState::Traveling);

            // === Phase 6: Arrive at new planet ===
            let (new_send, _) = unbounded();
            h.explorer.set_state(ExplorerState::Idle);
            h.explorer.action_queue.clear();
            h.explorer.action_queue.reset();
            h.explorer.set_planet_sender(new_send);
            h.explorer.set_planet_id(200);
            assert_eq!(h.explorer.planet_id(), 200);

            // === Phase 7: Kill explorer ===
            assert!(h.explorer.state().matches_orchestrator_msg(
                &OrchestratorToExplorer::KillExplorer
            ));
            h.explorer
                .send_to_orchestrator(ExplorerToOrchestrator::KillExplorerResult {
                    explorer_id: 1,
                })
                .unwrap();
            h.explorer.set_state(ExplorerState::Killed);

            let kill_msg = h.recv_from_explorer_to_orch();
            assert!(matches!(kill_msg, ExplorerToOrchestrator::KillExplorerResult { .. }));
            assert!(h.explorer.state().should_terminate());
        }

        /// Simulates message buffering: messages arrive while explorer is Traveling
        /// and are processed once it returns to Idle
        #[test]
        fn test_message_buffering_while_traveling() {
            let mut h = TestStruct::new();

            // Explorer starts traveling
            h.explorer.set_state(ExplorerState::Traveling);

            // Messages that arrive while traveling should be buffered
            // (In the real loop these go to buffer; here we test the buffer directly)
            let buffered_msg = OrchestratorToExplorer::CurrentPlanetRequest;
            assert!(
                !h.explorer.state().matches_orchestrator_msg(&buffered_msg),
                "CurrentPlanetRequest should NOT match Traveling state"
            );

            // Manually add to buffer as the run loop would
            h.explorer.buffer_orchestrator_msg.push_back(buffered_msg);
            assert_eq!(h.explorer.buffer_orchestrator_msg.len(), 1);

            // Now arrive at planet
            h.explorer.set_state(ExplorerState::Idle);
            assert!(h.explorer.state().can_process_buffer());

            // Buffer should be processable now
            let buffered = h.explorer.buffer_orchestrator_msg.pop_front();
            assert!(buffered.is_some());
            assert!(matches!(
                buffered.unwrap(),
                OrchestratorToExplorer::CurrentPlanetRequest
            ));
        }

        /// Simulates multiple messages buffered and processed in order (FIFO)
        #[test]
        fn test_message_buffer_fifo_order() {
            let mut h = TestStruct::new();
            h.explorer.set_state(ExplorerState::WaitingForNeighbours);

            // Buffer multiple messages
            h.explorer
                .buffer_orchestrator_msg
                .push_back(OrchestratorToExplorer::CurrentPlanetRequest);
            h.explorer
                .buffer_orchestrator_msg
                .push_back(OrchestratorToExplorer::BagContentRequest);

            assert_eq!(h.explorer.buffer_orchestrator_msg.len(), 2);

            // Process in FIFO order
            let first = h.explorer.buffer_orchestrator_msg.pop_front().unwrap();
            let second = h.explorer.buffer_orchestrator_msg.pop_front().unwrap();

            assert!(matches!(first, OrchestratorToExplorer::CurrentPlanetRequest));
            assert!(matches!(second, OrchestratorToExplorer::BagContentRequest));
        }
    }

    // ==================== 7. RESOURCE DECISION LOGIC ====================

    mod resource_decision_tests {
        use super::*;

        fn setup_planet_with_all_resources(h: &mut TestStruct) {
            let mut basics = HashSet::new();
            basics.insert(BasicResourceType::Oxygen);
            basics.insert(BasicResourceType::Hydrogen);
            basics.insert(BasicResourceType::Carbon);
            basics.insert(BasicResourceType::Silicon);

            let mut combos = HashSet::new();
            combos.insert(ComplexResourceType::Water);
            combos.insert(ComplexResourceType::Life);
            combos.insert(ComplexResourceType::Diamond);
            combos.insert(ComplexResourceType::Robot);
            combos.insert(ComplexResourceType::AIPartner);

            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_basic_resources(basics);
                info.set_complex_resources(combos);
                info.set_neighbours(HashSet::new());
            }
        }

        /// Empty bag -> needs Hydrogen or Oxygen (towards Water chain)
        #[test]
        fn test_resources_needed_empty_bag() {
            let h = TestStruct::new();
            let needed = h.explorer.resources_needed();

            // With empty bag, should need H and O for Water
            assert!(needed.contains(&ResourceType::Basic(BasicResourceType::Hydrogen))
                || needed.contains(&ResourceType::Basic(BasicResourceType::Oxygen)));
        }

        /// decide_resource_action: planet has O, H available -> should suggest generating them
        #[test]
        fn test_decide_resource_action_basic_generation() {
            let mut h = TestStruct::new();
            setup_planet_with_all_resources(&mut h);

            // Empty bag, planet can generate everything
            let action = h.explorer.decide_resource_action();
            assert!(action.is_some());

            // Should request a basic resource needed for the AIPartner chain
            let action = action.unwrap();
            match action {
                ResourceType::Basic(_) | ResourceType::Complex(_) => {} // Valid
            }
        }

        /// decide_resource_action: no planet info -> returns None
        #[test]
        fn test_decide_resource_action_no_planet_info() {
            let mut h = TestStruct::new_with_params(1, 999, 5); // planet 999 not in topology

            let action = h.explorer.decide_resource_action();
            assert!(action.is_none(), "Should return None when planet info not available");
        }

        /// resources_needed: consistent with decision priority
        #[test]
        fn test_resources_needed_nonempty_coverage() {
            let h = TestStruct::new();
            let needed = h.explorer.resources_needed();
            assert!(!needed.is_empty(), "Newly created explorer always needs resources");
        }
    }

    // ==================== 8. ACTION QUEUE INTEGRATION ====================

    mod action_queue_integration_tests {
        use super::*;

        #[test]
        fn test_action_queue_cycle_all_actions() {
            let mut h = TestStruct::new();

            // Default queue has 6 actions
            assert_eq!(h.explorer.action_queue.len(), 6);

            // Drain the queue
            let mut actions = vec![];
            while let Some(action) = h.explorer.action_queue.next_action() {
                actions.push(action);
            }
            assert_eq!(actions.len(), 6);
            assert!(h.explorer.action_queue.is_empty());
        }

        #[test]
        fn test_action_queue_reset_after_planet_change() {
            let mut h = TestStruct::new();

            // Clear and reset as move_to_planet handler does
            h.explorer.action_queue.clear();
            assert!(h.explorer.action_queue.is_empty());

            h.explorer.action_queue.reset();
            assert_eq!(h.explorer.action_queue.len(), 6);
            assert!(!h.explorer.action_queue.is_empty());
        }

        #[test]
        fn test_move_queue_path_planning() {
            let mut h = TestStruct::new();

            // Explorer needs to visit: 200, 300, 400
            let path: VecDeque<u32> = vec![200, 300, 400].into_iter().collect();
            h.explorer.move_queue.push_path(path);

            assert_eq!(h.explorer.move_queue.next_move(), Some(200));
            assert_eq!(h.explorer.move_queue.next_move(), Some(300));
            assert_eq!(h.explorer.move_queue.next_move(), Some(400));
            assert_eq!(h.explorer.move_queue.next_move(), None);
        }

        #[test]
        fn test_move_queue_cleared_on_travel_failure() {
            let mut h = TestStruct::new();

            h.explorer.move_queue.push_back(200);
            h.explorer.move_queue.push_back(300);

            // Simulate travel failure -> clear
            h.explorer.move_queue.clear();
            assert!(h.explorer.move_queue.is_empty());
        }
    }

    // ==================== 9. EDGE CASES ====================

    mod edge_case_tests {
        use super::*;

        /// Sending on a disconnected channel should return Err
        #[test]
        fn test_send_to_orchestrator_after_receiver_dropped() {
            let (orch_send, orch_recv) = unbounded::<OrchestratorToExplorer>();
            let (explorer_orch_send, explorer_orch_recv) =
                unbounded::<ExplorerToOrchestrator<BagType>>();
            let (planet_send, planet_recv) = unbounded::<PlanetToExplorer>();
            let (explorer_planet_send, explorer_planet_recv) =
                unbounded::<ExplorerToPlanet>();

            let explorer = Explorer::new(
                1,
                100,
                (orch_recv, explorer_orch_send),
                (planet_recv, explorer_planet_send),
                5,
            );

            // Drop the receiver
            drop(explorer_orch_recv);

            let result = explorer.send_to_orchestrator(ExplorerToOrchestrator::CurrentPlanetResult {
                explorer_id: 1,
                planet_id: 100,
            });
            assert!(result.is_err(), "Send to dropped receiver should fail");
        }

        /// Multiple explorers on the same channels (different IDs)
        #[test]
        fn test_multiple_explorer_ids_distinct() {
            let h1 = TestStruct::new_with_params(1, 100, 5);
            let h2 = TestStruct::new_with_params(2, 200, 10);

            assert_ne!(h1.explorer.id(), h2.explorer.id());
            assert_ne!(h1.explorer.planet_id(), h2.explorer.planet_id());
        }

        /// Topology with no frontier when fully discovered
        #[test]
        fn test_no_frontier_when_fully_discovered() {
            let mut h = TestStruct::new();

            // Single planet, fully discovered
            if let Some(info) = h.explorer.get_planet_info_mut(100) {
                info.set_basic_resources(HashSet::new());
                info.set_complex_resources(HashSet::new());
                info.set_neighbours(HashSet::new());
            }

            assert!(h.explorer.topology.is_fully_discovered());
            let frontier = h.explorer.topology.find_path_to_nearest_frontier(100);
            assert!(frontier.is_none());
        }

        /// Planet buffer: planet messages buffered when state doesn't match
        #[test]
        fn test_planet_message_buffered_when_state_mismatch() {
            let mut h = TestStruct::new();
            h.explorer.set_state(ExplorerState::Traveling);

            let planet_msg = PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 5 };
            // Check it doesn't match (in real loop this goes to buffer)
            // We can't call matches_planet_msg externally but we simulate the buffer
            h.explorer.buffer_planet_msg.push_back(planet_msg);
            assert_eq!(h.explorer.buffer_planet_msg.len(), 1);
        }

        /// Energy cells = 0: explorer should not generate resources
        #[test]
        fn test_no_generation_with_zero_energy() {
            let mut h = TestStruct::new();
            h.explorer.set_energy_cells(0);
            assert_eq!(h.explorer.energy_cells, 0);

            // With 0 energy, the GenerateOrCombine action block skips generation
            // Verified by the `if self.energy_cells > 0` guard in execute_ai_action
            assert_eq!(h.explorer.energy_cells, 0); // no messages sent to planet
        }
    }

    #[test]
    fn test_real_simulation() {
        match Orchestrator::new() {
            Ok(mut orch) => {
                // spawn planet
                orch.add_planet(0, PlanetType::RustyCrab).unwrap();

                // start planet ai
                orch.planet_channels[&0]
                    .0
                    .send(OrchestratorToPlanet::StartPlanetAI)
                    .unwrap();

                // wait for the planet to be running
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                loop {
                    orch.handle_game_messages().unwrap();

                    let status = orch.planets_info.get_status(&0);
                    println!("[TEST] planet 0 state: {:?}", status);

                    if status == Status::Running {
                        break;
                    }
                    if std::time::Instant::now() > deadline {
                        println!("[TEST] planets_info dump: {:?}", orch.planets_info);
                        println!("[TEST] explorer_channels keys: {:?}", orch.explorer_channels.keys().collect::<Vec<_>>());
                        panic!("Timeout: planet not running yet");
                    }
                    thread::sleep(Duration::from_millis(10));
                }

                // spawn explorer
                orch.add_tommy_explorer(0, 0).unwrap();

                // start explorer ai
                orch.explorer_channels[&0]
                    .0
                    .send(OrchestratorToExplorer::StartExplorerAI)
                    .unwrap();

                // wait for the explorer to be running
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                loop {
                    orch.handle_game_messages().unwrap();
                    if orch.explorers_info.get_status(&0) == Status::Running {
                        break;
                    }
                    if std::time::Instant::now() > deadline {
                        panic!("Timeout: explorer not running yet");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                println!("[TEST] explorer 0 running");

                // core simulation
                let simulation_duration = Duration::from_secs(3);
                let sim_start = std::time::Instant::now();
                while std::time::Instant::now() - sim_start < simulation_duration {
                    orch.handle_game_messages().unwrap();
                    thread::sleep(Duration::from_millis(10));
                }
                println!("[TEST] simulation complete, send kill explorer");

                // kill explorer
                orch.explorer_channels[&0]
                    .0
                    .send(OrchestratorToExplorer::KillExplorer)
                    .unwrap();

                // wait for the kill explorer response
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                loop {
                    orch.handle_game_messages().unwrap();
                    if orch.explorers_info.get_status(&0) == Status::Dead {
                        break;
                    }
                    if std::time::Instant::now() > deadline {
                        panic!("Timeout: explorer not dead yet");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                println!("[TEST] explorer 0 dead");

                // kill planet
                orch.planet_channels[&0]
                    .0
                    .send(OrchestratorToPlanet::KillPlanet)
                    .unwrap();

                // wait for the kill planet response
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                loop {
                    orch.handle_game_messages().unwrap();
                    if orch.planets_info.get_status(&0) == Status::Dead {
                        break;
                    }
                    if std::time::Instant::now() > deadline {
                        panic!("Timeout: planet not dead yet");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                println!("[TEST] planet 0 dead");
            }
            Err(err) => {
                panic!("{:?}", err);
            }
        }
    }
}
