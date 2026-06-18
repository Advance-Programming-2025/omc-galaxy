use crate::components::tommy_explorer::{Explorer, ExplorerState};

use common_game::components::resource::{BasicResource, ComplexResource, GenericResource};
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::planet_explorer::PlanetToExplorer;
use logging_utils::{log_message, warning_payload};
use crate::components::tommy_explorer::bag::IntoGenericResource;

/// Handles all messages from the planet.
pub fn handle_message(explorer: &mut Explorer, msg: PlanetToExplorer) -> Result<(), String> {
    match msg {
        PlanetToExplorer::SupportedResourceResponse { resource_list } => {
            update_basic_resources(explorer, resource_list);
            explorer.set_state(ExplorerState::Idle);
            Ok(())
        }
        PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
            update_complex_resources(explorer, combination_list);
            explorer.set_state(ExplorerState::Idle);
            Ok(())
        }
        PlanetToExplorer::GenerateResourceResponse { resource } => {
            put_basic_resource_in_bag(explorer, resource);
            explorer.set_state(ExplorerState::Idle);
            Ok(())
        }
        PlanetToExplorer::CombineResourceResponse { complex_response } => {
            put_complex_resource_in_bag(explorer, complex_response);
            explorer.set_state(ExplorerState::Idle);
            Ok(())
        }
        PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
            explorer.set_energy_cells(available_cells);
            explorer.set_state(ExplorerState::Idle);
            Ok(())
        }
        PlanetToExplorer::Stopped => {
            explorer.set_state(ExplorerState::Idle);
            Ok(())
        }
    }
}

/// Updates the basic resources information in the topology.
fn update_basic_resources(
    explorer: &mut Explorer,
    resource_list: std::collections::HashSet<common_game::components::resource::BasicResourceType>,
) {
    if let Some(planet_info) = explorer.get_planet_info_mut(explorer.planet_id()) {
        planet_info.set_basic_resources(resource_list);
        log_message!(
            ActorType::Planet,
            explorer.planet_id,
            ActorType::Explorer,
            explorer.explorer_id,
            EventType::MessagePlanetToExplorer,
            "supported resource response";
        );
    }
}

/// Updates the complex resources information in the topology.
fn update_complex_resources(
    explorer: &mut Explorer,
    combination_list: std::collections::HashSet<
        common_game::components::resource::ComplexResourceType,
    >,
) {
    if let Some(planet_info) = explorer.get_planet_info_mut(explorer.planet_id()) {
        planet_info.set_complex_resources(combination_list);
        log_message!(
            ActorType::Planet,
            explorer.planet_id,
            ActorType::Explorer,
            explorer.explorer_id,
            EventType::MessagePlanetToExplorer,
            "supported combination response";
        );
    }
}

/// Puts a basic resource in the explorer's bag.
pub fn put_basic_resource_in_bag(explorer: &mut Explorer, resource: Option<BasicResource>) {
    if let Some(resource) = resource {
        let new_resource = resource.into_generic_resource();
        explorer.insert_in_bag(new_resource);
        log_message!(
            ActorType::Planet,
            explorer.planet_id,
            ActorType::Explorer,
            explorer.explorer_id,
            EventType::MessagePlanetToExplorer,
            "generate resource response";
            "explorer data"=>format!("{:?}", explorer)
        );
    } else {
        explorer.set_energy_cells(0);
    }
}

/// Puts a complex resource in the explorer's bag.
pub fn put_complex_resource_in_bag(
    explorer: &mut Explorer,
    complex_response: Result<ComplexResource, (String, GenericResource, GenericResource)>,
) {
    match complex_response {
        Ok(complex_resource) => {
            let new_resource = complex_resource.into_generic_resource();
            explorer.insert_in_bag(new_resource);
            log_message!(
                ActorType::Planet,
                explorer.planet_id,
                ActorType::Explorer,
                explorer.explorer_id,
                EventType::MessagePlanetToExplorer,
                "combine resource response";
                "explorer data"=>format!("{:?}", explorer)
            );
        }
        Err((err_msg, res1, res2)) => {
            LogEvent::new(
                Some(Participant::new(ActorType::Planet, explorer.planet_id)),
                Some(Participant::new(ActorType::Explorer, explorer.explorer_id)),
                EventType::MessagePlanetToExplorer,
                Channel::Error,
                warning_payload!(
                    "CombineResourceResponse failed",
                    err_msg,
                    "put_complex_resource_in_bag()";
                    "explorer data"=>format!("{:?}", explorer)
                ),
            ).emit();

            // Put the resources back in the bag
            explorer.insert_in_bag(res1);
            explorer.insert_in_bag(res2);

            explorer.set_energy_cells(0);
        }
    }
}
