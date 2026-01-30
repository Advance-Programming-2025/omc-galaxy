use std::collections::{VecDeque};
use crossbeam_channel::{Receiver, Sender, select};

use common_game::components::resource::{ComplexResourceRequest, ComplexResourceType, GenericResource};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

use super::actions::{ActionQueue, ExplorerAction};
use super::bag::{Bag, BagType};
use super::state::ExplorerState;
use super::topology::{PlanetInfo, TopologyManager};
use super::handlers::{orchestrator, planet};

/// struct of the explorer
pub struct Explorer {
    explorer_id: u32,
    planet_id: u32,
    orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<BagType>>,
    ),
    planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    topology: TopologyManager,
    state: ExplorerState,
    bag: Bag,
    energy_cells: u32, // of the current planet
    buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>, // orchestrator messages that the explorer cannot respond to immediately
    buffer_planet_msg: VecDeque<PlanetToExplorer>, // planet messages that the explorer cannot respond to immediately
    action_queue: ActionQueue, // actions that the explorer can perform (sorted in the correct order)
}

impl Explorer {
    /// Creates a new Explorer connected to Orchestrator and the starting Planet
    pub fn new(
        explorer_id: u32,
        planet_id: u32,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<BagType>>,
        ),
        explorer_to_planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
        energy_cells: u32, // useful in the case in which the explorer starts mid-game
    ) -> Self {
        Self {
            explorer_id,
            planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: explorer_to_planet_channels,
            topology: TopologyManager::new(planet_id),
            state: ExplorerState::WaitingToStartExplorerAI,
            bag: Bag::new(),
            energy_cells,
            buffer_orchestrator_msg: VecDeque::new(),
            buffer_planet_msg: VecDeque::new(),
            action_queue: ActionQueue::new(),
        }
    }

    // ==================== Getter Methods ====================

    /// gets the explorer ID
    pub fn id(&self) -> u32 {
        self.explorer_id
    }

    /// gets the current planet ID
    pub fn planet_id(&self) -> u32 {
        self.planet_id
    }

    /// gets the current state
    pub fn state(&self) -> &ExplorerState {
        &self.state
    }

    /// gets information about a planet
    pub fn get_planet_info(&self, planet_id: ID) -> Option<&PlanetInfo> {
        self.topology.get(planet_id)
    }

    /// gets mutable information about a planet
    pub fn get_planet_info_mut(&mut self, planet_id: ID) -> Option<&mut PlanetInfo> {
        self.topology.get_mut(planet_id)
    }

    /// gets the bag content as resource types
    pub fn get_bag_content(&self) -> BagType {
        self.bag.to_resource_types()
    }

    // ==================== Setter Methods ====================

    /// sets the explorer state
    pub fn set_state(&mut self, state: ExplorerState) {
        self.state = state;
    }

    /// sets the planet ID
    pub fn set_planet_id(&mut self, planet_id: u32) {
        self.planet_id = planet_id;
    }

    /// sets the planet sender channel
    pub fn set_planet_sender(&mut self, sender: Sender<ExplorerToPlanet>) {
        self.planet_channels.1 = sender;
    }

    /// sets the energy cells
    pub fn set_energy_cells(&mut self, cells: u32) {
        self.energy_cells = cells;
    }

    // ==================== Communication Methods ====================

    /// sends a message to the orchestrator
    pub fn send_to_orchestrator(
        &self,
        msg: ExplorerToOrchestrator<BagType>,
    ) -> Result<(), crossbeam_channel::SendError<ExplorerToOrchestrator<BagType>>> {
        self.orchestrator_channels.1.send(msg)
    }

    /// sends a message to the planet
    pub fn send_to_planet(
        &self,
        msg: ExplorerToPlanet,
    ) -> Result<(), crossbeam_channel::SendError<ExplorerToPlanet>> {
        self.planet_channels.1.send(msg)
    }

    /// receives a message from the planet (blocking)
    pub fn receive_from_planet(&self) -> Result<PlanetToExplorer, crossbeam_channel::RecvError> {
        self.planet_channels.0.recv()
    }

    // ==================== Bag Methods ====================

    /// inserts a resource in the bag
    pub fn insert_in_bag(&mut self, resource: GenericResource) {
        self.bag.insert(resource);
    }

    /// creates a complex resource request
    pub fn make_complex_request(
        &mut self,
        resource_type: ComplexResourceType,
    ) -> Result<ComplexResourceRequest, String> {
        self.bag.make_complex_request(resource_type)
    }

    // ==================== Topology Methods ====================

    /// clears the topology
    pub fn clear_topology(&mut self) {
        self.topology.clear();
    }

    /// updates neighbors for a planet
    pub fn update_neighbors(&mut self, planet_id: ID, neighbors: Vec<ID>) {
        self.topology.update_neighbours(planet_id, neighbors);
    }

    // ==================== Main Loop ====================

    /// the explorer main loop
    pub fn run(&mut self) -> Result<(), String> {
        if self.state.should_terminate() {
            return Ok(());
        }

        // every iteration the explorer receives messages from both planet and orchestrator channels,
        // then it behaves based on the message received, if the message received and the explorer state
        // do not match together the message is pushed into the corresponding buffer, and it will be read
        // when the explorer will be in an "Idle" state
        loop {
            select! {
                // receive the orchestrator messages
                recv(self.orchestrator_channels.0) -> msg_orchestrator => {
                    match msg_orchestrator {
                        Ok(msg) => {
                            // the explorer handles the message only if he is in the correct state to do so
                            if self.state.matches_orchestrator_msg(&msg) {
                                // handle_message return Ok(true) if the explorer thread should terminate
                                let should_terminate = orchestrator::handle_message(self, msg)?;
                                if should_terminate {
                                    return Ok(());
                                }
                            } else {
                                // if the explorer is not in the correct state to handle the message,
                                // the message is buffered
                                self.buffer_orchestrator_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            println!("[EXPLORER DEBUG] Error in receiving the orchestrator message: {}", err);
                        }
                    }
                },
                // receive the planet messages
                recv(self.planet_channels.0) -> msg_planet => {
                    match msg_planet {
                        Ok(msg) => {
                            // the explorer handles the message only if he is in the correct state to do so
                            if self.state.matches_planet_msg(&msg) {
                                planet::handle_message(self, msg)?;
                            } else {
                                // if the explorer is not in the correct state to handle the message,
                                // the message is buffered
                                self.buffer_planet_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            println!("[EXPLORER DEBUG] Error in receiving the planet message: {}", err);
                        }
                    }
                }
                // default branch, here the explorer performs choices and actions
                // other than managing the buffered messages
                default => {
                    // priority to the buffered messages
                    match self.state {
                        ExplorerState::Idle => {
                            self.process_buffered_messages()?;
                            if self.state.should_terminate() {
                                return Ok(());
                            }
                        }
                        // if we are not in idle state we need to manage some other message
                        _ => continue,
                    }

                    // if the state is still idle after processing buffers, execute AI actions
                    if matches!(self.state, ExplorerState::Idle) {
                        self.execute_ai_action();
                    }
                }
            }
        }
    }

    // ==================== Buffer Management ====================

    /// processes all buffered messages
    fn process_buffered_messages(&mut self) -> Result<(), String> {
        if !self.state.can_process_buffer() {
            return Ok(());
        }

        // process orchestrator messages
        while let Some(msg) = self.buffer_orchestrator_msg.pop_front() {
            let should_terminate = orchestrator::handle_message(self, msg)?;
            if should_terminate {
                return Ok(());
            }
            if !self.state.can_process_buffer() {
                return Ok(());
            }
        }

        // process planet messages
        while let Some(msg) = self.buffer_planet_msg.pop_front() {
            planet::handle_message(self, msg)?;
            if !self.state.can_process_buffer() {
                return Ok(());
            }
        }

        Ok(())
    }

    // ==================== AI Logic ====================

    /// executes the next AI action
    fn execute_ai_action(&mut self) {
        // TODO: Implement the AI flow
        // 1) ask for neighbours (every time, they could change)
        // 2) ask for resources and combining rules (only if not memorized yet)
        // 3) generate/combine resources in order to achieve your explorer goal
        // 4) move (do it casually at first just to discover all the topology, then
        //    use the topology to visit the graph in some particular order)
        // 5) special behaviours for some specific planets (if they have special features)
        // 6) repeat

        if let Some(action) = self.action_queue.next_action() {
            match action {
                ExplorerAction::AskNeighbours => {
                    match self.send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest { explorer_id: self.explorer_id, current_planet_id: self.planet_id }) {
                        Ok(_) => {
                            // if the sending is successful change the state to WaitingForNeighbours
                            // and push back the action
                            self.set_state(ExplorerState::WaitingForNeighbours);
                            self.action_queue.push_back(action);
                        }
                        Err(err) => {
                            self.action_queue.push_front(action);
                            println!("[EXPLORER DEBUG] Error in sending NeighboursRequest: {}", err);
                        }
                    }
                }
                ExplorerAction::AskSupportedResources => {
                    match self.send_to_planet(ExplorerToPlanet::SupportedResourceRequest { explorer_id: self.explorer_id }) {
                        Ok(_) => {
                            // if the sending was successful change the state to WaitingForSupportedResources
                            self.set_state(ExplorerState::WaitingForSupportedResources);
                        }
                        Err(err) => {
                            self.action_queue.push_front(action);
                        }
                    }
                }
                ExplorerAction::AskSupportedCombinations => {
                    match self.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: self.explorer_id }) {
                        Ok(_) => {
                            // if the sending was successful change the state to WaitingForSupportedCombinations
                            self.set_state(ExplorerState::WaitingForSupportedCombinations);
                        }
                        Err(err) => {
                            self.action_queue.push_front(action);
                        }
                    }
                }
                ExplorerAction::GenerateOrCombine => {
                    // TODO: Implement generation/combination logic based on AI strategy
                    // IMPORTANT continue to generate/combine till the explorer can
                    self.action_queue.push_back(action);

                    // TODO decision on what to generate/combine
                    // if the topology isn't fully discovered simply generate/combine the useful resources
                    // otherwise:
                    // check what and how many resources the explorer has
                    // check what and how many resources are needed to complete the goal -> see the dependency graph of the resources
                    // maybe check what resources can be obtained from other planets in a possible path
                    // choose the resource based on the things written above
                    // generate/combine it
                }
                ExplorerAction::Move => {
                    // TODO: Implement movement logic based on AI strategy
                    self.action_queue.push_back(action);

                    // TODO decision on where to move
                    // 1st case -> the topology isn't fully discovered yet
                    // check the planets that still need to be visited
                    // choose the best path to visit those planets in the shortest way possible

                    // 2nd case -> the topology is fully discovered
                    // check what and how many resources the explorer has
                    // maybe check what resources can be obtained from other planets in a possible path
                    // choose the best path to achieve the goal

                }
            }
        }
    }
}
