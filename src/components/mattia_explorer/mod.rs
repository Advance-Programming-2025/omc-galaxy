mod bag;
mod resource_management;
mod states;
mod buffers;
mod handlers;
mod helpers;
mod explorer_ai;
mod planet_info;

use crate::components::mattia_explorer::bag::Bag;
use crate::components::mattia_explorer::buffers::manage_buffer_msg;
use crate::components::mattia_explorer::explorer_ai::{ai_core_function, ai_data};
use crate::components::mattia_explorer::handlers::{combine_resource_request, current_planet_request, generate_resource_request, kill_explorer, manage_combine_response, manage_generate_response, manage_supported_combination_response, manage_supported_resource_response, move_to_planet, neighbours_response, reset_explorer_ai, start_explorer_ai, stop_explorer_ai, supported_combination_request, supported_resource_request};
use crate::components::mattia_explorer::planet_info::PlanetInfo;
use crate::components::mattia_explorer::resource_management::ToGeneric;
use crate::components::mattia_explorer::states::{orch_msg_match_state, planet_msg_match_state, ExplorerState};
use common_game::components::resource::ResourceType;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{select, Receiver, Sender};
use std::cmp::PartialEq;
use std::collections::{HashMap, VecDeque};

// this is the struct of the explorer
pub struct Explorer {
    explorer_id: ID,
    planet_id: ID, //I assume that the travel isn't instant, so I put an Option we should manage the case the planet explodes
    orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
    ),
    planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    topology_info: HashMap<ID, PlanetInfo>,
    state: ExplorerState,
    bag: Bag,
    buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>, // orchestrator messages that the explorer cannot respond to immediately
    buffer_planet_msg: VecDeque<PlanetToExplorer>, // planet messages that the explorer cannot respond to immediately
    time: u64,
    ai_data: ai_data,
    current_planet_neighbors_update:bool,
    manual_mode: bool,
}

impl Explorer {
    // at creation, an Explorer should be connected to Orchestrator and the starting Planet
    pub fn new(
        explorer_id: u32,
        planet_id: u32,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
        ),
        explorer_to_planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    ) -> Self {
        let mut starting_topology_info = HashMap::new();
        starting_topology_info.insert(
            planet_id,
            PlanetInfo::new(0),
        );
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
            ai_data: ai_data::new(),
            current_planet_neighbors_update: false,
            manual_mode: true,
        }
    }

    // getter function for the id
    pub fn id(&self) -> u32 {
        self.explorer_id
    }

    //generic getter for planet_info
    pub fn get_planet_info(&self, planet_id: ID) -> Option<&PlanetInfo> {
        self.topology_info.get(&planet_id)
    }
    pub fn get_planet_info_mut(&mut self, planet_id: ID) -> Option<&mut PlanetInfo> {
        self.topology_info.get_mut(&planet_id)
    }
    //current planet getter
    pub fn get_current_planet_info(&self) -> Result<&PlanetInfo, &'static str> {
        match self.get_planet_info(self.planet_id){
            Some(info) => Ok(info),
            None => {
                Err("Planet not found")
            }
        }
    }
    pub fn get_current_planet_info_mut(&mut self) -> Result<&mut PlanetInfo, &'static str> {
        match self.get_planet_info_mut(self.planet_id){
            Some(info) => Ok(info),
            None => {
                Err("Planet not found")
            }
        }
    }

    // the explorer loop
    pub fn run(&mut self) -> Result<(), String> {
        // every iteration the explorer receives messages from both planet and orchestrator channels,
        // then it behaves based on the message received, if the message received and the explorer state
        // do not match together the message is pushed into the corresponding buffer, and it will be read
        // when the explorer will be in an "Idle" state
        loop {
            //this way should not panic
            self.time = self.time.wrapping_add(1);

            select! {
                recv(self.orchestrator_channels.0) -> msg_orchestrator => {
                    match msg_orchestrator {
                        Ok(msg) => {
                            if orch_msg_match_state(&self.state, &msg) {
                                match msg {
                                    OrchestratorToExplorer::StartExplorerAI => {
                                        start_explorer_ai(self)?;
                                    }
                                    OrchestratorToExplorer::ResetExplorerAI => {
                                        reset_explorer_ai(self)?;
                                    }
                                    OrchestratorToExplorer::StopExplorerAI => {
                                        stop_explorer_ai(self)?;
                                    }
                                    OrchestratorToExplorer::KillExplorer => {
                                        // TODO this action should be preemptive
                                        kill_explorer(self)?;
                                        return Ok(())
                                    }
                                    OrchestratorToExplorer::MoveToPlanet{ sender_to_new_planet, planet_id } => {
                                        // TODO use the planet_id variable (common crate v3)
                                        move_to_planet(self, sender_to_new_planet, planet_id)?;
                                    }
                                    OrchestratorToExplorer::CurrentPlanetRequest => {
                                        current_planet_request(self)?;
                                    }
                                    OrchestratorToExplorer::SupportedResourceRequest => {
                                        supported_resource_request(self)?;
                                    }
                                    OrchestratorToExplorer::SupportedCombinationRequest => {
                                        supported_combination_request(self)?;
                                    }
                                    OrchestratorToExplorer::GenerateResourceRequest{ to_generate } => {
                                        generate_resource_request(self, to_generate, true)?;
                                    }
                                    OrchestratorToExplorer::CombineResourceRequest{ to_generate } => {
                                        combine_resource_request(self, to_generate, true)?;
                                    }
                                    OrchestratorToExplorer::BagContentRequest => {
                                        // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                                        self.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: self.explorer_id, bag_content: self.bag.to_resource_types()}).map_err(|e| e.to_string())?;
                                    }
                                    OrchestratorToExplorer::NeighborsResponse{ neighbors } => {
                                        neighbours_response(self, neighbors);
                                    }
                                }
                            } else {
                                self.buffer_orchestrator_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            //todo logs
                        }
                    }
                },
                //even if the channel id disconnected we need to wait the kill msg to terminate the execution
                recv(self.planet_channels.0) -> msg_planet => {
                    match msg_planet {
                        Ok(msg) => {
                            if planet_msg_match_state(&self.state, &msg) {
                                match msg {
                                    PlanetToExplorer::SupportedResourceResponse{ resource_list } => {
                                        manage_supported_resource_response(self, resource_list)?;
                                    }
                                    PlanetToExplorer::SupportedCombinationResponse{ combination_list } => {
                                        manage_supported_combination_response(self, combination_list)?;
                                    }
                                    PlanetToExplorer::GenerateResourceResponse{ resource } => {
                                        manage_generate_response(self, resource)?;
                                    }
                                    PlanetToExplorer::CombineResourceResponse{ complex_response } => {
                                        manage_combine_response(self, complex_response)?;
                                    }
                                    PlanetToExplorer::AvailableEnergyCellResponse{ available_cells } => {
                                        match self.state{
                                            ExplorerState::Surveying {resources,combinations,energy_cells:true,orch_resource,orch_combination}=>{
                                                match self.topology_info.get_mut(&self.explorer_id){
                                                    Some(planet_info) => {
                                                        planet_info.update_charge_rate(available_cells, self.time);
                                                    }
                                                    None => {
                                                        //this should not happen
                                                    }
                                                }
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
                                                //todo logs this should not happen
                                            }
                                        }

                                    }
                                    PlanetToExplorer::Stopped => {
                                        // TODO gestire in base all'ai dell'explorer
                                        self.state = ExplorerState::Idle;
                                    }
                                }
                            } else {
                                self.buffer_planet_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            //todo logs
                        }
                    }
                }
                default => {
                    if !self.buffer_planet_msg.is_empty() || !self.buffer_orchestrator_msg.is_empty() {
                        manage_buffer_msg(self).map_err(|e| e.to_string())?;
                        //this is because manage_buffer_msg could possibly set the explorer state to killed
                        if self.state==ExplorerState::Killed{
                            return Ok(())
                        }
                    }
                    else if !self.manual_mode{
                        ai_core_function(self).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
}

use std::fmt;
use logging_utils::{get_receiver_id, get_sender_id};

impl fmt::Debug for Explorer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Explorer")
            .field("explorer_id", &self.explorer_id)
            .field("planet_id", &self.planet_id)
            .field("orchestrator_channels", &format!(
                "(RX: {:x}, TX: {:x})",
                get_receiver_id(&self.orchestrator_channels.0),
                get_sender_id(&self.orchestrator_channels.1)
            ))
            .field("planet_channels", &format!(
                "(RX: {:x}, TX: {:x})",
                get_receiver_id(&self.planet_channels.0),
                get_sender_id(&self.planet_channels.1)
            ))
            .field("topology_info", &self.topology_info)
            .field("state", &self.state)
            .field("bag", &self.bag)
            .field("time", &self.time)
            .field("current_planet_neighbors_update", &self.current_planet_neighbors_update)
            .field("manual_mode", &self.manual_mode)
            // Possiamo omettere i buffer se sono troppo lunghi o includerli normalmente
            .field("buffer_orchestrator_len", &self.buffer_orchestrator_msg.len())
            .field("buffer_planet_len", &self.buffer_planet_msg.len())
            .finish()
    }
}