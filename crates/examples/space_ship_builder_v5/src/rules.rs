use crate::math::get_neighbors;
use crate::voxel_loader::VoxelLoader;
use octa_force::egui::ahash::HashMap;
use octa_force::log::debug;

pub struct Rules {}

impl Rules {
    pub fn new(voxel_loader: &VoxelLoader) -> Self {
        let mut possible_neighbor_list = Vec::new();
        let mut node_id_index_map = Vec::new();
        for (pos, node_id) in voxel_loader.node_positions.iter() {
            if node_id.is_none() {
                // Dont add empty Nodes
                continue;
            }

            let r = node_id_index_map
                .iter()
                .position(|test_id| test_id == node_id);
            let node_id_index = if r.is_none() {
                possible_neighbor_list.push(HashMap::default());
                node_id_index_map.push(node_id.to_owned());
                node_id_index_map.len() - 1
            } else {
                r.unwrap()
            };

            for neighbor_offset in get_neighbors() {
                let neighbor_pos = pos.as_ivec3() + neighbor_offset * 4;

                if neighbor_pos.is_negative_bitmask() != 0 {
                    continue;
                }

                let neighbor_node_id = voxel_loader.node_positions.get(&neighbor_pos.as_uvec3());
                if neighbor_node_id.is_none() {
                    // No Node at neighbor pos

                    continue;
                }

                let possible_ids = possible_neighbor_list[node_id_index]
                    .entry(neighbor_offset)
                    .or_insert(Vec::new());

                possible_ids.push(neighbor_node_id)
            }
        }
        

        debug!("{:?}", possible_neighbor_list);

        Rules {}
    }
}
