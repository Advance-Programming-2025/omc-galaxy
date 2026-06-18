use crate::components::tommy_explorer::Explorer;
use crate::components::tommy_explorer::topology::TopologyManager;
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use std::collections::{HashMap, HashSet, VecDeque};

impl TopologyManager {
    /// Finds the shortest path to the nearest unexplored or partially explored planet.
    ///
    /// This method leverages a lazy Breadth-First Search (BFS) iterator to scan the
    /// known universe layer by layer. It stops as soon as it encounters a "frontier" node.
    ///
    /// Returns `Some(path)` if a frontier is found, or `None` if the entire topology
    /// has been fully discovered.
    pub fn find_path_to_nearest_frontier(&self, start_node: u32) -> Option<VecDeque<u32>> {
        // Initialize the custom BFS iterator starting from the current node
        let mut bfs = self.bfs_iter(start_node);

        // Lazily evaluate each node to find the first one that matches the frontier criteria
        let target = bfs.find(|&node| {
            match self.get(node) {
                // Node is not in the topology yet (it's a newly discovered neighbor)
                None => true,
                // Node is known, but we haven't queried all its resources or neighbors yet
                Some(info) => !info.is_complete(),
            }
        })?; // Early return None if the iterator is exhausted without finding a match

        // If the target is found, ask the iterator to reconstruct the route via the parent map
        Some(bfs.reconstruct_path(target))
    }

    /// Finds the shortest path to the nearest planet capable of providing the specified target resource.
    ///
    /// Uses a lazy BFS traversal to ensure the returned path requires the minimum number of jumps.
    ///
    /// Returns `Some(path)` to the target planet, or `None` if the resource is currently
    /// unavailable in the known topology.
    pub fn find_path_to_resource(
        &self,
        start_node: u32,
        target_res: ResourceType,
    ) -> Option<VecDeque<u32>> {
        // Initialize the BFS iterator to explore the topology outward from the current position
        let mut bfs = self.bfs_iter(start_node);

        // Find the first node that contains the specific resource we need
        let target = bfs.find(|&node| {
            if let Some(info) = self.get(node) {
                match target_res {
                    // Check if the planet can provide the required basic resource
                    ResourceType::Basic(b) => {
                        info.get_basic_resources().map_or(false, |s| s.contains(&b))
                    }
                    // Check if the planet's laboratories support the required complex combination
                    ResourceType::Complex(c) => info
                        .get_complex_resources()
                        .map_or(false, |s| s.contains(&c)),
                }
            } else {
                // If we have no info on the node, we safely skip it
                false
            }
        })?;

        // Reconstruct and return the shortest path to the successful node
        Some(bfs.reconstruct_path(target))
    }
}

/// Trait to define crafting dependencies
pub trait RecipeExt {
    /// Returns the needed resources and quantities
    fn ingredients(&self) -> Vec<(ResourceType, usize)>;

    /// Verifies if the bag contains the needed resources
    fn can_be_crafted(&self, bag: &[ResourceType]) -> bool;
}

impl RecipeExt for ComplexResourceType {
    fn ingredients(&self) -> Vec<(ResourceType, usize)> {
        match self {
            ComplexResourceType::Water => vec![
                (ResourceType::Basic(BasicResourceType::Hydrogen), 1),
                (ResourceType::Basic(BasicResourceType::Oxygen), 1),
            ],
            ComplexResourceType::Life => vec![
                (ResourceType::Complex(ComplexResourceType::Water), 1),
                (ResourceType::Basic(BasicResourceType::Carbon), 1),
            ],
            ComplexResourceType::Diamond => {
                vec![(ResourceType::Basic(BasicResourceType::Carbon), 2)]
            }
            ComplexResourceType::Robot => vec![
                (ResourceType::Basic(BasicResourceType::Silicon), 1),
                (ResourceType::Complex(ComplexResourceType::Life), 1),
            ],
            ComplexResourceType::AIPartner => vec![
                (ResourceType::Complex(ComplexResourceType::Robot), 1),
                (ResourceType::Complex(ComplexResourceType::Diamond), 1),
            ],
            _ => vec![],
        }
    }

    fn can_be_crafted(&self, bag: &[ResourceType]) -> bool {
        let mut counts = HashMap::new();
        for item in bag {
            *counts.entry(item.clone()).or_insert(0) += 1;
        }

        self.ingredients()
            .iter()
            .all(|(req_res, req_qty)| counts.get(req_res).unwrap_or(&0) >= req_qty)
    }
}

impl Explorer {
    /// Returns the absolute priority resource to craft
    pub fn get_production_priority(&self) -> ResourceType {
        let bag = self.bag.to_resource_types();
        self.calculate_priority(&bag)
    }

    /// Checks the bag of the explorer and finds the needed resource by looking at the
    /// dependency graph of the resources. The most complex resource needed is returned first
    fn calculate_priority(&self, bag: &[ResourceType]) -> ResourceType {
        if bag.contains(&ResourceType::Complex(ComplexResourceType::Robot))
            && bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond))
        {
            // if the explorer has robot and diamond
            return ResourceType::Complex(ComplexResourceType::AIPartner);
        }

        let carbon_count = bag
            .iter()
            .filter(|r| **r == ResourceType::Basic(BasicResourceType::Carbon))
            .count();
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond)) {
            if carbon_count >= 2 {
                // if he has no diamond but at least 2 carbon
                return ResourceType::Complex(ComplexResourceType::Diamond);
            }
            // if he has no diamond and max 1 carbon
            return ResourceType::Basic(BasicResourceType::Carbon);
        }

        // if the explorer doesn't have robot
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Robot)) {
            let has_silicon = bag.contains(&ResourceType::Basic(BasicResourceType::Silicon));
            let has_life = bag.contains(&ResourceType::Complex(ComplexResourceType::Life));

            if has_life {
                return if has_silicon {
                    // if he has life and silicon
                    ResourceType::Complex(ComplexResourceType::Robot)
                } else {
                    // if he has life and not silicon
                    ResourceType::Basic(BasicResourceType::Silicon)
                };
            }

            // if he has no life
            let has_water = bag.contains(&ResourceType::Complex(ComplexResourceType::Water));
            if has_water {
                return if carbon_count >= 1 {
                    // if he has water and carbon
                    ResourceType::Complex(ComplexResourceType::Life)
                } else {
                    // if he has water but no carbon
                    ResourceType::Basic(BasicResourceType::Carbon)
                };
            }

            // if he has no water
            let has_h = bag.contains(&ResourceType::Basic(BasicResourceType::Hydrogen));
            let has_o = bag.contains(&ResourceType::Basic(BasicResourceType::Oxygen));

            if has_h && has_o {
                // if he has both hydrogen and oxygen
                return ResourceType::Complex(ComplexResourceType::Water);
            }
            if !has_h {
                // if he has hydrogen but no oxygen
                return ResourceType::Basic(BasicResourceType::Hydrogen);
            }
            // if he has no hydrogen nor oxygen
            if !has_o {
                return ResourceType::Basic(BasicResourceType::Oxygen);
            }
        }

        // this shouldn't happen (all possible cases should have been taken in consideration)
        // println!(
        //     "[EXPLORER TOMMY DEBUG] Something went wrong in the decision of the next needed resource."
        // );
        ResourceType::Basic(BasicResourceType::Carbon)
    }

    /// Returns an HashSet containing all the resources needed
    pub fn resources_needed(&self) -> HashSet<ResourceType> {
        let bag = self.bag.to_resource_types();
        let mut res = HashSet::new();

        if bag.contains(&ResourceType::Complex(ComplexResourceType::Robot))
            && bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond))
        {
            // if the explorer has robot and diamond
            res.insert(ResourceType::Complex(ComplexResourceType::AIPartner));
        }

        let carbon_count = bag
            .iter()
            .filter(|r| **r == ResourceType::Basic(BasicResourceType::Carbon))
            .count();
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Diamond)) {
            if carbon_count >= 2 {
                // if he has no diamond but at least 2 carbon
                res.insert(ResourceType::Complex(ComplexResourceType::Diamond));
            } else {
                // if he has no diamond and max 1 carbon
                res.insert(ResourceType::Basic(BasicResourceType::Carbon));
            }
        }

        // if the explorer doesn't have robot
        if !bag.contains(&ResourceType::Complex(ComplexResourceType::Robot)) {
            let has_silicon = bag.contains(&ResourceType::Basic(BasicResourceType::Silicon));
            let has_life = bag.contains(&ResourceType::Complex(ComplexResourceType::Life));

            if has_life {
                if has_silicon {
                    // if he has life and silicon
                    res.insert(ResourceType::Complex(ComplexResourceType::Robot));
                } else {
                    // if he has life and not silicon
                    res.insert(ResourceType::Basic(BasicResourceType::Silicon));
                }
            }

            // if he has no life
            let has_water = bag.contains(&ResourceType::Complex(ComplexResourceType::Water));
            if has_water {
                if carbon_count >= 1 {
                    // if he has water and carbon
                    res.insert(ResourceType::Complex(ComplexResourceType::Life));
                } else {
                    // if he has water but no carbon
                    res.insert(ResourceType::Basic(BasicResourceType::Carbon));
                }
            }

            // if he has no water
            let has_h = bag.contains(&ResourceType::Basic(BasicResourceType::Hydrogen));
            let has_o = bag.contains(&ResourceType::Basic(BasicResourceType::Oxygen));

            if has_h && has_o {
                // if he has both hydrogen and oxygen
                res.insert(ResourceType::Complex(ComplexResourceType::Water));
            }
            if !has_h {
                // if he has hydrogen but no oxygen
                res.insert(ResourceType::Basic(BasicResourceType::Hydrogen));
            }
            // if he has no hydrogen nor oxygen
            if !has_o {
                // if he has no oxygen
                res.insert(ResourceType::Basic(BasicResourceType::Oxygen));
            }
        }

        res
    }

    /// Returns the resource to generate/combine based on the needs and the availability of the planet,
    /// or None if no resource can be crafted.
    pub fn decide_resource_action(&self) -> Option<ResourceType> {
        let current_planet_info = self.topology.get(self.planet_id)?;
        let needed = self.resources_needed();

        let bag_items = self.bag.to_resource_types();

        let craft_order = [
            ComplexResourceType::AIPartner,
            ComplexResourceType::Robot,
            ComplexResourceType::Diamond,
            ComplexResourceType::Life,
            ComplexResourceType::Water,
        ];

        // Pipeline for complex resources
        let complex_target = craft_order.into_iter()
            // the planet has to support the resource
            .filter(|c| {
                current_planet_info
                    .get_complex_resources()
                    .map_or(false, |set| set.contains(c))
            })
            // I need to be able to craft it with the ingredients in the bag
            .filter(|c| c.can_be_crafted(&bag_items))
            .map(ResourceType::Complex)
            .find(|res| needed.contains(res));

        // if there is a complex target return it
        if complex_target.is_some() {
            return complex_target;
        }

        // if not, search for a basic target
        current_planet_info
            .get_basic_resources()
            .and_then(|planet_basic| {
                planet_basic
                    .iter()
                    .map(|&b| ResourceType::Basic(b))
                    .find(|res| needed.contains(res))
            })
    }
}
