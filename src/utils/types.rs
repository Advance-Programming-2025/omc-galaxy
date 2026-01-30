use std::collections::BTreeMap;
use std::path::Iter;
use std::sync::{Arc, RwLock};
use std::fmt::Debug;

use common_game::components::energy_cell::EnergyCell;
use common_game::components::planet::{DummyPlanetState, Planet};
use common_game::components::resource::ResourceType;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use crossbeam_channel::{Receiver, Sender};

use crate::utils::Status;

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

pub type GalaxyTopology = Arc<RwLock<Vec<Vec<bool>>>>;
pub type PlanetStatus = Arc<RwLock<BTreeMap<u32, Status>>>;
pub type ExplorerStatus = Arc<RwLock<BTreeMap<u32, Status>>>;

pub type GalaxySnapshot = Vec<(u32, u32)>;


pub struct PlanetInfoMap{
    map: BTreeMap<u32, PlanetInfo>
}
impl PlanetInfoMap{
    pub fn new() -> Self{
        PlanetInfoMap{
            map: BTreeMap::new(),
        }
    }
    pub fn insert(&mut self, planet_id: u32, info: PlanetInfo){
        self.map.insert(planet_id, info);
    }
    pub fn insert_status(&mut self, planet_id: u32, status: Status){
        if let Some(planet_info) = self.map.get_mut(&planet_id){
            planet_info.status = status;
        }else{
            let new_info = PlanetInfo::from(status, vec![], 0, false);
            self.map.insert(planet_id, new_info);
        }
    }

    pub fn update_from_planet_state(&mut self, planet_id: u32, planet_state: DummyPlanetState){
        if let Some(planet_info) = self.map.get_mut(&planet_id){
            planet_info.energy_cells = planet_state.energy_cells;
            planet_info.charged_cells_count = planet_state.charged_cells_count;
            planet_info.rocket = planet_state.has_rocket;
        }
    }
    pub fn len(&self) -> usize{
        self.map.len()
    }
    pub fn get_status(&self, planet_id: &u32) -> Status{
        self.map.get(planet_id).unwrap().status
    }
    pub fn is_empty(&self) -> bool{
        self.map.is_empty()
    }
    pub fn is_paused(&self, planet_id: &u32) -> bool{
        if let Some(planet_info) = self.map.get(planet_id){
            return planet_info.status == Status::Paused;
        }
        false
    }
    pub fn is_dead(&self, planet_id: &u32) -> bool{
        if let Some(planet_info) = self.map.get(planet_id){
            return planet_info.status == Status::Dead;
        }
        false
    }
    pub fn is_running(&self, planet_id: &u32) -> bool{
        if let Some(planet_info) = self.map.get(planet_id){
            return planet_info.status == Status::Running;
        }
        false
    }   

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &PlanetInfo)> {
        self.map.iter()
    }
    pub fn count_survivors(&self) -> usize{
        self.map.values().filter(|info| info.status != Status::Dead).count()
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
pub struct PlanetInfo{
    pub status: Status,
    pub energy_cells: Vec<bool>,
    pub charged_cells_count: usize,
    pub rocket: bool,
}
impl PlanetInfo{
    pub fn from(
        status: Status,
        energy_cells: Vec<bool>,
        charged_cells_count: usize,
        rocket: bool,
    ) -> Self{
        PlanetInfo{
            status,
            energy_cells,
            charged_cells_count,
            rocket,
        }
    }
}


pub struct ExplorerInfoMap {
    map: BTreeMap<u32, ExplorerInfo>
}

impl ExplorerInfoMap {
    pub fn new() -> Self {
        ExplorerInfoMap {
            map: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, explorer_id: u32, info: ExplorerInfo) {
        self.map.insert(explorer_id, info);
    }

    pub fn insert_status(&mut self, explorer_id: u32, status: Status) {
        if let Some(explorer_info) = self.map.get_mut(&explorer_id) {
            explorer_info.status = status;
        } else {
            let new_info = ExplorerInfo::from(explorer_id, status, vec![], None);
            self.map.insert(explorer_id, new_info);
        }
    }

    pub fn update_bag(&mut self, explorer_id: u32, bag: Vec<ResourceType>) {
        if let Some(explorer_info) = self.map.get_mut(&explorer_id) {
            explorer_info.bag = bag;
        }
    }

    pub fn update_current_planet(&mut self, explorer_id: u32, planet_id: u32) {
        if let Some(explorer_info) = self.map.get_mut(&explorer_id) {
            explorer_info.current_planet_id = Some(planet_id);
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn get_status(&self, explorer_id: &u32) -> Status {
        self.map.get(explorer_id).unwrap().status
    }

    pub fn get_current_planet(&self, explorer_id: &u32) -> Option<u32> {
        self.map.get(explorer_id).map(|info| info.current_planet_id)?
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
        self.map.values().filter(|info| info.status != Status::Dead).count()
    }

    pub fn get(&self, explorer_id: &u32) -> Option<&ExplorerInfo> {
        self.map.get(explorer_id)
    }

    pub fn get_mut(&mut self, explorer_id: &u32) -> Option<&mut ExplorerInfo> {
        self.map.get_mut(explorer_id)
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
pub struct ExplorerInfo{
    pub id: u32,
    pub status: Status,
    pub bag: Vec<ResourceType>,
    pub current_planet_id: Option<u32>,
}

impl ExplorerInfo{
    pub fn from(
        id: u32,
        status: Status,
        bag: Vec<ResourceType>,
        current_planet_id: Option<u32>,
    ) -> Self {
        ExplorerInfo{
            id,
            status,
            bag,
            current_planet_id,
        }
    }
}
