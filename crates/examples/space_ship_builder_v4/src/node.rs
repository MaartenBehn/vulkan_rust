use crate::rotation::Rot;
use dot_vox::Color;
use octa_force::glam::{uvec3, vec4, Mat4, UVec3};

use crate::math::{to_1d, to_3d};
use std::hash::Hash;
use std::iter;
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
                let pos = to_3d(i as u32, NODE_SIZE);
                let pos_f = vec4(pos.x as f32, pos.y as f32, pos.z as f32, 1.0);
                let new_pos_f = mat.mul_vec4(pos_f);
                let new_pos = uvec3(
                    new_pos_f.x.round() as u32,
                    new_pos_f.y.round() as u32,
                    new_pos_f.z.round() as u32,
                );

                (new_pos, v)
            })
    }

    pub fn search_duplicate_node(&self, nodes: &Vec<Node>) -> Option<NodeID> {
        let rots = Rot::default().get_all_permutations();
        for (test_node_index, test_node) in nodes.iter().enumerate() {
            for rot in rots.clone().into_iter() {
                let mut same = true;
                for (rotated_pos, voxel) in test_node.get_rotated_voxels(rot) {
                    let voxel_index = to_1d(rotated_pos, NODE_SIZE);

                    if self.voxels[voxel_index] != voxel {
                        same = false;
                    }
                }

                if same {
                    return Some(NodeID::new(test_node_index, rot));
                }
            }
        }

        None
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
