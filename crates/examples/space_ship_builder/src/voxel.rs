use app::glam::{uvec3, UVec3, IVec3};
use app::anyhow::Result;
use dot_vox::Color;

use crate::{rotation::Rot, voxel_loader::{self, VoxelLoader}};

pub const NODE_SIZE: UVec3 = uvec3(8, 8, 8);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

pub type Voxel = u8;

pub struct NodeController {
    pub nodes: Vec<Node>,            // For every Node index
    pub node_ids: Vec<NodeId>,       // For every Node id
    pub rules: Vec<Vec<Rule>>
}

pub struct Node {
    voxels: [Voxel; NODE_VOXEL_LENGTH],
}

pub struct NodeId {
    pub index: usize,
    pub rot: Rot,
}

pub struct Rule {
    pub req: Vec<(IVec3, usize)>
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Material {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl NodeController {
    pub fn new(voxel_loader: VoxelLoader) -> Result<NodeController> {
        let (node_ids, rules) = Self::generate_rules(&voxel_loader)?;

        Ok(NodeController { 
            nodes: voxel_loader.nodes, 
            node_ids, 
            rules, 
        })
    }

    fn generate_rules(voxel_loader: &VoxelLoader) -> Result<(Vec<NodeId>, Vec<Vec<Rule>>)> {
        let node_ids = Vec::new();
        let rules = Vec::new();

        

        Ok((node_ids, rules))
    }
}


impl Node {
    pub fn new(voxels: [Voxel; NODE_VOXEL_LENGTH]) -> Self {
        Node { voxels }
    }
}

impl NodeId {
    pub fn new(index: usize, rot: Rot) -> NodeId {
        NodeId {
            index,
            rot,
        }
    }
}

impl Into::<u16> for NodeId {
    fn into(self) -> u16 {
        (self.index as u16) << 7 + <Rot as Into<u8>>::into(self.rot) as u16
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