use std::collections::HashSet;
use std::time::SystemTime;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

// struct that contains some
pub struct PlanetInfo {
    pub basic_resources: Option<HashSet<BasicResourceType>>,
    pub complex_resources: Option<HashSet<ComplexResourceType>>,
    pub neighbors: Option<HashSet<ID>>,
    pub energy_cells:u32,
    pub charge_rate:f32,
    pub timestamp_neighbors:u64,
    pub timestamp_energy:u64,
    pub safety_score:f32,

}
impl PlanetInfo {
    pub fn new(time: u64) -> Self {
        Self{
            basic_resources: None,
            complex_resources: None,
            neighbors: None,
            energy_cells: 0,
            charge_rate: 1.0,
            timestamp_neighbors:time,
            timestamp_energy:time,
            safety_score: 1.0,
        }
    }
    pub fn update_charge_rate(&mut self, current_energy: u32, current_time: u64) {
        if self.timestamp_energy == 0 {
            self.energy_cells = current_energy;
            self.timestamp_energy = current_time;
            return;
        }

        let delta_t = (current_time - self.timestamp_energy) as f32;
        if delta_t <= 0.0 { return; }

        // calculating the instant charge rate
        let instant_rate = (current_energy - self.energy_cells)as f32 / delta_t;

        // amortized average constant: 30% of the new value, 70% of old value
        let alpha = 0.3;

        self.charge_rate = (alpha * instant_rate) + ((1.0 - alpha) * self.charge_rate);

        self.energy_cells = current_energy;
        self.timestamp_energy = current_time;
    }
}

