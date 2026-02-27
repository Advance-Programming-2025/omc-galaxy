use crossbeam_channel::{Receiver, Sender, select};
use std::collections::{HashSet, VecDeque};

use super::actions::{ActionQueue, ExplorerAction, MoveQueue};
use super::bag::{Bag, BagType};
use super::handlers::{orchestrator, planet};
use super::state::ExplorerState;
use super::topology::{PlanetInfo, TopologyManager};
use crate::components::tommy_explorer::handlers::orchestrator::{
    combine_resource_request, generate_resource_request,
};
use common_game::components::resource::{
    BasicResourceType, ComplexResourceRequest, ComplexResourceType, GenericResource, ResourceType,
};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

/// struct of the explorer
pub struct Explorer {
    pub explorer_id: u32,
    pub(crate) planet_id: u32,
    pub orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<BagType>>,
    ),
    pub planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    pub(crate) topology: TopologyManager,
    pub state: ExplorerState,
    pub bag: Bag,
    pub energy_cells: u32, // of the current planet
    pub buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>, // orchestrator messages that the explorer cannot respond to immediately
    pub buffer_planet_msg: VecDeque<PlanetToExplorer>, // planet messages that the explorer cannot respond to immediately
    pub action_queue: ActionQueue, // actions that the explorer can perform (sorted in the correct order)
    pub move_queue: MoveQueue,
    manual_mode: bool,
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
            state: ExplorerState::Idle,
            bag: Bag::new(),
            energy_cells,
            buffer_orchestrator_msg: VecDeque::new(),
            buffer_planet_msg: VecDeque::new(),
            action_queue: ActionQueue::new(),
            move_queue: MoveQueue::new(),
            manual_mode: true,
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

    /// Gets the bag content as resource types.
    pub fn get_bag_content(&self) -> BagType {
        self.bag.to_resource_types()
    }

    // ==================== Setter Methods ====================

    /// Sets the explorer state.
    pub fn set_state(&mut self, state: ExplorerState) {
        self.state = state;
    }

    /// Sets the planet ID.
    pub fn set_planet_id(&mut self, planet_id: u32) {
        self.planet_id = planet_id;
    }

    /// Sets the planet sender channel.
    pub fn set_planet_sender(&mut self, sender: Sender<ExplorerToPlanet>) {
        self.planet_channels.1 = sender;
    }

    /// Sets the energy cells.
    pub fn set_energy_cells(&mut self, cells: u32) {
        self.energy_cells = cells;
    }

    /// Sets the manual mode to on.
    pub fn manual_mode_on(&mut self) {
        self.manual_mode = true;
    }

    /// Sets the manual mode to off.
    pub fn manual_mode_off(&mut self) {
        self.manual_mode = false;
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
                            println!("[EXPLORER TOMMY DEBUG] Error in receiving the orchestrator message: {}", err);
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
                            println!("[EXPLORER TOMMY DEBUG] Error in receiving the planet message: {}", err);
                        }
                    }
                }
                // default branch, here the explorer performs choices and actions
                // other than managing the buffered messages
                default => {
                    // priority to the buffered messages
                    if self.manual_mode {
                        continue;
                    }

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
                    self.action_queue.push_back(action);
                    match self.send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest {
                        explorer_id: self.explorer_id,
                        current_planet_id: self.planet_id,
                    }) {
                        Ok(_) => {
                            // if the sending is successful change the state to WaitingForNeighbours
                            // and push back the action
                            self.set_state(ExplorerState::WaitingForNeighbours);
                            println!("[EXPLORER TOMY DEBUG] AskNeighbours"); 
                        }
                        Err(err) => {
                            println!(
                                "[EXPLORER DEBUG] Error in sending NeighboursRequest: {}",
                                err
                            );
                        }
                    }
                }
                ExplorerAction::AskSupportedResources => {
                    // TODO make an "if not discovered then ..."
                    self.action_queue.push_back(action);
                    match self.send_to_planet(ExplorerToPlanet::SupportedResourceRequest {
                        explorer_id: self.explorer_id,
                    }) {
                        Ok(_) => {
                            // if the sending was successful change the state to WaitingForSupportedResources
                            self.set_state(ExplorerState::WaitingForSupportedResources);
                            println!("[EXPLORER TOMY DEBUG] AskSupportedResources");
                        }
                        Err(err) => {
                            // TODO
                        }
                    }
                }
                ExplorerAction::AskSupportedCombinations => {
                    // TODO make an "if not discovered then ..."
                    self.action_queue.push_back(action);
                    match self.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
                        explorer_id: self.explorer_id,
                    }) {
                        Ok(_) => {
                            // if the sending was successful change the state to WaitingForSupportedCombinations
                            self.set_state(ExplorerState::WaitingForSupportedCombinations);
                            println!("[EXPLORER TOMY DEBUG] AskSupportedCombinations");
                        }
                        Err(err) => {
                            // TODO
                        }
                    }
                }
                ExplorerAction::AskFreeCells => {
                    self.action_queue.push_back(action);
                    match self.send_to_planet(ExplorerToPlanet::AvailableEnergyCellRequest {
                        explorer_id: self.explorer_id,
                    }) {
                        Ok(_) => {
                            self.set_state(ExplorerState::WaitingForAvailableEnergyCells);
                            println!("[EXPLORER TOMY DEBUG] AvailableEnergyCellRequest");
                        }
                        Err(err) => {
                            // TODO
                        }
                    }
                }
                ExplorerAction::GenerateOrCombine => {
                    // TODO: Implement generation/combination logic based on AI strategy
                    // IMPORTANT continue to generate/combine till the explorer can

                    // TODO decision on what to generate/combine
                    // if the topology isn't fully discovered simply generate/combine the useful resources
                    // otherwise:
                    // check what and how many resources the explorer has
                    // check what and how many resources are needed to complete the goal -> see the dependency graph of the resources
                    // maybe check what resources can be obtained from other planets in a possible path
                    // choose the resource based on the things written above
                    // generate/combine it

                    self.action_queue.push_back(action);

                    if self.energy_cells > 0 {
                        println!("[EXPLORER TOMY DEBUG] GenerateOrCombine");
                        if let Some(resource) = self.decide_resource_action() {
                            match resource {
                                ResourceType::Basic(basic_resource) => match basic_resource {
                                    BasicResourceType::Oxygen => {
                                        generate_resource_request(self, BasicResourceType::Oxygen);
                                    }
                                    BasicResourceType::Hydrogen => {
                                        generate_resource_request(
                                            self,
                                            BasicResourceType::Hydrogen,
                                        );
                                    }
                                    BasicResourceType::Carbon => {
                                        generate_resource_request(self, BasicResourceType::Carbon);
                                    }
                                    BasicResourceType::Silicon => {
                                        generate_resource_request(self, BasicResourceType::Silicon);
                                    }
                                },
                                ResourceType::Complex(complex_resource) => match complex_resource {
                                    ComplexResourceType::Diamond => {
                                        combine_resource_request(
                                            self,
                                            ComplexResourceType::Diamond,
                                        );
                                    }
                                    ComplexResourceType::Water => {
                                        combine_resource_request(self, ComplexResourceType::Water);
                                    }
                                    ComplexResourceType::Life => {
                                        combine_resource_request(self, ComplexResourceType::Life);
                                    }
                                    ComplexResourceType::Robot => {
                                        combine_resource_request(self, ComplexResourceType::Robot);
                                    }
                                    ComplexResourceType::Dolphin => {
                                        combine_resource_request(
                                            self,
                                            ComplexResourceType::Dolphin,
                                        );
                                    }
                                    ComplexResourceType::AIPartner => {
                                        combine_resource_request(
                                            self,
                                            ComplexResourceType::AIPartner,
                                        );
                                    }
                                },
                            }
                        }
                    }
                }
                ExplorerAction::Move => {
                    // TODO: Implement movement logic based on AI strategy

                    // TODO decision on where to move
                    // 1st case -> the topology isn't fully discovered yet
                    // check the planets that still need to be visited
                    // choose the best path to visit those planets in the shortest way possible

                    // 2nd case -> the topology is fully discovered
                    // check what and how many resources the explorer has
                    // maybe check what resources can be obtained from other planets in a possible path
                    // choose the best path to achieve the goal

                    // TODO this is the exploring-phase
                    self.action_queue.push_back(action);

                    // obtain the needed resource
                    let resource = self.get_production_priority();
                    if let Some(path) = self.topology.find_path_to_nearest_frontier(self.planet_id)
                    {
                        // if the topology isn't fully discovered yet, continue exploring
                        self.move_queue.push_path(path)
                    } else if let Some(path) = self
                        .topology
                        .find_path_to_resource(self.planet_id, resource)
                    {
                        // else find the best path to reach the resource goal
                        self.move_queue.push_path(path)
                    }

                    // if the explorer has to move somewhere send a TravelToPlanetRequest
                    if let Some(next_planet) = self.move_queue.next_move() {
                        if self.topology.contains(next_planet) {
                            match self.send_to_orchestrator(
                                ExplorerToOrchestrator::TravelToPlanetRequest {
                                    explorer_id: self.explorer_id,
                                    current_planet_id: self.planet_id,
                                    dst_planet_id: next_planet,
                                },
                            ) {
                                Ok(_) => {
                                    self.set_state(ExplorerState::Traveling); // TODO should be Idle, in the case in which the planet is dead and can't respond
                                    println!("[EXPLORER TOMY DEBUG] Traveling");
                                }
                                Err(_) => { 
                                    self.move_queue.clear(); 
                                    println!("[EXPLORER TOMY DEBUG] Not traveling");
                                },
                            }
                        } else {
                            self.move_queue.clear();
                        }
                    }
                }
            }
        }
    }

    /// Checks the bag of the explorer and finds the needed resource by looking at the
    /// dependency graph of the resources. The most complex resource needed is returned first.
    fn get_production_priority(&self) -> ResourceType {
        let bag = self.bag.to_resource_types();

        if bag.contains(&ResourceType::Complex(ComplexResourceType::Robot))
            && bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond))
        {
            // if the explorer has robot and diamond
            return ResourceType::Complex(ComplexResourceType::AIPartner);
        }

        let carbon_count = bag
            .iter()
            .filter(|r| **r == ResourceType::Basic(BasicResourceType::Carbon))
            .count();
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond)) {
            if carbon_count >= 2 {
                // if he has no diamond but at least 2 carbon
                return ResourceType::Complex(ComplexResourceType::Diamond);
            }
            // if he has no diamond and max 1 carbon
            return ResourceType::Basic(BasicResourceType::Carbon);
        }

        // if the explorer doesn't have robot
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Robot)) {
            let has_silicon = bag.contains(&ResourceType::Basic(BasicResourceType::Silicon));
            let has_life = bag.contains(&ResourceType::Complex(ComplexResourceType::Life));

            if has_life {
                return if has_silicon {
                    // if he has life and silicon
                    ResourceType::Complex(ComplexResourceType::Robot)
                } else {
                    // if he has life and not silicon
                    ResourceType::Basic(BasicResourceType::Silicon)
                };
            }

            // if he has no life
            let has_water = bag.contains(&ResourceType::Complex(ComplexResourceType::Water));
            if has_water {
                return if carbon_count >= 1 {
                    // if he has water and carbon
                    ResourceType::Complex(ComplexResourceType::Life)
                } else {
                    // if he has water but no carbon
                    ResourceType::Basic(BasicResourceType::Carbon)
                };
            }

            // if he has no water
            let has_h = bag.contains(&ResourceType::Basic(BasicResourceType::Hydrogen));
            let has_o = bag.contains(&ResourceType::Basic(BasicResourceType::Oxygen));

            if has_h && has_o {
                // if he has both hydrogen and oxygen
                return ResourceType::Complex(ComplexResourceType::Water);
            }
            if !has_h {
                // if he has hydrogen but no oxygen
                return ResourceType::Basic(BasicResourceType::Hydrogen);
            }
            // if he has no hydrogen nor oxygen
            return ResourceType::Basic(BasicResourceType::Oxygen);
        }

        // this shouldn't happen (all possible cases should have been taken in consideration)
        println!(
            "[EXPLORER DEBUG] Something went wrong in the decision of the next needed resource."
        );
        ResourceType::Basic(BasicResourceType::Carbon)
    }

    /// Returns an HashSet containing all the resources needed.
    pub fn resources_needed(&self) -> HashSet<ResourceType> {
        let bag = self.bag.to_resource_types();
        let mut res = HashSet::new();

        if bag.contains(&ResourceType::Complex(ComplexResourceType::Robot))
            && bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond))
        {
            // if the explorer has robot and diamond
            res.insert(ResourceType::Complex(ComplexResourceType::AIPartner));
        }

        let carbon_count = bag
            .iter()
            .filter(|r| **r == ResourceType::Basic(BasicResourceType::Carbon))
            .count();
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond)) {
            if carbon_count >= 2 {
                // if he has no diamond but at least 2 carbon
                res.insert(ResourceType::Complex(ComplexResourceType::Diamond));
            }
            // if he has no diamond and max 1 carbon
            res.insert(ResourceType::Basic(BasicResourceType::Carbon));
        }

        // if the explorer doesn't have robot
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Robot)) {
            let has_silicon = bag.contains(&ResourceType::Basic(BasicResourceType::Silicon));
            let has_life = bag.contains(&ResourceType::Complex(ComplexResourceType::Life));

            if has_life {
                if has_silicon {
                    // if he has life and silicon
                    res.insert(ResourceType::Complex(ComplexResourceType::Robot));
                } else {
                    // if he has life and not silicon
                    res.insert(ResourceType::Basic(BasicResourceType::Silicon));
                }
            }

            // if he has no life
            let has_water = bag.contains(&ResourceType::Complex(ComplexResourceType::Water));
            if has_water {
                if carbon_count >= 1 {
                    // if he has water and carbon
                    res.insert(ResourceType::Complex(ComplexResourceType::Life));
                } else {
                    // if he has water but no carbon
                    res.insert(ResourceType::Basic(BasicResourceType::Carbon));
                }
            }

            // if he has no water
            let has_h = bag.contains(&ResourceType::Basic(BasicResourceType::Hydrogen));
            let has_o = bag.contains(&ResourceType::Basic(BasicResourceType::Oxygen));

            if has_h && has_o {
                // if he has both hydrogen and oxygen
                res.insert(ResourceType::Complex(ComplexResourceType::Water));
            }
            if !has_h {
                // if he has hydrogen but no oxygen
                res.insert(ResourceType::Basic(BasicResourceType::Hydrogen));
            }
            // if he has no hydrogen nor oxygen
            res.insert(ResourceType::Basic(BasicResourceType::Oxygen));
        }

        res
    }

    /// Returns the resource to generate/combine based on the needs and the availability of the planet,
    /// or None if no resource can be crafted.
    pub fn decide_resource_action(&self) -> Option<ResourceType> {
        let current_planet_info = self.topology.get(self.planet_id)?;
        let needed = self.resources_needed();

        // check
        if let Some(planet_complex) = current_planet_info.get_complex_resources() {
            // same order of the target for path research
            let craft_order = [
                ComplexResourceType::AIPartner,
                ComplexResourceType::Robot,
                ComplexResourceType::Diamond,
                ComplexResourceType::Life,
                ComplexResourceType::Water,
            ];

            for complex_ty in craft_order {
                let res_ty = ResourceType::Complex(complex_ty);
                // if the planet can craft it and we need it
                if planet_complex.contains(&complex_ty) && needed.contains(&res_ty) {
                    // if we can actually craft it
                    if self.can_actually_craft(complex_ty) {
                        return Some(res_ty);
                    }
                }
            }
        }

        // then we check basic resources
        if let Some(planet_basic) = current_planet_info.get_basic_resources() {
            for basic_ty in planet_basic {
                let res_ty = ResourceType::Basic(*basic_ty);
                if needed.contains(&res_ty) {
                    return Some(res_ty);
                }
            }
        }

        None
    }

    /// Helper function that checks if the explorer can craft a specific complex resource.
    fn can_actually_craft(&self, complex_ty: ComplexResourceType) -> bool {
        let bag = self.bag.to_resource_types();
        match complex_ty {
            ComplexResourceType::Water => {
                bag.contains(&ResourceType::Basic(BasicResourceType::Hydrogen))
                    && bag.contains(&ResourceType::Basic(BasicResourceType::Oxygen))
            }
            ComplexResourceType::Life => {
                bag.contains(&ResourceType::Complex(ComplexResourceType::Water))
                    && bag.contains(&ResourceType::Basic(BasicResourceType::Carbon))
            }
            ComplexResourceType::Diamond => {
                bag.iter()
                    .filter(|r| **r == ResourceType::Basic(BasicResourceType::Carbon))
                    .count()
                    >= 2
            }
            ComplexResourceType::Robot => {
                bag.contains(&ResourceType::Basic(BasicResourceType::Silicon))
                    && bag.contains(&ResourceType::Complex(ComplexResourceType::Life))
            }
            ComplexResourceType::AIPartner => {
                bag.contains(&ResourceType::Complex(ComplexResourceType::Robot))
                    && bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond))
            }
            _ => false,
        }
    }
}
