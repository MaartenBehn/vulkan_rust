use crate::rotation::Rot;
use dot_vox::Color;
use octa_force::glam::{ivec3, uvec3, vec4, IVec3, Mat4, UVec3};

use crate::math::{to_1d, to_3d};
use crate::rules::block::BlockNameIndex;
use octa_force::glam::Mat3;
use std::hash::Hash;
use std::iter::repeat;
use std::ops::Mul;

pub type NodeIndex = usize;
pub type Voxel = u8;

pub const VOXEL_EMPTY: Voxel = 0;
pub const NODE_SIZE: UVec3 = uvec3(4, 4, 4);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

pub const NODE_INDEX_EMPTY: NodeIndex = 0;
pub const NODE_INDEX_ANY: NodeIndex = NodeIndex::MAX;

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

    pub(crate) fn get_voxel_rot_offset(rot: Rot) -> IVec3 {
        let rot_bits: u8 = rot.into();
        ivec3(
            (rot_bits & (1 << 4) != 0).into(),
            (rot_bits & (1 << 5) != 0).into(),
            (rot_bits & (1 << 6) != 0).into(),
        )
    }

    fn rotate_voxel_pos(pos: UVec3, mat: Mat4, rot_offset: IVec3) -> UVec3 {
        let p = pos.as_ivec3() - (NODE_SIZE / 2).as_ivec3();
        let new_pos_f = mat.transform_vector3(p.as_vec3());
        (new_pos_f.round().as_ivec3() + (NODE_SIZE / 2).as_ivec3() - rot_offset).as_uvec3()
    }

    pub fn get_rotated_voxels(&self, rot: Rot) -> impl Iterator<Item = (UVec3, Voxel)> {
        let mat: Mat4 = rot.into();
        let rot_offset = Self::get_voxel_rot_offset(rot);

        self.voxels
            .into_iter()
            .enumerate()
            .zip(repeat((mat, rot_offset)))
            .map(|((i, v), (mat, rot_offset))| {
                let pos = to_3d(i as u32, NODE_SIZE);
                let new_pos = Self::rotate_voxel_pos(pos, mat, rot_offset);
                (new_pos, v)
            })
    }

    pub fn is_duplicate_node_id(&self, rot: Rot, other_node: &Node, other_rot: Rot) -> bool {
        let mut same = true;

        let mat: Mat3 = rot.into();
        let inv_rot: Rot = mat.inverse().into();
        let combined_rot = other_rot.mul(inv_rot);

        for (rotated_pos, voxel) in other_node.get_rotated_voxels(combined_rot) {
            let voxel_index = to_1d(rotated_pos, NODE_SIZE);

            if self.voxels[voxel_index] != voxel {
                same = false;
                break;
            }
        }

        same
    }

    pub fn shares_side_voxels(
        &self,
        rot: Rot,
        other_node: &Node,
        other_rot: Rot,
        side: IVec3,
    ) -> bool {
        let mat: Mat4 = rot.into();
        let other_mat: Mat4 = other_rot.into();

        let rot_offset = Self::get_voxel_rot_offset(rot);
        let other_rot_offset = Self::get_voxel_rot_offset(other_rot);

        let (index_i, index_j, index_k, k_pos, k_neg) = if side.x == 1 {
            (1, 2, 0, 3, 0)
        } else if side.x == -1 {
            (1, 2, 0, 0, 3)
        } else if side.y == 1 {
            (1, 0, 2, 0, 3)
        } else if side.y == -1 {
            (1, 0, 2, 3, 0)
        } else if side.z == 1 {
            (0, 1, 2, 0, 3)
        } else if side.z == -1 {
            (0, 1, 2, 3, 0)
        } else {
            unreachable!()
        };

        let mut same = true;
        for i in 0..4 {
            for j in 0..4 {
                let mut p = [0, 0, 0];
                p[index_i] = i;
                p[index_j] = j;
                p[index_k] = k_pos;
                let pos = UVec3::from(p);

                p[index_k] = k_neg;
                let other_pos = UVec3::from(p);

                let rotated_pos = Self::rotate_voxel_pos(pos, mat, rot_offset);
                let rotated_other_pos =
                    Self::rotate_voxel_pos(other_pos, other_mat, other_rot_offset);
                let voxel = self.voxels[to_1d(rotated_pos, NODE_SIZE)];
                let other_voxel = other_node.voxels[to_1d(rotated_other_pos, NODE_SIZE)];

                if voxel != other_voxel {
                    same = false;
                    break;
                }
            }
        }

        same
    }
}

impl Default for Node {
    fn default() -> Self {
        Self {
            voxels: [VOXEL_EMPTY; NODE_VOXEL_LENGTH],
        }
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
