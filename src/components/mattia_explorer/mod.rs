pub mod ai_params;
mod bag;
mod buffers;
mod explorer_ai;
mod handlers;
mod helpers;
mod planet_info;
mod resource_management;
mod states;
mod tests;

use crate::components::mattia_explorer::ai_params::AiParams;
use crate::components::mattia_explorer::bag::Bag;
use crate::components::mattia_explorer::buffers::manage_buffer_msg;
use crate::components::mattia_explorer::explorer_ai::{AiData, ai_core_function};
use crate::components::mattia_explorer::handlers::{combine_resource_request, current_planet_request, generate_resource_request, kill_explorer, manage_available_energy_cell_response, manage_combine_response, manage_generate_response, manage_supported_combination_response, manage_supported_resource_response, move_to_planet, neighbours_response, reset_explorer_ai, start_explorer_ai, stop_explorer_ai, supported_combination_request, supported_resource_request};
use crate::components::mattia_explorer::planet_info::PlanetInfo;
use crate::components::mattia_explorer::states::{
    ExplorerState, orch_msg_match_state, planet_msg_match_state,
};
use common_game::components::resource::ResourceType;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use std::collections::{HashMap, VecDeque};

/// struct of the explorer data
pub (super) struct Explorer {
    explorer_id: ID, //explorer id
    planet_id: ID,   //current planet id
    orchestrator_channels: (  // orchestrator channels
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
    ),
    planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),  //planet channels
    topology_info: HashMap<ID, PlanetInfo>, //hashmap containing the information of every planet
    state: ExplorerState,
    bag: Bag,
    buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>, // orchestrator messages that the explorer cannot respond to immediately
    buffer_planet_msg: VecDeque<PlanetToExplorer>, // planet messages that the explorer cannot respond to immediately
    time: u64,  // time measured in tick used by the explorer ai
    ai_data: AiData,  // data needed by the explorer ai
    current_planet_neighbors_update: bool,  //flag that states if the neighbors need update
    manual_mode: bool,  //flag that states if the explorer is in manual mode
}

impl Explorer {
    // at creation, an Explorer should be connected to Orchestrator and the starting Planet
    pub (super) fn new(
        explorer_id: u32,
        planet_id: u32,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
        ),
        explorer_to_planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    ) -> Self {
        Self::with_params(
            explorer_id,
            planet_id,
            explorer_to_orchestrator_channels,
            explorer_to_planet_channels,
            AiParams::default(),
        )
    }

    /// Creates an Explorer with custom AI parameters (for ML tuning)
    pub (super) fn with_params(
        explorer_id: u32,
        planet_id: u32,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
        ),
        explorer_to_planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
        ai_params: AiParams,
    ) -> Self {
        log_fn_call!(dir
            ActorType::Explorer,
            explorer_id,
            "Explorer::new()",
            explorer_id,
            planet_id;
            "explorer_to_orchestrator_channels" => format!("({}, {})", get_receiver_id(&explorer_to_orchestrator_channels.0), get_sender_id(&explorer_to_orchestrator_channels.1)),
            "explorer_to_planet_channels"=>format!("({}, {})", get_receiver_id(&explorer_to_planet_channels.0), get_sender_id(&explorer_to_planet_channels.1)),
        );
        let mut starting_topology_info = HashMap::new();
        starting_topology_info.insert(planet_id, PlanetInfo::new(0));
        Self {
            explorer_id,
            planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: explorer_to_planet_channels,
            topology_info: starting_topology_info,
            state: ExplorerState::Idle,
            bag: Bag::new(),
            buffer_orchestrator_msg: VecDeque::new(),
            buffer_planet_msg: VecDeque::new(),
            time: 1,
            ai_data: AiData::new(ai_params),
            current_planet_neighbors_update: false,
            manual_mode: false,
        }
    }

    /// getter function for the id
    pub fn id(&self) -> u32 {
        self.explorer_id
    }

    ///generic getters for planet_info
    fn get_planet_info(&self, planet_id: ID) -> Option<&PlanetInfo> {
        self.topology_info.get(&planet_id)
    }
    fn get_planet_info_mut(&mut self, planet_id: ID) -> Option<&mut PlanetInfo> {
        self.topology_info.get_mut(&planet_id)
    }
    /// current planet getters
    fn get_current_planet_info(&self) -> Result<&PlanetInfo, &'static str> {
        match self.get_planet_info(self.planet_id) {
            Some(info) => Ok(info),
            None => Err("Planet not found"),
        }
    }
    fn get_current_planet_info_mut(&mut self) -> Result<&mut PlanetInfo, &'static str> {
        match self.get_planet_info_mut(self.planet_id) {
            Some(info) => Ok(info),
            None => Err("Planet not found"),
        }
    }

    /// the explorer main loop
    ///
    /// every iteration the explorer receives messages from both planet and orchestrator channels,
    /// then it behaves based on the message received, if the message received and the explorer state
    /// do not match together the message is pushed into the corresponding buffer, and it will be read
    /// when the explorer will be in an "Idle" state
    pub fn run(&mut self) -> Result<(), String> {
        // Flag to track whether the planet channel is still alive
        let mut planet_channel_active = true;

        loop {
            self.time = self.time.wrapping_add(1);

            // Represents which channel fired and carries the received message (or disconnect error)
            enum Selected {
                Orchestrator(Result<OrchestratorToExplorer, crossbeam_channel::RecvError>),
                Planet(Result<PlanetToExplorer, crossbeam_channel::RecvError>),
                None,
            }

            let selected = {
                let mut sel = crossbeam_channel::Select::new();
                let orch_idx = sel.recv(&self.orchestrator_channels.0);
                let planet_idx;
                if planet_channel_active  {
                    planet_idx=Some(sel.recv(&self.planet_channels.0))
                }
                else{
                    planet_idx=None
                }

                match sel.try_select() {
                    // No message ready on any channel
                    Err(_) => Selected::None,
                    Ok(oper) if oper.index() == orch_idx => {
                        // Consume the operation and capture the message before dropping sel
                        Selected::Orchestrator(oper.recv(&self.orchestrator_channels.0))
                    }
                    Ok(oper) if planet_idx.is_some_and(|pi| oper.index() == pi) => {
                        //consume the message only if the planet is alive
                        Selected::Planet(oper.recv(&self.planet_channels.0))
                    }
                    // Unreachable: sel only contains the two indices above
                    Ok(_) => Selected::None,
                }
            };

            // processing the new message
            match selected {
                Selected::None => {
                    // processing buffed messages
                    log_internal_op!(
                    self,
                    "action"   => "no message in the channels",
                    "explorer_state" => format!("{:?}", self.state)
                );

                    if !self.buffer_planet_msg.is_empty() || !self.buffer_orchestrator_msg.is_empty() {
                        if let Err(err) = manage_buffer_msg(self) {
                            LogEvent::self_directed(
                                Participant::new(ActorType::Explorer, self.explorer_id),
                                EventType::InternalExplorerAction,
                                Channel::Warning,
                                warning_payload!(
                                "message_buffer_handler returned an error",
                                err,
                                "mattia_explorer::run()"
                            ),
                            )
                                .emit();
                        }
                        if self.state == ExplorerState::Killed {
                            return Ok(());
                        }
                    } else if !self.manual_mode && self.state == ExplorerState::Idle {
                        //buffers empty and not in manual mode => running ai
                        ai_core_function(self).map_err(|e| e.to_string())?;
                    }
                }

                Selected::Orchestrator(msg_result) => {
                    match msg_result {
                        //processing orchestrator message
                        Ok(msg) => {
                            log_message!(
                            ActorType::Orchestrator, 0u32,
                            ActorType::Explorer,     self.explorer_id,
                            EventType::MessageOrchestratorToExplorer,
                            "message received";
                            "msg"          => format!("{:?}", msg),
                            "explorer data" => format!("{:?}", self)
                        );

                            if orch_msg_match_state(&self.state, &msg) {
                                let ris = match msg {
                                    OrchestratorToExplorer::StartExplorerAI => {
                                        start_explorer_ai(self)
                                    }
                                    OrchestratorToExplorer::ResetExplorerAI => {
                                        reset_explorer_ai(self)
                                    }
                                    OrchestratorToExplorer::StopExplorerAI => {
                                        stop_explorer_ai(self)
                                    }
                                    OrchestratorToExplorer::KillExplorer => {
                                        if let Err(err) = kill_explorer(self) {
                                            LogEvent::new(
                                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                                Some(Participant::new(ActorType::Orchestrator, 0u32)),
                                                EventType::MessageExplorerToOrchestrator,
                                                Channel::Warning,
                                                warning_payload!(
                                                "kill_explorer() generated an error",
                                                err,
                                                "mattia_explorer::run()"
                                            ),
                                            ).emit();
                                        }
                                        // exiting the loop
                                        return Ok(());
                                    }
                                    OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id } => {
                                        // A new planet is being assigned: re-enable the planet channel
                                        planet_channel_active = true;
                                        move_to_planet(self, sender_to_new_planet, planet_id)
                                    }
                                    OrchestratorToExplorer::CurrentPlanetRequest => {
                                        current_planet_request(self)
                                    }
                                    OrchestratorToExplorer::SupportedResourceRequest => {
                                        supported_resource_request(self)
                                    }
                                    OrchestratorToExplorer::SupportedCombinationRequest => {
                                        supported_combination_request(self)
                                    }
                                    OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                                        generate_resource_request(self, to_generate, true)
                                    }
                                    OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                                        combine_resource_request(self, to_generate, true)
                                    }
                                    OrchestratorToExplorer::BagContentRequest => {
                                        self.orchestrator_channels
                                            .1
                                            .send(ExplorerToOrchestrator::BagContentResponse {
                                                explorer_id: self.explorer_id,
                                                bag_content: self.bag.to_resource_types(),
                                            })
                                            .map_err(|e| e.to_string())
                                    }
                                    OrchestratorToExplorer::NeighborsResponse { neighbors } => {
                                        neighbours_response(self, neighbors);
                                        Ok(())
                                    }
                                };

                                if let Err(err) = ris {
                                    LogEvent::self_directed(
                                        Participant::new(ActorType::Explorer, self.explorer_id),
                                        EventType::InternalExplorerAction,
                                        Channel::Warning,
                                        warning_payload!(
                                        "a handler of a OrchestratorToExplorer message returned an error",
                                        err,
                                        "mattia_explorer::run()"
                                    ),
                                    )
                                        .emit();
                                }
                            } else {
                                // Explorer is not in a state that can process this message: buffer it
                                self.buffer_orchestrator_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            LogEvent::self_directed(
                                Participant::new(ActorType::Explorer, self.explorer_id),
                                EventType::InternalExplorerAction,
                                Channel::Error,
                                warning_payload!(
                                "Fatal Error: receiving channel from orchestrator disconnected",
                                err,
                                "mattia_explorer::run()"
                            ),
                            )
                                .emit();
                            return Err(err.to_string());
                        }
                    }
                }

                Selected::Planet(msg_result) => {
                    match msg_result {
                        Ok(msg) => {
                            log_message!(
                            ActorType::Planet,   self.planet_id,
                            ActorType::Explorer, self.explorer_id,
                            EventType::MessagePlanetToExplorer,
                            "message received";
                            "msg"           => format!("{:?}", msg),
                            "explorer data" => format!("{:?}", self)
                        );

                            if planet_msg_match_state(&self.state, &msg) {
                                let ris = match msg {
                                    PlanetToExplorer::SupportedResourceResponse { resource_list } => {
                                        manage_supported_resource_response(self, resource_list)
                                    }
                                    PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
                                        manage_supported_combination_response(self, combination_list)
                                    }
                                    PlanetToExplorer::GenerateResourceResponse { resource } => {
                                        manage_generate_response(self, resource)
                                    }
                                    PlanetToExplorer::CombineResourceResponse { complex_response } => {
                                        manage_combine_response(self, complex_response)
                                    }
                                    PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                                        manage_available_energy_cell_response(self, available_cells)
                                    }
                                    PlanetToExplorer::Stopped => {
                                        self.state = ExplorerState::Idle;
                                        Ok(())
                                    }
                                };

                                if let Err(err) = ris {
                                    LogEvent::self_directed(
                                        Participant::new(ActorType::Explorer, self.explorer_id),
                                        EventType::InternalExplorerAction,
                                        Channel::Warning,
                                        warning_payload!(
                                        "a handler of a PlanetToExplorer message returned an error",
                                        err,
                                        "mattia_explorer::run()"
                                    ),
                                    )
                                        .emit();
                                }
                            } else {
                                // Explorer is not in a state that can process this message: buffer it
                                self.buffer_planet_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            // Planet has died: disable its channel to prevent a spin loop
                            // while waiting for the Kill message from the orchestrator
                            LogEvent::new(
                                Some(Participant::new(ActorType::Planet, self.planet_id)),
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                EventType::MessagePlanetToExplorer,
                                Channel::Error,
                                warning_payload!(
                                "receiving channel from planet disconnected",
                                err,
                                "mattia_explorer::run()"
                            ),
                            )
                                .emit();
                            // Channel will not be added to Select on the next iteration avoiding
                            // spin loop
                            planet_channel_active = false;
                        }
                    }
                }
            }

            sleep(Duration::from_millis(20));
        }
    }
}

use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use logging_utils::{
    LoggableActor, get_receiver_id, get_sender_id, log_fn_call, log_internal_op, log_message,
    warning_payload,
};
use std::fmt;
use std::thread::sleep;
use std::time::Duration;

impl fmt::Debug for Explorer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            .field("topology_info", &self.topology_info)
            .field("state", &self.state)
            .field("bag", &self.bag)
            .field("time", &self.time)
            .field(
                "current_planet_neighbors_update",
                &self.current_planet_neighbors_update,
            )
            .field("manual_mode", &self.manual_mode)
            // Possiamo omettere i buffer se sono troppo lunghi o includerli normalmente
            .field(
                "buffer_orchestrator_len",
                &self.buffer_orchestrator_msg.len(),
            )
            .field("buffer_planet_len", &self.buffer_planet_msg.len())
            .finish()
    }
}

impl LoggableActor for Explorer {
    fn actor_type(&self) -> ActorType {
        ActorType::Explorer
    }

    fn actor_id(&self) -> u32 {
        self.explorer_id
    }
}
