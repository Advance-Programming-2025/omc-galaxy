use crate::Orchestrator;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::logging::{ActorType, EventType};
use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use crossbeam_channel::Sender;
use logging_utils::{LoggableActor, log_fn_call, log_message};

impl Orchestrator {
    /// this method gets the sender used by all the "send methods" below
    pub fn get_sender_from_orchestrator_to_explorer(
        &self,
        explorer_id: u32,
    ) -> Result<&Sender<OrchestratorToExplorer>, String> {
        log_fn_call!(
            self,
            "get_sender_from_orchestrator_to_explorer()",
            explorer_id,
        );
        match self.explorer_channels.get(&explorer_id) {
            Some((sender, _)) => Ok(sender),
            None => Err(format!("No sender found for explorer {}", explorer_id)),
        }
    }

    /// sends the StartExplorerAI message
    pub fn send_start_explorer_ai(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_start_explorer_ai()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::StartExplorerAI)
            .map_err(|_| {
                format!(
                    "Failed to send start explorer AI to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "StartExplorerAI",
        );
        //LOG
        Ok(())
    }

    /// sends the ResetExplorerAI message
    pub fn send_reset_explorer_ai(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_reset_explorer_ai()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::ResetExplorerAI)
            .map_err(|_| {
                format!(
                    "Failed to send reset explorer AI to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "ResetExplorerAI",
        );
        //LOG
        Ok(())
    }

    /// sends the StopExplorerAI message
    pub fn send_stop_explorer_ai(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_stop_explorer_ai()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::StopExplorerAI)
            .map_err(|_| {
                format!(
                    "Failed to send stop explorer AI to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "StopExplorerAI",
        );
        //LOG
        Ok(())
    }

    /// sends the KillExplorer message
    pub fn send_kill_explorer_ai(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_kill_explorer_ai()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::KillExplorer)
            .map_err(|_| format!("Failed to send kill explorer to explorer {}", explorer_id))?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "KillExplorer",
        );
        //LOG
        Ok(())
    }

    /// gets the sender to the planet (from the explorer) and sends it with the MoveToPlanet message
    pub fn send_move_to_planet(&mut self, explorer_id: u32, planet_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_move_to_planet()", explorer_id, planet_id,);
        // get the sender from orchestrator to explorer
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        // get the sender from explorer to planet
        let sender_to_new_planet = match self.planet_channels.get(&planet_id) {
            Some((_, explorer_sender)) => Some(explorer_sender.clone()),
            None => None, // sender does not exist
        };

        // send the MoveToPlanet
        sender
            .send(OrchestratorToExplorer::MoveToPlanet {
                sender_to_new_planet,
                planet_id,
            })
            .map_err(|_| {
                format!(
                    "Failed to send move to planet {} to explorer {}",
                    planet_id, explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "MoveToPlanet",
        );
        //LOG
        Ok(())
    }

    /// sends the CurrentPlanetRequest message
    pub fn send_current_planet_request(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_current_planet_request()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::CurrentPlanetRequest)
            .map_err(|_| {
                format!(
                    "Failed to send current planet request to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "CurrentPlanetRequest",
        );
        //LOG
        Ok(())
    }

    /// sends the SupportedResourceRequest message
    pub fn send_supported_resource_request(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_supported_resource_request()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::SupportedResourceRequest)
            .map_err(|_| {
                format!(
                    "Failed to send supported resource request to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "SupportedResourceRequest",
        );
        //LOG
        Ok(())
    }

    /// sends the SupportedCombinationRequest message
    pub fn send_supported_combination_request(&mut self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_supported_combination_request()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::SupportedCombinationRequest)
            .map_err(|_| {
                format!(
                    "Failed to send supported combination request to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "SupportedCombinationRequest",
        );
        //LOG
        Ok(())
    }

    /// sends the GenerateResourceRequest message
    pub fn send_generate_resource_request(
        &self,
        explorer_id: u32,
        to_generate: BasicResourceType,
    ) -> Result<(), String> {
        log_fn_call!(
            self,
            "send_generate_resource_request()",
            explorer_id,
            to_generate,
        );
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::GenerateResourceRequest { to_generate })
            .map_err(|_| {
                format!(
                    "Failed to send generate resource request to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "GenerateResourceRequest",
        );
        //LOG
        Ok(())
    }

    /// sends the CombineResourceRequest message
    pub fn send_combine_resource_request(
        &mut self,
        explorer_id: u32,
        to_combine: ComplexResourceType,
    ) -> Result<(), String> {
        log_fn_call!(
            self,
            "send_combine_resource_request()",
            explorer_id,
            to_combine,
        );
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::CombineResourceRequest {
                to_generate: to_combine,
            })
            .map_err(|_| {
                format!(
                    "Failed to send combine resource request to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "CombineResourceRequest",
        );
        //LOG
        Ok(())
    }

    /// sends the BagContentRequest message
    pub fn send_bag_content_request(&self, explorer_id: u32) -> Result<(), String> {
        log_fn_call!(self, "send_bag_content_request()", explorer_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;

        sender
            .send(OrchestratorToExplorer::BagContentRequest)
            .map_err(|_| {
                format!(
                    "Failed to send bag content request to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "BagContentRequest",
        );
        //LOG
        Ok(())
    }

    /// gets the neighbors and sends them with the NeighborsResponse message
    pub fn send_neighbours_response(
        &mut self,
        explorer_id: u32,
        planet_id: u32,
    ) -> Result<(), String> {
        log_fn_call!(self, "send_neighbors_response()", explorer_id, planet_id,);
        let sender = self.get_sender_from_orchestrator_to_explorer(explorer_id)?;
        // Translate the real planet_id to its matrix index via the lookup table
        let matrix_idx = self
            .galaxy_lookup
            .get(&planet_id)
            .map(|(idx, _)| *idx as usize)
            .ok_or_else(|| format!("planet_id {} not found in galaxy_lookup", planet_id))?;
        // the neighbors are obtained from the galaxy_topology adjacent matrix
        // and translated back to real planet_ids via the reverse lookup
        let neighbors: Vec<u32> = {
            let guard = &self.galaxy_topology;

            guard
                .get(matrix_idx)
                .into_iter() // Handles the Option if the index is out of bounds
                .flat_map(|row| {
                    row.iter().enumerate().filter_map(|(i, &is_connected)| {
                        // only return the real planet_id if the connection exists (true)
                        if is_connected {
                            self.galaxy_reverse_lookup.get(&(i as u32)).copied()
                        } else {
                            None
                        }
                    })
                })
                .collect()
        };

        sender
            .send(OrchestratorToExplorer::NeighborsResponse { neighbors })
            .map_err(|_| {
                format!(
                    "Failed to send neighbors response to explorer {}",
                    explorer_id
                )
            })?;

        //LOG
        log_message!(
            ActorType::Orchestrator,
            0u32,
            ActorType::Explorer,
            explorer_id,
            EventType::MessageOrchestratorToExplorer,
            "NeighborsResponse",
        );
        //LOG
        Ok(())
    }
}
