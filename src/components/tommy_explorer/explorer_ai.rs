use std::collections::{HashMap, HashSet, VecDeque};
use common_game::components::resource::ResourceType;
use crate::components::tommy_explorer::topology::TopologyManager;

impl TopologyManager {
    /// Returns a path (list of Planet IDs) to the nearest planet that
    /// hasn't been fully explored yet to perform a BFS-based visit.
    /// Returns None if the topology has already been fully discovered.
    pub fn find_path_to_nearest_frontier(&self, start_node: u32) -> Option<VecDeque<u32>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        // maps a node to its parent in the search tree to reconstruct the path
        let mut parent_map = HashMap::new();

        queue.push_back(start_node);
        visited.insert(start_node);

        while let Some(current) = queue.pop_front() {
            let info = self.get(current)?;

            if !info.is_complete() {
                // planet exists, but we don't have its resource info
                return Some(self.reconstruct_path(parent_map, current));
            }

            // if we have neighbour info, add them to the BFS queue
            if let Some(neighbours) = &info.neighbours {
                for &neighbor in neighbours {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        parent_map.insert(neighbor, current);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        None
    }

    /// Takes an HashMap that contains the dependencies parent-child and a target planet,
    /// and returns the path to reach that target. Used in both find_path_to_nearest_frontier
    /// and find_path_to_resource.
    fn reconstruct_path(&self, parent_map: HashMap<u32, u32>, target: u32) -> VecDeque<u32> {
        let mut path = VecDeque::new();
        let mut curr = target;
        while let Some(&parent) = parent_map.get(&curr) {
            path.push_front(curr);
            curr = parent;
        }
        path
    }

    /// Finds the fastest path to a planet that has the specified resource through a BFS.
    /// Returns Some(path) if the path exists, None otherwise.
    pub fn find_path_to_resource(&self, start_id: u32, target: ResourceType) -> Option<VecDeque<u32>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent_map = HashMap::new();

        queue.push_back(start_id);
        visited.insert(start_id);

        while let Some(current) = queue.pop_front() {
            if let Some(info) = self.get(current) {

                // verify that the planet has the target resource
                let can_provide = match target {
                    ResourceType::Basic(b) => info.get_basic_resources().map_or(false, |s| s.contains(&b)),
                    ResourceType::Complex(c) => info.get_complex_resources().map_or(false, |s| s.contains(&c)),
                };

                if can_provide {
                    return Some(self.reconstruct_path(parent_map, current));
                }

                if let Some(neighbours) = info.get_neighbours() {
                    for &neighbor in neighbours {
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor);
                            parent_map.insert(neighbor, current);
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }
        None
    }

}
