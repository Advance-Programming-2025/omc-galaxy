use std::collections::{HashMap, HashSet};
use std::fmt::Display;
//use std::sync::{mpsc, LockResult};
use crossbeam_channel::{Sender, Receiver, select};
// use std::time::SystemTime;
use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::BasicResourceType::Carbon;
use common_game::components::resource::ComplexResourceType::Diamond;
use common_game::components::resource::ComplexResourceType;
use common_game::components::resource::{BasicResource, BasicResourceType, Combinator, ComplexResource, ComplexResourceRequest, Dolphin, Generator, GenericResource};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{
    ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
};

use crate::components::energy_stacks::stacks::{initialize_free_cell_stack, push_free_cell, push_charged_cell, peek_charged_cell_index, get_free_cell_index, get_charged_cell_index};
use common_game::protocols::messages::OrchestratorToPlanet::Asteroid;
use common_game::logging::{ActorType, Channel, Payload, EventType, LogEvent};
use common_game::logging::EventType::{MessageOrchestratorToPlanet, MessagePlanetToExplorer, MessagePlanetToOrchestrator};
use crossbeam_channel::internal::SelectHandle;
use log::max_level;
///////////////////////////////////////////////////////////////////////////////////////////
// CrabRave Constructor
///////////////////////////////////////////////////////////////////////////////////////////
const RCV_MSG_LOG_CHNL: Channel=Channel::Info; // change this 2 in order to change the channel of the logs
const ACK_MSG_LOG_CHNL: Channel=Channel::Info;
#[macro_export]
macro_rules! log_msg {
    ($event:expr, $channel:expr) => {
        {
            match $channel{
                Channel::Info => {log::info!("{}", $event);},
                Channel::Debug => {log::debug!("{}", $event);},
                Channel::Error=>{log::error!("{}", $event);},
                Channel::Trace=>{log::trace!("{}", $event);},
                Channel::Warning=>{log::warn!("{}", $event);},
            }
        }
    };
}
pub struct CrabRaveConstructor;

impl CrabRaveConstructor {
    pub fn new(
        id: u32,
        orchestrator_channels: (
            Receiver<OrchestratorToPlanet>,
            Sender<PlanetToOrchestrator>,
        ),
        explorer_channels: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        let (planet_type, ai, gen_rules, comb_rules, orchestrator_channels, explorer_channels) = (
            PlanetType::C,
            AI::new(),
            vec![Carbon],
            vec![Diamond],
            orchestrator_channels,
            explorer_channels,
        );
        //LOG
        let mut payload= Payload::new();
        payload.insert(String::from("gen_rules"), gen_rules.iter().map(|x| x.res_to_string()+", ").collect());
        payload.insert(String::from("comb_rules"), comb_rules.iter().map(|x| x.res_to_string()+", ").collect());
        payload.insert("Message".to_string(), "New planet created".to_string());
        //it would be nice to log if the orchestrator is connected but i don't think is possible neither with std::sync nor with crossbeam_channel
        // without actually send and receive something
        //LOG
        let new_planet = Planet::new(
            id,
            planet_type,
            Box::new(ai),
            gen_rules,
            comb_rules,
            orchestrator_channels,
            explorer_channels,
        )?;
        //LOG
        let event= LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, id.to_string(), EventType::MessageOrchestratorToPlanet, Channel::Info, payload);
        log::info!("{}", event);
        //LOG
        Ok(new_planet)
    }
}

///////////////////////////////////////////////////////////////////////////////////////////
// PlanetAI
///////////////////////////////////////////////////////////////////////////////////////////

// REMEMBER -> N_CELLS is in energy_stacks.rs mod

pub struct AI;

impl AI {
    fn new() -> Self {
        // the cell stack needs to be started here
        // otherwise it would get reset when the AI
        // gets stopped
        //LOG
        let mut payload = Payload::new();
        payload.insert(String::from("Message"), String::from("New AI created"));
        let event= LogEvent::new(ActorType::Planet, 0u64, ActorType::Planet, "0".to_string(), EventType::InternalPlanetAction, Channel::Info, payload);
        log::info!("{}", event);
        //LOG
        initialize_free_cell_stack(); // REVIEW rimuovere se si sceglie la vecchia implementazione
        Self
    }
}

impl PlanetAI for AI {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        match msg {
            OrchestratorToPlanet::InternalStateRequest => {
                //LOG
                let mut payload = Payload::new();
                payload.insert("PlanetState".to_string(), format!("{:?}",PlanetState::to_dummy(&state)));
                payload.insert(String::from("Message"), String::from("Internal state request"));
                let event= LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, state.id().clone().to_string(), EventType::MessageOrchestratorToPlanet, RCV_MSG_LOG_CHNL, payload);
                log_msg!(event, RCV_MSG_LOG_CHNL);
                let mut payload_ris=Payload::new();
                payload_ris.insert(String::from("ACK Response of InternalStateRequest"), format!("planet_id: {:?}, planet_state: {:?}", state.id().clone(),PlanetState::to_dummy(&state)));
                let event_ris=LogEvent::new(ActorType::Planet, state.id().clone(), ActorType::Orchestrator, "0".to_string(), MessagePlanetToOrchestrator, ACK_MSG_LOG_CHNL, payload_ris);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id().clone(),
                    planet_state: PlanetState::to_dummy(&state)
                                                 // timestamp: SystemTime::now(),
                })
            }
            OrchestratorToPlanet::Sunray(sunray) => {
                // for i in 0..N_CELLS {
                //     // non ho trovato un modo per ottenere il vettore, l'unico modo penso sia quello di ciclare
                //     if !state.cell(i).is_charged() {
                //         state.cell_mut(i).charge(sunray);
                //         return Some(PlanetToOrchestrator::SunrayAck {
                //             planet_id: state.id(),
                //             // timestamp: SystemTime::now(),
                //         });
                //     }
                // }
                // None
                let mut payload_ris = Payload::new();
                let mut ris=None;
                if let Some(idx) = get_free_cell_index() {
                    state.cell_mut(idx as usize).charge(sunray);
                    push_charged_cell(idx);

                    payload_ris.insert("Message".to_string(), "SunrayAck".to_string());
                    payload_ris.insert(String::from("Result"), String::from("EnergyCell charged"));
                    payload_ris.insert(String::from("EnergyCell index"), format!("{}", idx));
                    payload_ris.insert(String::from("Response data"), format!("planet_id: {}", state.id().clone()));

                    ris = Some(PlanetToOrchestrator::SunrayAck {
                        planet_id: state.id().clone(),
                    })
                }
                else{
                    payload_ris.insert("Response to".to_string(), "Sunray".to_string());
                    payload_ris.insert(String::from("Result"), String::from("No free cell found"));
                }

                //LOG
                let mut payload = Payload::new();
                payload.insert(String::from("Message"), String::from("Sunray"));
                let event=LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, state.id().clone().to_string(), EventType::MessageOrchestratorToPlanet, RCV_MSG_LOG_CHNL, payload);
                log_msg!(event, RCV_MSG_LOG_CHNL);
                let event_ris=LogEvent::new(ActorType::Planet, state.id().clone(), ActorType::Orchestrator, "0".to_string(), MessagePlanetToOrchestrator, ACK_MSG_LOG_CHNL, payload_ris);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                ris
            }
            _ => {
                //LOG TODO add more information
                let mut payload = Payload::new();
                payload.insert(String::from("Message"), "message behaviour not defined".to_string());
                let event=LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, state.id().clone().to_string(), EventType::MessageOrchestratorToPlanet, Channel::Error, payload);
                log_msg!(event, Channel::Error);
                None
            },
        }
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: id } => {
                // restituisce la prima cell carica, se c'è
                // DO NOT REMOVE -> the following commented lines are the old implementation, so do not remove them till the final decision of the implementation
                // for i in 0..N_CELLS {
                //     if state.cell(i).is_charged() {
                //         return Some(PlanetToExplorer::AvailableEnergyCellResponse {
                //             available_cells: i as u32,
                //         });
                //     }
                // }
                // None

                let mut payload_ris = Payload::new();

                let mut ris=None;
                if let Some(idx) = peek_charged_cell_index() {
                    payload_ris.insert("Message".to_string(), "AvailableEnergyCellResponse".to_string());
                    payload_ris.insert(String::from("Result"), "EnergyCell available".to_string());
                    payload_ris.insert(String::from("EnergyCell index"), format!("{}", idx));
                    ris= Some(PlanetToExplorer::AvailableEnergyCellResponse {
                        available_cells: idx
                    })
                }
                else{
                    payload_ris.insert("Response to".to_string(), "AvailableEnergyCellRequest".to_string());
                    payload_ris.insert(String::from("Result"), "No EnergyCell available".to_string());
                }

                //LOG
                let mut payload = Payload::new();
                payload.insert(String::from("Message"), String::from("Available EnergyCell request"));
                let event=LogEvent::new(ActorType::Explorer, id, ActorType::Planet, state.id().clone().to_string(), EventType::MessageExplorerToPlanet, RCV_MSG_LOG_CHNL, payload);
                let event_ris=LogEvent::new(ActorType::Planet, state.id().clone(), ActorType::Orchestrator, "0".to_string(), MessagePlanetToOrchestrator, ACK_MSG_LOG_CHNL, payload_ris);
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                ris
            }
            ExplorerToPlanet::SupportedResourceRequest { explorer_id: id } => {
                //LOG
                let mut payload = Payload::new();
                let mut payload_ris=Payload::new();
                payload.insert(String::from("Message"), String::from("Supported resource request"));
                payload_ris.insert(String::from("Message"), "Supported resource response".to_string());
                payload_ris.insert("Result".to_string(), format!("resource_list: {:?})", generator.all_available_recipes()));
                let event=LogEvent::new(ActorType::Explorer, id, ActorType::Planet, state.id().clone().to_string(), EventType::MessageExplorerToPlanet, RCV_MSG_LOG_CHNL, payload);
                let event_ris=LogEvent::new(ActorType::Planet, state.id().clone(), ActorType::Explorer, id.to_string(), EventType::MessagePlanetToExplorer, ACK_MSG_LOG_CHNL, payload_ris);
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes()
                })
            }
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id: id } => {
                //LOG
                let mut payload = Payload::new();
                let mut payload_ris=Payload::new();
                payload.insert(String::from("Message"), String::from("Supported combination request"));
                payload_ris.insert(String::from("Message"), "Supported combination response".to_string());
                payload_ris.insert("Result".to_string(), format!("combination_list: {:?}", combinator.all_available_recipes()));
                let event=LogEvent::new(ActorType::Explorer, id, ActorType::Planet, state.id().clone().to_string(), EventType::MessageExplorerToPlanet, RCV_MSG_LOG_CHNL, payload);
                let event_ris=LogEvent::new(ActorType::Planet, state.id().clone(), ActorType::Explorer, id.to_string(), MessagePlanetToExplorer, ACK_MSG_LOG_CHNL, payload_ris);
                log_msg!(event, RCV_MSG_LOG_CHNL);
                log_msg!(event_ris, ACK_MSG_LOG_CHNL);
                //LOG
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes() 
                })
            }

            //TODO use explorer_id to send the gen resource to correct Explorer
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id,
                resource,
            } => {
                //LOG
                let mut payload = Payload::new();
                payload.insert("Message".to_string(), "Generate resource request".to_string());
                payload.insert("requested resource".to_string(), format!("{:?}", resource));
                let event=LogEvent::new(ActorType::Explorer, explorer_id, ActorType::Planet, state.id().clone().to_string(), EventType::MessageExplorerToPlanet, Channel::Info, payload);
                log::info!("{}", event);
                //LOG
                //TODO add log for the response
                let requested_resource = resource;
                // controllo se c'è una cella carica
                // DO NOT REMOVE -> the following commented lines are the old implementation, so do not remove them till the final decision of the implementation
                // let cell_idx = (0..N_CELLS).find(|&i| state.cell(i).is_charged());
                // if let Some(cell_idx) = cell_idx {
                if let Some(cell_idx) = get_charged_cell_index() {
                    // se c'è una cella carica
                    // ottengo la cella da passare al generator
                    //TODO add a detailed log (debug)
                    let cell = state.cell_mut(cell_idx as usize); // TODO remove the "as usize" if using the old implementation of getting the index of energy cell
                    // pattern matching per generare la risorsa corretta
                    let generated_resource = match requested_resource {
                        BasicResourceType::Carbon => {
                            generator.make_carbon(cell).map(BasicResource::Carbon)
                        } // make_ controlla già se la risorsa è presente in generator
                        BasicResourceType::Silicon => {
                            generator.make_silicon(cell).map(BasicResource::Silicon)
                        }
                        BasicResourceType::Oxygen => {
                            generator.make_oxygen(cell).map(BasicResource::Oxygen)
                        }
                        BasicResourceType::Hydrogen => {
                            generator.make_hydrogen(cell).map(BasicResource::Hydrogen)
                        }
                    };
                    // verifico il risultato di state.generator.make...
                    match generated_resource {
                        Ok(resource) => {
                            push_free_cell(cell_idx);
                            return Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(resource),
                            });
                        }
                        Err(err) => {
                            push_charged_cell(cell_idx);
                            println!("{}", err);
                        }
                    }
                } else {
                    //TODO change this in a error log
                    println!("No available cell found"); // non dovrebbe accadere, si spera che l'explorer chieda se ce ne è una libera
                }
                Some(PlanetToExplorer::GenerateResourceResponse {
                    //TA: TODO ritorno come ho fatto o direttamente None?
                    //DDC: io terrei cosi', esplicita il fatto che questo sia un caso
                    //di errore ma comunque atteso. dipende anche dalla spec
                    resource: None,
                })
            }
            //TODO use explorer_id to send the gen resource to correct Explorer
            ExplorerToPlanet::CombineResourceRequest { explorer_id, msg: resource } => { //renamed msg to resouce to be more consistent with generateresourcerequest
                // searching the index of the first free cell
                // DO NOT REMOVE -> the following commented lines are the old implementation, so do not remove them till the final decision of the implementation
                // let cell_idx = (0..N_CELLS).find(|&i| state.cell(i).is_charged());
                // if let Some(cell_idx) = cell_idx {
                //LOG
                let mut payload = Payload::new();
                payload.insert("Message".to_string(), "Combine resource request".to_string());
                payload.insert("requested complex resource".to_string(), format!("{:?}", resource));
                let event=LogEvent::new(ActorType::Explorer, explorer_id, ActorType::Planet, state.id().clone().to_string(), EventType::MessageExplorerToPlanet, Channel::Info, payload);
                log::info!("{}", event);
                //LOG
                // TODO add log of the response
                if let Some(cell_idx) = get_charged_cell_index() {
                    let cell = state.cell_mut(cell_idx as usize); // TODO remove the "as usize" if using the old implementation of getting the index of energy cell
                    // pattern matching to generate the correct resource
                    let complex_resource: Result<ComplexResource, (String, GenericResource, GenericResource)> = match resource {
                        ComplexResourceRequest::Water(r1, r2) => combinator
                            .make_water(r1, r2, cell)
                            .map(ComplexResource::Water)
                            .map_err(|(e, r1, r2)| { (e , GenericResource::BasicResources(BasicResource::Hydrogen(r1)), GenericResource::BasicResources(BasicResource::Oxygen(r2)))}),
                        ComplexResourceRequest::Diamond(r1, r2) => combinator
                            .make_diamond(r1, r2, cell)
                            .map(ComplexResource::Diamond)
                            .map_err(|(e, r1, r2)| { (e , GenericResource::BasicResources(BasicResource::Carbon(r1)), GenericResource::BasicResources(BasicResource::Carbon(r2)))}),
                        ComplexResourceRequest::Life(r1, r2) => combinator
                            .make_life(r1, r2, cell)
                            .map(ComplexResource::Life)
                            .map_err(|(e, r1, r2)| { (e , GenericResource::ComplexResources(ComplexResource::Water(r1)), GenericResource::BasicResources(BasicResource::Carbon(r2)))}),

                        ComplexResourceRequest::Robot(r1, r2) => combinator
                            .make_robot(r1, r2, cell)
                            .map(ComplexResource::Robot)
                            .map_err(|(e, r1, r2)| { (e , GenericResource::BasicResources(BasicResource::Silicon(r1)), GenericResource::ComplexResources(ComplexResource::Life(r2)))}),

                        ComplexResourceRequest::Dolphin(r1, r2) => combinator
                            .make_dolphin(r1, r2, cell)
                            .map(ComplexResource::Dolphin)
                            .map_err(|(e, r1, r2)| { (e , GenericResource::ComplexResources(ComplexResource::Water(r1)), GenericResource::ComplexResources(ComplexResource::Life(r2)))}),

                        ComplexResourceRequest::AIPartner(r1, r2) => combinator
                            .make_aipartner(r1, r2, cell)
                            .map(ComplexResource::AIPartner)
                            .map_err(|(e, r1, r2)| { (e , GenericResource::ComplexResources(ComplexResource::Robot(r1)), GenericResource::ComplexResources(ComplexResource::Diamond(r2)))}),

                    };
                    // checking the result of complex_resource
                    return match complex_resource {
                        Ok(resource) => {
                            push_free_cell(cell_idx);
                            Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Ok(resource),
                            })
                        }
                        Err(err) => {
                            push_charged_cell(cell_idx);
                            println!("{}", err.0);
                            //TODO change this to log error
                            Some(PlanetToExplorer::CombineResourceResponse {
                                complex_response: Err(err),
                            })
                        }
                    }
                } else {
                    //TODO change this to log
                    println!("No available cell found");
                    let (ret1,ret2) = match resource {
                        ComplexResourceRequest::Water(r1, r2) => {
                            (GenericResource::BasicResources(BasicResource::Hydrogen(r1)),
                            GenericResource::BasicResources(BasicResource::Oxygen(r2)))
                        }
                        ComplexResourceRequest::AIPartner(r1, r2) => {
                            (GenericResource::ComplexResources(ComplexResource::Robot(r1)),
                            GenericResource::ComplexResources(ComplexResource::Diamond(r2)))
                        }
                        ComplexResourceRequest::Life(r1, r2) => {
                            (GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)))
                        }
                        ComplexResourceRequest::Diamond(r1, r2) => {
                            (GenericResource::BasicResources(BasicResource::Carbon(r1)),
                            GenericResource::BasicResources(BasicResource::Carbon(r2)))
                        }
                        ComplexResourceRequest::Dolphin(r1, r2) => {
                            (GenericResource::ComplexResources(ComplexResource::Water(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)))
                        }
                        ComplexResourceRequest::Robot(r1, r2) => {
                            (GenericResource::BasicResources(BasicResource::Silicon(r1)),
                            GenericResource::ComplexResources(ComplexResource::Life(r2)))
                        }
                    };
                    return Some(PlanetToExplorer::CombineResourceResponse { complex_response: Err(("no available cell".to_string(),ret1, ret2)) });
                }
                None
            }
        }
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        //if the planet can't build rockets, you're screwed
        //LOG
        let mut payload= Payload::new();
        payload.insert("Message".to_string(), "Asteroid".to_string());
        //LOG
        // TODO add detailed (debug) logging
        let mut ris=None;
        if !state.can_have_rocket() {
            ris=None;
        }

        //if you've already got a rocket ready, use it!
        else if state.has_rocket() {
            ris= state.take_rocket();
        }

        //try to build a rocket if you have any energy left
        else if let Some(idx) = get_charged_cell_index() {
            match state.build_rocket(idx as usize) {
                Ok(_) => {
                    push_free_cell(idx);
                    println!("Used a charged cell at index {}, to build a rocket", idx);
                    return state.take_rocket();
                }
                //build failed, log the error and return none
                Err(err) => {
                    push_charged_cell(idx);
                    //TODO change this to log
                    println!("{}", err);
                    return None;
                }
            }
        }
        if ris.is_none() {
            payload.insert("Result".to_string(), "no rocket available".to_string());
        }
        else{
            payload.insert("Result".to_string(), "a rocket is available".to_string());
        }
        let event=LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, state.id().clone().to_string(), MessageOrchestratorToPlanet, Channel::Debug, payload);
        log::info!("{}", event);
        ris
        //shouldn't be able to get here, but just in case...
        //None
    }

    fn start(&mut self, state: &PlanetState) {
        //println!("Planet {} AI started", state.id());
        let mut payload= Payload::new();
        payload.insert("Message".to_string(), "Planet AI started".to_string());
        let event=LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, state.id().clone().to_string(), MessageOrchestratorToPlanet, Channel::Debug, payload);
        log::info!("{}", event);
        // TODO non ho capito bene cosa deve fare planet.ai.start, deve creare il thread o lo fa l'orchestrator?
        // Mi sembra che lo start AI semplicemente dia il via al loop che permette l'AI di gestire le azioni
        // TODO non so se ha senso mettere l'inizializzazione degli stack qui o se va messa quando creaiamo AI
        // initialize_free_cell_stack() // TODO remove if the choice is the old implementation
    }

    fn stop(&mut self, _state: &PlanetState) { // mismatched names of state
        //println!("Planet AI stopped");
        let mut payload= Payload::new();
        payload.insert("Message".to_string(), "Planet AI started".to_string());
        let event=LogEvent::new(ActorType::Orchestrator, 0u64, ActorType::Planet, _state.id().clone().to_string(), MessageOrchestratorToPlanet, Channel::Debug, payload);
        log::info!("{}", event);
        // TODO stessa cosa di "start"
    }
}

pub trait ResToString{
    fn res_to_string(&self) -> String;
}

impl ResToString for BasicResourceType{
    fn res_to_string(&self) -> String {
        match self {
            BasicResourceType::Carbon=>String::from("carbon"),
            BasicResourceType::Hydrogen=>String::from("hydrogen"),
            BasicResourceType::Oxygen=>String::from("oxygen"),
            BasicResourceType::Silicon=>String::from("silicon"),
        }
    }
}
impl ResToString for ComplexResourceType{
    fn res_to_string(&self) -> String {
        match self {
            ComplexResourceType::AIPartner=>String::from("AIPartner"),
            ComplexResourceType::Diamond=>String::from("Diamond"),
            ComplexResourceType::Life=>String::from("Life"),
            ComplexResourceType::Robot=>String::from("Robot"),
            ComplexResourceType::Water=>String::from("Water"),
            ComplexResourceType::Dolphin => String::from("Dolphin"),
        }
    }
}

