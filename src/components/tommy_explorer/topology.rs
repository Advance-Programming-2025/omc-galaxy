use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;
use std::collections::{HashMap, HashSet};

/// Struct that contains information about a planet.
#[derive(Debug, Clone)]
pub struct PlanetInfo {
    pub basic_resources: Option<HashSet<BasicResourceType>>,
    pub complex_resources: Option<HashSet<ComplexResourceType>>,
    pub neighbours: Option<HashSet<ID>>,
}

impl PlanetInfo {
    /// Creates a new PlanetInfo with no information.
    pub fn new() -> Self {
        Self {
            basic_resources: None,
            complex_resources: None,
            neighbours: None,
        }
    }

    /// Creates a PlanetInfo with all fields set.
    pub fn with_data(
        basic_resources: HashSet<BasicResourceType>,
        complex_resources: HashSet<ComplexResourceType>,
        neighbours: HashSet<ID>,
    ) -> Self {
        Self {
            basic_resources: Some(basic_resources),
            complex_resources: Some(complex_resources),
            neighbours: Some(neighbours),
        }
    }

    /// Checks if we have complete information about this planet.
    pub fn is_complete(&self) -> bool {
        self.basic_resources.is_some()
            && self.complex_resources.is_some()
            && self.neighbours.is_some()
    }

    /// Gets the basic resources.
    pub fn get_basic_resources(&self) -> Option<&HashSet<BasicResourceType>> {
        self.basic_resources.as_ref()
    }

    /// Gets the complex resources.
    pub fn get_complex_resources(&self) -> Option<&HashSet<ComplexResourceType>> {
        self.complex_resources.as_ref()
    }

    /// Gets the neighbours.
    pub fn get_neighbours(&self) -> Option<&HashSet<ID>> {
        self.neighbours.as_ref()
    }

    /// Updates the basic resources' information.
    // should be used only once per planet
    pub fn set_basic_resources(&mut self, resources: HashSet<BasicResourceType>) {
        self.basic_resources = Some(resources);
    }

    /// Updates the complex resources' information.
    // should be used only once per planet
    pub fn set_complex_resources(&mut self, resources: HashSet<ComplexResourceType>) {
        self.complex_resources = Some(resources);
    }

    /// Updates the neighbours information.
    pub fn set_neighbours(&mut self, neighbours: HashSet<ID>) {
        self.neighbours = Some(neighbours);
    }
}

impl Default for PlanetInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct that manages the topology information for all known planets.
// ex TopologyInfo
pub struct TopologyManager {
    planets: HashMap<ID, PlanetInfo>,
}

impl TopologyManager {
    /// Creates a new TopologyManager with a starting planet.
    pub fn new(starting_planet_id: ID) -> Self {
        let mut planets = HashMap::new();
        planets.insert(starting_planet_id, PlanetInfo::new());
        Self { planets }
    }

    /// Gets information about a planet, creating an entry if it doesn't exist.
    pub fn get_or_create(&mut self, planet_id: ID) -> &mut PlanetInfo {
        self.planets
            .entry(planet_id)
            .or_insert_with(PlanetInfo::new)
    }

    /// Gets information about a planet (read-only).
    pub fn get(&self, planet_id: ID) -> Option<&PlanetInfo> {
        self.planets.get(&planet_id)
    }

    /// Gets mutable information about a planet.
    pub fn get_mut(&mut self, planet_id: ID) -> Option<&mut PlanetInfo> {
        self.planets.get_mut(&planet_id)
    }

    /// Adds multiple planets to the topology.
    pub fn add_planets(&mut self, planet_ids: &[ID]) {
        for &planet_id in planet_ids {
            self.planets
                .entry(planet_id)
                .or_insert_with(PlanetInfo::new);
        }
    }

    /// Updates neighbours for a planet.
    pub fn update_neighbours(&mut self, planet_id: ID, neighbours: Vec<ID>) {
        // add all neighbours to the topology if they don't exist
        self.add_planets(&neighbours);

        // update the planet's neighbour information
        if let Some(info) = self.planets.get_mut(&planet_id) {
            info.set_neighbours(neighbours.into_iter().collect());
        }
    }

    /// Clears all topology information.
    pub fn clear(&mut self) {
        self.planets.clear();
    }

    /// Returns all known planet IDs.
    pub fn known_planets(&self) -> Vec<ID> {
        self.planets.keys().copied().collect()
    }

    /// Checks if a planet is in the topology.
    pub fn contains(&self, planet_id: ID) -> bool {
        self.planets.contains_key(&planet_id)
    }

    /// Checks if all the known planets' information are complete.
    pub fn is_fully_discovered(&self) -> bool {
        self.known_planets().iter().all(|id| {
            if let Some(planet_info) = self.planets.get(id) {
                planet_info.is_complete()
            } else {
                false
            }
        })
    }
}
