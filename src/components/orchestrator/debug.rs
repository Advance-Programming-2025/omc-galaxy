use crate::{components::orchestrator::Orchestrator};
use logging_utils::debug_println;

impl Orchestrator {
    pub(crate) fn print_planets_state(&self) {
        // for (id, status) in &self.planets_status{
        //     print!("({}, {:?})",id, status);
        // }
        debug_println!("{:?}", self.planets_status);
    }
    pub(crate) fn print_galaxy_topology(&self) {
        debug_println!("{:?}", self.galaxy_topology);
    }
    pub(crate) fn print_orch(&self) {
        debug_println!("Orchestrator running");
    }
}
