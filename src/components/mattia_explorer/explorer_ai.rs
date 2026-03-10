use crate::components::mattia_explorer::ai_params::AiParams;
use crate::components::mattia_explorer::helpers::gather_info_from_planet;
use crate::components::mattia_explorer::planet_info::PlanetInfo;
use crate::components::mattia_explorer::states::ExplorerState;
use crate::components::mattia_explorer::ActorType;
use crate::components::mattia_explorer::Explorer;
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use logging_utils::{log_fn_call, log_internal_op, LoggableActor};
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AIActionType {
    Produce(BasicResourceType),
    Combine(ComplexResourceType),
    MoveTo(ID),
    SurveyNeighbors,
    SurveyEnergy,
    Wait,
    RunAway,
}
#[derive(Debug)]
pub struct AIAction {
    pub produce_resource: HashMap<BasicResourceType, f32>, //not sure if this will be useful, because I think it is useless to waste energy cell in making resources
    pub combine_resource: HashMap<ComplexResourceType, f32>,
    pub move_to: HashMap<ID, f32>,
    pub survey_energy_cells: f32,
    pub survey_neighbors: f32,
    pub wait: f32,
    pub run_away: f32,
}
impl AIAction {
    pub fn new() -> Self {
        let mut produce_resource: HashMap<BasicResourceType, f32> = HashMap::new();
        let mut combine_resource: HashMap<ComplexResourceType, f32> = HashMap::new();
        //basic
        produce_resource.insert(BasicResourceType::Silicon, 0.0);
        produce_resource.insert(BasicResourceType::Carbon, 0.0);
        produce_resource.insert(BasicResourceType::Oxygen, 0.0);
        produce_resource.insert(BasicResourceType::Hydrogen, 0.0);
        //complex
        combine_resource.insert(ComplexResourceType::Diamond, 0.0);
        combine_resource.insert(ComplexResourceType::Robot, 0.0);
        combine_resource.insert(ComplexResourceType::Life, 0.0);
        combine_resource.insert(ComplexResourceType::Water, 0.0);
        combine_resource.insert(ComplexResourceType::AIPartner, 0.0);
        combine_resource.insert(ComplexResourceType::Dolphin, 0.0);
        AIAction {
            produce_resource,
            combine_resource,
            move_to: HashMap::new(),
            survey_energy_cells: 0.0,
            survey_neighbors: 0.0,
            wait: 0.15,
            run_away: 0.0,
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
    dolphin: f32,
}
impl ResourceNeeds {
    pub fn new() -> Self {
        Self {
            oxygen: 0.0,
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
    // return the total need of a resource
    pub fn get_effective_need(&self, resource: ResourceType, params: &AiParams) -> f32 {
        let pf = params.propagation_factor;
        match resource {
            //level 4
            ResourceType::Complex(ComplexResourceType::AIPartner) => self.ai_partner,

            // level 3
            ResourceType::Complex(ComplexResourceType::Robot) => {
                let ai_partner_need = self.get_effective_need(
                    ResourceType::Complex(ComplexResourceType::AIPartner),
                    params,
                );
                (self.robot + ai_partner_need * pf).min(1.0)
            }
            ResourceType::Complex(ComplexResourceType::Dolphin) => self.dolphin,

            // level 2
            ResourceType::Complex(ComplexResourceType::Life) => {
                let robot_need = self
                    .get_effective_need(ResourceType::Complex(ComplexResourceType::Robot), params);
                let dolphin_need = self.get_effective_need(
                    ResourceType::Complex(ComplexResourceType::Dolphin),
                    params,
                );
                (self.life + robot_need * pf + dolphin_need * pf).min(1.0)
            }
            ResourceType::Complex(ComplexResourceType::Diamond) => {
                let ai_partner_need = self.get_effective_need(
                    ResourceType::Complex(ComplexResourceType::AIPartner),
                    params,
                );
                (self.diamond + ai_partner_need * pf).min(1.0)
            }

            // level 1
            ResourceType::Complex(ComplexResourceType::Water) => {
                let life_need = self
                    .get_effective_need(ResourceType::Complex(ComplexResourceType::Life), params);
                let dolphin_need = self.get_effective_need(
                    ResourceType::Complex(ComplexResourceType::Dolphin),
                    params,
                );
                (self.water + life_need * pf + dolphin_need * pf).min(1.0)
            }

            // level 0: basic resources
            ResourceType::Basic(BasicResourceType::Carbon) => {
                let diamond_need = self.get_effective_need(
                    ResourceType::Complex(ComplexResourceType::Diamond),
                    params,
                );
                let life_need = self
                    .get_effective_need(ResourceType::Complex(ComplexResourceType::Life), params);
                (self.carbon + diamond_need * pf + life_need * pf).min(1.0)
            }
            ResourceType::Basic(BasicResourceType::Oxygen) => {
                let water_need = self
                    .get_effective_need(ResourceType::Complex(ComplexResourceType::Water), params);
                (self.oxygen + water_need * pf).min(1.0)
            }
            ResourceType::Basic(BasicResourceType::Hydrogen) => {
                let water_need = self
                    .get_effective_need(ResourceType::Complex(ComplexResourceType::Water), params);
                (self.hydrogen + water_need * pf).min(1.0)
            }
            ResourceType::Basic(BasicResourceType::Silicon) => {
                let robot_need = self
                    .get_effective_need(ResourceType::Complex(ComplexResourceType::Robot), params);
                (self.silicon + robot_need * pf).min(1.0)
            }
        }
    }
}
#[derive(Debug)]
pub struct AiData {
    pub resource_needs: ResourceNeeds,
    pub ai_action: AIAction,
    pub last_action: Option<AIActionType>,
    pub last_action_planet_id: Option<ID>,
    pub params: AiParams,
}
impl AiData {
    pub fn new(params: AiParams) -> Self {
        Self {
            resource_needs: ResourceNeeds::new(),
            ai_action: AIAction::new(),
            last_action: None,
            last_action_planet_id: None,
            params,
        }
    }
}

fn calculate_time_decay(planet_timestamp: u64, current_time: u64, params: &AiParams) -> f32 {
    if planet_timestamp == 0 {
        //planet never visited
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)]
        let delta_t = (current_time - planet_timestamp) as f32;

        // e^(-lambda*delta_t)
        (-params.lambda * delta_t).exp()
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

fn add_noise(value: f32, params: &AiParams) -> f32 {
    let mut rng = rand::rng();
    let noise = rng.random_range((1.0 - params.randomness_range)..=(1.0 + params.randomness_range));
    #[allow(clippy::cast_possible_truncation)]
    (value * noise as f32).clamp(0.0, 1.0)
}

fn predict_energy_cells(
    current_energy: Option<u32>,
    charge_rate: Option<f32>,
    time_elapsed: u64,
    max_cells: u32,
    params: &AiParams,
) -> u32 {
    let energy = current_energy.unwrap_or(1); //default value of 1 energy cells
    let rate = charge_rate.unwrap_or(0.0);
    // Cap prediction horizon to avoid over-optimism
    let prediction_time = time_elapsed.min(params.max_prediction_horizon);

    // Calculate predicted energy accumulation
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let energy_gained = (rate * prediction_time as f32) as i32;

    // Cannot exceed max capacity
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let result = (energy as i32)
        .saturating_add(energy_gained)
        .clamp(0, max_cells as i32) as u32;
    result
}

fn estimate_current_energy(
    planet_info: &PlanetInfo,
    current_time: u64,
    params: &AiParams,
) -> (u32, f32) {
    let time_elapsed = current_time.saturating_sub(planet_info.timestamp_energy);
    let max_cells = calculate_max_number_cells(planet_info);

    // Predict current energy
    let predicted_energy = predict_energy_cells(
        planet_info.energy_cells,
        planet_info.charge_rate,
        time_elapsed,
        max_cells,
        params,
    );

    // Confidence in prediction decreases with time elapsed
    let confidence = if planet_info.energy_cells.is_none() {
        // No energy info at all
        0.0
    } else if time_elapsed <= params.perfect_info_max_time {
        1.0 // Perfect information
    } else if time_elapsed <= params.max_energy_info_age {
        // 1 to 0.5
        #[allow(clippy::cast_precision_loss)]
        let decay = time_elapsed as f32 / (params.max_energy_info_age as f32 * 2.0);
        1.0 - decay
    } else {
        0.3 // Low confidence for very old data
    }
    .max(0.1); // Minimum confidence

    (predicted_energy, confidence)
}

pub fn calc_utility(explorer: &mut Explorer) -> Result<(), String> {
    // updating planet safety score for every known ids
    let known_ids: Vec<ID> = explorer.topology_info.keys().copied().collect();
    for id in known_ids {
        let _ = calculate_safety_score(explorer, Some(id));
    }
    //temporary variables to contain results
    let mut temp_produce = HashMap::new();
    let mut temp_combine = HashMap::new();
    let mut temp_move = HashMap::new();
    let charge_rate;
    //clearing move_to utility values
    explorer.ai_data.ai_action.move_to.clear();
    {
        // getting current planet info
        let planet_info = explorer.get_current_planet_info()?;
        charge_rate = planet_info.charge_rate;
        // base resource production
        let base_resources_present = planet_info.basic_resources.as_ref();
        let produce_keys: Vec<BasicResourceType> = explorer
            .ai_data
            .ai_action
            .produce_resource
            .keys()
            .copied()
            .collect();
        // for every base resource in the planet production set updates the utility value
        for res_type in produce_keys {
            let score = match base_resources_present {
                Some(map) => {
                    if map.contains(&res_type) {
                        score_basic_resource_production(explorer, res_type)?
                    } else {
                        // basic resource not found in the planet
                        0.0
                    }
                } // basic resource not found in the planet
                None => 0.0,
            };
            temp_produce.insert(res_type, score);
        }
        // complex resource utility calculation
        let complex_resources_present = planet_info.complex_resources.as_ref();
        let combine_keys: Vec<ComplexResourceType> = explorer
            .ai_data
            .ai_action
            .combine_resource
            .keys()
            .copied()
            .collect();

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
    explorer.ai_data.ai_action.move_to = temp_move;
    explorer.ai_data.ai_action.produce_resource = temp_produce;
    explorer.ai_data.ai_action.combine_resource = temp_combine;

    //Survey energy and neighbors
    explorer.ai_data.ai_action.survey_energy_cells = score_survey_energy(explorer)?;
    explorer.ai_data.ai_action.survey_neighbors = score_survey_neighbors(explorer)?;

    // wait with bonus for safe planet: high charge rate and safety
    let params = &explorer.ai_data.params;
    let wait_base = params.wait_base;
    let wait_bonus = if charge_rate.is_some_and(|x| x > 0.0)
        && explorer
            .get_current_planet_info()?
            .inferred_planet_type
            .as_ref()
            .is_some_and(super::planet_info::PlanetClassType::can_have_rocket)
    {
        params.wait_bonus
    } else {
        0.0
    };
    explorer.ai_data.ai_action.wait = (wait_base + wait_bonus).clamp(0.0, 1.0);

    // calculating run away values:
    // using pow to make it more reactive when the safeness is low
    let safety_warning = explorer.ai_data.params.safety_warning;
    let safety_score = {
        explorer
            .get_current_planet_info()?
            .safety_score
            .unwrap_or(safety_warning) //optimistic prediction
    };
    explorer.ai_data.ai_action.run_away = (1.0 - safety_score).powi(2).clamp(0.0, 1.0);
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn score_basic_resource_production(
    explorer: &Explorer,
    resource_type: BasicResourceType,
) -> Result<f32, &'static str> {
    let params = &explorer.ai_data.params;
    //get current planet info
    let planet_info = explorer.get_current_planet_info()?;

    let energy_cells = planet_info.energy_cells.unwrap_or(1).max(1);
    //total resource in the bag
    let resource_count = explorer
        .bag
        .count(ResourceType::Basic(resource_type))
        .max(1);
    //calculating reliability of the energy data
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time, params);

    let base = explorer
        .ai_data
        .resource_needs
        .get_effective_need(ResourceType::Basic(resource_type), params)
        * (1.0 / resource_count as f32) //less resource -> more needs
        * (1.0 - (1.0 / energy_cells as f32)) //less energy cells -> more conservative
        * (if planet_info.charge_rate.unwrap_or(0.0) > 0f32 { //considering charge rate
            1.0
        } else {
            0.8
        })
        * (reliability * 0.2 + 0.8); //in this case the reliability on the information about the energy cells it isn't very important
                                     //adding some randomness
    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);

    Ok((base * noise_factor).clamp(0.0, 1.0))
}

#[allow(clippy::cast_precision_loss)]
fn score_complex_resource_production(
    explorer: &Explorer,
    resource_type: ComplexResourceType,
) -> Result<f32, &'static str> {
    let params = &explorer.ai_data.params;
    //getting info
    let planet_info = explorer.get_current_planet_info()?;

    let energy_cells = planet_info.energy_cells.unwrap_or(1).max(1);
    //calculating number of complex resources
    let resource_count = explorer
        .bag
        .count(ResourceType::Complex(resource_type))
        .max(1);
    //reliability of energy data
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time, params);

    let mut base = explorer
        .ai_data
        .resource_needs
        .get_effective_need(ResourceType::Complex(resource_type), params) //getting needs of resources
        * (1.0 / resource_count as f32)  //less resource -> more needs
        * (1.0 - (1.0 / energy_cells as f32)) //less energy cells -> more conservative
        * (if planet_info.charge_rate.unwrap_or(0.0) > 0f32 { //considering charge rate
            1.0
        } else {
            0.8
        })
        * (reliability * 0.2 + 0.8); //in this case the reliability on the information about the energy cells it isn't very important

    let (_, _, has_a, _, has_b) = explorer.bag.can_craft(resource_type); //considering if the explorer has the necessary resource
    let readiness_factor = match (has_a, has_b) {
        (true, true) => 1.0,
        (true, false) | (false, true) => 0.666,
        (false, false) => 0.333,
    };

    base *= readiness_factor;
    //adding some randomness
    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);

    Ok((base * noise_factor).clamp(0.0, 1.0))
}
//very important
#[allow(clippy::cast_precision_loss)]
fn calculate_safety_score(
    explorer: &mut Explorer,
    planet_id: Option<ID>,
) -> Result<f32, &'static str> {
    let params = explorer.ai_data.params.clone();
    let explorer_time = explorer.time; //getting explorer ai tick
    let planet_info = match planet_id {
        //getting planet info
        Some(id) => explorer
            .get_planet_info_mut(id)
            .ok_or("Planet info not found in topology")?,
        None => explorer.get_current_planet_info_mut()?,
    };
    // Predict current energy considering charge rate
    let (predicted_energy, energy_confidence) =
        estimate_current_energy(planet_info, explorer_time, &params);

    // Sustainability: planet can maintain or improve defense capability
    let sustainability = if planet_info.charge_rate.unwrap_or(0.0) > params.min_active_charge_rate {
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
    let physical_safety = if effective_energy >= params.energy_cells_defense_threshold as f32 {
        0.6 + (energy_ratio * 0.4) // 0.6 to 1.0 for defended planets
    } else if effective_energy > 0.0 {
        0.3 + (energy_ratio * 0.3) // 0.3 to 0.6 for some energy
    } else {
        0.2 // Minimum baseline even with no energy
    };

    //calculating reliability of the topology data
    let neighbors_reliability =
        calculate_time_decay(planet_info.timestamp_neighbors, explorer_time, &params);
    // Bonus for the connectivity
    let escape_factor = match planet_info.neighbors.as_ref() {
        None => 0.3, // Unknown neighbors = assume some exist
        Some(neighbours) => match neighbours.len() {
            0 => 0.2,
            1 => 0.5,
            2 => 0.8,
            _ => 1.0,
        },
    };
    let pessimistic_minimum = 0.15;
    let adjusted_escape_factor = (escape_factor * neighbors_reliability) //adjusting escape factor with reliability
        + (pessimistic_minimum * (1.0 - neighbors_reliability));
    let rocket = if planet_info //checking if the planet can have a rocket
        .inferred_planet_type
        .as_ref()
        .is_some_and(super::planet_info::PlanetClassType::can_have_rocket)
    {
        1.0
    } else {
        0.5 // penalty because the planet does not have a rocket
    };
    //adding some randomness
    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);
    //calculating safety score as sum of weighted factors
    let safety_score = ((sustainability * params.safety_weight_sustainability
        + physical_safety * rocket * params.safety_weight_physical
        + adjusted_escape_factor * params.safety_weight_escape)
        * noise_factor)
        .clamp(0.0, 1.0);
    planet_info.safety_score = Some(safety_score);
    Ok(safety_score)
}

//calculating the utility of updating neighbors
fn score_survey_neighbors(explorer: &Explorer) -> Result<f32, &'static str> {
    let params = &explorer.ai_data.params;
    //getting planet info
    let planet_info = explorer.get_current_planet_info()?;
    //getting reliability of neighbors data
    let reliability = calculate_time_decay(planet_info.timestamp_neighbors, explorer.time, params);

    // Base utility from staleness
    let staleness_component = (1.0 - reliability) * 0.7;

    // Small bonus if current planet is unsafe (want to know escape routes)
    let safety_bonus = if let Some(safety) = planet_info.safety_score {
        if safety < params.safety_warning {
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
    //adding some randomness
    let noise = add_noise(1.0, params);

    Ok((base * noise).clamp(0.0, 1.0))
}

// calculating the utility of updating energy cells
#[allow(clippy::cast_precision_loss)]
fn score_survey_energy(explorer: &Explorer) -> Result<f32, &'static str> {
    let params = &explorer.ai_data.params;
    //getting planet info
    let planet_info = explorer.get_current_planet_info()?;

    // data reliability
    let reliability = calculate_time_decay(planet_info.timestamp_energy, explorer.time, params);
    let energy_age = explorer.time.saturating_sub(planet_info.timestamp_energy);
    //if charge_rate is high, old data is VERY unreliable
    let charge_rate_uncertainty =
        if planet_info.charge_rate.unwrap_or(0.0) >= params.min_active_charge_rate {
            // Fast charging planet: energy could have changed a lot
            let max_cells = calculate_max_number_cells(planet_info);
            let potential_change =
                (planet_info.charge_rate.unwrap_or(0.0) * energy_age as f32) / max_cells as f32;
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
    let threat_multiplier = if planet_info.safety_score.unwrap_or(params.safety_warning)
        < params.safety_warning
        && planet_info
            .inferred_planet_type
            .as_ref()
            .is_some_and(super::planet_info::PlanetClassType::can_have_rocket)
    {
        1.3
    } else {
        1.0
    };

    let base =
        (0.15 + staleness_component + charge_rate_uncertainty + no_info_boost) * threat_multiplier;
    //adding some noise
    let noise = add_noise(1.0, params);

    Ok((base * noise).clamp(0.0, 1.0))
}

// calculating the utility to move to near planet
// this need the run away factor to be already computed
fn score_move_to(explorer: &Explorer, target_id: ID) -> Result<f32, &'static str> {
    let params = &explorer.ai_data.params;
    //getting target planet info
    let target_info = explorer
        .get_planet_info(target_id)
        .ok_or("Target planet info missing")?;
    //getting current planet info
    let current_info = explorer.get_current_planet_info()?;
    //getting safety score
    let current_safety = current_info.safety_score.unwrap_or(params.safety_warning);
    // Predict target energy
    let (predicted_target_energy, target_energy_confidence) =
        estimate_current_energy(target_info, explorer.time, params);

    if current_safety < params.safety_warning {
        // Emergency mode: move towards safer planets
        let target_safety = target_info.safety_score.unwrap_or(params.safety_warning);

        // Bonus for planets with good predicted energy (can defend)
        let energy_bonus = if predicted_target_energy >= 1 {
            0.2 * target_energy_confidence
        } else {
            0.0
        };

        // Bonus for actively charging planets (sustainable defense)
        let charge_bonus = if target_info.charge_rate.unwrap_or(0.0) > params.min_active_charge_rate
        {
            0.1
        } else {
            0.0
        };

        let base_score = target_safety + energy_bonus + charge_bonus;
        let noise = add_noise(1.0, params);

        Ok((base_score * noise).clamp(0.0, 1.0))
    } else {
        // Exploration mode: move towards less known planets
        let data_reliability =
            calculate_time_decay(target_info.timestamp_neighbors, explorer.time, params);
        let exploration_value = 1.0 - data_reliability;

        // But still consider safety
        let safety_factor =
            if target_info.safety_score.unwrap_or(params.safety_warning) < params.safety_critical {
                0.3 // Penalize very dangerous planets
            } else if target_info.safety_score.unwrap_or(params.safety_warning)
                < explorer
                    .get_current_planet_info()?
                    .safety_score
                    .unwrap_or(params.safety_warning)
            {
                //some penalization for less safe planets
                0.6
            } else {
                0.8
            };

        let base_score = exploration_value * safety_factor;
        let noise = add_noise(1.0, params);

        Ok((base_score * noise).clamp(0.0, 1.0))
    }
}
//used to check if the explorer can safely escape, or if it is even useful
fn can_run_away(actions: &AIAction, explorer: &Explorer) -> bool {
    let params = &explorer.ai_data.params;
    if actions.run_away <= 0.0 {
        return false;
    }
    let Ok(current_info) = explorer.get_current_planet_info() else {
        return false;
    };
    let current_safety = current_info.safety_score.unwrap_or(params.safety_warning);
    let Some(neighbors) = &current_info.neighbors else {
        return false;
    };
    for neighbor_id in neighbors {
        let Some(planet_info) = explorer.topology_info.get(neighbor_id) else {
            continue;
        };
        let planet_safety = planet_info.safety_score.unwrap_or(params.safety_warning);
        if planet_safety > current_safety + params.safety_min_diff
            || planet_info.inferred_planet_type.is_none()
            || current_safety <= params.safety_critical
        {
            //if the destination planet is more safe or in some optimistic scenario or with panic
            return true;
        }
    }
    false
}
//this function uses the previus action taken in order to check the utility value now
//to see if it is still useful
fn action_utility(
    actions: &AIAction,
    action: &AIActionType,
    explorer: &Explorer,
    last_action_planet_id: Option<ID>,
) -> Option<f32> {
    if last_action_planet_id != Some(explorer.planet_id) {
        return None;
    }
    match action {
        AIActionType::Produce(resource) => actions.produce_resource.get(resource).copied(),
        AIActionType::Combine(resource) => actions.combine_resource.get(resource).copied(),
        AIActionType::MoveTo(id) => actions.move_to.get(id).copied(),
        AIActionType::SurveyNeighbors => Some(actions.survey_neighbors),
        AIActionType::SurveyEnergy => Some(actions.survey_energy_cells),
        AIActionType::Wait => Some(actions.wait),
        AIActionType::RunAway => {
            if can_run_away(actions, explorer) {
                Some(actions.run_away)
            } else {
                None
            }
        }
    }
}

//function used to take the best action
fn find_best_action(
    actions: &AIAction,
    explorer: &Explorer,
    last_action: Option<&AIActionType>,
    last_action_planet_id: Option<ID>,
) -> Option<AIActionType> {
    let params = &explorer.ai_data.params;
    let mut max_val = -1.0;
    let mut best: Option<AIActionType> = None;

    // MoveTo
    for (id, val) in &actions.move_to {
        //in order to reduce ping pong between 2 planets
        if *val > max_val
            && explorer
                .ai_data
                .last_action_planet_id
                .is_some_and(|x| x != *id)
        {
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
        max_val = actions.wait;
        best = Some(AIActionType::Wait);
    }

    // runaway
    if can_run_away(actions, explorer) && actions.run_away > max_val {
        max_val = actions.run_away;
        best = Some(AIActionType::RunAway);
    }
    //if it is still useful we can take the same action of before reducing hysteresis and ping pong
    if let Some(previous) = last_action
        && let Some(previous_val) =
            action_utility(actions, previous, explorer, last_action_planet_id)
    {
        if best.is_none() {
            return Some(previous.clone());
        }
        if previous_val + params.action_hysteresis_margin >= max_val {
            return Some(previous.clone());
        }
    }

    best
}

// ai core function that is called at every explorer cycle
#[allow(clippy::too_many_lines)]
pub fn ai_core_function(explorer: &mut Explorer) -> Result<(), Box<dyn std::error::Error>> {
    //LOG
    log_fn_call!(explorer, "ai_core_function", explorer,);
    //LOG
    let base_resource = explorer
        .get_current_planet_info()?
        .basic_resources
        .is_none();
    let comp_resource = explorer
        .get_current_planet_info()?
        .complex_resources
        .is_none();
    //used the first time the explorer get on a new planet to bypass the ai and survey directly the neighbors or resources
    if explorer.current_planet_neighbors_update
        || explorer.get_current_planet_info()?.neighbors.is_none()
    {
        log_internal_op!(explorer, "updating neighbors");
        explorer.current_planet_neighbors_update = false;
        explorer.state = ExplorerState::WaitingForNeighbours;
        match explorer
            .orchestrator_channels
            .1
            .send(ExplorerToOrchestrator::NeighborsRequest {
                explorer_id: explorer.explorer_id,
                current_planet_id: explorer.planet_id,
            }) {
            Ok(()) => {
                return Ok(());
            }
            Err(err) => {
                explorer.state = ExplorerState::Idle;
                return Err(Box::new(err));
            }
        }
    } else if base_resource || comp_resource {
        log_internal_op!(explorer, "surveying resources");
        explorer.state = ExplorerState::Surveying {
            resources: base_resource,
            combinations: comp_resource,
            energy_cells: false,
            orch_resource: false,
            orch_combination: false,
        };
        gather_info_from_planet(explorer)?;
    } else {
        //calculating utility of every action
        calc_utility(explorer)?;
        log_internal_op!(
            explorer,
            "utility scores" => format!("{:?}",explorer.ai_data.ai_action),
            "explorer state" =>format!("{:?}", explorer),
        );
        //getting the predicted best action
        let best_action = find_best_action(
            &explorer.ai_data.ai_action,
            explorer,
            explorer.ai_data.last_action.as_ref(),
            explorer.ai_data.last_action_planet_id,
        );
        log_internal_op!(
            explorer,
            "action to be taken" => format!("{:?}", best_action)
        );
        if let Some(ai_action) = best_action {
            explorer.ai_data.last_action = Some(ai_action.clone());
            explorer.ai_data.last_action_planet_id = Some(explorer.planet_id);
            match ai_action {
                AIActionType::RunAway => {
                    //if the best action to escape from this planet we choose the best planet to go to
                    let mut max: (&ID, &f32) = (&0, &0.0);
                    for planet in &explorer.ai_data.ai_action.move_to {
                        if planet.1 > max.1 {
                            max = planet;
                        }
                    }
                    if *max.0 != 0 {
                        //making sure that there is a planet to move to
                        explorer.state = ExplorerState::Traveling;
                        log_internal_op!(explorer, "action"=>"sending TravelToPlanetRequest", "planet_id"=>*max.0);
                        match explorer.orchestrator_channels.1.send(
                            ExplorerToOrchestrator::TravelToPlanetRequest {
                                explorer_id: explorer.explorer_id,
                                current_planet_id: explorer.planet_id,
                                dst_planet_id: *max.0,
                            },
                        ) {
                            Ok(()) => return Ok(()),
                            Err(err) => {
                                explorer.state = ExplorerState::Idle;
                                return Err(Box::new(err));
                            }
                        }
                    }
                }
                AIActionType::MoveTo(id) => {
                    explorer.state = ExplorerState::Traveling;
                    log_internal_op!(explorer, "action"=>"sending TravelToPlanetRequest", "planet_id"=>id);
                    match explorer.orchestrator_channels.1.send(
                        ExplorerToOrchestrator::TravelToPlanetRequest {
                            explorer_id: explorer.explorer_id,
                            current_planet_id: explorer.planet_id,
                            dst_planet_id: id,
                        },
                    ) {
                        Ok(()) => {
                            return Ok(());
                        }
                        Err(err) => {
                            explorer.state = ExplorerState::Idle;
                            return Err(Box::new(err));
                        }
                    }
                }
                AIActionType::SurveyNeighbors => {
                    explorer.state = ExplorerState::WaitingForNeighbours;
                    log_internal_op!(explorer, "sending NeighborsRequest");
                    match explorer.orchestrator_channels.1.send(
                        ExplorerToOrchestrator::NeighborsRequest {
                            explorer_id: explorer.explorer_id,
                            current_planet_id: explorer.planet_id,
                        },
                    ) {
                        Ok(()) => {
                            return Ok(());
                        }
                        Err(err) => {
                            explorer.state = ExplorerState::Idle;
                            return Err(Box::new(err));
                        }
                    }
                }
                AIActionType::SurveyEnergy => {
                    explorer.state = ExplorerState::Surveying {
                        resources: false,
                        combinations: false,
                        energy_cells: true,
                        orch_resource: false,
                        orch_combination: false,
                    };
                    match gather_info_from_planet(explorer) {
                        Ok(()) => {
                            return Ok(());
                        }
                        Err(err) => {
                            explorer.state = ExplorerState::Idle;
                            return Err(err);
                        }
                    }
                }
                AIActionType::Produce(res) => {
                    explorer.state = ExplorerState::GeneratingResource {
                        orchestrator_response: false,
                    };

                    log_internal_op!(explorer, "sending GenerateResourceRequest");
                    match explorer.planet_channels.1.send(
                        ExplorerToPlanet::GenerateResourceRequest {
                            explorer_id: 0,
                            resource: res,
                        },
                    ) {
                        Ok(()) => {
                            return Ok(());
                        }
                        Err(err) => {
                            explorer.state = ExplorerState::Idle;
                            return Err(Box::new(err));
                        }
                    }
                }
                AIActionType::Combine(res) => {
                    explorer.state = ExplorerState::CombiningResources {
                        orchestrator_response: false,
                    };
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
                            log_internal_op!(explorer, "sending CombineResourceRequest");
                            match explorer.planet_channels.1.send(
                                ExplorerToPlanet::CombineResourceRequest {
                                    explorer_id: explorer.explorer_id,
                                    msg: complex_resource_req,
                                },
                            ) {
                                Ok(()) => {
                                    return Ok(());
                                }
                                Err(err) => {
                                    explorer.state = ExplorerState::Idle;
                                    return Err(Box::new(err));
                                }
                            }
                        }
                        Err(err) => {
                            explorer.state = ExplorerState::Idle;
                            return Err(err.into());
                        }
                    }
                }
                AIActionType::Wait => {}
            }
        }
    }
    Ok(())
}
