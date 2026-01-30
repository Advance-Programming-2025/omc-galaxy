use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet, VecDeque};
use common_game::components::planet::Planet;
use common_game::components::resource::{AIPartner, BasicResource, BasicResourceType, Carbon, ComplexResource, ComplexResourceRequest, ComplexResourceType, Diamond, Dolphin, GenericResource, Hydrogen, Life, Oxygen, ResourceType, Robot, Silicon, Water};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator::SupportedResourceResult;
use crossbeam_channel::{Receiver, Sender, select};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

pub trait ToGeneric{
    fn res_to_generic(self) -> GenericResource;
}
impl ToGeneric for BasicResource {
    fn res_to_generic(self) -> GenericResource {
        match self {
            BasicResource::Oxygen(oxygen) => { oxygen.to_generic() }
            BasicResource::Hydrogen(hydrogen) => { hydrogen.to_generic() }
            BasicResource::Carbon(carbon) => { carbon.to_generic() }
            BasicResource::Silicon(silicon) => { silicon.to_generic()}
        }
    }
}
impl ToGeneric for ComplexResource {
    fn res_to_generic(self) -> GenericResource {
        match self {
            ComplexResource::Diamond(diamond) => { diamond.to_generic() }
            ComplexResource::Water(water) => { water.to_generic() }
            ComplexResource::Life(life) => { life.to_generic() }
            ComplexResource::Robot(robot) => { robot.to_generic() }
            ComplexResource::Dolphin(dolphin) => { dolphin.to_generic() }
            ComplexResource::AIPartner(ai_partner) => { ai_partner.to_generic() }
        }
    }
}
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

// struct that contains some
struct PlanetInfo {
    basic_resources: Option<HashSet<BasicResourceType>>,
    complex_resources: Option<HashSet<ComplexResourceType>>,
    neighbours: Option<HashSet<ID>>,
}

// TODO memorizzare topologia, celle libere (utili per AI se non ci sono 2 explorer), risorse generate/combinate per ogni pianeta

// these are the states of the explorer state machine
// todo non sono sicuro che in questo modo si abbia la certezza dell'accoppiamento risposta-richiesta
#[derive(PartialEq)]
pub enum ExplorerState {
    Idle,
    WaitingToStartExplorerAI,
    WaitingForNeighbours,
    Traveling,
    GeneratingResource{
        orchestrator_response: bool,
    },
    CombiningResources{
        orchestrator_response: bool,
    },
    // WaitingForSupportedResources{
    //     orchestrator_response:bool,
    // },
    // WaitingForSupportedCombinations{
    //     orchestrator_response:bool,
    // },
    // WaitingForAvailableEnergyCells,
    Surveying{
        resources: bool,
        combinations: bool,
        energy_cells: bool,
        orch_resource: bool,
        orch_combination: bool,
    },
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
        (ExplorerState::GeneratingResource { orchestrator_response: _ }, PlanetToExplorer::GenerateResourceResponse { .. }) => true,
        (ExplorerState::CombiningResources{ orchestrator_response: _ }, PlanetToExplorer::CombineResourceResponse { .. }) => true,
        (ExplorerState::Surveying {resources: true, ..}, PlanetToExplorer::SupportedResourceResponse {..}) => true,
        (ExplorerState::Surveying {combinations: true, ..}, PlanetToExplorer::SupportedCombinationResponse { ..}) => true,
        (ExplorerState::Surveying {energy_cells:true, ..}, PlanetToExplorer::AvailableEnergyCellResponse { ..}) => true,
        // (ExplorerState::WaitingForSupportedResources{ orchestrator_response: _ }, PlanetToExplorer::SupportedResourceResponse { .. }) => true,
        // (ExplorerState::WaitingForSupportedCombinations{ orchestrator_response: _ }, PlanetToExplorer::CombineResourceResponse { .. }) => true,
        // (ExplorerState::WaitingForAvailableEnergyCells, PlanetToExplorer::AvailableEnergyCellResponse { .. }) => true,
        _ => false,
    }
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
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                                        start_explorer_ai(self)?;
                                    }
                                    OrchestratorToExplorer::ResetExplorerAI => {
                                        reset_explorer_ai(self)?;
                                    }
                                    OrchestratorToExplorer::StopExplorerAI => {
                                        stop_explorer_ai(self)?;
                                    }
                                    OrchestratorToExplorer::KillExplorer => {
                                        // TODO this action should be preemptive
                                        kill_explorer(self)?;
                                        return Ok(())
                                    }
                                    OrchestratorToExplorer::MoveToPlanet{ sender_to_new_planet, planet_id } => {
                                        // TODO use the planet_id variable (common crate v3)
                                        move_to_planet(self, sender_to_new_planet, planet_id)?;
                                    }
                                    OrchestratorToExplorer::CurrentPlanetRequest => {
                                        current_planet_request(self)?;
                                    }
                                    OrchestratorToExplorer::SupportedResourceRequest => {
                                        supported_resource_request(self)?;
                                    }
                                    OrchestratorToExplorer::SupportedCombinationRequest => {
                                        supported_combination_request(self)?;
                                    }
                                    OrchestratorToExplorer::GenerateResourceRequest{ to_generate } => {
                                        generate_resource_request(self, to_generate, true)?;
                                    }
                                    OrchestratorToExplorer::CombineResourceRequest{ to_generate } => {
                                        combine_resource_request(self, to_generate)?;
                                    }
                                    OrchestratorToExplorer::BagContentRequest => {
                                        // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                                        self.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: self.explorer_id, bag_content: self.bag.to_resource_types()})?;
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
                            //todo logs
                            println!("[EXPLORER DEBUG] Error in receiving the orchestrator message: {}", err);
                        }
                    }
                },
                //even if the channel id disconnected we need to wait the kill msg to terminate the execution
                recv(self.planet_channels.0) -> msg_planet => {
                    match msg_planet {
                        Ok(msg) => {
                            if planet_msg_match_state(&self.state, &msg) {
                                match msg {
                                    PlanetToExplorer::SupportedResourceResponse{ resource_list } => {
                                        manage_supported_resource_response(self, resource_list)?;
                                    }
                                    PlanetToExplorer::SupportedCombinationResponse{ combination_list } => {
                                        manage_supported_combination_response(self, combination_list)?;
                                    }
                                    PlanetToExplorer::GenerateResourceResponse{ resource } => {
                                        manage_generate_response(self, resource)?;
                                    }
                                    PlanetToExplorer::CombineResourceResponse{ complex_response } => {
                                        manage_combine_response(self, complex_response)?;
                                    }
                                    PlanetToExplorer::AvailableEnergyCellResponse{ available_cells } => {
                                        match self.state{
                                            ExplorerState::Surveying {resources,combinations,energy_cells:true,orch_resource,orch_combination}=>{
                                                self.energy_cells = available_cells;
                                                if !resources && !combinations{
                                                    self.state = ExplorerState::Idle;
                                                }
                                                else{
                                                    self.state = ExplorerState::Surveying {
                                                        resources,
                                                        combinations,
                                                        energy_cells:false,
                                                        orch_resource,
                                                        orch_combination,
                                                    };
                                                }
                                            }
                                            _ => {
                                                //todo logs this should not happen
                                            }
                                        }

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
                    if !self.buffer_planet_msg.is_empty() || !self.buffer_orchestrator_msg.is_empty() {
                        manage_buffer_msg(self)?;
                        if self.state==ExplorerState::Killed{
                            return Ok(())
                        }
                    }
                    else{
                        //todo ai
                    }
                }
            }
        }
    }
}

// this function put the explorer in the condition to receive messages (idle state),
// it is called when the explorer receives the StartExplorerAI message
pub fn start_explorer_ai(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::StartExplorerAIResult {explorer_id: explorer.explorer_id}) {
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Start explorer AI result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending start explorer AI result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function resets the topology known by the explorer,
// it is called when the explorer receives the ResetExplorerAI message
//todo not really sure but maybe i need to change this
pub fn reset_explorer_ai(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::ResetExplorerAIResult {explorer_id: explorer.explorer_id}
    ){
        Ok(_) => {
            // TODO reset anche dell'inventario?
            explorer.topology_info.clear();
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Reset explorer AI result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending reset explorer AI result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function put the explorer in the condition to wait for a StartExplorerAI message (WaitingToStartExplorerAI state),
// it is called when the explorer receives the StopExplorerAI message
pub fn stop_explorer_ai(explorer: &mut Explorer)->Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::StopExplorerAIResult {explorer_id: explorer.explorer_id}
    ){
        Ok(_) => {
            explorer.state = ExplorerState::WaitingToStartExplorerAI;
            println!("[EXPLORER DEBUG] Stop explorer AI result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending stop explorer AI result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function puts the explorer in the Killed state waiting for the thread to be killed
pub fn kill_explorer(explorer: &mut Explorer) ->Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::KillExplorerResult {explorer_id: explorer.explorer_id}
    ){
        Ok(_) => {
            explorer.state = ExplorerState::Killed;
            println!("[EXPLORER DEBUG] Kill explorer result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending kill explorer result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

// this function sets the sender_to_planet of the explorer struct
pub fn move_to_planet(
    explorer: &mut Explorer,
    sender_to_new_planet: Option<Sender<ExplorerToPlanet>>,
    planet_id: ID,
) -> Result<(), Box<dyn std::error::Error>> {
    explorer.state = ExplorerState::Idle;
    match sender_to_new_planet {
        Some(sender) => {
            explorer.planet_channels.1 = sender;
            explorer.planet_id = planet_id; //todo rimuovere next_planet_id
            println!("[EXPLORER DEBUG] Sender channel set correctly");
            //todo logs
            Ok(())
        }
        None => { //the explorer cannot move
            println!("[EXPLORER DEBUG] Sender channel is None.");
            //todo logs
            Err("Sender channel is None.".into())
        }
    }
}

// this function sends the current planet id to the orchestrator
pub fn current_planet_request(explorer: &mut Explorer)->Result<(), Box<dyn std::error::Error>> {
    match explorer.orchestrator_channels.1.send(
        ExplorerToOrchestrator::CurrentPlanetResult {
            explorer_id: explorer.explorer_id,
            planet_id: explorer.planet_id
        }
    ){
        Ok(_) => {
            explorer.state = ExplorerState::Idle;
            println!("[EXPLORER DEBUG] Current planet result sent correctly.");
            //todo logs
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error sending current planet result: {:?}",err);
            //todo logs
            Err(err.into())
        }
    }
}

pub fn gather_info_from_planet(explorer: &mut Explorer)->Result<(), Box<dyn std::error::Error>> {
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

// this function sends the basic resources supported by the current planet to the orchestrator
// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
// response and then send it back to the orchestrator)
pub fn supported_resource_request(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.topology_info.get(&explorer.planet_id){
        Some(planet_info) => {
            match &planet_info.basic_resources{
                Some(basic_resources) => {
                    explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedResourceResult {
                        explorer_id: explorer.explorer_id,
                        supported_resources: basic_resources.clone(),
                    })?;
                }
                None => {
                    //this should not happen
                    //todo logs
                    match explorer.state{
                        ExplorerState::Idle =>{
                            explorer.state = ExplorerState::Surveying{
                                resources: true,
                                combinations: false,
                                energy_cells: false,
                                orch_resource: true,
                                orch_combination: false,
                            };
                            gather_info_from_planet(explorer)?;
                        }
                        _=>{
                            //todo logs this should not happen
                        }
                    }
                }
            }
        }
        None => {
            //this should not happen
            //todo logs
            match explorer.state{
                ExplorerState::Idle =>{
                    explorer.state = ExplorerState::Surveying{
                        resources: true,
                        combinations: true,
                        energy_cells: true,
                        orch_resource: true,
                        orch_combination: false,
                    };
                    gather_info_from_planet(explorer)?;
                }
                _=>{
                    //todo logs this should not happen
                }
            }
        }
    }
    Ok(())
}

// this function sends the complex resources supported by the current planet to the orchestrator
// (if the explorer doesn't know the supported resources, it asks for them to the planet, wait for the
// response and then send it back to the orchestrator)
pub fn supported_combination_request(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.topology_info.get(&explorer.planet_id){
        Some(planet_info) => {
            match &planet_info.complex_resources{
                Some(complex_resource) => {
                    explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedCombinationResult {
                        explorer_id: explorer.explorer_id,
                        combination_list: complex_resource.clone(),
                    })?;
                }
                None => {
                    //this should not happen
                    //todo logs
                    match explorer.state{
                        ExplorerState::Idle =>{
                            explorer.state = ExplorerState::Surveying{
                                resources: false,
                                combinations: true,
                                energy_cells: false,
                                orch_resource: false,
                                orch_combination: true,
                            };
                            gather_info_from_planet(explorer)?;
                        }
                        _=>{
                            //todo logs this should not happen
                        }
                    }
                }
            }
        }
        None => {
            //this should not happen
            //todo logs
            match explorer.state{
                ExplorerState::Idle =>{
                    explorer.state = ExplorerState::Surveying{
                        resources: true,
                        combinations: true,
                        energy_cells: true,
                        orch_resource: false,
                        orch_combination: true,
                    };
                    gather_info_from_planet(explorer)?;
                }
                _=>{
                    //todo logs this should not happen
                }
            }
        }
    }
    Ok(())
}

// this function sends the GenerateResourceRequest, waits for the planet response, and,
// if successful puts the resource in the bag
pub fn generate_resource_request(explorer: &mut Explorer, to_generate: BasicResourceType, to_orchestrator:bool) -> Result<(), Box<dyn std::error::Error>> {
    explorer.state = ExplorerState::GeneratingResource {orchestrator_response:true};
    explorer.planet_channels.1.send(ExplorerToPlanet::GenerateResourceRequest {
        explorer_id: explorer.explorer_id,
        resource: to_generate,
    })?;
    Ok(())
}



// this function sends the CombineResourceRequest, waits for the planet response, and,
// if successful puts the resource in the bag
pub fn combine_resource_request(explorer: &mut Explorer, to_generate: ComplexResourceType) -> Result<(), Box<dyn std::error::Error>> {
    explorer.state = ExplorerState::CombiningResources {orchestrator_response:true};
    let complex_resource_req = match to_generate {
        //provide the requested resources from the bag for each combination
        ComplexResourceType::Diamond => explorer.bag.make_diamond_request(),
        ComplexResourceType::Water => explorer.bag.make_water_request(),
        ComplexResourceType::Life => explorer.bag.make_life_request(),
        ComplexResourceType::Robot => explorer.bag.make_robot_request(),
        ComplexResourceType::Dolphin => explorer.bag.make_dolphin_request(),
        ComplexResourceType::AIPartner => explorer.bag.make_ai_partner_request(),
    };
    match complex_resource_req {
        Ok(complex_resource_req) => {
            explorer.planet_channels.1.send(ExplorerToPlanet::CombineResourceRequest {
                explorer_id: explorer.explorer_id,
                msg: complex_resource_req,
            })?;
            Ok(())
        }
        Err(err) => {
            println!("[EXPLORER DEBUG] Error generating complex resource request {}",err);
            //todo logs
            explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::CombineResourceResponse {
                explorer_id:explorer.explorer_id,
                generated: Err(err),
            })?;
            Ok(())//this could happen and it is totally fine
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
    //todo logs
    match explorer.topology_info.get_mut(&explorer.planet_id){
        Some(planet_info) => {
            planet_info.neighbours = Some(neighbors.into_iter().collect());
        }
        None => {
            explorer.topology_info.insert(
                explorer.planet_id,
                PlanetInfo {
                    basic_resources: None,
                    complex_resources: None,
                    neighbours: Some(neighbors.into_iter().collect()),
                }
            );
        }
    }
}

pub fn manage_supported_resource_response(
    explorer: &mut Explorer,
    resource_list:HashSet<BasicResourceType>
) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::Surveying {resources:true ,combinations,energy_cells,orch_resource,orch_combination}=>{
            match explorer.topology_info.get_mut(&explorer.planet_id) {
                Some(planet_info) => {
                    planet_info.basic_resources = Some(resource_list.clone());
                }
                None => {
                    explorer.topology_info.insert(
                        explorer.planet_id,
                        PlanetInfo {
                            basic_resources: Some(resource_list.clone()),
                            complex_resources: None,
                            neighbours: None,
                        }
                    );
                }
            }
            if orch_resource{
                explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedResourceResult {
                    explorer_id: explorer.explorer_id,
                    supported_resources: resource_list
                })?;
            }
            if !combinations && !energy_cells {
                explorer.state = ExplorerState::Idle;
            }
            else{
                explorer.state = ExplorerState::Surveying {
                    resources:false,
                    combinations,
                    energy_cells,
                    orch_resource:false,
                    orch_combination,
                };
            }
        }
        _ => {
            //todo this should not happen but it is not a problem
        }
    }
    Ok(())
}

pub fn manage_supported_combination_response(
    explorer: &mut Explorer,
    combination_list:HashSet<ComplexResourceType>,
)-> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::Surveying {resources ,combinations:true,energy_cells,orch_resource,orch_combination}=>{
            match explorer.topology_info.get_mut(&explorer.planet_id) {
                Some(planet_info) => {
                    planet_info.complex_resources = Some(combination_list.clone());
                }
                None => {
                    explorer.topology_info.insert(
                        explorer.planet_id,
                        PlanetInfo {
                            basic_resources: None,
                            complex_resources: Some(combination_list.clone()),
                            neighbours: None,
                        }
                    );
                }
            }
            if orch_combination{
                explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::SupportedCombinationResult {
                    explorer_id: explorer.explorer_id,
                    combination_list
                })?;
            }
            if !resources && !energy_cells {
                explorer.state = ExplorerState::Idle;
            }
            else{
                explorer.state = ExplorerState::Surveying {
                    resources,
                    combinations:false,
                    energy_cells,
                    orch_resource,
                    orch_combination:false,
                };
            }
        }
        _ => {
            //todo this should not happen but it is not a problem
        }
    }
    Ok(())
}

pub fn manage_generate_response(
    explorer: &mut Explorer,
    resource: Option<BasicResource>,
)-> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::GeneratingResource {orchestrator_response}=>{
            match resource {
                Some(resource) => {
                    explorer.bag.insert(resource.res_to_generic());
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::GenerateResourceResponse {
                                explorer_id: explorer.explorer_id,
                                generated: Ok(())
                            }
                        )?;
                    }
                }
                None => {
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::GenerateResourceResponse {
                                explorer_id: explorer.explorer_id,
                                generated: Err("Cannot generate resource".to_string())
                            }
                        )?;
                    }
                }
            }
            explorer.state = ExplorerState::Idle;
        }
        _ => {
            //todo this should non happen
        }
    }
    Ok(())
}
pub fn manage_combine_response(
    explorer: &mut Explorer,
    complex_response:  Result<ComplexResource, (String, GenericResource, GenericResource)>
) -> Result<(), Box<dyn std::error::Error>> {
    match explorer.state {
        ExplorerState::CombiningResources {orchestrator_response}=>{
            match complex_response {
                Ok(complex_resource) => {
                    explorer.bag.insert(complex_resource.res_to_generic());
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                           ExplorerToOrchestrator::CombineResourceResponse {
                               explorer_id:explorer.explorer_id,
                               generated: Ok(())
                           }
                        )?;
                    }
                }
                Err((err,r1, r2))=>{
                    //todo logs
                    explorer.bag.insert(r1);
                    explorer.bag.insert(r2);
                    if orchestrator_response{
                        explorer.orchestrator_channels.1.send(
                           ExplorerToOrchestrator::CombineResourceResponse {
                               explorer_id: explorer.explorer_id,
                               generated: Err("Cannot combine resource".to_string())
                           }
                        )?;
                    }
                }
            }
            explorer.state = ExplorerState::Idle;
        }
        _ => {
            //todo this should non happen
        }
    }
    Ok(())
}


// this function manages all the messages that were put in the buffers
// (in the same way the explorer usually manages them)
pub fn manage_buffer_msg(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    if !explorer.buffer_orchestrator_msg.is_empty() {
        //this should never panic
        if orch_msg_match_state(&explorer.state, explorer.buffer_orchestrator_msg.front().unwrap()) {
            let msg=explorer.buffer_orchestrator_msg.pop_front().unwrap();
            match msg {
                OrchestratorToExplorer::StartExplorerAI => {
                    start_explorer_ai(explorer)?;
                }
                OrchestratorToExplorer::ResetExplorerAI => {
                    reset_explorer_ai(explorer)?;
                }
                OrchestratorToExplorer::StopExplorerAI => {
                    stop_explorer_ai(explorer)?;
                }
                OrchestratorToExplorer::KillExplorer => {
                    // I don't think it is possible to arrive here
                    kill_explorer(explorer)?;
                    return Ok(()) //todo gestire questo caso nel loop principale
                }
                OrchestratorToExplorer::MoveToPlanet {
                    sender_to_new_planet,
                    planet_id,
                } => {
                    move_to_planet(explorer, sender_to_new_planet, planet_id)?;
                }
                OrchestratorToExplorer::CurrentPlanetRequest => {
                    current_planet_request(explorer)?;
                }
                OrchestratorToExplorer::SupportedResourceRequest => {
                    supported_resource_request(explorer)?;
                }
                OrchestratorToExplorer::SupportedCombinationRequest => {
                    supported_combination_request(explorer)?;
                }
                OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                    generate_resource_request(explorer, to_generate, true)?;
                }
                OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                    combine_resource_request(explorer, to_generate)?;
                }
                OrchestratorToExplorer::BagContentRequest => {
                    // IMPORTANTE restituisce un vettore contenente i resource type e non gli item in se
                    explorer.orchestrator_channels.1.send(ExplorerToOrchestrator::BagContentResponse {explorer_id: explorer.explorer_id, bag_content: explorer.bag.to_resource_types()})?;
                }
                OrchestratorToExplorer::NeighborsResponse { neighbors } => {
                    neighbours_response(explorer, neighbors);
                }
            }
        }
    }
    if !explorer.buffer_planet_msg.is_empty() {
        //this should not panic
        if planet_msg_match_state(&explorer.state, explorer.buffer_planet_msg.front().unwrap()) {
            let msg=explorer.buffer_planet_msg.pop_front().unwrap();
            match msg {
                PlanetToExplorer::SupportedResourceResponse { resource_list } => {
                    manage_supported_resource_response(explorer, resource_list)?;
                }
                PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
                    manage_supported_combination_response(explorer, combination_list)?;
                }
                PlanetToExplorer::GenerateResourceResponse { resource } => {
                    manage_generate_response(explorer, resource)?;
                }
                PlanetToExplorer::CombineResourceResponse { complex_response } => {
                    manage_combine_response(explorer, complex_response)?;
                }
                PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                    match explorer.state{
                        ExplorerState::Surveying {resources,combinations,energy_cells:true,orch_resource,orch_combination}=>{
                            explorer.energy_cells = available_cells;
                            if !resources && !combinations{
                                explorer.state = ExplorerState::Idle;
                            }
                            else{
                                explorer.state = ExplorerState::Surveying {
                                    resources,
                                    combinations,
                                    energy_cells:false,
                                    orch_resource,
                                    orch_combination,
                                };
                            }
                        }
                        _ => {
                            //todo logs this should not happen
                        }
                    }
                }
                PlanetToExplorer::Stopped => {
                    // TODO gestire in base all'ai dell'explorer
                    explorer.state = ExplorerState::Idle;
                }
            }
        }
    }
    Ok(())
}
