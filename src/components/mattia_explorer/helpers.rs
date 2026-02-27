use crate::components::mattia_explorer::Explorer;
use crate::components::mattia_explorer::states::ExplorerState;
use common_game::logging::{ActorType, Channel, EventType, LogEvent, Participant};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use logging_utils::LoggableActor;
use logging_utils::{log_internal_op, warning_payload};

pub fn gather_info_from_planet(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::Surveying {
            resources,
            combinations,
            energy_cells,
            orch_resource,
            orch_combination,
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
            //todo log warning, it shouldn't be possible to have a different state, but it is not a critical error
            LogEvent::self_directed(
                Participant::new(ActorType::Explorer, explorer.explorer_id),
                EventType::InternalExplorerAction,
                Channel::Warning,
                warning_payload!(
                    "cannot send survey message to planet",
                    "_",
                    "gather_info_from_planet()"
                ),
            )
            .emit();
            return Ok(());
        }
    }
    Ok(())
}
