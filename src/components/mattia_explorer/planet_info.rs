use std::collections::HashSet;
use std::time::SystemTime;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

// struct that contains some
pub struct PlanetInfo {
    pub basic_resources: Option<HashSet<BasicResourceType>>,
    pub complex_resources: Option<HashSet<ComplexResourceType>>,
    pub neighbours: Option<HashSet<ID>>,
    pub energy_cells:u32,
    pub sunray_rate:f32,
    pub asteroid_rate:f32,
    pub timestamp:u64,
    pub safety_score:f32,

}
impl PlanetInfo {
    pub fn new(time: u64) -> Self {
        Self{
            basic_resources: None,
            complex_resources: None,
            neighbours: None,
            energy_cells: 0,
            sunray_rate: 0.0,
            asteroid_rate: 0.0,
            timestamp:time,
            safety_score: 1.0,
        }
    }
}
