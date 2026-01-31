use std::collections::HashSet;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, ResourceType};
use common_game::components::resource::GenericResource::BasicResources;
use common_game::utils::ID;
use rand::Rng;
use crate::components::mattia_explorer::{Explorer, Bag};
use crate::components::mattia_explorer::planet_info::PlanetInfo;
//this value will affect the noise level of utility calculations
const RANDOMNESS_RANGE: f64 =0.1;
//this value will influence how careful is the explorer in considering old values for utility calculations
const LAMBDA: f32=0.005;
const PROPAGATION_FACTOR: f32=0.8;
pub enum AIAction{
    ProduceResource(BasicResourceType), //not sure if this will be useful, because I think it is useless to waste energy cell in making resources
    CombineResource(ComplexResourceType),
    MoveTo(ID),
    SurveyPlanet {
        resource: bool,
        combination: bool,
        energy_cells: bool,
    },
    Wait,
    RunAway
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
    global_sunray_rate: f32,
    global_asteroid_rate: f32,
    resource_needs: ResourceNeeds,
}
impl ai_data {
    pub fn new()->Self{
        Self{
            global_asteroid_rate: 0.0,
            global_sunray_rate: 0.0,
            resource_needs: ResourceNeeds::new(),
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

pub fn calc_utility(ai_action: AIAction, ai_data: &mut ai_data, explorer: &Explorer) -> Result<f32, &'static str> {
    match ai_action {
        AIAction::ProduceResource(resource_type) => {
            // Non estraiamo più planet_info qui per evitare il doppio borrow
            score_basic_resource_production(explorer, resource_type)
        }
        AIAction::CombineResource(resource_type) => {
            score_complex_resource_production(explorer, resource_type)
        }
        AIAction::SurveyPlanet { .. } => { todo!() }
        AIAction::MoveTo(_) => { todo!() }
        AIAction::Wait => { todo!() }
        AIAction::RunAway => { todo!() }
    }
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
        * reliability;

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
        * reliability;

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


fn calculate_safety_score(planet_info: &PlanetInfo, explorer: &Explorer) -> f32{
    let sustainability = if planet_info.charge_rate>0f32{1.0}else{0.5};
    let physical_safety = 1.0 - (1.0 / planet_info.energy_cells.max(1) as f32);

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

    (sustainability * physical_safety * escape_factor).clamp(0.0, 1.0)
}

fn calculate_rocket_probability(planet_info: &PlanetInfo) -> Result<f32, &'static str> {
todo!()
}