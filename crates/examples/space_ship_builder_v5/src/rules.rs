use crate::math::get_neighbors;
use crate::node::{BlockIndex, NodeID};
use crate::voxel_loader::VoxelLoader;
use octa_force::egui::ahash::HashMap;
use octa_force::glam::IVec3;

pub struct Rules {
    pub node_rules: Vec<HashMap<IVec3, Vec<NodeID>>>,
    pub block_rules: Vec<HashMap<IVec3, Vec<BlockIndex>>>,
    pub map_rules_index_to_node_id: Vec<NodeID>,

    pub affected_by_block: Vec<Vec<IVec3>>,
    pub affected_by_node: HashMap<NodeID, Vec<IVec3>>,
}

impl Rules {
    pub fn new(voxel_loader: &VoxelLoader) -> Self {
        let mut possible_node_neighbor_list = Vec::new();
        let mut possible_block_neighbor_list = Vec::new();
        let mut node_id_index_map = Vec::new();

        let mut affected_by_block = Vec::new();
        for _ in 0..voxel_loader.block_names.len() {
            affected_by_block.push(Vec::new())
        }

        let mut affected_by_node = HashMap::default();

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
                let neighbor_offset = offset;
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

                possible_ids.push(neighbor_node_id.unwrap().to_owned());

                let mut affected = affected_by_node
                    .entry(neighbor_node_id.unwrap().to_owned())
                    .or_insert(Vec::new());
                affected.push(neighbor_offset * -1);
            }

            // Neighbor Blocks
            let block_pos = (pos.as_ivec3() / 2) * 2;
            let in_block_pos = pos.as_ivec3() % 2;

            for offset in get_neighbors() {
                let neighbor_offset = offset * 2;
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

                let block_neigbor_offset = neighbor_offset - in_block_pos;
                let possible_ids = possible_block_neighbor_list[node_id_index]
                    .entry(block_neigbor_offset)
                    .or_insert(Vec::new());

                possible_ids.push(neighbor_block_index.unwrap().to_owned());

                // Affected Blocks
                affected_by_block[neighbor_block_index.unwrap().to_owned()]
                    .push(block_neigbor_offset * -1);
            }
        }

        affected_by_block.iter_mut().for_each(|offsets| {
            offsets.sort_by(|p, q| p.element_sum().cmp(&q.element_sum()));
            offsets.dedup()
        });

        Rules {
            node_rules: possible_node_neighbor_list,
            block_rules: possible_block_neighbor_list,
            map_rules_index_to_node_id: node_id_index_map,
            affected_by_block,
            affected_by_node,
        }
    }
}
