use std::collections::{HashMap, HashSet, VecDeque};
use common_game::components::resource::{AIPartner, BasicResource, BasicResourceType, Carbon, ComplexResource, ComplexResourceRequest, ComplexResourceType, Diamond, Dolphin, GenericResource, Hydrogen, Life, Oxygen, ResourceType, Robot, Silicon, Water};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use crossbeam_channel::{Receiver, Sender, select};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

struct Bag {
    oxygen: Vec<Oxygen>,
    hydrogen: Vec<Hydrogen>,
    carbon: Vec<Carbon>,
    silicon: Vec<Silicon>,
    diamond: Vec<Diamond>,
    water: Vec<Water>,
    life: Vec<Life>,
    robot: Vec<Robot>,
    dolphin: Vec<Dolphin>,
    ai_partner: Vec<AIPartner>,
}

impl Bag {
    // creates an empty bag
    fn new() -> Self {
        Self {
            oxygen: vec![],
            hydrogen: vec![],
            carbon: vec![],
            silicon: vec![],
            diamond: vec![],
            water: vec![],
            life: vec![],
            robot: vec![],
            dolphin: vec![],
            ai_partner: vec![],
        }
    }

    // inserts a resource in the bag
    fn insert(&mut self, res: GenericResource) {
        match res {
            // base
            GenericResource::BasicResources(BasicResource::Oxygen(val)) => self.oxygen.push(val),
            GenericResource::BasicResources(BasicResource::Hydrogen(val)) => self.hydrogen.push(val),
            GenericResource::BasicResources(BasicResource::Carbon(val)) => self.carbon.push(val),
            GenericResource::BasicResources(BasicResource::Silicon(val)) => self.silicon.push(val),
            //complex
            GenericResource::ComplexResources(ComplexResource::Diamond(val)) => self.diamond.push(val),
            GenericResource::ComplexResources(ComplexResource::Water(val)) => self.water.push(val),
            GenericResource::ComplexResources(ComplexResource::Life(val)) => self.life.push(val),
            GenericResource::ComplexResources(ComplexResource::Robot(val)) => self.robot.push(val),
            GenericResource::ComplexResources(ComplexResource::Dolphin(val)) => self.dolphin.push(val),
            GenericResource::ComplexResources(ComplexResource::AIPartner(val)) => self.ai_partner.push(val),
        }
    }

    // takes a resource from the bag if it exists
    fn take_resource(&mut self, ty: ResourceType) -> Option<GenericResource> {
        match ty {
            // Basic Resources
            ResourceType::Basic(BasicResourceType::Oxygen) =>self.oxygen.pop().map(|v| GenericResource::BasicResources(BasicResource::Oxygen(v))),
            ResourceType::Basic(BasicResourceType::Hydrogen) =>self.hydrogen.pop().map(|v| GenericResource::BasicResources(BasicResource::Hydrogen(v))),
            ResourceType::Basic(BasicResourceType::Carbon) =>self.carbon.pop().map(|v| GenericResource::BasicResources(BasicResource::Carbon(v))),
            ResourceType::Basic(BasicResourceType::Silicon) =>self.silicon.pop().map(|v| GenericResource::BasicResources(BasicResource::Silicon(v))),
            // Complex Resources
            ResourceType::Complex(ComplexResourceType::Diamond) =>self.diamond.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Diamond(v))),
            ResourceType::Complex(ComplexResourceType::Water) =>self.water.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Water(v))),
            ResourceType::Complex(ComplexResourceType::Life) =>self.life.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Life(v))),
            ResourceType::Complex(ComplexResourceType::Robot) =>self.robot.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Robot(v))),
            ResourceType::Complex(ComplexResourceType::Dolphin) =>self.dolphin.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Dolphin(v))),
            ResourceType::Complex(ComplexResourceType::AIPartner) =>self.ai_partner.pop().map(|v| GenericResource::ComplexResources(ComplexResource::AIPartner(v))),
        }
    }

    // tells if a resource is contained in the bag
    //todo potrebbe non servire
    fn contains(&self, ty: ResourceType) -> bool {
        match ty {
            // Basic Resources
            ResourceType::Basic(BasicResourceType::Oxygen) => !self.oxygen.is_empty(),
            ResourceType::Basic(BasicResourceType::Hydrogen) => !self.hydrogen.is_empty(),
            ResourceType::Basic(BasicResourceType::Carbon) => !self.carbon.is_empty(),
            ResourceType::Basic(BasicResourceType::Silicon) => !self.silicon.is_empty(),

            // Complex Resources
            ResourceType::Complex(ComplexResourceType::Diamond) => !self.diamond.is_empty(),
            ResourceType::Complex(ComplexResourceType::Water) => !self.water.is_empty(),
            ResourceType::Complex(ComplexResourceType::Life) => !self.life.is_empty(),
            ResourceType::Complex(ComplexResourceType::Robot) => !self.robot.is_empty(),
            ResourceType::Complex(ComplexResourceType::Dolphin) => !self.dolphin.is_empty(),
            ResourceType::Complex(ComplexResourceType::AIPartner) => !self.ai_partner.is_empty(),
        }
    }

    // returns a BagType containing all the ResourceType in the bag

    // this is needed because the bag cannot give his ownership to the orchestrator and cannot be passed as a reference
    pub fn to_resource_types(&self) -> Vec<ResourceType> {
        let total_size = self.oxygen.len() + self.hydrogen.len() + self.carbon.len() + self.silicon.len() + self.diamond.len() + self.water.len() + self.life.len() + self.robot.len() + self.dolphin.len() + self.ai_partner.len();
        let mut types = Vec::with_capacity(total_size); //this way the vec is already of the right size
        for _ in 0..self.oxygen.len() { types.push(ResourceType::Basic(BasicResourceType::Oxygen)); }
        for _ in 0..self.hydrogen.len() { types.push(ResourceType::Basic(BasicResourceType::Hydrogen)); }
        for _ in 0..self.carbon.len() { types.push(ResourceType::Basic(BasicResourceType::Carbon)); }
        for _ in 0..self.silicon.len() { types.push(ResourceType::Basic(BasicResourceType::Silicon)); }
        // complex
        for _ in 0..self.diamond.len() { types.push(ResourceType::Complex(ComplexResourceType::Diamond)); }
        for _ in 0..self.water.len() { types.push(ResourceType::Complex(ComplexResourceType::Water)); }
        for _ in 0..self.life.len() { types.push(ResourceType::Complex(ComplexResourceType::Life)); }
        for _ in 0..self.robot.len() { types.push(ResourceType::Complex(ComplexResourceType::Robot)); }
        for _ in 0..self.dolphin.len() { types.push(ResourceType::Complex(ComplexResourceType::Dolphin)); }
        for _ in 0..self.ai_partner.len() { types.push(ResourceType::Complex(ComplexResourceType::AIPartner)); }
        types
    }

    // the following methods are the ones to combine resources
    //they are all used in order to avoid code duplication
    fn make_diamond_request(&mut self) -> Result<ComplexResourceRequest, String> {
        // Check that the explorer has 2 carbons before taking any
        let carbon_count = self.carbon.len();

        if carbon_count < 2 {
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .ok_or("Missing resource")?
            .to_carbon()?;

        let c2 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .ok_or("Missing resource")?
            .to_carbon()?;

        Ok(ComplexResourceRequest::Diamond(c1, c2))
    }
    fn make_water_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if self.oxygen.is_empty() && self.hydrogen.is_empty(){
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Hydrogen))
            .ok_or("Missing resource")?
            .to_hydrogen()?;
        let c2 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Oxygen))
            .ok_or("Missing resource")?
            .to_oxygen()?;

        Ok(ComplexResourceRequest::Water(c1, c2))
    }
    fn make_life_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if self.water.is_empty() && self.carbon.is_empty()
        {
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Complex(ComplexResourceType::Water))
            .ok_or("Missing resource")?
            .to_water()?;
        let c2 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .ok_or("Missing resource")?
            .to_carbon()?;

        Ok(ComplexResourceRequest::Life(c1, c2))
    }
    fn make_robot_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if self.life.is_empty() && self.silicon.is_empty()
        {
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Silicon))
            .ok_or("Missing resource")?
            .to_silicon()?;
        let c2 = self
            .take_resource(ResourceType::Complex(ComplexResourceType::Life))
            .ok_or("Missing resource")?
            .to_life()?;

        Ok(ComplexResourceRequest::Robot(c1, c2))
    }
    fn make_dolphin_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if self.life.is_empty() && self.water.is_empty()
        {
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Complex(ComplexResourceType::Water))
            .ok_or("Missing resource")?
            .to_water()?;
        let c2 = self
            .take_resource(ResourceType::Complex(ComplexResourceType::Life))
            .ok_or("Missing resource")?
            .to_life()?;

        Ok(ComplexResourceRequest::Dolphin(c1, c2))
    }
    fn make_ai_partner_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if self.diamond.is_empty() && self.robot.is_empty()
        {
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Complex(ComplexResourceType::Robot))
            .ok_or("Missing resource")?
            .to_robot()?;
        let c2 = self
            .take_resource(ResourceType::Complex(ComplexResourceType::Diamond))
            .ok_or("Missing resource")?
            .to_diamond()?;

        Ok(ComplexResourceRequest::AIPartner(c1, c2))
    }
}

// struct that contains some
struct PlanetInfo {
    basic_resources: Option<HashSet<BasicResourceType>>,
    complex_resources: Option<HashSet<ComplexResourceType>>,
    neighbours: Option<HashSet<ID>>,
}

// TODO memorizzare topologia, celle libere (utili per AI se non ci sono 2 explorer), risorse generate/combinate per ogni pianeta

// these are the states of the explorer state machine
pub enum ExplorerState {
    Idle,
    WaitingToStartExplorerAI,
    WaitingForNeighbours,
    Traveling,
    GeneratingResource,
    CombiningResources,
    WaitingForSupportedResources,
    WaitingForSupportedCombinations,
    WaitingForAvailableEnergyCells,
    Killed,
}

// this function checks if the orchestrator message received is the one expected (based on the explorer state)
pub fn orch_msg_match_state(explorer_state: &ExplorerState, msg: &OrchestratorToExplorer) -> bool {
    match (explorer_state, msg) {
        (ExplorerState::Idle, _) => true,
        (ExplorerState::WaitingToStartExplorerAI, OrchestratorToExplorer::StartExplorerAI) => true,
        (ExplorerState::WaitingForNeighbours, OrchestratorToExplorer::NeighborsResponse { .. }) => true,
        (ExplorerState::Traveling, OrchestratorToExplorer::MoveToPlanet { .. }) => true,
        _ => false,
    }
}

// this function checks if the planet message received is the one expected (based on the explorer state)
pub fn planet_msg_match_state(explorer_state: &ExplorerState, msg: &PlanetToExplorer) -> bool {
    match (explorer_state, msg) {
        (ExplorerState::Idle, _) => true,
        (ExplorerState::GeneratingResource, PlanetToExplorer::GenerateResourceResponse { .. }) => true,
        (ExplorerState::CombiningResources, PlanetToExplorer::CombineResourceResponse { .. }) => true,
        (ExplorerState::WaitingForSupportedResources, PlanetToExplorer::SupportedResourceResponse { .. }) => true,
        (ExplorerState::WaitingForSupportedCombinations, PlanetToExplorer::CombineResourceResponse { .. }) => true,
        (ExplorerState::WaitingForAvailableEnergyCells, PlanetToExplorer::AvailableEnergyCellResponse { .. }) => true,
        _ => false,
    }
}

// this function put the explorer in the condition to receive messages (idle state),
// it is called when the explorer receives the StartExplorerAI message
pub fn start_explorer_ai(explorer: &mut Explorer) {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::StartExplorerAIResult {explorer_id: explorer.explorer_id}) {
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Start explorer AI result sent correctly.")
            //todo logs
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending start explorer AI result: {:?}",
                err
            );
            //todo logs
            // TODO killare il thread / panicare o non gestire l'errore?
        }
    }
}

// this function resets the topology known by the explorer,
// it is called when the explorer receives the ResetExplorerAI message
pub fn reset_explorer_ai(explorer: &mut Explorer) {
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::ResetExplorerAIResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => {
            // TODO reset anche dell'inventario?
            explorer.topology_info.clear();
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Reset explorer AI result sent correctly.")
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending reset explorer AI result: {:?}",
                err
            );
        }
    }
}

// this function put the explorer in the condition to wait for a StartExplorerAI message (WaitingToStartExplorerAI state),
// it is called when the explorer receives the StopExplorerAI message
pub fn stop_explorer_ai(explorer: &mut Explorer) {
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::StopExplorerAIResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => {
            explorer.state = ExplorerState::WaitingToStartExplorerAI;
            println!("[EXPLORER DEBUG] Stop explorer AI result sent correctly.")
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending stop explorer AI result: {:?}",
                err
            );
        }
    }
}

// this function puts the explorer in the Killed state waiting for the thread to be killed
pub fn kill_explorer(explorer: &mut Explorer) {
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::KillExplorerResult {
            explorer_id: explorer.explorer_id,
        }) {
        Ok(_) => {
            explorer.state = ExplorerState::Killed;
            println!("[EXPLORER DEBUG] Kill explorer result sent correctly.")
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending kill explorer result: {:?}",
                err
            );
        }
    }
}

// this function sets the sender_to_planet of the explorer struct
pub fn move_to_planet(
    explorer: &mut Explorer,
    sender_to_new_planet: Option<Sender<ExplorerToPlanet>>,
) {
    explorer.state = ExplorerState::Idle;
    match sender_to_new_planet {
        Some(sender) => {
            explorer.planet_channels.1 = sender;
            explorer.planet_id = explorer.next_planet_id.clone();
            println!("[EXPLORER DEBUG] Sender channel set correctly");
        }
        None => {
            println!("[EXPLORER DEBUG] Sender channel is None.");
        }
    }
}

// this function sends the current planet id to the orchestrator
pub fn current_planet_request(explorer: &mut Explorer) {
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::CurrentPlanetResult {
            explorer_id: explorer.explorer_id,
            planet_id: explorer.planet_id,
        }) {
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Current planet result sent correctly.")
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending current planet result: {:?}",
                err
            );
        }
    }
}

// this function sends the basic resources supported by the current planet to the orchestrator
// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
// response and then send it back to the orchestrator)
pub fn supperted_resource_request(explorer: &mut Explorer) {
    let mut supported_resources = HashSet::new();
    if explorer.topology_info.contains_key(&explorer.planet_id)
        && let Some(planet_info) = explorer.topology_info.get(&explorer.planet_id)
    {
        match &planet_info.basic_resources {
            Some(basic_resources) => {
                supported_resources = basic_resources.clone();
            }
            None => {}
        }
    } else {
        match explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::SupportedResourceRequest {
                explorer_id: explorer.explorer_id,
            }) {
            Ok(_) => {
                println!(
                    "[EXPLORER DEBUG] Supported resource request sent correctly from explorer."
                );
            }
            Err(err) => {
                println!(
                    "[EXPLORER DEBUG] Error sending supported resource request from explorer: {:?}",
                    err
                );
            }
        }
        match explorer.planet_channels.0.recv() {
            Ok(res) => match res {
                PlanetToExplorer::SupportedResourceResponse { resource_list } => {
                    supported_resources = resource_list;
                }
                _ => {
                    println!("[EXPLORER DEBUG] Unexpected response to SupportedResourceRequest.");
                }
            },
            Err(err) => {
                println!(
                    "[EXPLORER DEBUG] Error receiving supported resources from planet: {:?}",
                    err
                );
            }
        }
    }
    match explorer
        .orchestrator_channels
        .1
        .send(ExplorerToOrchestrator::SupportedResourceResult {
            explorer_id: explorer.explorer_id,
            supported_resources,
        }) {
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!(
                "[EXPLORER DEBUG] Supported resource result sent correctly from explorer to orchestrator."
            );
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending supported resource result from explorer to orchestrator: {:?}",
                err
            );
        }
    }
}

// this function sends the complex resources supported by the current planet to the orchestrator
// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
// response and then send it back to the orchestrator)
pub fn supported_combination_request(explorer: &mut Explorer) {
    let mut supported_combinations = HashSet::new();
    if explorer.topology_info.contains_key(&explorer.planet_id)
        && let Some(planet_info) = explorer.topology_info.get(&explorer.planet_id)
    {
        match &planet_info.complex_resources {
            Some(basic_resources) => {
                supported_combinations = basic_resources.clone();
            }
            None => {}
        }
    } else {
        match explorer
            .planet_channels
            .1
            .send(ExplorerToPlanet::SupportedCombinationRequest {
                explorer_id: explorer.explorer_id,
            }) {
            Ok(_) => {
                println!(
                    "[EXPLORER DEBUG] Supported combination request sent correctly from explorer."
                );
            }
            Err(err) => {
                println!(
                    "[EXPLORER DEBUG] Error sending supported combination request from explorer: {:?}",
                    err
                );
            }
        }
        match explorer.planet_channels.0.recv() {
            Ok(res) => match res {
                PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
                    supported_combinations = combination_list;
                }
                _ => {
                    println!(
                        "[EXPLORER DEBUG] Unexpected response to SupportedCombinationRequest."
                    );
                }
            },
            Err(err) => {
                println!(
                    "[EXPLORER DEBUG] Error receiving supported combinations from planet: {:?}",
                    err
                );
            }
        }
    }
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::SupportedCombinationResult {
            explorer_id: explorer.explorer_id,
            combination_list: supported_combinations,
        },
    ) {
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!(
                "[EXPLORER DEBUG] Supported combination result sent correctly from explorer to orchestrator."
            );
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending supported combination result from explorer to orchestrator: {:?}",
                err
            );
        }
    }
}

// this function sends the GenerateResourceRequest, waits for the planet response, and,
// if successful puts the resource in the bag
pub fn generate_resource_request(explorer: &mut Explorer, to_generate: BasicResourceType) {
    match explorer
        .planet_channels
        .1
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id: explorer.explorer_id,
            resource: to_generate,
        }) {
        Ok(_) => {
            println!("[EXPLORER DEBUG] Generate resource request correctly");
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error sending generate resource request {}",
                err
            );
        }
    }
    match explorer.planet_channels.0.recv() {
        Ok(msg) => match msg {
            PlanetToExplorer::GenerateResourceResponse { resource } => {
                put_basic_resource_in_the_bag(explorer, resource);
            }
            _ => println!("[EXPLORER DEBUG] Unexpected response to generate resource request"),
        },
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error receiving generate resource response {}",
                err
            );
        }
    }
}

// this function puts a basic resource in the explorer bag
pub fn put_basic_resource_in_the_bag(explorer: &mut Explorer, resource: Option<BasicResource>) {
    if let Some(resource) = resource {
        let new_resource = match resource {
            BasicResource::Oxygen(oxygen) => oxygen.to_generic(),
            BasicResource::Hydrogen(hydrogen) => hydrogen.to_generic(),
            BasicResource::Carbon(carbon) => carbon.to_generic(),
            BasicResource::Silicon(silicon) => silicon.to_generic(),
        };
        explorer.bag.insert(new_resource);
    }
}

// this function sends the CombineResourceRequest, waits for the planet response, and,
// if successful puts the resource in the bag
pub fn combine_resource_request(explorer: &mut Explorer, to_generate: ComplexResourceType) {
    let complex_resource_req = match to_generate {
        // TODO provide the requested resources from the bag for each combination
        ComplexResourceType::Diamond => explorer.bag.make_diamond_request(),
        ComplexResourceType::Water => explorer.bag.make_water_request(),
        ComplexResourceType::Life => explorer.bag.make_life_request(),
        ComplexResourceType::Robot => explorer.bag.make_robot_request(),
        ComplexResourceType::Dolphin => explorer.bag.make_dolphin_request(),
        ComplexResourceType::AIPartner => explorer.bag.make_ai_partner_request(),
    };
    match complex_resource_req {
        Ok(complex_resource_req) => {
            match explorer
                .planet_channels
                .1
                .send(ExplorerToPlanet::CombineResourceRequest {
                    explorer_id: explorer.explorer_id,
                    msg: complex_resource_req,
                }) {
                Ok(_) => {
                    println!("[EXPLORER DEBUG] Combine resource request sent correctly");
                }
                Err(err) => {
                    println!(
                        "[EXPLORER DEBUG] Error sending combine resource request {}",
                        err
                    );
                }
            }
            match explorer.planet_channels.0.recv() {
                Ok(msg) => {
                    match msg {
                        PlanetToExplorer::CombineResourceResponse { complex_response } => {
                            match complex_response {
                                Ok(complex_resource) => {
                                    // ComplexResource does not have the method "to_generic" but each single complex resource does (so that seems the only way to cast to GenericResource)
                                    let generic_resource = match complex_resource {
                                        ComplexResource::Diamond(d) => d.to_generic(),
                                        ComplexResource::Water(w) => w.to_generic(),
                                        ComplexResource::Life(l) => l.to_generic(),
                                        ComplexResource::Robot(r) => r.to_generic(),
                                        ComplexResource::Dolphin(d) => d.to_generic(),
                                        ComplexResource::AIPartner(a) => a.to_generic(),
                                    };
                                    explorer.bag.insert(generic_resource);
                                }
                                Err(err) => {
                                    println!(
                                        "[EXPLORER DEBUG] Error receiving CombineResourceResponse: {:?}",
                                        err
                                    )
                                }
                            }
                        }
                        _ => println!(
                            "[EXPLORER DEBUG] Unexpected response to combine resource request"
                        ),
                    }
                }
                Err(err) => {
                    println!(
                        "[EXPLORER DEBUG] Error receiving combine resource response {}",
                        err
                    );
                }
            }
        }
        Err(err) => {
            println!(
                "[EXPLORER DEBUG] Error generating complex resource request {}",
                err
            );
        }
    }
}

// this function puts a complex resource in the explorer bag
pub fn put_complex_resource_in_the_bag(
    explorer: &mut Explorer,
    complex_response: Result<ComplexResource, (String, GenericResource, GenericResource)>,
) {
    if let Ok(complex_resource) = complex_response {
        let new_resource = match complex_resource {
            ComplexResource::Diamond(diamond) => diamond.to_generic(),
            ComplexResource::Water(water) => water.to_generic(),
            ComplexResource::Life(life) => life.to_generic(),
            ComplexResource::Robot(robot) => robot.to_generic(),
            ComplexResource::Dolphin(dolphin) => dolphin.to_generic(),
            ComplexResource::AIPartner(ai_partner) => ai_partner.to_generic(),
        };
        explorer.bag.insert(new_resource);
    }
}

// this function updates the neighbours of the current planet
pub fn neighbours_response(explorer: &mut Explorer, neighbors: Vec<ID>) {
    explorer.state = ExplorerState::Idle;
    for &neighbour in &neighbors {
        explorer
            .topology_info
            .entry(neighbour)
            .or_insert(PlanetInfo {
                basic_resources: None,
                complex_resources: None,
                neighbours: None,
            });
    }
    let planet_info = explorer
        .topology_info
        .get_mut(&explorer.planet_id)
        .expect("Planet should already be in topology"); // TODO se non è nella topologia aggiornare la topologia e ripetere

    planet_info.neighbours = Some(neighbors.into_iter().collect());
}

// this is the struct of the explorer
pub struct Explorer {
    explorer_id: u32,
    planet_id: u32, //I assume that the travel isn't instant, so I put an Option we should manage the case the planet explodes
    next_planet_id: u32, // needed if the travelToPlanet doesn't go well
    orchestrator_channels: (
        Receiver<OrchestratorToExplorer>,
        Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
    ),
    planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
    topology_info: HashMap<ID, PlanetInfo>,
    state: ExplorerState,
    bag: Bag,
    energy_cells: u32, // of the current planet
    buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>, // orchestrator messages that the explorer cannot respond to immediately
    buffer_planet_msg: VecDeque<PlanetToExplorer>, // planet messages that the explorer cannot respond to immediately
}

impl Explorer {
    // at creation, an Explorer should be connected to Orchestrator and the starting Planet
    pub fn new(
        explorer_id: u32,
        planet_id: u32,
        explorer_to_orchestrator_channels: (
            Receiver<OrchestratorToExplorer>,
            Sender<ExplorerToOrchestrator<Vec<ResourceType>>>,
        ),
        explorer_to_planet_channels: (Receiver<PlanetToExplorer>, Sender<ExplorerToPlanet>),
        energy_cells: u32, // useful in the case in which the explorer starts mid-game
    ) -> Self {
        let mut starting_topology_info = HashMap::new();
        starting_topology_info.insert(
            planet_id,
            PlanetInfo {
                basic_resources: None,
                complex_resources: None,
                neighbours: None,
            },
        );
        Self {
            explorer_id,
            planet_id,
            next_planet_id: planet_id,
            orchestrator_channels: explorer_to_orchestrator_channels,
            planet_channels: explorer_to_planet_channels,
            topology_info: starting_topology_info,
            state: ExplorerState::WaitingToStartExplorerAI,
            bag: Bag::new(),
            energy_cells,
            buffer_orchestrator_msg: VecDeque::new(),
            buffer_planet_msg: VecDeque::new(),
        }
    }

    // getter function for the id
    pub fn id(&self) -> u32 {
        self.explorer_id
    }

    // the explorer loop
    pub fn run(&mut self) {
        // every iteration the explorer receives messages from both planet and orchestrator channels,
        // then it behaves based on the message received, if the message received and the explorer state
        // do not match together the message is pushed into the corresponding buffer, and it will be read
        // when the explorer will be in an "Idle" state
        loop {
            select! {
                recv(self.orchestrator_channels.0) -> msg_orchestrator => {
                    match msg_orchestrator {
                        Ok(msg) => {
                            if orch_msg_match_state(&self.state, &msg) {
                                match msg {
                                    OrchestratorToExplorer::StartExplorerAI => {
                                        start_explorer_ai(self);
                                    }
                                    OrchestratorToExplorer::ResetExplorerAI => {
                                        reset_explorer_ai(self);
                                    }
                                    OrchestratorToExplorer::StopExplorerAI => {
                                        stop_explorer_ai(self);
                                    }
                                    OrchestratorToExplorer::KillExplorer => {
                                        // TODO this action should be preemptive
                                        kill_explorer(self);
                                    }
                                    OrchestratorToExplorer::MoveToPlanet{ sender_to_new_planet, planet_id } => {
                                        // TODO use the planet_id variable (common crate v3)
                                        move_to_planet(self, sender_to_new_planet);
                                    }
                                    OrchestratorToExplorer::CurrentPlanetRequest => {
                                        current_planet_request(self);
                                    }
                                    OrchestratorToExplorer::SupportedResourceRequest => {
                                        // + devo fare un'attesa bloccante per ricevere le risorse supportate e poi rispondere o vado avanti? -> al momento attesa bloccante
                                        supperted_resource_request(self);
                                    }
                                    OrchestratorToExplorer::SupportedCombinationRequest => {
                                        // + devo fare un'attesa bloccante per ricevere le combinazioni supportate e poi rispondere o vado avanti?
                                        supported_combination_request(self);
                                    }
                                    OrchestratorToExplorer::GenerateResourceRequest{ to_generate } => {
                                        generate_resource_request(self, to_generate);
                                    }
                                    OrchestratorToExplorer::CombineResourceRequest{ to_generate } => {
                                        // TODO verify first if the explorer has the resources to generate the combined one
                                        combine_resource_request(self, to_generate);
                                    }
                                    OrchestratorToExplorer::BagContentRequest => {
                                        // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                                        match self.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: self.explorer_id, bag_content: self.bag.to_resource_types()}) {
                                            Ok(_) => {
                                                println!("[EXPLORER DEBUG] BagContent response sent correctly");
                                            }
                                            Err(err) => {
                                                println!("[EXPLORER DEBUG] Error sending bag content response: {}", err);
                                            }
                                        }
                                    }
                                    OrchestratorToExplorer::NeighborsResponse{ neighbors } => {
                                        neighbours_response(self, neighbors);
                                    }
                                }
                            } else {
                                self.buffer_orchestrator_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            println!("[EXPLORER DEBUG] Error in receiving the orchestrator message: {}", err);
                        }
                    }
                },
                recv(self.planet_channels.0) -> msg_planet => {
                    match msg_planet {
                        Ok(msg) => {
                            if planet_msg_match_state(&self.state, &msg) {
                                match msg {
                                    PlanetToExplorer::SupportedResourceResponse{ resource_list } => {
                                        match self.topology_info.get_mut(&self.planet_id) {
                                            Some(planet_info) => {
                                                planet_info.basic_resources = Some(resource_list);
                                            }
                                            None => {
                                                // TODO (non dovrebbe accadere) inserire il pianeta nella topologia e poi inserire la resource list
                                            }
                                        }
                                    }
                                    PlanetToExplorer::SupportedCombinationResponse{ combination_list } => {
                                        match self.topology_info.get_mut(&self.planet_id) {
                                            Some(planet_info) => {
                                                planet_info.complex_resources = Some(combination_list);
                                            }
                                            None => {
                                                // TODO (non dovrebbe accadere) inserire il pianeta nella topologia e poi inserire la combination list
                                            }
                                        }
                                    }
                                    PlanetToExplorer::GenerateResourceResponse{ resource } => {
                                        if let Some(resource) = resource {
                                            let new_resource = match resource {
                                                BasicResource::Oxygen(oxygen) => { oxygen.to_generic() }
                                                BasicResource::Hydrogen(hydrogen) => { hydrogen.to_generic() }
                                                BasicResource::Carbon(carbon) => { carbon.to_generic() }
                                                BasicResource::Silicon(silicon) => { silicon.to_generic() }
                                            };
                                            self.bag.insert(new_resource);
                                        }
                                    }
                                    PlanetToExplorer::CombineResourceResponse{ complex_response } => {
                                        if let Ok(complex_resource) = complex_response {
                                            let new_resource = match complex_resource {
                                                ComplexResource::Diamond(diamond) => { diamond.to_generic() }
                                                ComplexResource::Water(water) => { water.to_generic() }
                                                ComplexResource::Life(life) => { life.to_generic() }
                                                ComplexResource::Robot(robot) => { robot.to_generic() }
                                                ComplexResource::Dolphin(dolphin) => { dolphin.to_generic() }
                                                ComplexResource::AIPartner(ai_partner) => { ai_partner.to_generic() }
                                            };
                                            self.bag.insert(new_resource);
                                        }
                                    }
                                    PlanetToExplorer::AvailableEnergyCellResponse{ available_cells } => {
                                        self.energy_cells = available_cells;
                                    }
                                    PlanetToExplorer::Stopped => {
                                        // TODO gestire in base all'ai dell'explorer
                                        self.state = ExplorerState::Idle;
                                    }
                                }
                            } else {
                                self.buffer_planet_msg.push_back(msg);
                            }
                        }
                        Err(err) => {
                            println!("[EXPLORER DEBUG] Error in receiving the planet message: {}", err);
                        }
                    }
                }
                default => {
                    match self.state {
                        ExplorerState::Idle => {
                            // TODO gestisci i messaggi nel buffer
                            manage_buffer_msg(self);
                        }
                        _ => { return }
                    }
                    match self.state {
                        ExplorerState::Idle => {
                            // TODO qui va l'AI vera e propria

                        }
                        _ => { return }
                    }
                }
            }
        }
    }
}

// this function manages all the messages that were put in the buffers
// (in the same way the explorer usually manages them)
pub fn manage_buffer_msg(explorer: &mut Explorer) {
    match explorer.state {
        ExplorerState::Idle => {}
        _ => return,
    }
    while let Some(msg) = explorer.buffer_orchestrator_msg.pop_front() {
        match msg {
            OrchestratorToExplorer::StartExplorerAI => {
                start_explorer_ai(explorer);
            }
            OrchestratorToExplorer::ResetExplorerAI => {
                reset_explorer_ai(explorer);
            }
            OrchestratorToExplorer::StopExplorerAI => {
                stop_explorer_ai(explorer);
            }
            OrchestratorToExplorer::KillExplorer => {
                // TODO this action should be preemptive
                kill_explorer(explorer);
            }
            OrchestratorToExplorer::MoveToPlanet {
                sender_to_new_planet,
                planet_id,
            } => {
                //TODO: use the new planet_id variable (common crate v3)
                move_to_planet(explorer, sender_to_new_planet);
            }
            OrchestratorToExplorer::CurrentPlanetRequest => {
                current_planet_request(explorer);
            }
            OrchestratorToExplorer::SupportedResourceRequest => {
                // + devo fare un'attesa bloccante per ricevere le risorse supportate e poi rispondere o vado avanti? -> al momento attesa bloccante
                supperted_resource_request(explorer);
            }
            OrchestratorToExplorer::SupportedCombinationRequest => {
                // + devo fare un'attesa bloccante per ricevere le combinazioni supportate e poi rispondere o vado avanti?
                supported_combination_request(explorer);
            }
            OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                generate_resource_request(explorer, to_generate);
            }
            OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                // TODO verify first if the explorer has the resources to generate the combined one
                combine_resource_request(explorer, to_generate);
            }
            OrchestratorToExplorer::BagContentRequest => {
                // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                match explorer.orchestrator_channels.1.send(
                    ExplorerToOrchestrator::BagContentResponse {
                        explorer_id: explorer.explorer_id,
                        bag_content: explorer.bag.to_resource_types(),
                    },
                ) {
                    Ok(_) => {
                        println!("[EXPLORER DEBUG] BagContent response sent correctly");
                    }
                    Err(err) => {
                        println!(
                            "[EXPLORER DEBUG] Error sending bag content response: {}",
                            err
                        );
                    }
                }
            }
            OrchestratorToExplorer::NeighborsResponse { neighbors } => {
                neighbours_response(explorer, neighbors);
            }
        }
        match explorer.state {
            ExplorerState::Idle => {}
            _ => return,
        }
    }
    match explorer.state {
        ExplorerState::Idle => {}
        _ => return,
    }
    while let Some(msg) = explorer.buffer_planet_msg.pop_front() {
        match msg {
            PlanetToExplorer::SupportedResourceResponse { resource_list } => {
                match explorer.topology_info.get_mut(&explorer.planet_id) {
                    Some(planet_info) => {
                        planet_info.basic_resources = Some(resource_list);
                    }
                    None => {
                        // TODO (non dovrebbe accadere) inserire il pianeta nella topologia e poi inserire la resource list
                    }
                }
            }
            PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
                match explorer.topology_info.get_mut(&explorer.planet_id) {
                    Some(planet_info) => {
                        planet_info.complex_resources = Some(combination_list);
                    }
                    None => {
                        // TODO (non dovrebbe accadere) inserire il pianeta nella topologia e poi inserire la combination list
                    }
                }
            }
            PlanetToExplorer::GenerateResourceResponse { resource } => {
                put_basic_resource_in_the_bag(explorer, resource);
            }
            PlanetToExplorer::CombineResourceResponse { complex_response } => {
                put_complex_resource_in_the_bag(explorer, complex_response)
            }
            PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                explorer.energy_cells = available_cells;
            }
            PlanetToExplorer::Stopped => {
                // TODO gestire in base all'ai dell'explorer
                explorer.state = ExplorerState::Idle;
            }
        }
        match explorer.state {
            ExplorerState::Idle => {}
            _ => return,
        }
    }
}
