use common_game::components::resource::{BasicResource, ComplexResource, GenericResource};
use common_game::protocols::planet_explorer::PlanetToExplorer;
use crate::components::tommy_explorer::{Explorer, ExplorerState};

/// handles all messages from the planet
pub fn handle_message(explorer: &mut Explorer, msg: PlanetToExplorer) -> Result<(), String> {
    match msg {
        PlanetToExplorer::SupportedResourceResponse { resource_list } => {
            update_basic_resources(explorer, resource_list);
            Ok(())
        }
        PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
            update_complex_resources(explorer, combination_list);
            Ok(())
        }
        PlanetToExplorer::GenerateResourceResponse { resource } => {
            put_basic_resource_in_bag(explorer, resource);
            Ok(())
        }
        PlanetToExplorer::CombineResourceResponse { complex_response } => {
            put_complex_resource_in_bag(explorer, complex_response);
            Ok(())
        }
        PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
            explorer.set_energy_cells(available_cells);
            Ok(())
        }
        PlanetToExplorer::Stopped => {
            explorer.set_state(ExplorerState::WaitingToStartExplorerAI);
            Ok(())
        }
    }
}

/// updates the basic resources information in the topology
fn update_basic_resources(
    explorer: &mut Explorer,
    resource_list: std::collections::HashSet<common_game::components::resource::BasicResourceType>,
) {
    if let Some(planet_info) = explorer.get_planet_info_mut(explorer.planet_id()) {
        planet_info.set_basic_resources(resource_list);
    } else {
        println!("[EXPLORER DEBUG] Planet not in topology when updating basic resources");
    }
}

/// updates the complex resources information in the topology
fn update_complex_resources(
    explorer: &mut Explorer,
    combination_list: std::collections::HashSet<common_game::components::resource::ComplexResourceType>,
) {
    if let Some(planet_info) = explorer.get_planet_info_mut(explorer.planet_id()) {
        planet_info.set_complex_resources(combination_list);
    } else {
        println!("[EXPLORER DEBUG] Planet not in topology when updating complex resources");
    }
}

/// puts a basic resource in the explorer bag
pub fn put_basic_resource_in_bag(explorer: &mut Explorer, resource: Option<BasicResource>) {
    if let Some(resource) = resource {
        let new_resource = match resource {
            BasicResource::Oxygen(oxygen) => oxygen.to_generic(),
            BasicResource::Hydrogen(hydrogen) => hydrogen.to_generic(),
            BasicResource::Carbon(carbon) => carbon.to_generic(),
            BasicResource::Silicon(silicon) => silicon.to_generic(),
        };
        explorer.insert_in_bag(new_resource);
    }
}

/// puts a complex resource in the explorer bag
pub fn put_complex_resource_in_bag(
    explorer: &mut Explorer,
    complex_response: Result<ComplexResource, (String, GenericResource, GenericResource)>,
) {
    match complex_response {
        Ok(complex_resource) => {
            let new_resource = match complex_resource {
                ComplexResource::Diamond(diamond) => diamond.to_generic(),
                ComplexResource::Water(water) => water.to_generic(),
                ComplexResource::Life(life) => life.to_generic(),
                ComplexResource::Robot(robot) => robot.to_generic(),
                ComplexResource::Dolphin(dolphin) => dolphin.to_generic(),
                ComplexResource::AIPartner(ai_partner) => ai_partner.to_generic(),
            };
            explorer.insert_in_bag(new_resource);
        }
        Err((err_msg, res1, res2)) => {
            println!("[EXPLORER DEBUG] Error receiving CombineResourceResponse: {}", err_msg);
            // Put the resources back in the bag
            explorer.insert_in_bag(res1);
            explorer.insert_in_bag(res2);
        }
    }
}
