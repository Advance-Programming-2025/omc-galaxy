use crate::components::mattia_explorer::Explorer;
use crate::components::mattia_explorer::states::ExplorerState;
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use logging_utils::LoggableActor;
use logging_utils::{log_internal_op, warning_payload};

/// this function takes the explorer, and based on its state sends the
/// correct messages to the planet in order to survey the amount of energy cells,
/// the supported resource and the supported combination
pub fn gather_info_from_planet(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
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
                    })?;
            }
            if combinations {
                log_internal_op!(explorer, "sending SupportedCombinationRequest");
                explorer
                    .planet_channels
                    .1
                    .send(ExplorerToPlanet::SupportedCombinationRequest {
                        explorer_id: explorer.explorer_id,
                    })?;
            }
            if energy_cells {
                log_internal_op!(explorer, "sending AvailableEnergyCellRequest");
                explorer
                    .planet_channels
                    .1
                    .send(ExplorerToPlanet::AvailableEnergyCellRequest {
                        explorer_id: explorer.explorer_id,
                    })?;
            }
        }
        _ => {
            LogEvent::self_directed(
                Participant::new(ActorType::Explorer, explorer.explorer_id),
                EventType::InternalExplorerAction,
                Channel::Warning,
                warning_payload!(
                    "cannot send survey message to planet",
                    "the explorer is not in the state: Surveying",
                    "gather_info_from_planet()"
                ),
            )
            .emit();
            return Ok(());
        }
    }
    Ok(())
}
