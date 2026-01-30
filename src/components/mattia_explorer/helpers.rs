use crate::components::mattia_explorer::states::ExplorerState;
use crate::components::mattia_explorer::Explorer;
use common_game::protocols::planet_explorer::ExplorerToPlanet;

pub fn gather_info_from_planet(explorer: &mut Explorer) ->Result<(), Box<dyn std::error::Error>> {
    match explorer.state{
        ExplorerState::Surveying { resources, combinations, energy_cells , orch_resource, orch_combination} => {
            if resources{
                explorer.planet_channels.1.send(
                    ExplorerToPlanet::SupportedResourceRequest {explorer_id: explorer.explorer_id}
                )?;
            }
            if combinations{
                explorer.planet_channels.1.send(
                    ExplorerToPlanet::SupportedCombinationRequest {explorer_id: explorer.explorer_id}
                )?;
            }
            if energy_cells{
                explorer.planet_channels.1.send(
                    ExplorerToPlanet::AvailableEnergyCellRequest {explorer_id: explorer.explorer_id}
                )?;
            }
        }
        _ =>{
            //todo log warning, it shouldn't be possible to have a different state, but it is not a critical error
            return Ok(())
        }
    }
    Ok(())
}