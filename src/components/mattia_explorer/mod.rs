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
use crate::components::mattia_explorer::handlers::{
    combine_resource_request, current_planet_request, generate_resource_request, kill_explorer,
    manage_combine_response, manage_generate_response, manage_supported_combination_response,
    manage_supported_resource_response, move_to_planet, neighbours_response, reset_explorer_ai,
    start_explorer_ai, stop_explorer_ai, supported_combination_request, supported_resource_request,
};
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
use crossbeam_channel::{Receiver, Sender, select};
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
        loop {
            // this way should not panic
            // counter of the time elapsed
            self.time = self.time.wrapping_add(1);

            select! {
                recv(self.orchestrator_channels.0) -> msg_orchestrator => {
                    // receiving from orchestrator channel
                    match msg_orchestrator {
                        Ok(msg) => {
                            //LOG
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
                            //LOG
                            if orch_msg_match_state(&self.state, &msg) {
                                // in this state the explorer can process the command
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
                                        if let Err(err)=kill_explorer(self){
                                            LogEvent::new(
                                                Some(Participant::new(ActorType::Explorer,self.explorer_id)),
                                                Some(Participant::new(ActorType::Orchestrator,0u32)),
                                                EventType::MessageExplorerToOrchestrator,
                                                Channel::Warning,
                                                warning_payload!(
                                                    "kill_explorer() generated an error",
                                                    err,
                                                    "mattia_explorer::run()"
                                                )
                                            ).emit();
                                        }
                                        // exiting the loop
                                        return Ok(())
                                    }
                                    OrchestratorToExplorer::MoveToPlanet{ sender_to_new_planet, planet_id } => {
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
                                    OrchestratorToExplorer::GenerateResourceRequest{ to_generate } => {
                                        generate_resource_request(self, to_generate, true)
                                    }
                                    OrchestratorToExplorer::CombineResourceRequest{ to_generate } => {
                                        combine_resource_request(self, to_generate, true)
                                    }
                                    OrchestratorToExplorer::BagContentRequest => {
                                        // return a vector of resource types
                                        self.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: self.explorer_id, bag_content: self.bag.to_resource_types()}).map_err(|e| e.to_string())
                                    }
                                    OrchestratorToExplorer::NeighborsResponse{ neighbors } => {
                                        neighbours_response(self, neighbors);
                                        Ok(()) //todo fare che neighbours response restituisca un risultato?
                                    }
                                };
                                if let Err(err)=ris{
                                    LogEvent::self_directed(
                                        Participant::new(ActorType::Explorer,self.explorer_id),
                                        EventType::InternalExplorerAction,
                                        Channel::Warning,
                                        warning_payload!(
                                            "a handler of a OrchestratorToExplorer message returned an error",
                                            err,
                                            "mattia_explorer::run()"
                                        )
                                    ).emit();
                                }
                            } else {
                                // if the explorer can't process the command now it pushes it in the buffer
                                self.buffer_orchestrator_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            LogEvent::self_directed(
                                Participant::new(ActorType::Explorer,self.explorer_id),
                                EventType::InternalExplorerAction,
                                Channel::Error,
                                warning_payload!(
                                    "Fatal Error: receiving channel from orchestrator disconnected",
                                    err,
                                    "mattia_explorer::run()"
                                )
                            ).emit();
                            return Err(err.to_string());
                        }
                    }
                },
                //even if the channel id disconnected we need to wait the kill msg to terminate the execution
                recv(self.planet_channels.0) -> msg_planet => {
                    match msg_planet {
                        Ok(msg) => {
                            //LOG
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
                            //LOG
                            if planet_msg_match_state(&self.state, &msg) {
                                // the message can be processed now
                                let ris = match msg {
                                    PlanetToExplorer::SupportedResourceResponse{ resource_list } => {
                                        manage_supported_resource_response(self, resource_list)
                                    }
                                    PlanetToExplorer::SupportedCombinationResponse{ combination_list } => {
                                        manage_supported_combination_response(self, combination_list)
                                    }
                                    PlanetToExplorer::GenerateResourceResponse{ resource } => {
                                        manage_generate_response(self, resource)
                                    }
                                    PlanetToExplorer::CombineResourceResponse{ complex_response } => {
                                        manage_combine_response(self, complex_response)
                                    }
                                    PlanetToExplorer::AvailableEnergyCellResponse{ available_cells } => { //todo aggiungere un handler specifico?
                                        match self.state{
                                            ExplorerState::Surveying {resources,combinations,energy_cells:true,orch_resource,orch_combination}=>{
                                                if let Some(planet_info)=self.topology_info.get_mut(&self.planet_id){
                                                    planet_info.update_charge_rate(available_cells, self.time, self.ai_data.params.charge_rate_alpha,self.explorer_id);
                                                } // it is impossible that the explorer doesn't have the planet in its topoplogy
                                                if !resources && !combinations{
                                                    self.state = ExplorerState::Idle;
                                                }
                                                else{
                                                    self.state = ExplorerState::Surveying {
                                                        resources,
                                                        combinations,
                                                        energy_cells:false,
                                                        orch_resource,
                                                        orch_combination,
                                                    };
                                                }
                                            }
                                            _ => {
                                                LogEvent::new(
                                                    Some(Participant::new(ActorType::Planet, self.planet_id)),
                                                    Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                                    EventType::MessagePlanetToExplorer,
                                                    Channel::Warning,
                                                    warning_payload!(
                                                        "received AvailableEnergyCellResponse while not in Surveying state\
                                                        this should not happen",
                                                        "",
                                                        "Explorer::run()";
                                                        "explorer state"=>format!("{:?}", self.state)
                                                    )
                                                ).emit()
                                            }
                                        }
                                        Ok(())

                                    }
                                    PlanetToExplorer::Stopped => {
                                        self.state = ExplorerState::Idle;
                                        Ok(())
                                    }
                                };
                                if let Err(err)=ris{
                                    LogEvent::self_directed(
                                        Participant::new(ActorType::Explorer, self.explorer_id),
                                        EventType::InternalExplorerAction,
                                        Channel::Warning,
                                        warning_payload!(
                                            "a handler of a PlanetToExplorer message returned an error",
                                            err,
                                            "mattia_explorer::run()"
                                        )
                                    ).emit();
                                }
                            } else {
                                // the explorer is not in a state that can process messages
                                self.buffer_planet_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            // in this case the explorer probably will receive a kill message
                            LogEvent::new(
                                Some(Participant::new(ActorType::Planet, self.planet_id)),
                                Some(Participant::new(ActorType::Explorer, self.explorer_id)),
                                EventType::MessagePlanetToExplorer,
                                Channel::Error,
                                warning_payload!(
                                    "receiving channel from planet disconnected",
                                    err,
                                    "mattia_explorer::run()"
                                )
                            ).emit();
                        }
                    }
                }
                default => {
                    log_internal_op!(
                        self,
                        "action"=>"no message in the channels",
                        "explorer_state"=>format!("{:?}", self.state)
                    );
                    if !self.buffer_planet_msg.is_empty() || !self.buffer_orchestrator_msg.is_empty() {
                        // processing buffered messages
                        if let Err(err)=manage_buffer_msg(self){
                            LogEvent::self_directed(
                                Participant::new(ActorType::Explorer, self.explorer_id),
                                EventType::InternalExplorerAction,
                                Channel::Warning,
                                warning_payload!(
                                    "message_buffer_handler returned an error",
                                    err,
                                    "mattia_explorer::run()"
                                )
                            ).emit();
                        }
                        //this is because manage_buffer_msg could possibly set the explorer state to killed
                        if self.state==ExplorerState::Killed{
                            return Ok(())
                        }
                    }
                    else if !self.manual_mode && self.state==ExplorerState::Idle{
                        //running the ai if the explorer is not in manual mode
                        ai_core_function(self).map_err(|e| e.to_string())?;
                    }
                }
            }
            //in order to reduce busy waiting
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
