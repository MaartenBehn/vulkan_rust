use crate::rotation::Rot;
use dot_vox::Color;
use octa_force::glam::{ivec3, uvec3, vec4, Mat3, Mat4, UVec3, IVec3};

use crate::math::{to_1d, to_3d, to_3d_i};
use crate::rules::Rules;
use crate::voxel_loader::VoxelLoader;
use octa_force::log::error;
use std::hash::Hash;
use std::iter;
use std::iter::repeat;
use std::ops::Mul;
use std::path::Iter;

pub type NodeIndex = usize;
pub type BlockIndex = usize;
pub type Voxel = u8;

pub const VOXEL_EMPTY: Voxel = 0;
pub const NODE_SIZE: UVec3 = uvec3(4, 4, 4);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

pub const NODE_INDEX_EMPTY: NodeIndex = 0;
pub const NODE_INDEX_ANY: NodeIndex = NodeIndex::MAX;

pub const BLOCK_INDEX_EMPTY: BlockIndex = 0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Node {
    pub voxels: [Voxel; NODE_VOXEL_LENGTH],
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct NodeID {
    pub index: NodeIndex,
    pub rot: Rot,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Material {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Node {
    pub fn new(voxels: [Voxel; NODE_VOXEL_LENGTH]) -> Self {
        Node { voxels }
    }


    fn get_voxel_rot_offset(rot: Rot) -> IVec3 {
        let rot_bits: u8 = rot.into();
        ivec3(
            (rot_bits & (1 << 4) != 0).into(),
            (rot_bits & (1 << 5) != 0).into(),
            (rot_bits & (1 << 6) != 0).into(),
        )
    }
    
    fn rotate_voxel_pos(pos: UVec3, mat: Mat4, rot_offset: IVec3) -> UVec3 {
        let p = pos.as_ivec3() - (NODE_SIZE / 2).as_ivec3()
        let pos_f = vec4(p.x as f32, p.y as f32, p.z as f32, 1.0);
        let new_pos_f = mat.mul_vec4(pos_f);
        (ivec3(
            new_pos_f.x.round() as i32,
            new_pos_f.y.round() as i32,
            new_pos_f.z.round() as i32,
        ) + (NODE_SIZE / 2).as_ivec3()
            - rot_offset)
            .as_uvec3()
    }

    pub fn get_rotated_voxels(&self, rot: Rot) -> impl Iterator<Item = (UVec3, Voxel)> {
        let mat: Mat4 = rot.into();
        let rot_offset = Self::get_voxel_rot_offset(rot);
        
        self.voxels
            .into_iter()
            .enumerate()
            .zip(repeat((mat, rot_offset)))
            .map(|((i, v),(mat, rot_offset))| {
                let pos = to_3d(i as u32, NODE_SIZE);
                let new_pos = Self::rotate_voxel_pos(pos, mat, rot_offset);
                (new_pos, v)
            })
    }

    /*
    pub fn is_duplicate_node_id(
        node_id: &NodeID,
        test_id: &NodeID,
        rules: &VoxelLoader,
    ) -> bool {
        if node_id.is_empty() || test_id.is_empty() {
            return node_id.is_empty() && test_id.is_empty();
        }

        let mut same = true;

        let node = &voxel_loader.nodes[node_id.index];
        let test_node = &voxel_loader.nodes[test_id.index];
        let mat: Mat3 = node_id.rot.into();
        let inv_rot: Rot = mat.inverse().into();
        let combined_rot = test_id.rot.mul(inv_rot);

        for (rotated_pos, voxel) in test_node.get_rotated_voxels(combined_rot) {
            let voxel_index = to_1d(rotated_pos, NODE_SIZE);

            if node.voxels[voxel_index] != voxel {
                same = false;
            }
        }

        same
    }
     */
    
    pub fn shares_side_voxels(voxels: impl Iterator<Item = (UVec3, Voxel)>, other_voxels: impl Iterator<Item = (UVec3, Voxel)>, side: IVec3) -> bool {
        
    }
}

impl NodeID {
    pub fn new(index: NodeIndex, rot: Rot) -> NodeID {
        NodeID { index, rot }
    }

    pub fn empty() -> NodeID {
        NodeID {
            index: NODE_INDEX_EMPTY,
            rot: Default::default(),
        }
    }
    pub fn any() -> NodeID {
        NodeID {
            index: NODE_INDEX_ANY,
            rot: Default::default(),
        }
    }

    pub fn is_empty(self) -> bool {
        self.index == NODE_INDEX_EMPTY
    }

    pub fn is_any(self) -> bool {
        self.index == NODE_INDEX_ANY
    }

    pub fn is_some(self) -> bool {
        self.index != NODE_INDEX_EMPTY && self.index != NODE_INDEX_ANY
    }
}

impl Default for NodeID {
    fn default() -> Self {
        Self::empty()
    }
}

impl Into<u32> for NodeID {
    fn into(self) -> u32 {
        if self.is_empty() {
            0
        } else {
            ((self.index as u32) << 7) + <Rot as Into<u8>>::into(self.rot) as u32
        }
    }
}

impl From<NodeIndex> for NodeID {
    fn from(value: NodeIndex) -> Self {
        NodeID::new(value, Rot::default())
    }
}

impl From<Material> for [u8; 4] {
    fn from(color: Material) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}
impl From<&Material> for [u8; 4] {
    fn from(color: &Material) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}
impl From<Color> for Material {
    fn from(value: Color) -> Self {
        Material {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}
impl From<&Color> for Material {
    fn from(value: &Color) -> Self {
        Material {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}
