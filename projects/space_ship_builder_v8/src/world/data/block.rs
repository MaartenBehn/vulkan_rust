use crate::math::rotation::Rot;
use crate::math::{oct_positions, to_1d_i};
use crate::rules::Rules;
use crate::world::data::node::NodeID;
use octa_force::glam::{IVec3, Mat4};

pub type BlockNameIndex = u8;
pub const BLOCK_INDEX_EMPTY: BlockNameIndex = 0;

pub type BlockIndex = usize;
pub const VOXEL_PER_BLOCK_SIDE: i32 = 8;

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Block {
    pub node_ids: [NodeID; 8],
}

impl Block {
    pub fn from_node_ids(node_ids: [NodeID; 8]) -> Self {
        Self { node_ids }
    }

    pub fn from_node_ids_slice(node_ids: &[NodeID]) -> Self {
        let mut ids = [NodeID::empty(); 8];
        for i in 0..8 {
            ids[i] = node_ids[i].to_owned();
        }

        Self { node_ids: ids }
    }

    pub fn from_single_node_id(node_id: NodeID) -> Self {
        let mut node_ids = [NodeID::empty(); 8];
        for (i, r) in node_id.rot.get_all_flipped().into_iter().enumerate() {
            if node_id.is_empty() {
                node_ids[i] = NodeID::empty();
            } else {
                node_ids[i] = NodeID::new(node_id.index, r);
            }
        }

        Self { node_ids }
    }

    pub fn rotate(&self, rot: Rot, rules: &mut Rules) -> Block {
        let mat: Mat4 = rot.into();

        let rot_offset = rot.rot_offset();

        let mut rotated_node_ids = [NodeID::empty(); 8];
        for (node_id, pos) in self.node_ids.iter().zip(oct_positions().iter()) {
            let rotated_node_id = NodeID::new(node_id.index, rot * node_id.rot);
            let rotated_node_id = rules.get_duplicate_node_id(rotated_node_id);

            let p = *pos - IVec3::ONE;
            let pos_f = mat.transform_vector3(p.as_vec3());
            let rot_pos = pos_f.round().as_ivec3() + IVec3::ONE - rot_offset;

            let index = to_1d_i(rot_pos, IVec3::ONE * 2) as usize;
            rotated_node_ids[index] = rotated_node_id;
        }

        Block::from_node_ids(rotated_node_ids)
    }

    pub fn is_duplicate(&self, other_block: &Block, rules: &mut Rules) -> Option<Rot> {
        for rot in Rot::default().get_all_permutations() {
            let rotated_block = self.rotate(rot, rules);

            if rotated_block == *other_block {
                return Some(rot);
            }
        }

        return None;
    }
}
