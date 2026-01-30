use crate::components::mattia_explorer::helpers::gather_info_from_planet;
use crate::components::mattia_explorer::resource_management::ToGeneric;
use crate::components::mattia_explorer::states::ExplorerState;
use crate::components::mattia_explorer::{Explorer, PlanetInfo};
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, GenericResource};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::Sender;
use std::collections::HashSet;

// this function put the explorer in the condition to receive messages (idle state),
// it is called when the explorer receives the StartExplorerAI message
pub fn start_explorer_ai(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::StartExplorerAIResult {explorer_id: explorer.explorer_id}) {
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Start explorer AI result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending start explorer AI result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function resets the topology known by the explorer,
// it is called when the explorer receives the ResetExplorerAI message
//todo not really sure but maybe i need to change this
pub fn reset_explorer_ai(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::ResetExplorerAIResult {explorer_id: explorer.explorer_id}
    ){
        Ok(_) => {
            // TODO reset anche dell'inventario?
            explorer.topology_info.clear();
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Reset explorer AI result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending reset explorer AI result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function put the explorer in the condition to wait for a StartExplorerAI message (WaitingToStartExplorerAI state),
// it is called when the explorer receives the StopExplorerAI message
pub fn stop_explorer_ai(explorer: &mut Explorer)->Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::StopExplorerAIResult {explorer_id: explorer.explorer_id}
    ){
        Ok(_) => {
            explorer.state = ExplorerState::WaitingToStartExplorerAI;
            println!("[EXPLORER DEBUG] Stop explorer AI result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending stop explorer AI result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function puts the explorer in the Killed state waiting for the thread to be killed
pub fn kill_explorer(explorer: &mut Explorer) ->Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::KillExplorerResult {explorer_id: explorer.explorer_id}
    ){
        Ok(_) => {
            explorer.state = ExplorerState::Killed;
            println!("[EXPLORER DEBUG] Kill explorer result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending kill explorer result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function sets the sender_to_planet of the explorer struct
pub fn move_to_planet(
    explorer: &mut Explorer,
    sender_to_new_planet: Option<Sender<ExplorerToPlanet>>,
    planet_id: ID,
) -> Result<(), Box<dyn std::error::Error>> {
    explorer.state = ExplorerState::Idle;
    match sender_to_new_planet {
        Some(sender) => {
            explorer.planet_channels.1 = sender;
            explorer.planet_id = planet_id; //todo rimuovere next_planet_id
            println!("[EXPLORER DEBUG] Sender channel set correctly");
            //todo logs
            Ok(())
        }
        None => { //the explorer cannot move
            println!("[EXPLORER DEBUG] Sender channel is None.");
            //todo logs
            Err("Sender channel is None.".into())
        }
    }
}

// this function sends the current planet id to the orchestrator
pub fn current_planet_request(explorer: &mut Explorer)->Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::CurrentPlanetResult {
            explorer_id: explorer.explorer_id,
            planet_id: explorer.planet_id
        }
    ){
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Current planet result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending current planet result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function sends the basic resources supported by the current planet to the orchestrator
// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
// response and then send it back to the orchestrator)
pub fn supported_resource_request(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.topology_info.get(&explorer.planet_id){
        Some(planet_info) => {
            match &planet_info.basic_resources{
                Some(basic_resources) => {
                    explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedResourceResult {
                        explorer_id: explorer.explorer_id,
                        supported_resources: basic_resources.clone(),
                    })?;
                }
                None => {
                    //this should not happen
                    //todo logs
                    match explorer.state{
                        ExplorerState::Idle =>{
                            explorer.state = ExplorerState::Surveying{
                                resources: true,
                                combinations: false,
                                energy_cells: false,
                                orch_resource: true,
                                orch_combination: false,
                            };
                            gather_info_from_planet(explorer)?;
                        }
                        _=>{
                            //todo logs this should not happen
                        }
                    }
                }
            }
        }
        None => {
            //this should not happen
            //todo logs
            match explorer.state{
                ExplorerState::Idle =>{
                    explorer.state = ExplorerState::Surveying{
                        resources: true,
                        combinations: true,
                        energy_cells: true,
                        orch_resource: true,
                        orch_combination: false,
                    };
                    gather_info_from_planet(explorer)?;
                }
                _=>{
                    //todo logs this should not happen
                }
            }
        }
    }
    Ok(())
}

// this function sends the complex resources supported by the current planet to the orchestrator
// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
// response and then send it back to the orchestrator)
pub fn supported_combination_request(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.topology_info.get(&explorer.planet_id){
        Some(planet_info) => {
            match &planet_info.complex_resources{
                Some(complex_resource) => {
                    explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedCombinationResult {
                        explorer_id: explorer.explorer_id,
                        combination_list: complex_resource.clone(),
                    })?;
                }
                None => {
                    //this should not happen
                    //todo logs
                    match explorer.state{
                        ExplorerState::Idle =>{
                            explorer.state = ExplorerState::Surveying{
                                resources: false,
                                combinations: true,
                                energy_cells: false,
                                orch_resource: false,
                                orch_combination: true,
                            };
                            gather_info_from_planet(explorer)?;
                        }
                        _=>{
                            //todo logs this should not happen
                        }
                    }
                }
            }
        }
        None => {
            //this should not happen
            //todo logs
            match explorer.state{
                ExplorerState::Idle =>{
                    explorer.state = ExplorerState::Surveying{
                        resources: true,
                        combinations: true,
                        energy_cells: true,
                        orch_resource: false,
                        orch_combination: true,
                    };
                    gather_info_from_planet(explorer)?;
                }
                _=>{
                    //todo logs this should not happen
                }
            }
        }
    }
    Ok(())
}

// this function sends the GenerateResourceRequest, waits for the planet response, and,
// if successful puts the resource in the bag
pub fn generate_resource_request(explorer: &mut Explorer, to_generate: BasicResourceType, to_orchestrator:bool) -> Result<(), Box<dyn std::error::Error>> {
    explorer.state = ExplorerState::GeneratingResource {orchestrator_response:true};
    explorer.planet_channels.1.send(ExplorerToPlanet::GenerateResourceRequest {
        explorer_id: explorer.explorer_id,
        resource: to_generate,
    })?;
    Ok(())
}



// this function sends the CombineResourceRequest, waits for the planet response, and,
// if successful puts the resource in the bag
pub fn combine_resource_request(explorer: &mut Explorer, to_generate: ComplexResourceType) -> Result<(), Box<dyn std::error::Error>> {
    explorer.state = ExplorerState::CombiningResources {orchestrator_response:true};
    let complex_resource_req = match to_generate {
        //provide the requested resources from the bag for each combination
        ComplexResourceType::Diamond => explorer.bag.make_diamond_request(),
        ComplexResourceType::Water => explorer.bag.make_water_request(),
        ComplexResourceType::Life => explorer.bag.make_life_request(),
        ComplexResourceType::Robot => explorer.bag.make_robot_request(),
        ComplexResourceType::Dolphin => explorer.bag.make_dolphin_request(),
        ComplexResourceType::AIPartner => explorer.bag.make_ai_partner_request(),
    };
    match complex_resource_req {
        Ok(complex_resource_req) => {
            explorer.planet_channels.1.send(ExplorerToPlanet::CombineResourceRequest {
                explorer_id: explorer.explorer_id,
                msg: complex_resource_req,
            })?;
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error generating complex resource request {}",err);
            //todo logs
            explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::CombineResourceResponse {
                explorer_id:explorer.explorer_id,
                generated: Err(err),
            })?;
            Ok(())//this could happen and it is totally fine
        }
    }
}

// this function updates the neighbours of the current planet
pub fn neighbours_response(explorer: &mut Explorer, neighbors: Vec<ID>) {
    explorer.state = ExplorerState::Idle;
    for &neighbour in &neighbors {
        explorer
            .topology_info
            .entry(neighbour)
            .or_insert(PlanetInfo::new(explorer.time));
    }
    //todo logs
    match explorer.topology_info.get_mut(&explorer.planet_id){
        Some(planet_info) => {
            planet_info.neighbours = Some(neighbors.into_iter().collect());
        }
        None => {
            explorer.topology_info.insert(
                explorer.planet_id,
                PlanetInfo::new(explorer.time)
            );
        }
    }
}

pub fn manage_supported_resource_response(
    explorer: &mut Explorer,
    resource_list:HashSet<BasicResourceType>
) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::Surveying {resources:true ,combinations,energy_cells,orch_resource,orch_combination}=>{
            match explorer.topology_info.get_mut(&explorer.planet_id) {
                Some(planet_info) => {
                    planet_info.basic_resources = Some(resource_list.clone());
                }
                None => {
                    explorer.topology_info.insert(
                        explorer.planet_id,
                        PlanetInfo::new(explorer.time)
                    );
                }
            }
            if orch_resource{
                explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedResourceResult {
                    explorer_id: explorer.explorer_id,
                    supported_resources: resource_list
                })?;
            }
            if !combinations && !energy_cells {
                explorer.state = ExplorerState::Idle;
            }
            else{
                explorer.state = ExplorerState::Surveying {
                    resources:false,
                    combinations,
                    energy_cells,
                    orch_resource:false,
                    orch_combination,
                };
            }
        }
        _ => {
            //todo this should not happen but it is not a problem
        }
    }
    Ok(())
}

pub fn manage_supported_combination_response(
    explorer: &mut Explorer,
    combination_list:HashSet<ComplexResourceType>,
)-> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::Surveying {resources ,combinations:true,energy_cells,orch_resource,orch_combination}=>{
            match explorer.topology_info.get_mut(&explorer.planet_id) {
                Some(planet_info) => {
                    planet_info.complex_resources = Some(combination_list.clone());
                }
                None => {
                    explorer.topology_info.insert(
                        explorer.planet_id,
                        PlanetInfo::new(explorer.time)
                    );
                }
            }
            if orch_combination{
                explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedCombinationResult {
                    explorer_id: explorer.explorer_id,
                    combination_list
                })?;
            }
            if !resources && !energy_cells {
                explorer.state = ExplorerState::Idle;
            }
            else{
                explorer.state = ExplorerState::Surveying {
                    resources,
                    combinations:false,
                    energy_cells,
                    orch_resource,
                    orch_combination:false,
                };
            }
        }
        _ => {
            //todo this should not happen but it is not a problem
        }
    }
    Ok(())
}

pub fn manage_generate_response(
    explorer: &mut Explorer,
    resource: Option<BasicResource>,
)-> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::GeneratingResource {orchestrator_response}=>{
            match resource {
                Some(resource) => {
                    explorer.bag.insert(resource.res_to_generic());
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::GenerateResourceResponse {
                                explorer_id: explorer.explorer_id,
                                generated: Ok(())
                            }
                        )?;
                    }
                }
                None => {
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::GenerateResourceResponse {
                                explorer_id: explorer.explorer_id,
                                generated: Err("Cannot generate resource".to_string())
                            }
                        )?;
                    }
                }
            }
            explorer.state = ExplorerState::Idle;
        }
        _ => {
            //todo this should non happen
        }
    }
    Ok(())
}
pub fn manage_combine_response(
    explorer: &mut Explorer,
    complex_response:  Result<ComplexResource, (String, GenericResource, GenericResource)>
) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::CombiningResources {orchestrator_response}=>{
            match complex_response {
                Ok(complex_resource) => {
                    explorer.bag.insert(complex_resource.res_to_generic());
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::CombineResourceResponse {
                                explorer_id:explorer.explorer_id,
                                generated: Ok(())
                            }
                        )?;
                    }
                }
                Err((err,r1, r2))=>{
                    //todo logs
                    explorer.bag.insert(r1);
                    explorer.bag.insert(r2);
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::CombineResourceResponse {
                                explorer_id: explorer.explorer_id,
                                generated: Err("Cannot combine resource".to_string())
                            }
                        )?;
                    }
                }
            }
            explorer.state = ExplorerState::Idle;
        }
        _ => {
            //todo this should non happen
        }
    }
    Ok(())
}
