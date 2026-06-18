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
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender, select};
use logging_utils::{get_receiver_id, get_sender_id, log_fn_call, log_message, warning_payload};
use std::collections::{HashSet, VecDeque};
use std::fmt;

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
    accept_death: bool,
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
        // LOG
        log_fn_call!(dir
            ActorType::Explorer,
            explorer_id,
            "Explorer::new()",
            explorer_id,
            planet_id;
            "explorer_to_orchestrator_channels" => format!("({}, {})", get_receiver_id(&explorer_to_orchestrator_channels.0), get_sender_id(&explorer_to_orchestrator_channels.1)),
            "explorer_to_planet_channels"=>format!("({}, {})", get_receiver_id(&explorer_to_planet_channels.0), get_sender_id(&explorer_to_planet_channels.1)),
        );
        // LOG
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
            accept_death: false,
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
            // println!("[TOMMY EXPLORER INFO] planet: {}", self.planet_id);
            select! {
                // receive the orchestrator messages
                recv(self.orchestrator_channels.0) -> msg_orchestrator => {
                    match msg_orchestrator {
                        Ok(msg) => {
                            // LOG
                            log_message!(
                                ActorType::Orchestrator,
                                0u32,
                                ActorType::Explorer,
                                self.explorer_id,
                                EventType::MessageOrchestratorToExplorer,
                                "message received";
                                "msg"=>format!("{:?}", msg),
                                "explorer data"=>format!("{:?}", self)
                            );
                            // LOG

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
                            // LOG
                            LogEvent::new(
                                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                EventType::MessageOrchestratorToExplorer,
                                Channel::Error,
                                warning_payload!(
                                    "receiving channel from orchestrator disconnected",
                                    err,
                                    "tommy_explorer::run()"
                                )
                            ).emit();
                            // LOG
                            return Err(err.to_string());
                        }
                    }
                },
                // receive the planet messages
                recv(self.planet_channels.0) -> msg_planet => {
                    match msg_planet {
                        Ok(msg) => {
                            // LOG
                            log_message!(
                                ActorType::Planet,
                                self.planet_id,
                                ActorType::Explorer,
                                self.explorer_id,
                                EventType::MessagePlanetToExplorer,
                                "message received";
                                "msg"=>format!("{:?}", msg),
                                "explorer data"=>format!("{:?}", self)
                            );
                            // LOG

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
                            // LOG
                            LogEvent::new(
                                Some(Participant::new(ActorType::Planet, self.planet_id)),
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                EventType::MessagePlanetToExplorer,
                                Channel::Error,
                                warning_payload!(
                                    "receiving channel from planet disconnected",
                                    err,
                                    "tommy_explorer::run()"
                                )
                            ).emit();
                            // LOG
                        }
                    }
                }
                // default branch, here the explorer performs choices and actions
                // other than managing the buffered messages
                default => {
                    // priority to the buffered messages
                    if self.manual_mode {
                        std::thread::sleep(std::time::Duration::from_millis(20));
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
                        _ => {
                            std::thread::sleep(std::time::Duration::from_millis(20));
                            continue;
                        },
                    }

                    // if the state is still idle after processing buffers, execute AI actions
                    if matches!(self.state, ExplorerState::Idle) {
                        self.execute_ai_action();
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
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
        // 1) ask for neighbours (every time, they could change)
        // 2) ask for resources and combining rules (only if not memorized yet)
        // 3) generate/combine resources in order to achieve your explorer goal
        // 4) move (do it casually at first just to discover all the topology, then
        //    use the topology to visit the graph in some particular order)
        // 5) special behaviours for some specific planets (if they have special features)
        // 6) repeat

        if let Some(action) = self.action_queue.next_action()
            && !self.accept_death
        {
            match action {
                ExplorerAction::AskNeighbours => {
                    self.action_queue.push_back(action);
                    match self.send_to_orchestrator(ExplorerToOrchestrator::NeighborsRequest {
                        explorer_id: self.explorer_id,
                        current_planet_id: self.planet_id,
                    }) {
                        Ok(_) => {
                            // if the sending is successful change the state to WaitingForNeighbours
                            self.set_state(ExplorerState::WaitingForNeighbours);

                            log_message!(
                                ActorType::Explorer,
                                self.explorer_id,
                                ActorType::Orchestrator,
                                0u32, // L'ID dell'orchestrator è convenzionalmente 0
                                EventType::MessageExplorerToOrchestrator,
                                "neighbors request sent";
                                "planet_id" => self.planet_id.to_string()
                            );
                        }
                        Err(err) => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                EventType::MessageExplorerToOrchestrator,
                                Channel::Error,
                                warning_payload!(
                                    "NeighborsRequest not sent",
                                    err.to_string(),
                                    "execute_ai_action()";
                                    "explorer data" => format!("{:?}", self)
                                ),
                            ).emit();
                        }
                    }
                }
                ExplorerAction::AskSupportedResources => {
                    // push back the action
                    self.action_queue.push_back(action);

                    // skip the action if the supported resources are already known
                    if let Some(info) = self.get_planet_info(self.planet_id) {
                        if info.get_basic_resources().is_some() {
                            return;
                        }
                    }

                    match self.send_to_planet(ExplorerToPlanet::SupportedResourceRequest {
                        explorer_id: self.explorer_id,
                    }) {
                        Ok(_) => {
                            // if the sending was successful change the state to WaitingForSupportedResources
                            self.set_state(ExplorerState::WaitingForSupportedResources);

                            log_message!(
                                ActorType::Explorer,
                                self.explorer_id,
                                ActorType::Planet,
                                self.planet_id,
                                EventType::MessageExplorerToPlanet,
                                "supported resource request sent"
                            );
                        }
                        Err(err) => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                Some(Participant::new(ActorType::Planet, self.planet_id)),
                                EventType::MessageExplorerToPlanet,
                                Channel::Error,
                                warning_payload!(
                                    "SupportedResourceRequest not sent",
                                    err.to_string(),
                                    "execute_ai_action()";
                                    "explorer data" => format!("{:?}", self)
                                ),
                            ).emit();
                        }
                    }
                }
                ExplorerAction::AskSupportedCombinations => {
                    // push back the action
                    self.action_queue.push_back(action);

                    // skip the action if the complex resources are already known
                    if let Some(info) = self.get_planet_info(self.planet_id) {
                        if info.get_complex_resources().is_some() {
                            return;
                        }
                    }

                    match self.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
                        explorer_id: self.explorer_id,
                    }) {
                        Ok(_) => {
                            // if the sending was successful change the state to WaitingForSupportedCombinations
                            self.set_state(ExplorerState::WaitingForSupportedCombinations);

                            log_message!(
                                ActorType::Explorer,
                                self.explorer_id,
                                ActorType::Planet,
                                self.planet_id,
                                EventType::MessageExplorerToPlanet,
                                "supported combination request sent"
                            );
                        }
                        Err(err) => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                Some(Participant::new(ActorType::Planet, self.planet_id)),
                                EventType::MessageExplorerToPlanet,
                                Channel::Error,
                                warning_payload!(
                                    "SupportedCombinationRequest not sent",
                                    err.to_string(),
                                    "execute_ai_action()";
                                    "explorer data" => format!("{:?}", self)
                                ),
                            ).emit();
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

                            log_message!(
                                ActorType::Explorer,
                                self.explorer_id,
                                ActorType::Planet,
                                self.planet_id,
                                EventType::MessageExplorerToPlanet,
                                "available energy cell request sent"
                            );
                        }
                        Err(err) => {
                            LogEvent::new(
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                Some(Participant::new(ActorType::Planet, self.planet_id)),
                                EventType::MessageExplorerToPlanet,
                                Channel::Error,
                                warning_payload!(
                                    "AvailableEnergyCellRequest not sent",
                                    err.to_string(),
                                    "execute_ai_action()";
                                    "explorer data" => format!("{:?}", self)
                                ),
                            ).emit();
                        }
                    }
                }
                ExplorerAction::GenerateOrCombine => {
                    // IMPORTANT continue to generate/combine till the explorer can

                    // if the topology isn't fully discovered simply generate/combine the useful resources
                    // otherwise:
                    // check what and how many resources the explorer has
                    // check what and how many resources are needed to complete the goal -> see the dependency graph of the resources
                    // maybe check what resources can be obtained from other planets in a possible path
                    // choose the resource based on the things written above
                    // generate/combine it

                    self.action_queue.push_back(action);

                    if self.energy_cells > 0 {
                        if let Some(resource) = self.decide_resource_action() {
                            match resource {
                                ResourceType::Basic(basic_resource) => match basic_resource {
                                    BasicResourceType::Oxygen => {
                                        generate_resource_request(self, BasicResourceType::Oxygen, false);
                                    }
                                    BasicResourceType::Hydrogen => {
                                        generate_resource_request(self, BasicResourceType::Hydrogen, false);
                                    }
                                    BasicResourceType::Carbon => {
                                        generate_resource_request(self, BasicResourceType::Carbon, false);
                                    }
                                    BasicResourceType::Silicon => {
                                        generate_resource_request(self, BasicResourceType::Silicon, false);
                                    }
                                },
                                ResourceType::Complex(complex_resource) => match complex_resource {
                                    ComplexResourceType::Diamond => {
                                        combine_resource_request(self, ComplexResourceType::Diamond, false);
                                    }
                                    ComplexResourceType::Water => {
                                        combine_resource_request(self, ComplexResourceType::Water, false);
                                    }
                                    ComplexResourceType::Life => {
                                        combine_resource_request(self, ComplexResourceType::Life, false);
                                    }
                                    ComplexResourceType::Robot => {
                                        combine_resource_request(self, ComplexResourceType::Robot, false);
                                    }
                                    ComplexResourceType::Dolphin => {
                                        combine_resource_request(self, ComplexResourceType::Dolphin, false);
                                    }
                                    ComplexResourceType::AIPartner => {
                                        combine_resource_request(self, ComplexResourceType::AIPartner, false);
                                    }
                                },
                            }
                        }
                    }
                }
                ExplorerAction::Move => {
                    // 1st case -> the topology isn't fully discovered yet
                    // check the planets that still need to be visited
                    // choose the best path to visit those planets in the shortest way possible

                    // 2nd case -> the topology is fully discovered
                    // check what and how many resources the explorer has
                    // maybe check what resources can be obtained from other planets in a possible path
                    // choose the best path to achieve the goal

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
                    } else {
                        self.accept_death = true;
                    }

                    let mut next_planet = self.move_queue.next_move();

                    if next_planet == Some(self.planet_id) {
                        next_planet = self.move_queue.next_move();
                    }

                    // Wander instinct UNIVERSAL
                    if next_planet.is_none() {
                        let can_craft_here = self.decide_resource_action().is_some();

                        let stuck_no_energy = can_craft_here && self.energy_cells == 0;

                        let stuck_no_path = !can_craft_here;

                        if stuck_no_energy || stuck_no_path {
                            if let Some(info) = self.topology.get(self.planet_id) {
                                if let Some(neighbours) = info.get_neighbours() {
                                    next_planet = neighbours.iter().next().copied();
                                }
                            }
                        }
                    }

                    // if the explorer has to move somewhere send a TravelToPlanetRequest
                    if let Some(target_planet) = next_planet {
                        if self.topology.contains(target_planet) {
                            match self.send_to_orchestrator(
                                ExplorerToOrchestrator::TravelToPlanetRequest {
                                    explorer_id: self.explorer_id,
                                    current_planet_id: self.planet_id,
                                    dst_planet_id: target_planet,
                                },
                            ) {
                                Ok(_) => {
                                    self.set_state(ExplorerState::Traveling);

                                    log_message!(
                                        ActorType::Explorer,
                                        self.explorer_id,
                                        ActorType::Orchestrator,
                                        0u32,
                                        EventType::MessageExplorerToOrchestrator,
                                        "travel to planet request sent";
                                        "target_planet" => target_planet.to_string()
                                    );
                                }
                                Err(err) => {
                                    self.move_queue.clear();

                                    LogEvent::new(
                                        Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                        Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                        EventType::MessageExplorerToOrchestrator,
                                        Channel::Error,
                                        warning_payload!(
                                            "TravelToPlanetRequest not sent",
                                            err.to_string(),
                                            "execute_ai_action()";
                                            "target_planet" => target_planet.to_string(),
                                            "explorer data" => format!("{:?}", self)
                                        ),
                                    ).emit();
                                }
                            }
                        } else {
                            self.move_queue.clear();
                        }
                    }
                }
            }
        }
    }
}

impl fmt::Debug for Explorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Explorer")
            .field("explorer_id", &self.explorer_id)
            .field("planet_id", &self.planet_id)
            .field(
                "orchestrator_channels",
                &format!(
                    "(RX: {:x}, TX: {:x})",
                    get_receiver_id(&self.orchestrator_channels.0),
                    get_sender_id(&self.orchestrator_channels.1)
                ),
            )
            .field(
                "planet_channels",
                &format!(
                    "(RX: {:x}, TX: {:x})",
                    get_receiver_id(&self.planet_channels.0),
                    get_sender_id(&self.planet_channels.1)
                ),
            )
            .field("topology", &self.topology)
            .field("state", &self.state)
            .field("bag", &self.bag)
            .field("manual_mode", &self.manual_mode)
            .field(
                "buffer_orchestrator_len",
                &self.buffer_orchestrator_msg.len(),
            )
            .field("buffer_planet_len", &self.buffer_planet_msg.len())
            .finish()
    }
}
