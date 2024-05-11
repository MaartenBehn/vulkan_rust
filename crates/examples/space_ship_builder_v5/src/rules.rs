use crate::math::get_neighbors;
use crate::voxel_loader::VoxelLoader;
use octa_force::egui::ahash::HashMap;
use octa_force::glam::IVec3;
use octa_force::log::debug;
use crate::node::{BlockIndex, NodeID};

pub struct Rules {
    pub node_neighbors: Vec<HashMap<IVec3, Vec<NodeID>>>,
    pub block_neighbors:  Vec<HashMap<IVec3, Vec<BlockIndex>>>,
    pub node_id_index_map: Vec<NodeID>,
}

impl Rules {
    pub fn new(voxel_loader: &VoxelLoader) -> Self {
        let mut possible_node_neighbor_list = Vec::new();
        let mut possible_block_neighbor_list = Vec::new();
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
                possible_node_neighbor_list.push(HashMap::default());
                possible_block_neighbor_list.push(HashMap::default());

                node_id_index_map.push(node_id.to_owned());
                node_id_index_map.len() - 1
            } else {
                r.unwrap()
            };

            // Neighbor Nodes
            for offset in get_neighbors() {
                let neighbor_offset = offset * 4;
                let neighbor_pos = pos.as_ivec3() + neighbor_offset;

                if neighbor_pos.is_negative_bitmask() != 0 {
                    continue;
                }

                let neighbor_node_id = voxel_loader.node_positions.get(&neighbor_pos.as_uvec3());
                if neighbor_node_id.is_none() {
                    // No Node at neighbor pos
                    continue;
                }

                let possible_ids = possible_node_neighbor_list[node_id_index]
                    .entry(neighbor_offset)
                    .or_insert(Vec::new());

                possible_ids.push(neighbor_node_id.unwrap().to_owned())
            }

            // Neighbor Blocks
            let block_pos = (pos.as_ivec3() / 8) * 8;
            let in_block_pos = pos.as_ivec3() % 8;

            for offset in get_neighbors() {
                let neighbor_offset = offset * 8;
                let neighbor_pos = block_pos + neighbor_offset;

                if neighbor_pos.is_negative_bitmask() != 0 {
                    continue;
                }

                let neighbor_block_index =
                    voxel_loader.block_positions.get(&neighbor_pos.as_uvec3());
                if neighbor_block_index.is_none() {
                    // No Block at neighbor pos
                    continue;
                }

                let possible_ids = possible_block_neighbor_list[node_id_index]
                    .entry(neighbor_offset - in_block_pos)
                    .or_insert(Vec::new());

                possible_ids.push(neighbor_block_index.unwrap().to_owned());
            }
        }

        debug!("{:?}", possible_node_neighbor_list);

        Rules {
            node_neighbors: possible_node_neighbor_list,
            block_neighbors: possible_block_neighbor_list,
            node_id_index_map,
        }
    }
}
