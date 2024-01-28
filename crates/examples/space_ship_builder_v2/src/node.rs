use std::mem::size_of;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, uvec3, IVec3, UVec3};
use app::log;
use app::vulkan::ash::vk::ExtTransformFeedbackFn;
use dot_vox::Color;

use crate::pattern_config::Config;
use crate::voxel_loader;
use crate::{rotation::Rot, voxel_loader::VoxelLoader};

pub type NodeIndex = usize;
pub type BlockIndex = usize;
pub type Voxel = u8;

pub const NODE_INDEX_NONE: NodeIndex = NodeIndex::MAX;
pub const BLOCK_INDEX_NONE: BlockIndex = BlockIndex::MAX;
pub const VOXEL_EMPTY: Voxel = 0;

pub const NODE_SIZE: UVec3 = uvec3(8, 8, 8);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

#[derive(Clone, Debug)]
pub struct NodeController {
    pub nodes: Vec<Node>,
    pub mats: [Material; 256],
    pub blocks: Vec<Block>,
    pub pattern: [Vec<Pattern>; 256],
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Node {
    voxels: [Voxel; NODE_VOXEL_LENGTH],
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

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Block {
    pub name: String,
    pub node_index: NodeIndex,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pattern {
    id: NodeID,
}

impl NodeController {
    pub fn new(voxel_loader: VoxelLoader) -> Result<NodeController> {
        let pattern = Self::make_pattern(&voxel_loader)?;

        Ok(NodeController {
            nodes: voxel_loader.nodes,
            mats: voxel_loader.mats,
            blocks: voxel_loader.blocks,
            pattern: pattern,
        })
    }

    fn make_pattern(voxel_loader: &VoxelLoader) -> Result<[Vec<Pattern>; 256]> {
        let config_to_id = [
            Config::from([true, false, false, false, false, false, false, false]),
            Config::from([true, true, false, false, false, false, false, false]),
            Config::from([true, false, false, true, false, false, false, false]),
            Config::from([true, false, false, false, false, false, false, true]),
            Config::from([true, true, true, false, false, false, false, false]),
            Config::from([true, true, false, false, false, false, true, false]),
            Config::from([true, false, false, true, false, true, false, false]),
            Config::from([true, true, true, true, false, false, false, false]),
            Config::from([true, true, true, false, true, false, false, false]),
            Config::from([true, true, true, false, false, true, false, false]),
            Config::from([true, true, true, false, false, false, false, true]),
            Config::from([true, true, false, false, false, false, true, true]),
            Config::from([true, false, false, true, false, true, true, false]),
            Config::from([true, true, true, true, true, false, false, false]),
            Config::from([true, true, true, false, true, false, false, true]),
            Config::from([true, true, true, false, false, true, true, false]),
            Config::from([true, true, true, true, true, true, false, false]),
            Config::from([true, true, true, true, true, false, false, true]),
            Config::from([true, true, true, false, false, true, true, true]),
            Config::from([true, true, true, true, true, true, true, false]),
            Config::from([true, true, true, true, true, true, true, true]),
        ];

        let mut pattern = std::array::from_fn(|_| Vec::new());

        for block in voxel_loader.pattern.iter() {
            let parts: Vec<_> = block.name.split("_").collect();
            if parts.len() != 2 {
                bail!("Invalid Pattern name! Not exactly one underscore in name!")
            }

            //let name = parts[0];
            let r = parts[1].parse::<i32>();
            let id = if r.is_ok() {
                r.unwrap()
            } else {
                bail!("Invalid Pattern name! Part after underscore is not a number!")
            };

            let config = config_to_id[id as usize - 1];
            let possibilities = config.get_possibilities();

            for (c, rot) in possibilities.into_iter() {
                let index: usize = c.into();

                pattern[index].push(Pattern::new(NodeID::new(block.node_index, rot)));
            }
        }

        Ok(pattern)
    }
}

impl Node {
    pub fn new(voxels: [Voxel; NODE_VOXEL_LENGTH]) -> Self {
        Node { voxels }
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
            log::warn!("None Node Id was converted!");
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

impl Block {
    pub fn new(name: String, node_index: NodeIndex) -> Self {
        Block { name, node_index }
    }

    pub fn get_node_id(&self) -> NodeID {
        NodeID {
            index: self.node_index,
            rot: Rot::default(),
        }
    }
}

impl Pattern {
    pub fn new(node_id: NodeID) -> Self {
        Pattern { id: node_id }
    }
}
