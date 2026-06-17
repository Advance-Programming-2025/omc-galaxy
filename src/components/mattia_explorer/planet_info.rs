use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;
use logging_utils::log_fn_call;
use std::collections::HashSet;

#[derive(Debug)]
/// enum used to classify the type of every planet
pub(super) enum PlanetClassType {
    A,
    B,
    C,
    D,
}
impl PlanetClassType {
    //getters of basic information based on inferred planet type
    pub(super) fn can_have_rocket(&self) -> bool {
        match self {
            PlanetClassType::A => true,
            PlanetClassType::B => false,
            PlanetClassType::C => true,
            PlanetClassType::D => false,
        }
    }
    pub(super) fn max_energy_cells(&self) -> u32 {
        match self {
            PlanetClassType::A => 5,
            PlanetClassType::B => 1,
            PlanetClassType::C => 1,
            PlanetClassType::D => 5,
        }
    }
}

#[derive(Debug)]
/// main struct that stores information about the planet
pub(super) struct PlanetInfo {
    pub basic_resources: Option<HashSet<BasicResourceType>>,
    pub complex_resources: Option<HashSet<ComplexResourceType>>,
    pub neighbors: Option<HashSet<ID>>,
    pub energy_cells: Option<u32>,
    pub charge_rate: Option<f32>,  //inferred charge rate
    pub timestamp_neighbors: u64,  //last time tick that the neighbors were updated
    pub timestamp_energy: u64,     //last time tick that energy cells were updated
    pub safety_score: Option<f32>, //calculated safety score of the planet
    pub inferred_planet_type: Option<PlanetClassType>,
}
impl PlanetInfo {
    pub(super) fn new(time: u64) -> Self {
        log_fn_call!(
            dir
            ActorType::Explorer,
            0u32,
            "PlanetInfo::new()",
        );
        Self {
            basic_resources: None,
            complex_resources: None,
            neighbors: None,
            energy_cells: None,
            charge_rate: None,
            timestamp_neighbors: time,
            timestamp_energy: time,
            safety_score: None,
            inferred_planet_type: None,
        }
    }
    /// this method update the charge rate of the planet, based on the available information
    pub(super) fn update_charge_rate(
        &mut self,
        current_energy: u32,
        current_time: u64,
        charge_rate_alpha: f32,
        explorer_id: u32,
    ) {
        log_fn_call!(dir
            ActorType::Explorer,
            explorer_id,
            "update_charge_rate()",
            current_energy,
            current_time
        );
        // first visit of the planet
        if self.timestamp_energy == 0 || self.energy_cells.is_none() {
            self.energy_cells = Some(current_energy);
            self.timestamp_energy = current_time;
            // cannot set charge rate
            return;
        }
        // time interval
        let delta_t = (current_time.saturating_sub(self.timestamp_energy)) as f32;
        if delta_t <= 0.0 {
            //guard in order to skip division by 0
            self.energy_cells = Some(current_energy);
            return;
        }
        // previous energy count
        let previous_energy = self.energy_cells.unwrap_or(current_energy) as f32;
        // instant charge rate calculation
        let instant_rate = (current_energy as f32 - previous_energy) / delta_t;

        // amortized average
        let alpha = charge_rate_alpha;
        //amortizing the charge rate value
        let new_rate = match self.charge_rate {
            Some(old_rate) => (alpha * instant_rate) + ((1.0 - alpha) * old_rate),
            None => instant_rate,
        };

        // updating
        self.charge_rate = Some(new_rate);
        self.energy_cells = Some(current_energy);
        self.timestamp_energy = current_time;
    }
    /// this method tries to infer the planet type based on the information available
    pub(super) fn calculate_planet_type(&mut self) -> Result<(), String> {
        log_fn_call!(
            dir
            ActorType::Explorer,
            0u32,
            "calculate_planet_type()",
        );
        match (&self.basic_resources, &self.complex_resources) {
            //this should not happen
            (None, _) => Err("planet_info.basic_resources are None".to_string()),
            (_, None) => Err("planet_info.complex_resources are None".to_string()),
            (Some(basic_resources), Some(complex_resources)) => {
                let comp_len = complex_resources.len();
                let base_len = basic_resources.len();
                if comp_len > 1 {
                    //in this case the planet type is C
                    self.inferred_planet_type = Some(PlanetClassType::C);
                } else if comp_len == 1 {
                    //in this case the planet could be also C, but B is more likely
                    self.inferred_planet_type = Some(PlanetClassType::B);
                } else if base_len > 1 {
                    //in this case the planet type is D
                    self.inferred_planet_type = Some(PlanetClassType::D);
                } else {
                    //in this case the planet could be D, but A is more likely
                    self.inferred_planet_type = Some(PlanetClassType::A);
                }
                Ok(())
            }
        }
    }
}
