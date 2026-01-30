mod bag;
mod resource_management;
mod states;
mod buffers;
mod handlers;
mod helpers;

use crate::components::mattia_explorer::buffers::manage_buffer_msg;
use crate::components::mattia_explorer::handlers::{combine_resource_request, current_planet_request, generate_resource_request, kill_explorer, manage_combine_response, manage_generate_response, manage_supported_combination_response, manage_supported_resource_response, move_to_planet, neighbours_response, reset_explorer_ai, start_explorer_ai, stop_explorer_ai, supported_combination_request, supported_resource_request};
use crate::components::mattia_explorer::resource_management::ToGeneric;
use crate::components::mattia_explorer::states::{orch_msg_match_state, planet_msg_match_state, ExplorerState};
use crate::components::tommy_explorer::bag::Bag;
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{select, Receiver, Sender};
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet, VecDeque};

// struct that contains some
struct PlanetInfo {
    basic_resources: Option<HashSet<BasicResourceType>>,
    complex_resources: Option<HashSet<ComplexResourceType>>,
    neighbours: Option<HashSet<ID>>,
}


// this is the struct of the explorer
pub struct Explorer {
    explorer_id: u32,
    planet_id: u32, //I assume that the travel isn't instant, so I put an Option we should manage the case the planet explodes
    next_planet_id: u32, // needed if the travelToPlanet doesn't go well
    orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
    ),
    planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    topology_info: HashMap<ID, PlanetInfo>,
    state: ExplorerState,
    bag: Bag,
    energy_cells: u32, // of the current planet
    buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>, // orchestrator messages that the explorer cannot respond to immediately
    buffer_planet_msg: VecDeque<PlanetToExplorer>, // planet messages that the explorer cannot respond to immediately
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
        energy_cells: u32, // useful in the case in which the explorer starts mid-game
    ) -> Self {
        let mut starting_topology_info = HashMap::new();
        starting_topology_info.insert(
            planet_id,
            PlanetInfo {
                basic_resources: None,
                complex_resources: None,
                neighbours: None,
            },
        );
        Self {
            explorer_id,
            planet_id,
            next_planet_id: planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: explorer_to_planet_channels,
            topology_info: starting_topology_info,
            state: ExplorerState::WaitingToStartExplorerAI,
            bag: Bag::new(),
            energy_cells,
            buffer_orchestrator_msg: VecDeque::new(),
            buffer_planet_msg: VecDeque::new(),
        }
    }

    // getter function for the id
    pub fn id(&self) -> u32 {
        self.explorer_id
    }

    // the explorer loop
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // every iteration the explorer receives messages from both planet and orchestrator channels,
        // then it behaves based on the message received, if the message received and the explorer state
        // do not match together the message is pushed into the corresponding buffer, and it will be read
        // when the explorer will be in an "Idle" state
        loop {
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
                                        combine_resource_request(self, to_generate)?;
                                    }
                                    OrchestratorToExplorer::BagContentRequest => {
                                        // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                                        self.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: self.explorer_id, bag_content: self.bag.to_resource_types()})?;
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
                            println!("[EXPLORER DEBUG] Error in receiving the orchestrator message: {}", err);
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
                                                self.energy_cells = available_cells;
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
                            println!("[EXPLORER DEBUG] Error in receiving the planet message: {}", err);
                        }
                    }
                }
                default => {
                    if !self.buffer_planet_msg.is_empty() || !self.buffer_orchestrator_msg.is_empty() {
                        manage_buffer_msg(self)?;
                        if self.state==ExplorerState::Killed{
                            return Ok(())
                        }
                    }
                    else{
                        //todo ai
                    }
                }
            }
        }
    }
}


