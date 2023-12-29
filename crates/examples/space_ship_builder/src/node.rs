use std::collections::HashMap;

use app::anyhow::Result;
use app::glam::{uvec3, IVec3, UVec3};
use app::log;
use dot_vox::Color;

use crate::math::get_neigbor_offsets;
use crate::{rotation::Rot, voxel_loader::VoxelLoader};

pub const NODE_SIZE: UVec3 = uvec3(8, 8, 8);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

pub type Voxel = u8;

#[derive(Clone, Debug, Default)]
pub struct NodeController {
    pub nodes: Vec<Node>,
    pub rules: HashMap<NodeID, HashMap<IVec3, Vec<NodeID>>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Node {
    voxels: [Voxel; NODE_VOXEL_LENGTH],
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct NodeID {
    pub index: usize,
    pub rot: Rot,
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
        let rules = Self::generate_rules(&voxel_loader)?;

        Ok(NodeController {
            nodes: voxel_loader.nodes,
            rules,
        })
    }

    fn generate_rules(
        voxel_loader: &VoxelLoader,
    ) -> Result<HashMap<NodeID, HashMap<IVec3, Vec<NodeID>>>> {
        let mut rules: HashMap<NodeID, HashMap<IVec3, Vec<NodeID>>> = HashMap::new();

        let neigbors = get_neigbor_offsets();

        for (pos, id) in voxel_loader.rules.iter() {
            for n in neigbors.iter() {
                let r = pos.as_ivec3() + *n;
                let key = if r.cmplt(IVec3::ZERO).any() {
                    continue;
                } else {
                    r.as_uvec3()
                };

                if voxel_loader.rules.contains_key(&key) {
                    let ids = rules
                        .entry(*id)
                        .or_insert(HashMap::new())
                        .entry(*n)
                        .or_insert(vec![]);

                    let id = voxel_loader.rules.get(&key).unwrap();
                    if !ids.contains(id) {
                        ids.push(id.to_owned());
                    }
                }
            }
        }

        Ok(rules)
    }
}

impl Node {
    pub fn new(voxels: [Voxel; NODE_VOXEL_LENGTH]) -> Self {
        Node { voxels }
    }
}

impl NodeID {
    pub fn new(index: usize, rot: Rot) -> NodeID {
        NodeID { index, rot }
    }

    pub fn none() -> NodeID {
        NodeID::default()
    }

    pub fn is_none(self) -> bool {
        self.index == usize::MAX
    }

    pub fn is_some(self) -> bool {
        self.index != usize::MAX
    }
}

impl Default for NodeID {
    fn default() -> Self {
        Self {
            index: usize::MAX,
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
