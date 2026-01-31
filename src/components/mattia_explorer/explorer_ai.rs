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
    pub fn get(&mut self, resource_type: ResourceType)->f32{
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
}
pub struct ai_data{
    global_sunray_rate: f32,
    global_asteroid_rate: f32,
    resource_needs: ResourceNeeds,
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

pub fn calc_utility(ai_action: AIAction, ai_data: & mut ai_data, explorer: &mut Explorer) -> Result<f32, &'static str> {
    match ai_action{
        AIAction::ProduceResource(resource_type) => {
            let planet_info=explorer.get_current_planet_info()?;
            Ok(score_basic_resource_production(
                explorer,
                &planet_info,
                resource_type,
                ai_data.resource_needs.get(ResourceType::Basic(resource_type)),
            ))
        }
        AIAction::CombineResource(_) => {}
        AIAction::MoveTo(_) => {}
        AIAction::SurveyPlanet { .. } => {}
        AIAction::Wait => {}
    }
}

fn score_basic_resource_production(
    explorer: &Explorer,
    planet_info: &PlanetInfo,
    resource_type: BasicResourceType,
    need: f32,
) -> f32 {
    let energy_cells = planet_info.energy_cells.max(1);
    let resource_count = explorer.bag.count(ResourceType::Basic(resource_type)).max(1);
    let reliability = calculate_time_decay(planet_info.timestamp, explorer.time);
    let base = need
        * (1.0 / resource_count as f32)
        * (1.0 - (1.0 / energy_cells as f32))
        * (planet_info.charge_rate / planet_info.discharge_rate.max(0.0001))
        * reliability;

    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);

    (base * noise_factor).clamp(0.0, 1.0)
}

fn score_complex_resource_production(
    explorer: &Explorer,
    planet_info: &PlanetInfo,
    resource_type: BasicResourceType,
    need: f32,
)->f32{
    let energy_cells = planet_info.energy_cells.max(1);
    let resource_count = explorer.bag.count(ResourceType::Basic(resource_type)).max(1);
    let reliability = calculate_time_decay(planet_info.timestamp, explorer.time);
    let base = need
        * (1.0 / resource_count as f32)
        * (1.0 - (1.0 / energy_cells as f32))
        * (planet_info.charge_rate / planet_info.discharge_rate.max(0.0001))
        * reliability;

    let mut rng = rand::rng();
    let noise_factor: f32 = rng.random_range(0.95..=1.05);

    (base * noise_factor).clamp(0.0, 1.0)
}