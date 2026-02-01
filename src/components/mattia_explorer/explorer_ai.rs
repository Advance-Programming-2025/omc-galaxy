use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, ResourceType};
use common_game::components::resource::GenericResource::BasicResources;
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use rand::Rng;
use crate::components::mattia_explorer::{Explorer, Bag};
use crate::components::mattia_explorer::helpers::gather_info_from_planet;
use crate::components::mattia_explorer::planet_info::PlanetInfo;
use crate::components::mattia_explorer::states::ExplorerState;

//this value will affect the noise level of utility calculations
const RANDOMNESS_RANGE: f64 =0.1;
//this value will influence how careful is the explorer in considering old values for utility calculations
const LAMBDA: f32=0.005;
const PROPAGATION_FACTOR: f32=0.8;
const SAFETY_TRESHOLD: f32=0.4; //todo update this dynamically

enum AIActionType {
    Produce(BasicResourceType),
    Combine(ComplexResourceType),
    MoveTo(ID),
    SurveyNeighbors,
    SurveyEnergy,
    Wait,
    RunAway,
}
pub struct AIAction{
    pub produce_resource:HashMap<BasicResourceType, f32>, //not sure if this will be useful, because I think it is useless to waste energy cell in making resources
    pub combine_resource:HashMap<ComplexResourceType, f32>,
    pub move_to:HashMap<ID, f32>,
    pub survey_energy_cells:f32,
    pub survey_neighbors:f32,
    pub wait:f32,
    pub run_away:f32
}
impl AIAction{
    pub fn new()->Self{
        let mut produce_resource:HashMap<BasicResourceType, f32>= HashMap::new();
        let mut combine_resource:HashMap<ComplexResourceType, f32> = HashMap::new();
        //basic
        produce_resource.insert(BasicResourceType::Silicon, 0.0);
        produce_resource.insert(BasicResourceType::Carbon, 0.0);
        produce_resource.insert(BasicResourceType::Oxygen, 0.0);
        produce_resource.insert(BasicResourceType::Hydrogen, 0.0);
        //complex
        combine_resource.insert(ComplexResourceType::Diamond,0.0);
        combine_resource.insert(ComplexResourceType::Robot, 0.0);
        combine_resource.insert(ComplexResourceType::Life, 0.0);
        combine_resource.insert(ComplexResourceType::Water, 0.0);
        combine_resource.insert(ComplexResourceType::AIPartner, 0.0);
        combine_resource.insert(ComplexResourceType::Dolphin, 0.0);
        AIAction{
            produce_resource,
            combine_resource,
            move_to: HashMap::new(),
            survey_energy_cells: 0.0,
            survey_neighbors: 0.0,
            wait: 0.2,
            run_away: 0.0
        }
    }
}

//this is because just in case i need it but at the moment the ai will not have any
//benefit from producing any resources
pub struct ResourceNeeds {
    oxygen: f32,
    carbon: f32,
    silicon: f32,
    hydrogen: f32,
    water: f32,
    life: f32,
    robot: f32,
    diamond: f32,
    ai_partner: f32,
    dolphin:f32
}

impl ResourceNeeds {
    pub fn new()->Self{
        Self{
            oxygen:0.0,
            carbon: 0.0,
            silicon: 0.0,
            hydrogen: 0.0,
            water: 0.0,
            life: 0.0,
            robot: 0.0,
            diamond: 0.0,
            ai_partner: 0.0,
            dolphin: 0.0,
        }
    }
    pub fn get(&self, resource_type: ResourceType)->f32{
        match resource_type {
            //basic
            ResourceType::Basic(BasicResourceType::Oxygen)=>self.oxygen,
            ResourceType::Basic(BasicResourceType::Hydrogen) => {self.hydrogen},
            ResourceType::Basic(BasicResourceType::Carbon) => {self.carbon},
            ResourceType::Basic(BasicResourceType::Silicon) => {self.silicon},
            //complex
            ResourceType::Complex(ComplexResourceType::Water) => {self.water},
            ResourceType::Complex(ComplexResourceType::Diamond) => {self.diamond},
            ResourceType::Complex(ComplexResourceType::Life) => {self.life}
            ResourceType::Complex(ComplexResourceType::Robot) => {self.robot}
            ResourceType::Complex(ComplexResourceType::Dolphin) => {self.dolphin}
            ResourceType::Complex(ComplexResourceType::AIPartner) => {self.ai_partner}
        }
    }
    pub fn get_mut(&mut self, resource_type: ResourceType) -> &mut f32 {
        match resource_type {
            // basic
            ResourceType::Basic(BasicResourceType::Oxygen) => &mut self.oxygen,
            ResourceType::Basic(BasicResourceType::Hydrogen) => &mut self.hydrogen,
            ResourceType::Basic(BasicResourceType::Carbon) => &mut self.carbon,
            ResourceType::Basic(BasicResourceType::Silicon) => &mut self.silicon,

            // complex
            ResourceType::Complex(ComplexResourceType::Water) => &mut self.water,
            ResourceType::Complex(ComplexResourceType::Diamond) => &mut self.diamond,
            ResourceType::Complex(ComplexResourceType::Life) => &mut self.life,
            ResourceType::Complex(ComplexResourceType::Robot) => &mut self.robot,
            ResourceType::Complex(ComplexResourceType::Dolphin) => &mut self.dolphin,
            ResourceType::Complex(ComplexResourceType::AIPartner) => &mut self.ai_partner,
        }
    }
    // return the total need of a resource
    pub fn get_effective_need(&self, resource: ResourceType) -> f32 {
        match resource {
            //level 4
            ResourceType::Complex(ComplexResourceType::AIPartner) => {
                self.ai_partner
            }

            // level 3
            ResourceType::Complex(ComplexResourceType::Robot) => {
                let ai_partner_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::AIPartner));
                (self.robot + ai_partner_need * PROPAGATION_FACTOR).min(1.0)
            }
            ResourceType::Complex(ComplexResourceType::Dolphin) => {
                self.dolphin
            }

            // level 2
            ResourceType::Complex(ComplexResourceType::Life) => {
                let robot_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Robot));
                let dolphin_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Dolphin));
                (self.life + robot_need * PROPAGATION_FACTOR + dolphin_need * PROPAGATION_FACTOR).min(1.0)
            }
            ResourceType::Complex(ComplexResourceType::Diamond) => {
                let ai_partner_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::AIPartner));
                (self.diamond + ai_partner_need * PROPAGATION_FACTOR).min(1.0)
            }

            // level 1
            ResourceType::Complex(ComplexResourceType::Water) => {
                let life_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Life));
                let dolphin_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Dolphin));
                (self.water + life_need * PROPAGATION_FACTOR + dolphin_need * PROPAGATION_FACTOR).min(1.0)
            }

            // level 0: basic resources
            ResourceType::Basic(BasicResourceType::Carbon) => {
                let diamond_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Diamond));
                let life_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Life));
                (self.carbon + diamond_need * PROPAGATION_FACTOR + life_need * PROPAGATION_FACTOR).min(1.0)
            }
            ResourceType::Basic(BasicResourceType::Oxygen) => {
                let water_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Water));
                (self.oxygen + water_need * PROPAGATION_FACTOR).min(1.0)
            }
            ResourceType::Basic(BasicResourceType::Hydrogen) => {
                let water_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Water));
                (self.hydrogen + water_need * PROPAGATION_FACTOR).min(1.0)
            }
            ResourceType::Basic(BasicResourceType::Silicon) => {
                let robot_need = self.get_effective_need(ResourceType::Complex(ComplexResourceType::Robot));
                (self.silicon + robot_need * PROPAGATION_FACTOR).min(1.0)
            }
        }
    }
}
pub struct ai_data{
    pub global_sunray_rate: f32, //todo i don't think these 2 values are useful
    pub global_asteroid_rate: f32,
    pub resource_needs: ResourceNeeds,
    pub ai_action: AIAction
}
impl ai_data {
    pub fn new()->Self{
        Self{
            global_asteroid_rate: 0.0,
            global_sunray_rate: 0.0,
            resource_needs: ResourceNeeds::new(),
            ai_action: AIAction::new()
        }
    }
}

fn calculate_time_decay(planet_timestamp: u64, current_time: u64) -> f32 {
    if planet_timestamp ==0{ //planet never visited
        0.0
    }
    else{
        let delta_t = (current_time - planet_timestamp) as f32;
        let lambda = 0.005;

        // e^(-lambda*delta_t)
        (-lambda * delta_t).exp()
    }
}

pub fn calc_utility(explorer: &mut Explorer) -> Result<(), &'static str> {
    // updating planet safety score
    let known_ids: Vec<ID> = explorer.topology_info.keys().cloned().collect();
    for id in known_ids {
       match update_planet_safety(explorer, id){
           Ok(_) => {}
           Err(err) => {
               //todo logs
           }
       }
    }
    //temporary variables
    let mut temp_produce = HashMap::new();
    let mut temp_combine = HashMap::new();
    let mut temp_move = HashMap::new();
    let charge_rate;
    //clearing move_to utility values
    explorer.ai_data.ai_action.move_to.clear();

    {
        // current planet info
        let planet_info = explorer.get_current_planet_info()?;
        charge_rate = planet_info.charge_rate;
        // base resource production
        let base_resources_present = planet_info.basic_resources.as_ref();
        let produce_keys: Vec<BasicResourceType> =
            explorer.ai_data.ai_action.produce_resource.keys().cloned().collect();
        // for every base resource in the planet production set updates the utility value
        for res_type in produce_keys {
            let score = match base_resources_present {
                Some(map) => {
                    if map.contains(&res_type) {
                        score_basic_resource_production(explorer, res_type)?
                    } else { // basic resource not found in the planet
                        0.0
                    }
                } // basic resource not found in the planet
                None => 0.0,
            };
            temp_produce.insert(res_type, score);
        }
        // complex resource utility calculation
        let complex_resources_present = planet_info.complex_resources.as_ref();
        let combine_keys: Vec<ComplexResourceType> =
            explorer.ai_data.ai_action.combine_resource.keys().cloned().collect();

        for res_type in combine_keys {
            let score = match complex_resources_present {
                Some(map) => {
                    if map.contains(&res_type) {
                        score_complex_resource_production(explorer, res_type)?
                    } else {
                        0.0
                    }
                }
                None => 0.0,
            };
            temp_combine.insert(res_type, score);
        }
        // 4) Movimento verso vicini noti
        if let Some(neighbors) = &planet_info.neighbors {
            for neighbor_id in neighbors {
                match score_move_to(explorer, *neighbor_id) {
                    Ok(score) => {
                        temp_move.insert(*neighbor_id, score);
                    }
                    Err(_) => {
                        temp_move.insert(*neighbor_id, 0.0);
                    }
                }
            }
        }
    }

    //Survey utilities
    explorer.ai_data.ai_action.survey_energy_cells = score_survey_energy(explorer)?;
    explorer.ai_data.ai_action.survey_neighbors = score_survey_neighbors(explorer)?;

    // wait with bonus for positive planet charge rate
    let wait_base = 0.2f32;
    let wait_bonus = if charge_rate > 0.0 { 0.1 } else { 0.0 };
    explorer.ai_data.ai_action.wait = (wait_base + wait_bonus).clamp(0.0, 1.0);

    // calculating run away values:
    // using pow to make it more reactive when the safeness is low
    let safety_score = {
        explorer.get_current_planet_info()?.safety_score
    };
    explorer.ai_data.ai_action.run_away = (1.0 - safety_score).powi(2).clamp(0.0, 1.0);
    Ok(())
}

fn score_basic_resource_production(
    explorer: &Explorer,
    resource_type: BasicResourceType,
) -> Result<f32, &'static str> {
    let planet_info = explorer.get_current_planet_info()?;

    let energy_cells = planet_info.energy_cells.max(1);
    let resource_count = explorer.bag.count(ResourceType::Basic(resource_type)).max(1);
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time);

    let base = explorer.ai_data.resource_needs.get_effective_need(ResourceType::Basic(resource_type))
        * (1.0 / resource_count as f32)
        * (1.0 - (1.0 / energy_cells as f32))
        * (if planet_info.charge_rate > 0f32 { 1.0 } else { 0.8 })
        * (reliability*0.2 +0.8); //in this case the reliability on the information about the energy cells it isn't very important

    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);

    Ok((base * noise_factor).clamp(0.0, 1.0))
}

fn score_complex_resource_production(
    explorer: &Explorer,
    resource_type: ComplexResourceType,
) -> Result<f32, &'static str> {
    let planet_info = explorer.get_current_planet_info()?;

    let energy_cells = planet_info.energy_cells.max(1);
    let resource_count = explorer.bag.count(ResourceType::Complex(resource_type)).max(1);
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time);

    let mut base = explorer.ai_data.resource_needs.get_effective_need(ResourceType::Complex(resource_type))
        * (1.0 / resource_count as f32)
        * (1.0 - (1.0 / energy_cells as f32))
        * (if planet_info.charge_rate > 0f32 { 1.0 } else { 0.8 })
        * (reliability*0.2 +0.8); //in this case the reliability on the information about the energy cells it isn't very important

    // can_craft ora viene chiamato direttamente da explorer.bag
    let (_, _, has_a, _, has_b) = explorer.bag.can_craft(resource_type);
    let readiness_factor = match (has_a, has_b) {
        (true, true) => 1.0,
        (true, false) | (false, true) => 0.666,
        (false, false) => 0.333,
    };

    base *= readiness_factor;

    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);

    Ok((base * noise_factor).clamp(0.0, 1.0))
}


fn calculate_safety_score(explorer: &mut Explorer) -> Result<f32, &'static str>{
    let explorer_time=explorer.time.clone();
    let planet_info = explorer.get_current_planet_info_mut()?;
    let sustainability = if planet_info.charge_rate>0f32{1.0}else{0.5};
    let physical_safety = 1.0 - (1.0 / planet_info.energy_cells.max(1) as f32);
    //calculating reliability of the topology data
    let neighbors_reliability = calculate_time_decay(planet_info.timestamp_neighbors, explorer_time);
    // Bonus for the connectivity
    let escape_factor = match planet_info.neighbors.as_ref() {
        None => {0.0}
        Some(neighbours) => {
            match neighbours.len(){
                0 => 0.0,
                1 => 0.4,
                2 => 0.8,
                _ => 1.0,
            }
        }
    };
    let pessimistic_minimum = 0.2;
    let adjusted_escape_factor = (escape_factor * neighbors_reliability)
        + (pessimistic_minimum * (1.0 - neighbors_reliability));
    let rocket=calculate_rocket_probability(planet_info)?;
    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);
    let safety_score= (sustainability * physical_safety * adjusted_escape_factor*rocket*noise_factor).clamp(0.0, 1.0);
    planet_info.safety_score=safety_score;
    Ok(safety_score)
}

fn update_planet_safety(explorer: &mut Explorer, planet_id: ID) -> Result<f32, &'static str> {
    let explorer_time = explorer.time;
    // Recuperiamo le info del pianeta specifico
    let planet_info = match explorer.get_planet_info_mut(planet_id){
        Some(planet_info) => planet_info,
        None => {Err("Planet not found")?}
    };

    let sustainability = if planet_info.charge_rate > 0.0 { 1.0 } else { 0.5 };
    let physical_safety = 1.0 - (1.0 / planet_info.energy_cells.max(1) as f32);

    // Affidabilità dei dati basata su quando è stata fatta l'ultima survey
    let neighbors_reliability = calculate_time_decay(planet_info.timestamp_neighbors, explorer_time);

    let escape_factor = match planet_info.neighbors.as_ref() {
        None => 0.0,
        Some(neighbours) => match neighbours.len() {
            0 => 0.0,
            1 => 0.4,
            2 => 0.8,
            _ => 1.0,
        },
    };

    let pessimistic_minimum = 0.2;
    let adjusted_escape_factor = (escape_factor * neighbors_reliability)
        + (pessimistic_minimum * (1.0 - neighbors_reliability));

    let rocket = calculate_rocket_probability(planet_info)?;

    // Aggiorniamo il campo interno per riferimenti futuri
    let safety_score = (sustainability * physical_safety * adjusted_escape_factor * rocket).clamp(0.0, 1.0);
    planet_info.safety_score = safety_score;

    Ok(safety_score)
}

fn calculate_rocket_probability(planet_info: &PlanetInfo) -> Result<f32, &'static str> {
    match (&planet_info.basic_resources, &planet_info.complex_resources){
        //this should not happen
        (None,_)=>{
            Err("planet_info.basic_resources are None")
        }
        (_, None)=>{
            Err("planet_info.complex_resources are None")
        }
        (Some(basic_resources),Some(complex_resources)) => {
            let comp_len= complex_resources.len();
            let base_len=basic_resources.len();
            if comp_len >1{
                //in this case the planet type is C
                Ok(1.0)
            }
            else if comp_len == 1{
                //in this case the planet could be also C, but B is more likely
                Ok(2.0)
            }
            else if base_len >1{
                //in this case the planet type is D
                Ok(2.0)
            }
            else{
                //in this case the planet could be D, but A is more likely
                Ok(1.0)
            }
        }

    }
}

//calculating the utility of updating neighbors
fn score_survey_neighbors(explorer: &Explorer) -> Result<f32, &'static str> {
    let planet_info = explorer.get_current_planet_info()?;
    // critic information for navigation
    // safety score is basically calculated on data eta and number of escape routes
    let base = ((1.0 - planet_info.safety_score) * 0.9);

    let mut rng = rand::rng();
    let noise: f32 = rng.random_range(0.95..=1.05);
    Ok((base * noise).clamp(0.0, 1.0))
}

// calculating the utility of updating energy cells
fn score_survey_energy(explorer: &Explorer) -> Result<f32, &'static str> {
    let planet_info = explorer.get_current_planet_info()?;

    // data reliability
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time);

    // not as important ad neighbors
    let base = 0.15 + (1.0 - reliability) * 0.5;

    let mut rng = rand::rng();
    let noise: f32 = rng.random_range(0.95..=1.05);

    Ok((base * noise).clamp(0.0, 1.0))
}


// calculating the utility to move to near planet
// this need the run away factor to be already computed
fn score_move_to(explorer: &Explorer, target_id: ID) -> Result<f32, &'static str> {
    let target_info = explorer.get_planet_info(target_id).ok_or("Target planet info missing")?;

    let current_safety = explorer.ai_data.ai_action.run_away;

    let base_score = if current_safety > SAFETY_TRESHOLD {
        // if this planet is safe we can explore the neighbors
        let data_reliability = calculate_time_decay(target_info.timestamp_neighbors, explorer.time);
        (1.0 - data_reliability)
    } else {
        // if this planet is not safe we have to escape
        target_info.safety_score
    };

    // adding noise
    let mut rng = rand::rng();
    let noise: f32 = rng.random_range(0.98..=1.02);

    Ok((base_score * noise).clamp(0.0, 1.0))
}

fn find_best_action(actions: &AIAction) -> Option<AIActionType> {
    let mut max_val = -1.0;
    let mut best = None;

    // runaway
    if actions.run_away > max_val {
        max_val = actions.run_away;
        best = Some(AIActionType::RunAway);
    }

    // MoveTo
    for (id, val) in &actions.move_to {
        if *val > max_val {
            max_val = *val;
            best = Some(AIActionType::MoveTo(*id));
        }
    }

    // Survey
    if actions.survey_neighbors > max_val {
        max_val = actions.survey_neighbors;
        best = Some(AIActionType::SurveyNeighbors);
    }

    if actions.survey_energy_cells > max_val {
        max_val = actions.survey_energy_cells;
        best = Some(AIActionType::SurveyEnergy);
    }

    // Production
    for (res, val) in &actions.produce_resource {
        if *val > max_val {
            max_val = *val;
            best = Some(AIActionType::Produce(*res));
        }
    }
    // combination
    for (res, val) in &actions.combine_resource {
        if *val > max_val {
            max_val = *val;
            best = Some(AIActionType::Combine(*res));
        }
    }

    // Wait
    if actions.wait > max_val {
        best = Some(AIActionType::Wait);
    }

    best
}

pub fn ai_core_function(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    let base_resource =explorer.get_current_planet_info()?.basic_resources.is_none();
    let comp_resource= explorer.get_current_planet_info()?.complex_resources.is_none();
    if explorer.current_planet_neighbors_update || explorer.get_current_planet_info()?.neighbors.is_none(){
        explorer.state=ExplorerState::WaitingForNeighbours;
        explorer.orchestrator_channels.1.send(
            ExplorerToOrchestrator::NeighborsRequest {
                explorer_id: explorer.explorer_id,
                current_planet_id: explorer.planet_id,
            }
        )?;
    }
    else if base_resource||comp_resource {
        explorer.state=ExplorerState::Surveying {
            resources: !base_resource,
            combinations: !comp_resource,
            energy_cells: false,
            orch_resource: false,
            orch_combination: false,
        };
        gather_info_from_planet(explorer)?;
    }
    else{
        calc_utility(explorer)?;
        match find_best_action(&explorer.ai_data.ai_action){
            Some(ai_action) => {
                match ai_action {
                    AIActionType::RunAway => {
                        let mut max:(&ID, &f32)=(&0, &0.0);
                        for planet in &explorer.ai_data.ai_action.move_to{
                            if planet.1>max.1{
                                max=planet
                            }
                        }
                        if *max.0!=0{ //making sure that there is a planet to move to
                            explorer.state=ExplorerState::Traveling;
                            explorer.orchestrator_channels.1.send(
                                ExplorerToOrchestrator::TravelToPlanetRequest {
                                    explorer_id: explorer.explorer_id,
                                    current_planet_id: explorer.planet_id,
                                    dst_planet_id: *max.0,
                                }
                            )?;
                        }
                    }
                    AIActionType::MoveTo(id) => {
                        explorer.state=ExplorerState::Traveling;
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::TravelToPlanetRequest {
                                explorer_id: explorer.explorer_id,
                                current_planet_id: explorer.planet_id,
                                dst_planet_id: id,
                            }
                        )?;
                    }
                    AIActionType::SurveyNeighbors => {
                        explorer.state=ExplorerState::WaitingForNeighbours;
                        explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::NeighborsRequest {
                                explorer_id: explorer.explorer_id,
                                current_planet_id: explorer.planet_id,
                            }
                        )?;
                    }
                    AIActionType::SurveyEnergy => {
                        explorer.state=ExplorerState::Surveying{
                            resources: false,
                            combinations: false,
                            energy_cells: true,
                            orch_resource: false,
                            orch_combination: false,
                        };
                        gather_info_from_planet(explorer)?;
                    }
                    AIActionType::Produce(res) => {
                        explorer.state=ExplorerState::GeneratingResource { orchestrator_response: false };
                        explorer.planet_channels.1.send(
                            ExplorerToPlanet::GenerateResourceRequest {
                                explorer_id: 0,
                                resource: res
                            }
                        )?;
                    }
                    AIActionType::Combine(res) => {
                        explorer.state=ExplorerState::GeneratingResource { orchestrator_response: false };
                        let complex_resource_req = match res {
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
                            }
                            Err(err) => {
                                //todo logs
                            }
                        }
                    }
                    AIActionType::Wait => {}
                }
            }
            None => {
                //wait
            }
        }

    }
    Ok(())
}