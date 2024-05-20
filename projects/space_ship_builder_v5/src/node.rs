use crate::rotation::Rot;
use dot_vox::Color;
use octa_force::glam::{ivec3, uvec3, vec4, Mat3, Mat4, UVec3};

use crate::math::{to_1d, to_3d, to_3d_i};
use crate::rules::Rules;
use crate::voxel_loader::VoxelLoader;
use octa_force::log::error;
use std::hash::Hash;
use std::iter;
use std::ops::Mul;
use std::path::Iter;

pub type NodeIndex = usize;
pub type BlockIndex = usize;
pub type PatternIndex = usize;
pub type Voxel = u8;

pub const BLOCK_INDEX_EMPTY: BlockIndex = 0;

pub const NODE_INDEX_NONE: NodeIndex = NodeIndex::MAX;
pub const VOXEL_EMPTY: Voxel = 0;

pub const NODE_SIZE: UVec3 = uvec3(4, 4, 4);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;
pub const EMPYT_PATTERN_INDEX: PatternIndex = 0;

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

    pub fn get_rotated_voxels(&self, rot: Rot) -> impl Iterator<Item = (UVec3, Voxel)> {
        let mat: Mat4 = rot.into();
        self.voxels
            .into_iter()
            .enumerate()
            .zip(iter::repeat(mat))
            .map(|((i, v), mat)| {
                let pos = to_3d_i(i as i32, NODE_SIZE.as_ivec3()) - (NODE_SIZE / 2).as_ivec3();
                let pos_f = vec4(pos.x as f32, pos.y as f32, pos.z as f32, 1.0);
                let new_pos_f = mat.mul_vec4(pos_f);
                let new_pos = (ivec3(
                    new_pos_f.x.round() as i32,
                    new_pos_f.y.round() as i32,
                    new_pos_f.z.round() as i32,
                ) + (NODE_SIZE / 2).as_ivec3())
                .as_uvec3();

                if new_pos.cmpge(UVec3::ONE * 4).any() {
                    error!("Invalid rotation")
                }

                (new_pos, v)
            })
    }

    pub fn is_duplicate_node_id(
        node_id: &NodeID,
        test_id: &NodeID,
        voxel_loader: &VoxelLoader,
    ) -> bool {
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
}

impl NodeID {
    pub fn new(index: NodeIndex, rot: Rot) -> NodeID {
        NodeID { index, rot }
    }

    pub fn none() -> NodeID {
        NodeID::default()
    }

    pub fn is_none(self) -> bool {
        self.index == NODE_INDEX_NONE
    }

    pub fn is_some(self) -> bool {
        self.index != NODE_INDEX_NONE
    }
}

impl Default for NodeID {
    fn default() -> Self {
        Self {
            index: NODE_INDEX_NONE,
            rot: Default::default(),
        }
    }
}

impl Into<u32> for NodeID {
    fn into(self) -> u32 {
        if self.is_none() {
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
