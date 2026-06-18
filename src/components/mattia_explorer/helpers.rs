use crate::components::mattia_explorer::states::ExplorerState;
use crate::components::mattia_explorer::Explorer;
use common_game::logging::{ActorType, EventType, LogEvent, Participant};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use logging_utils::LoggableActor;
use logging_utils::log_internal_op;

/// this function takes the explorer, and based on its state sends the
/// correct messages to the planet in order to survey the amount of energy cells,
/// the supported resource and the supported combination
pub(super) fn gather_info_from_planet(explorer: &mut Explorer) -> Result<(), String> {
    match explorer.state {
        ExplorerState::Surveying {
            resources,
            combinations,
            energy_cells,
            orch_resource: _orch_resource,
            orch_combination: _orch_combination,
        } => {
            if resources {
                log_internal_op!(explorer, "sending SupportedResourceRequest");
                explorer
                    .planet_channels
                    .1
                    .send(ExplorerToPlanet::SupportedResourceRequest {
                        explorer_id: explorer.explorer_id,
                    })
                    .map_err(|e| format!("Error sending SupportedResourceRequest: {}", e))?;
            }
            if combinations {
                log_internal_op!(explorer, "sending SupportedCombinationRequest");
                explorer
                    .planet_channels
                    .1
                    .send(ExplorerToPlanet::SupportedCombinationRequest {
                        explorer_id: explorer.explorer_id,
                    })
                    .map_err(|e| format!("Error sending SupportedCombinationRequest: {}", e))?;
            }
            if energy_cells {
                log_internal_op!(explorer, "sending AvailableEnergyCellRequest");
                explorer
                    .planet_channels
                    .1
                    .send(ExplorerToPlanet::AvailableEnergyCellRequest {
                        explorer_id: explorer.explorer_id,
                    })
                    .map_err(|e| format!("Error sending AvailableEnergyCellRequest: {}", e))?;
            }
        }
        _ => {
            return Err(format!(
                "gather_info_from_planet(): explorer not in Surveying state (actual: {:?})",
                explorer.state
            ));
        }
    }
    Ok(())
}
