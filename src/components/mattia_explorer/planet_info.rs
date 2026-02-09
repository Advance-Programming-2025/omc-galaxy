use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;
use std::collections::HashSet;
use common_game::logging::ActorType;
use logging_utils::log_fn_call;
use crate::utils::registry::PlanetType;

#[derive(Debug)]
pub enum PlanetClassType{
    A,
    B,
    C,
    D,
}
impl PlanetClassType{
    pub fn can_have_rocket(&self) -> bool {
        match self {
            PlanetClassType::A => { true }
            PlanetClassType::B => {false}
            PlanetClassType::C => {true}
            PlanetClassType::D => {false}
        }
    }
    pub fn max_energy_cells(&self) -> u32 {
        match self {
            PlanetClassType::A => {5}
            PlanetClassType::B => {1}
            PlanetClassType::C => {1}
            PlanetClassType::D => {5}
        }
    }
}

#[derive(Debug)]
pub struct PlanetInfo {
    pub basic_resources: Option<HashSet<BasicResourceType>>,
    pub complex_resources: Option<HashSet<ComplexResourceType>>,
    pub neighbors: Option<HashSet<ID>>,
    pub energy_cells:Option<u32>,
    pub charge_rate:Option<f32>,
    pub timestamp_neighbors:u64,
    pub timestamp_energy:u64,
    pub safety_score:Option<f32>,
    pub inferred_planet_type: Option<PlanetClassType>,
}
impl PlanetInfo {
    pub fn new(time: u64) -> Self {
        Self{
            basic_resources: None,
            complex_resources: None,
            neighbors: None,
            energy_cells: None,
            charge_rate: None,
            timestamp_neighbors:time,
            timestamp_energy:time,
            safety_score: None,
            inferred_planet_type: None,
        }
    }
    pub fn update_charge_rate(&mut self, current_energy: u32, current_time: u64) { //todo importare explorer_id per log
        log_fn_call!(dir
            ActorType::Explorer,
            0u32,
            "update_charge_rate()",
            current_energy,
            current_time
        );
        // first visit
        if self.timestamp_energy == 0 || self.energy_cells.is_none() {
            self.energy_cells = Some(current_energy);
            self.timestamp_energy = current_time;
            // cannot set charge rate
            return;
        }

        let delta_t = (current_time.saturating_sub(self.timestamp_energy)) as f32;
        if delta_t <= 0.0 {
            // in order to skip division by 0
            self.energy_cells = Some(current_energy);
            return;
        }

        let previous_energy = self.energy_cells.unwrap_or(current_energy) as f32;
        let instant_rate = (current_energy as f32 - previous_energy) / delta_t;

        // amortized average
        let alpha = 0.3;

        let new_rate = match self.charge_rate {
            Some(old_rate) => {
                (alpha * instant_rate) + ((1.0 - alpha) * old_rate)
            }
            None => {
                instant_rate
            }
        };

        // updating
        self.charge_rate = Some(new_rate);
        self.energy_cells = Some(current_energy);
        self.timestamp_energy = current_time;
    }
    pub fn calculate_planet_type(&mut self)->Result<(), String> {
        match (&self.basic_resources, &self.complex_resources){
            //this should not happen
            (None,_)=>{
                Err("planet_info.basic_resources are None".to_string())
            }
            (_, None)=>{
                Err("planet_info.complex_resources are None".to_string())
            }
            (Some(basic_resources),Some(complex_resources)) => {
                let comp_len= complex_resources.len();
                let base_len=basic_resources.len();
                if comp_len >1{
                    //in this case the planet type is C
                    self.inferred_planet_type=Some(PlanetClassType::C);
                }
                else if comp_len == 1{
                    //in this case the planet could be also C, but B is more likely
                    self.inferred_planet_type=Some(PlanetClassType::B);
                }
                else if base_len >1{
                    //in this case the planet type is D
                    self.inferred_planet_type=Some(PlanetClassType::D);
                }
                else{
                    //in this case the planet could be D, but A is more likely
                    self.inferred_planet_type=Some(PlanetClassType::A);
                }
                Ok(())
            }

        }
    }
}

