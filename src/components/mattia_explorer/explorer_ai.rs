use crate::components::mattia_explorer::ActorType;
use crate::components::mattia_explorer::helpers::gather_info_from_planet;
use crate::components::mattia_explorer::planet_info::{PlanetClassType, PlanetInfo};
use crate::components::mattia_explorer::states::ExplorerState;
use crate::components::mattia_explorer::Explorer;
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use rand::Rng;
use std::collections::HashMap;
use std::hash::Hash;
use logging_utils::{log_fn_call, log_internal_op, LoggableActor};

/// Noise level for utility calculations
const RANDOMNESS_RANGE: f64 = 0.1;
/// Decay factor for outdated information
const LAMBDA: f32 = 0.005;
/// Resource need propagation (kept for future resource-gathering modes)
const PROPAGATION_FACTOR: f32 = 0.8;
// --- SAFETY THRESHOLDS ---
/// Critical danger threshold - immediate evacuation
const SAFETY_CRITICAL: f32 = 0.3;  //todo update this as the medium of the safeness of lowest 1/3 of the planet
/// Warning threshold - start looking for safer planets
const SAFETY_WARNING: f32 = 0.6; //todo update this as the medium of the safeness of medium 1/3 of the planet
/// Comfortable safety level
const SAFETY_COMFORTABLE: f32 = 0.85;  //todo update this as the medium of the safeness of highest 1/3 of the planet
// --- DEFENSE PROBABILITY THRESHOLDS ---
/// Energy cells threshold to assume planet has rockets
const ENERGY_CELLS_DEFENSE_THRESHOLD: u32 = 2;
/// Probability threshold to consider a planet "likely defended"
const HIGH_DEFENSE_PROBABILITY: f32 = 0.6;
/// Probability threshold to consider a planet "possibly defended"
const MEDIUM_DEFENSE_PROBABILITY: f32 = 0.3;
// --- INFORMATION STALENESS ---
/// Max age (in ticks) before neighbor info is considered stale
const MAX_NEIGHBOR_INFO_AGE: u64 = 100;
/// Max age before energy info is considered stale
const MAX_ENERGY_INFO_AGE: u64 = 50;
// --- EXPLORATION VS SAFETY BALANCE ---
/// Base utility for exploring unknown planets (when safe)
const EXPLORATION_BASE_UTILITY: f32 = 0.7;
/// Penalty multiplier for information staleness
const STALENESS_PENALTY_FACTOR: f32 = 0.01;
/// Minimum utility to consider moving (prevents thrashing)
const MIN_MOVEMENT_UTILITY: f32 = 0.4;//todo
// --- CHARGE RATE BASED PREDICTIONS ---
/// Minimum charge rate to consider planet "actively charging" (1 energy every 5 ticks)
const MIN_ACTIVE_CHARGE_RATE: f32 = 0.05;
/// Maximum ticks into future to predict (avoid over-optimistic projections)
const MAX_PREDICTION_HORIZON: u64 = 100;
/// maximum ticks for considering a value perfectly unchanged
const PERFECT_INFO_MAX_TIME: u64 = 10;

#[derive(Debug)]
enum AIActionType {
    Produce(BasicResourceType),
    Combine(ComplexResourceType),
    MoveTo(ID),
    SurveyNeighbors,
    SurveyEnergy,
    Wait,
    RunAway,
}
#[derive(Debug)]
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
            wait: 0.15,
            run_away: 0.0
        }
    }
}

//this is because just in case i need it but at the moment the ai will not have any
//benefit from producing any resources
#[derive(Debug)]
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
#[derive(Debug)]
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

        // e^(-lambda*delta_t)
        (-LAMBDA * delta_t).exp()
    }
}

fn calculate_max_number_cells(planet_info: &PlanetInfo) -> u32 {
    // Use inferred planet type if available
    if let Some(planet_type) = &planet_info.inferred_planet_type {
        planet_type.max_energy_cells()
    } else {
        // Default optimistic assumption if type not yet inferred
        3
    }
}

fn add_noise(value: f32) -> f32 {
    let mut rng = rand::rng();
    let noise = rng.random_range((1.0 - RANDOMNESS_RANGE)..=(1.0 + RANDOMNESS_RANGE));
    (value * noise as f32).clamp(0.0, 1.0)
}

fn predict_energy_cells(
    current_energy: Option<u32>,
    charge_rate: Option<f32>,
    time_elapsed: u64,
    max_cells: u32,
) -> u32 {

    let energy = current_energy.unwrap_or(1); //default value of 1 energy cells
    let rate = charge_rate.unwrap_or(0.0);
    // Cap prediction horizon to avoid over-optimism
    let prediction_time = time_elapsed.min(MAX_PREDICTION_HORIZON);

    // Calculate predicted energy accumulation
    let energy_gained = (rate * prediction_time as f32) as i32;

    // Cannot exceed max capacity
    (energy as i32).saturating_add(energy_gained).clamp(0, max_cells as i32) as u32
}

fn estimate_current_energy(
    planet_info: &PlanetInfo,
    current_time: u64,
) -> (u32, f32) {
    let time_elapsed = current_time.saturating_sub(planet_info.timestamp_energy);
    let max_cells = calculate_max_number_cells(planet_info);

    // Predict current energy
    let predicted_energy = predict_energy_cells(
        planet_info.energy_cells,
        planet_info.charge_rate,
        time_elapsed,
        max_cells,
    );

    // Confidence in prediction decreases with time elapsed
    let confidence = if planet_info.energy_cells.is_none() {
        // No energy info at all
        0.0
    } else if time_elapsed <= PERFECT_INFO_MAX_TIME {
        1.0 // Perfect information
    } else if time_elapsed <= MAX_ENERGY_INFO_AGE { // 1 to 0.5
        1.0 - (time_elapsed as f32 / (MAX_ENERGY_INFO_AGE as f32 * 2.0))
    } else {
        0.3 // Low confidence for very old data
    }.max(0.1); // Minimum confidence

    (predicted_energy, confidence)
}

pub fn calc_utility(explorer: &mut Explorer) -> Result<(), &'static str> {
    // updating planet safety score
    let known_ids: Vec<ID> = explorer.topology_info.keys().cloned().collect();
    for id in known_ids {
       match calculate_safety_score(explorer, Some(id)){
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
        // Movement towards known neighbors
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
    explorer.ai_data.ai_action.move_to=temp_move;
    explorer.ai_data.ai_action.produce_resource=temp_produce;
    explorer.ai_data.ai_action.combine_resource=temp_combine;

    //Survey utilities
    explorer.ai_data.ai_action.survey_energy_cells = score_survey_energy(explorer)?;
    explorer.ai_data.ai_action.survey_neighbors = score_survey_neighbors(explorer)?;

    // wait with bonus for positive planet charge rate
    let wait_base = 0.08f32;
    let wait_bonus = if charge_rate.is_some_and(|x| x> 0.0)  { 0.1 } else { 0.0 };
    explorer.ai_data.ai_action.wait = (wait_base + wait_bonus).clamp(0.0, 1.0);

    // calculating run away values:
    // using pow to make it more reactive when the safeness is low
    let safety_score = {
        explorer.get_current_planet_info()?.safety_score.unwrap_or(SAFETY_WARNING) //optimistic prediction
    };
    explorer.ai_data.ai_action.run_away = (1.0 - safety_score).powi(2).clamp(0.0, 1.0);
    Ok(())
}

fn score_basic_resource_production(
    explorer: &Explorer,
    resource_type: BasicResourceType,
) -> Result<f32, &'static str> {
    let planet_info = explorer.get_current_planet_info()?;

    let energy_cells = planet_info.energy_cells.unwrap_or(1).max(1);
    let resource_count = explorer.bag.count(ResourceType::Basic(resource_type)).max(1);
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time);

    let base = explorer.ai_data.resource_needs.get_effective_need(ResourceType::Basic(resource_type))
        * (1.0 / resource_count as f32)
        * (1.0 - (1.0 / energy_cells as f32))
        * (if planet_info.charge_rate.unwrap_or(0.0) > 0f32 { 1.0 } else { 0.8 })
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

    let energy_cells = planet_info.energy_cells.unwrap_or(1).max(1);
    let resource_count = explorer.bag.count(ResourceType::Complex(resource_type)).max(1);
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time);

    let mut base = explorer.ai_data.resource_needs.get_effective_need(ResourceType::Complex(resource_type))
        * (1.0 / resource_count as f32)
        * (1.0 - (1.0 / energy_cells as f32))
        * (if planet_info.charge_rate.unwrap_or(0.0) > 0f32 { 1.0 } else { 0.8 })
        * (reliability*0.2 +0.8); //in this case the reliability on the information about the energy cells it isn't very important

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


fn calculate_safety_score(explorer: &mut Explorer, planet_id:Option<ID>) -> Result<f32, &'static str>{
    let explorer_time=explorer.time.clone();
    let planet_info = match planet_id {
        Some(id) => explorer.get_planet_info_mut(id).ok_or("Planet info not found in topology")?,
        None => explorer.get_current_planet_info_mut()?
    };
    // Predict current energy considering charge rate
    let (predicted_energy, energy_confidence) = estimate_current_energy(planet_info, explorer_time);

    // Sustainability: planet can maintain or improve defense capability
    let sustainability = if planet_info.charge_rate.unwrap_or(0.0) > MIN_ACTIVE_CHARGE_RATE {
        1.0 // Actively charging
    } else if planet_info.charge_rate.unwrap_or(0.0) > 0.0 {
        0.7 // Slow charging
    } else {
        0.5 // Not charging
    };

    // Physical safety: based on predicted energy
    // Use predicted energy weighted by confidence
    // if confidence is low we use the last registered info
    let effective_energy = (predicted_energy as f32 * energy_confidence)
        + (planet_info.energy_cells.unwrap_or(1) as f32 * (1.0 - energy_confidence)); //default value of 1 energy cells
    let max_cells = calculate_max_number_cells(planet_info) as f32;


    // Physical safety scales with energy/max ratio
    let energy_ratio = (effective_energy / max_cells).clamp(0.0, 1.0);
    let physical_safety = if effective_energy >= ENERGY_CELLS_DEFENSE_THRESHOLD as f32 {
        0.6 + (energy_ratio * 0.4) // 0.6 to 1.0 for defended planets
    } else if effective_energy > 0.0 {
        0.3 + (energy_ratio * 0.3) // 0.3 to 0.6 for some energy
    } else {
        0.2 // Minimum baseline even with no energy
    };

    //calculating reliability of the topology data
    let neighbors_reliability = calculate_time_decay(planet_info.timestamp_neighbors, explorer_time);
    // Bonus for the connectivity
    let escape_factor = match planet_info.neighbors.as_ref() {
        None => {0.3} // Unknown neighbors = assume some exist
        Some(neighbours) => {
            match neighbours.len(){
                0 => 0.2,
                1 => 0.5,
                2 => 0.8,
                _ => 1.0,
            }
        }
    };
    let pessimistic_minimum = 0.15;
    let adjusted_escape_factor = (escape_factor * neighbors_reliability)
        + (pessimistic_minimum * (1.0 - neighbors_reliability));
    let rocket=if planet_info.inferred_planet_type.as_ref().is_some_and(|x| x.can_have_rocket()){
        1.0
    }else{
        0.5 // penalty because the planet does not have a rocket todo forse questi valori sono troppo radicali
    };
    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);
    let safety_score= (sustainability * physical_safety * adjusted_escape_factor*rocket*noise_factor).clamp(0.0, 1.0);
    planet_info.safety_score=Some(safety_score);
    Ok(safety_score)
}



//calculating the utility of updating neighbors
fn score_survey_neighbors(explorer: &Explorer) -> Result<f32, &'static str> { //todo tenere in conto il tempo anche
    let planet_info = explorer.get_current_planet_info()?;

    let reliability = calculate_time_decay(planet_info.timestamp_neighbors, explorer.time);

    // Base utility from staleness
    let staleness_component = (1.0 - reliability) * 0.7;

    // Small bonus if current planet is unsafe (want to know escape routes)
    let safety_bonus = if let Some(safety) = planet_info.safety_score {
        if safety < SAFETY_WARNING {
            0.2 // Moderate bonus when threatened
        } else {
            0.0
        }
    } else {
        0.1 // Unknown safety = some bonus
    };

    // High priority if we don't know neighbors at all
    let unknown_bonus = if planet_info.neighbors.is_none() {
        0.3
    } else {
        0.0
    };

    let base = 0.1 + staleness_component + safety_bonus + unknown_bonus;

    let noise = add_noise(1.0);

    // // critic information for navigation
    // // safety score is calculated on data eta, number of escape routes and defense capability
    // let base = ((1.0 - planet_info.safety_score.unwrap_or(SAFETY_WARNING)) * 0.9); //todo troppo influente, meglio dare priorità all'età dei dati
    //
    // let mut rng = rand::rng();
    // let noise: f32 = rng.random_range(0.95..=1.05);
    Ok((base * noise).clamp(0.0, 1.0))
}

// calculating the utility of updating energy cells
fn score_survey_energy(explorer: &Explorer) -> Result<f32, &'static str> {
    let planet_info = explorer.get_current_planet_info()?;

    // data reliability
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time);
    let energy_age = explorer.time.saturating_sub(planet_info.timestamp_energy);
    // NEW: If charge_rate is high, old data is VERY unreliable
    let charge_rate_uncertainty = if planet_info.charge_rate.unwrap_or(0.0) >= MIN_ACTIVE_CHARGE_RATE {
        // Fast charging planet: energy could have changed a lot
        let max_cells = calculate_max_number_cells(planet_info);
        //todo questa formula non mi convince
        let potential_change = (planet_info.charge_rate.unwrap_or(0.0) * energy_age as f32) / max_cells as f32;
        potential_change.min(0.5) // Cap at 0.5 additional uncertainty
    } else {
        0.0
    };

    // Base utility: increases with data staleness
    let staleness_component = (1.0 - reliability) * 0.5;

    // If we have no information at all, high priority
    let no_info_boost = if planet_info.energy_cells.is_none() {
        0.3
    } else {
        0.0
    };

    //if current safety is low, knowing energy is critical
    let threat_multiplier = if planet_info.safety_score.unwrap_or(SAFETY_WARNING) < SAFETY_WARNING {
        1.5
    } else {
        1.0
    };

    let base = (0.15 + staleness_component + charge_rate_uncertainty + no_info_boost) * threat_multiplier;

    let noise = add_noise(1.0);

    Ok((base * noise).clamp(0.0, 1.0))
}


// calculating the utility to move to near planet
// this need the run away factor to be already computed
fn score_move_to(explorer: &Explorer, target_id: ID) -> Result<f32, &'static str> {
    let target_info = explorer.get_planet_info(target_id).ok_or("Target planet info missing")?;
    let current_info = explorer.get_current_planet_info()?;

    let current_safety = current_info.safety_score.unwrap_or(SAFETY_WARNING);
    // Predict target energy
    let (predicted_target_energy, target_energy_confidence) =
        estimate_current_energy(target_info, explorer.time);


    if current_safety < SAFETY_WARNING {
        // Emergency mode: move towards safer planets
        let target_safety = target_info.safety_score.unwrap_or(SAFETY_WARNING);

        // Bonus for planets with good predicted energy (can defend)
        let energy_bonus = if predicted_target_energy >= 1 {
            0.2 * target_energy_confidence
        } else {
            0.0
        };

        // Bonus for actively charging planets (sustainable defense)
        let charge_bonus = if target_info.charge_rate.unwrap_or(0.0) > MIN_ACTIVE_CHARGE_RATE {
            0.1
        } else {
            0.0
        };

        let base_score = target_safety + energy_bonus + charge_bonus;
        let noise = add_noise(1.0);

        Ok((base_score * noise).clamp(0.0, 1.0))
    } else {
        // Exploration mode: move towards less known planets
        let data_reliability = calculate_time_decay(target_info.timestamp_neighbors, explorer.time);
        let exploration_value = 1.0 - data_reliability;

        // But still consider safety
        let safety_factor = if target_info.safety_score.unwrap_or(SAFETY_WARNING) < SAFETY_CRITICAL {
            0.5 // Penalize very dangerous planets
        } else {
            1.0
        };

        let base_score = exploration_value * safety_factor;
        let noise = add_noise(1.0);

        Ok((base_score * noise).clamp(0.0, 1.0))
    }
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
    //LOG
    log_fn_call!(
        explorer,
        "ai_core_function",
        explorer,
    );
    //LOG
    let base_resource =explorer.get_current_planet_info()?.basic_resources.is_none();
    let comp_resource= explorer.get_current_planet_info()?.complex_resources.is_none();
    if explorer.current_planet_neighbors_update || explorer.get_current_planet_info()?.neighbors.is_none(){
        log_internal_op!(
            explorer,
            "updating neighbors"
        );
        explorer.current_planet_neighbors_update=false;
        explorer.state=ExplorerState::WaitingForNeighbours;
        explorer.orchestrator_channels.1.send(
            ExplorerToOrchestrator::NeighborsRequest {
                explorer_id: explorer.explorer_id,
                current_planet_id: explorer.planet_id,
            }
        )?;
    }
    else if base_resource||comp_resource {
        log_internal_op!(
            explorer,
            "surveying resources"
        );
        explorer.state=ExplorerState::Surveying {
            resources: base_resource,
            combinations: comp_resource,
            energy_cells: false,
            orch_resource: false,
            orch_combination: false,
        };
        gather_info_from_planet(explorer)?;
    }
    else{
        calc_utility(explorer)?;
        log_internal_op!(
            explorer,
            "utility scores" => format!("{:?}",explorer.ai_data.ai_action),
            "explorer state" =>format!("{:?}", explorer),
        );
        let best_action=find_best_action(&explorer.ai_data.ai_action);
        log_internal_op!(
            explorer,
            "action to be taken" => format!("{:?}", best_action)
        );
        match best_action{
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