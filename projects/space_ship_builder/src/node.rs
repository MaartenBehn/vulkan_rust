use std::collections::HashMap;

use dot_vox::Color;
use octa_force::anyhow::Result;
use octa_force::glam::{ivec3, uvec3, IVec3, UVec3};
use octa_force::log;

use crate::math::get_neigbor_offsets;
use crate::ship::{Cell, PID};
use crate::{rotation::Rot, voxel_loader::VoxelLoader};

pub const NODE_SIZE: UVec3 = uvec3(8, 8, 8);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

pub type Voxel = u8;

#[derive(Clone, Debug)]
pub struct NodeController {
    pub nodes: Vec<Node>,
    pub rules: Vec<Rule>,
    pub full_wave: Vec<PID>,
    pub mats: [Material; 256],
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

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Rule {
    pub req: HashMap<IVec3, NodeID>,
}

pub type RuleIndex = usize;

impl NodeController {
    pub fn new(voxel_loader: VoxelLoader) -> Result<NodeController> {
        let mut rules = Vec::new();
        let mut full_wave = HashMap::new();

        let neigbors = get_neigbor_offsets();
        for (pos, id) in voxel_loader.rules.iter() {
            let mut rule = Rule::new();

            for neigbor_offset in neigbors.iter() {
                let r = pos.as_ivec3() + *neigbor_offset;
                let neigbor_pos = if r.cmplt(IVec3::ZERO).any() {
                    continue;
                } else {
                    r.as_uvec3()
                };
                let offset = *neigbor_offset * ivec3(1, 1, -1); // Because Y in Magica Voxel is wierd

                if !voxel_loader.rules.contains_key(&neigbor_pos) {
                    continue;
                }

                rule.req.insert(
                    offset,
                    voxel_loader.rules.get(&neigbor_pos).unwrap().to_owned(),
                );

                full_wave
                    .entry(*id)
                    .or_insert(PID {
                        p_id: *id,
                        rules: HashMap::new(),
                    })
                    .rules
                    .entry(offset)
                    .or_insert(Vec::new())
                    .push(rules.len());
            }

            rules.push(rule);
        }

        Ok(NodeController {
            nodes: voxel_loader.nodes,
            rules,
            full_wave: full_wave.values().cloned().collect(),
            mats: voxel_loader.mats,
        })
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

impl Rule {
    pub fn new() -> Rule {
        Rule {
            req: HashMap::new(),
        }
    }
}
