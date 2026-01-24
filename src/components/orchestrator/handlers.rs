use common_game::{
    logging::{ActorType, Channel, EventType, LogEvent, Participant},
    protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator},
};
use crossbeam_channel::select;

use crate::{
    components::orchestrator::{Orchestrator, macros::LOG_ACTORS_ACTIVITY},
    debug_println, log_message, log_orch_fn, log_orch_internal, payload,
    utils::Status,
    warning_payload,
};

impl Orchestrator {
    /// Handle the planet messages that are sent through the orchestrator's
    /// communication channels.
    ///
    /// This function serves as an entry point to all the messages that originate
    /// from the planets that need the orchestrator's intervention; no logic is
    /// actually present.
    ///
    /// * `msg` - the message to pass along to other functions
    pub(crate) fn handle_planet_message(
        &mut self,
        msg: PlanetToOrchestrator,
    ) -> Result<(), String> {
        //LOG
        log_orch_fn!(
            "handle_planet_message()";
            "message_type"=>format!("{:?}", msg)
        );
        //LOG

        match msg {
            PlanetToOrchestrator::SunrayAck { planet_id } => {
                debug_println!("SunrayAck from: {}", planet_id);
                
                self.emit_sunray_ack(planet_id);
                //LOG
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "SunrayAck",
                    planet_id
                );
                //LOG
            }
            PlanetToOrchestrator::AsteroidAck { planet_id, rocket } => {
                debug_println!("AsteroidAck from: {}", planet_id);
                //LOG
                log_message!(
                    ActorType::Planet, planet_id,
                    ActorType::Orchestrator, 0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "AsteroidAck",
                    planet_id;
                    "has_rocket"=>rocket.is_some()
                );
                //LOG
                match rocket {
                    Some(_) => {}
                    None => {
                        //If you have the id then surely that planet exist so we can unwrap without worring
                        //TODO it seems fine to me but just to be more precise we could add error handling
                        let sender = &self.planet_channels.get(&planet_id).unwrap().0;

                        //Send KillPlanet message, if it returns Err then the planet it's already killed
                        //TODO we could log this too
                        let _log = sender
                            .send(OrchestratorToPlanet::KillPlanet)
                            .map_err(|_| "Unable to send to planet: {planet_id}");

                        //LOG
                        log_message!(
                            ActorType::Orchestrator, 0u32,
                            ActorType::Planet, planet_id,
                            EventType::MessageOrchestratorToPlanet,
                            "KillPlanet",
                            planet_id;
                            "reason"=>"no rocket to deflect asteroid"
                        );
                        //LOG

                        //Update planet State
                        self.planets_status
                            .write()
                            .unwrap()
                            .insert(planet_id, Status::Dead);
                        //LOG
                        log_orch_internal!({
                            "action"=>"planet status updated to Dead",
                            "planet_id"=>planet_id
                        });
                        //LOG
                        //TODO we need to do a check if some explorer is on that planet
                    }
                }
            }
            // PlanetToOrchestrator::IncomingExplorerResponse { planet_id, res }=>{},
            //TODO at this point this functions don't do anything at all
            PlanetToOrchestrator::InternalStateResponse {
                planet_id,
                planet_state,
            } => {
                //LOG
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "InternalStateResponse",
                    planet_id,
                    planet_state,
                );
                //LOG
            }
            PlanetToOrchestrator::KillPlanetResult { planet_id } => {
                debug_println!("Planet killed: {}", planet_id);
                let event = LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "Message"=>"KillPlanetResult",
                        "planet_id"=>planet_id,
                    ),
                );
                event.emit();
            }
            // PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, res }=>{},
            PlanetToOrchestrator::StartPlanetAIResult { planet_id } => {
                //LOG
                let event = LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"StartPlanetAIResult",
                        "planet_id"=>planet_id
                    ),
                );
                event.emit();
                //LOG
            }
            PlanetToOrchestrator::StopPlanetAIResult { planet_id } => {
                //LOG
                let event = LogEvent::new(
                    Some(Participant::new(ActorType::Planet, planet_id)),
                    Some(Participant::new(ActorType::Orchestrator, 0u32)),
                    EventType::MessagePlanetToOrchestrator,
                    LOG_ACTORS_ACTIVITY,
                    payload!(
                        "message"=>"StopPlanetAIResult",
                        "planet_id"=>planet_id
                    ),
                );
                event.emit();
                //LOG
            }
            PlanetToOrchestrator::Stopped { planet_id } => {
                log_message!(
                    ActorType::Planet,
                    planet_id,
                    ActorType::Orchestrator,
                    0u32,
                    EventType::MessagePlanetToOrchestrator,
                    "Stopped",
                    planet_id
                )
            }
            _ => {
                let event = LogEvent::self_directed(
                    Participant::new(ActorType::Orchestrator, 0u32),
                    EventType::MessagePlanetToOrchestrator,
                    Channel::Warning,
                    warning_payload!(
                        "unhandled planet message",
                        "_",
                        "handle_planet_message()";
                    ),
                );
                event.emit();
            }
        }
        Ok(())
    }

    /// Handle the planet messages that are sent through the orchestrator's
    /// communication channels.
    ///
    /// This function serves as an entry point to all the messages that need the
    /// orchestrator's intervention; no logic is actually present.
    pub fn handle_game_messages(&mut self) -> Result<(), String> {
        //LOG
        log_orch_fn!("handle_game_messages()");
        //LOG
        select! {
            recv(self.receiver_orch_planet)->msg=>{
                let msg_unwraped = match msg{
                    Ok(res)=>res,
                    Err(e)=>{
                        //LOG
                        let event=LogEvent::self_directed(
                            Participant::new(ActorType::Orchestrator, 0u32),
                            EventType::InternalOrchestratorAction,
                            Channel::Warning,
                            warning_payload!(
                                "Cannot receive message from planets",
                                e,
                                "handle_game_messages()"
                            )
                        );
                        event.emit();
                        //LOG
                        return Err("Cannot receive message from planets".to_string())
                    },
                };
                self.handle_planet_message(msg_unwraped)?;
            }
            recv(self.receiver_orch_explorer)->_msg=>{
                //TODO to finish this function
                todo!()
            }
            default=>{}
        }

        Ok(())
    }
}
