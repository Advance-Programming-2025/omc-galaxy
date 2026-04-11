use common_game::logging::ActorType;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;

use common_game::components::planet::{DummyPlanetState, Planet};
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use crossbeam_channel::{Receiver, Sender};
use logging_utils::log_internal_op;

use crate::utils::Status;
use crate::utils::registry::PlanetType;

pub type PlanetFactory = Box<
    dyn Fn(
            Receiver<OrchestratorToPlanet>,
            Sender<PlanetToOrchestrator>,
            Receiver<ExplorerToPlanet>,
            u32,
        ) -> Result<Planet, String>
        + Send
        + Sync
        + 'static,
>;

pub type GalaxyTopologyNotLock = Vec<Vec<bool>>;
pub type PlanetStatusNotLock = BTreeMap<u32, Status>;
pub type ExplorerStatusNotLock = BTreeMap<u32, Status>;

pub type GalaxyTopology = Vec<Vec<bool>>;

pub type GalaxySnapshot = Vec<(u32, u32)>;

pub struct PlanetInfoMap {
    pub(crate) map: BTreeMap<u32, PlanetInfo>,
}
impl PlanetInfoMap {
    pub fn new() -> Self {
        PlanetInfoMap {
            map: BTreeMap::new(),
        }
    }
    pub fn insert(&mut self, planet_id: u32, info: PlanetInfo) {
        self.map.insert(planet_id, info);
        log_internal_op!(dir ActorType::Planet, planet_id, "action"=>format!("inserted new planet in PlanetInfoMap: {}", planet_id));
    }

    ///Warning! this function overwrite the old value if there is
    pub fn insert_status(&mut self, planet_id: u32, name: PlanetType, status: Status, basic: Option<HashSet<BasicResourceType>>, complex: Option<HashSet<ComplexResourceType>>) {
        let new_info = PlanetInfo::from(name, status, vec![], 0, false, basic, complex);
        log_internal_op!(dir ActorType::Planet, planet_id, "action"=>format!("new status inserted in PlanetInfoMap, planet_id:{}, planet_info:{:?}", planet_id, new_info));
        self.map.insert(planet_id, new_info);
    }

    pub fn contains(&self, explorer_id: &u32) -> bool {
        self.map.contains_key(explorer_id)
    }

    pub fn update_status(&mut self, planet_id: u32, status: Status) -> Result<(), String> {
        if let Some(planet_info) = self.map.get_mut(&planet_id) {
            planet_info.status = status;
            log_internal_op!(dir ActorType::Planet, planet_id, "action"=>format!("planet: {} status updated to: {:?}", planet_id, status));
            Ok(())
        } else {
            Err("planet info is not already present".to_string())
        }
    }
    pub fn update_supported_resources(
        &mut self,
        planet_id: u32,
        supported_resources: HashSet<BasicResourceType>,
    ) -> Result<(), String> {
        if let Some(planet_info) = self.map.get_mut(&planet_id) {
            log_internal_op!(dir ActorType::Planet, planet_id, "action"=> format!("planet: {} supported resources updated to: {:?}", planet_id, supported_resources));
            planet_info.supported_resources = Some(supported_resources);
            Ok(())
        } else {
            Err("planet info is not already present".to_string())
        }
    }
    pub fn update_supported_combination(
        &mut self,
        planet_id: u32,
        supported_combination: HashSet<ComplexResourceType>,
    ) -> Result<(), String> {
        if let Some(planet_info) = self.map.get_mut(&planet_id) {
            log_internal_op!(dir ActorType::Planet, planet_id, "action"=> format!("planet: {} supported resource combinations updated to: {:?}", planet_id, supported_combination));
            planet_info.supported_combination = Some(supported_combination);
            Ok(())
        } else {
            Err("planet info is not already present".to_string())
        }
    }

    pub fn update_from_planet_state(&mut self, planet_id: u32, planet_state: DummyPlanetState) {
        if let Some(planet_info) = self.map.get_mut(&planet_id) {
            log_internal_op!(dir ActorType::Planet, planet_id, "action"=>format!(
                "updated planet info from DummyPlanetState, new energy_cells: {:?}, new charged_cells_count: {}, new has_rocket: {}",  planet_state.energy_cells, planet_state.charged_cells_count, planet_state.has_rocket
            ));
            planet_info.energy_cells = planet_state.energy_cells;
            planet_info.charged_cells_count = planet_state.charged_cells_count;
            planet_info.rocket = planet_state.has_rocket;
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn get_status(&self, planet_id: &u32) -> Status {
        self.map.get(planet_id).unwrap().status
    }

    pub fn get_info(&self, planet_id: u32) -> Option<&PlanetInfo> {
        self.map.get(&planet_id)
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    pub fn is_paused(&self, planet_id: &u32) -> bool {
        if let Some(planet_info) = self.map.get(planet_id) {
            return planet_info.status == Status::Paused;
        }
        false
    }
    pub fn is_dead(&self, planet_id: &u32) -> bool {
        if let Some(planet_info) = self.map.get(planet_id) {
            return planet_info.status == Status::Dead;
        }
        false
    }
    pub fn is_running(&self, planet_id: &u32) -> bool {
        if let Some(planet_info) = self.map.get(planet_id) {
            return planet_info.status == Status::Running;
        }
        false
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &PlanetInfo)> {
        self.map.iter()
    }
    pub fn count_survivors(&self) -> usize {
        self.map
            .values()
            .filter(|info| info.status != Status::Dead)
            .count()
    }

    pub(crate) fn get_list_id_alive(&self) -> Vec<u32> {
        let mut res = Vec::new();
        for (&id, info) in &self.map {
            if info.status == Status::Running || info.status == Status::Paused {
                res.push(id);
            }
        }
        res
    }
}

impl Clone for PlanetInfoMap {
    fn clone(&self) -> Self {
        PlanetInfoMap {
            map: self.map.clone(),
        }
    }
}

impl Debug for PlanetInfoMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //print only the status of each planet for brevity
        let mut debug_map = BTreeMap::new();
        for (id, info) in &self.map {
            debug_map.insert(id, &info.status);
        }
        debug_map.fmt(f)
    }
}
#[derive(PartialEq, Debug, Clone)]
pub struct PlanetInfo {
    pub name: PlanetType,
    pub status: Status,
    pub energy_cells: Vec<bool>,
    pub charged_cells_count: usize,
    pub rocket: bool,
    pub supported_resources: Option<HashSet<BasicResourceType>>,
    pub supported_combination: Option<HashSet<ComplexResourceType>>,
}
impl PlanetInfo {
    pub fn from(
        name: PlanetType,
        status: Status,
        energy_cells: Vec<bool>,
        charged_cells_count: usize,
        rocket: bool,
        supported_resources: Option<HashSet<BasicResourceType>>,
        supported_combination: Option<HashSet<ComplexResourceType>>,
    ) -> Self {
        PlanetInfo {
            name,
            status,
            energy_cells,
            charged_cells_count,
            rocket,
            supported_resources,
            supported_combination,
        }
    }

    pub fn get_free_energy_cells(&self) -> u32 {
        self.energy_cells.iter().filter(|&&x| x).count() as u32
    }
}

pub struct ExplorerInfoMap {
    pub(crate) map: BTreeMap<u32, ExplorerInfo>,
}

impl ExplorerInfoMap {
    pub fn new() -> Self {
        ExplorerInfoMap {
            map: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, explorer_id: u32, info: ExplorerInfo) {
        self.map.insert(explorer_id, info);
        log_internal_op!(dir ActorType::Explorer, explorer_id, "action"=>format!("inserted new explorer in ExplorerInfoMap: {}", explorer_id));
    }

    pub fn insert_status(&mut self, explorer_id: u32, status: Status) {
        if let Some(explorer_info) = self.map.get_mut(&explorer_id) {
            explorer_info.status = status;
            log_internal_op!(dir ActorType::Explorer, explorer_id, "action"=>format!("explorer: {} status updated to: {:?}", explorer_id, status));
        }
    }

   
    pub fn update_bag(&mut self, explorer_id: u32, bag: Vec<ResourceType>) {
        if let Some(explorer_info) = self.map.get_mut(&explorer_id) {
            log_internal_op!(dir ActorType::Explorer, explorer_id, "action"=>format!("explorer: {} bag updated to: {:?}", explorer_id, bag));
            explorer_info.bag = bag;
        }
    }

    pub fn update_current_planet(&mut self, explorer_id: u32, planet_id: u32) {
        if let Some(explorer_info) = self.map.get_mut(&explorer_id) {
            log_internal_op!(dir ActorType::Explorer, explorer_id, "action"=>format!("explorer: {} current planet updated to: {}", explorer_id, planet_id));
            explorer_info.current_planet_id = planet_id;
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn get_status(&self, explorer_id: &u32) -> Option<Status> {
        self.map.get(explorer_id).map(|a| a.status)
    }

    pub fn get_current_planet(&self, explorer_id: &u32) -> Option<u32> {
        self.map.get(explorer_id).map(|a| a.current_planet_id)
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn is_paused(&self, explorer_id: &u32) -> bool {
        if let Some(explorer_info) = self.map.get(explorer_id) {
            return explorer_info.status == Status::Paused;
        }
        false
    }

    pub fn is_dead(&self, explorer_id: &u32) -> bool {
        if let Some(explorer_info) = self.map.get(explorer_id) {
            return explorer_info.status == Status::Dead;
        }
        false
    }

    pub fn is_running(&self, explorer_id: &u32) -> bool {
        if let Some(explorer_info) = self.map.get(explorer_id) {
            return explorer_info.status == Status::Running;
        }
        false
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &ExplorerInfo)> {
        self.map.iter()
    }

    pub fn count_survivors(&self) -> usize {
        self.map
            .values()
            .filter(|info| info.status != Status::Dead)
            .count()
    }

    pub fn get(&self, explorer_id: &u32) -> Option<&ExplorerInfo> {
        self.map.get(explorer_id)
    }

    pub fn get_mut(&mut self, explorer_id: &u32) -> Option<&mut ExplorerInfo> {
        self.map.get_mut(explorer_id)
    }

    pub fn get_bag(&self, explorer_id: &u32) -> Option<&Vec<ResourceType>> {
        self.map.get(explorer_id).map(|a| &a.bag)
    }
    pub fn get_planet(&self, explorer_id: &u32) -> Option<u32> {
        self.map.get(explorer_id).map(|a| a.current_planet_id)
    }
    pub fn get_id(&self, explorer_id: &u32) -> Option<u32> {
        self.map.get(explorer_id).map(|a| a.id)
    }
}

impl Clone for ExplorerInfoMap {
    fn clone(&self) -> Self {
        ExplorerInfoMap {
            map: self.map.clone(),
        }
    }
}

impl Debug for ExplorerInfoMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print only the status of each explorer for brevity
        let mut debug_map = BTreeMap::new();
        for (id, info) in &self.map {
            debug_map.insert(id, &info.status);
        }
        debug_map.fmt(f)
    }
}

#[derive(Clone)]
pub struct ExplorerInfo {
    pub id: u32,
    pub status: Status,
    pub bag: Vec<ResourceType>,
    pub current_planet_id: u32,
    pub move_to_planet_id: i32,
}

impl ExplorerInfo {
    pub fn from(id: u32, status: Status, bag: Vec<ResourceType>, current_planet_id: u32) -> Self {
        ExplorerInfo {
            id,
            status,
            bag,
            current_planet_id,
            move_to_planet_id: -1, //at this time is not relevant
        }
    }
}
