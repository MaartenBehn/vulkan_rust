use crate::math::{oct_positions, to_1d, to_1d_i};
use crate::node::{Node, NodeID, Voxel, NODE_SIZE};
use crate::rotation::Rot;
use crate::rules::Rules;
use octa_force::anyhow::bail;
use octa_force::glam::{BVec3, EulerRot, IVec3, Mat3, Mat4, Quat, Vec3};
use std::mem;
use std::ops::Mul;

pub type BlockNameIndex = usize;
pub const BLOCK_INDEX_EMPTY: BlockNameIndex = 0;

pub type BlockIndex = usize;

#[derive(Copy, Clone, Default, Debug)]
pub struct Block {
    pub node_ids: [NodeID; 8],
}

impl Block {
    pub fn from_node_ids(node_ids: [NodeID; 8]) -> Self {
        Self { node_ids }
    }

    pub fn from_single_node_id(node_id: NodeID) -> Self {
        let mut node_ids = [NodeID::empty(); 8];
        for (i, r) in node_id.rot.get_all_flipped().into_iter().enumerate() {
            node_ids[i] = NodeID::new(node_id.index, r);
        }

        Self { node_ids }
    }

    pub fn flip(&self, flip: BVec3, rules: &mut Rules) -> Block {
        let mut node_ids = self.node_ids.to_owned();

        if flip.x {
            node_ids.swap(0, 1);
            node_ids.swap(2, 3);
            node_ids.swap(4, 5);
            node_ids.swap(6, 7);
        }

        if flip.y {
            node_ids.swap(0, 2);
            node_ids.swap(1, 3);
            node_ids.swap(4, 6);
            node_ids.swap(5, 7);
        }

        if flip.z {
            node_ids.swap(0, 4);
            node_ids.swap(1, 5);
            node_ids.swap(2, 6);
            node_ids.swap(3, 7);
        }

        Block::from_node_ids(node_ids)
    }

    pub fn rotate(&self, rot: Rot, rules: &mut Rules) -> Block {
        let mat: Mat4 = rot.into();

        let rot_offset = Node::get_voxel_rot_offset(rot);

        let mut rotated_node_ids = [NodeID::empty(); 8];
        for (node_id, pos) in self.node_ids.iter().zip(oct_positions().iter()) {
            let rotated_node_id = NodeID::new(node_id.index, node_id.rot * rot);
            let rotated_node_id = rules.get_duplicate_node_id(rotated_node_id);

            let p = *pos - IVec3::ONE;
            let pos_f = mat.transform_vector3(p.as_vec3());
            let rot_pos = pos_f.round().as_ivec3() + IVec3::ONE - rot_offset;

            let index = to_1d_i(rot_pos, IVec3::ONE * 2) as usize;
            rotated_node_ids[index] = rotated_node_id;
        }

        Block::from_node_ids(rotated_node_ids)
    }
}
