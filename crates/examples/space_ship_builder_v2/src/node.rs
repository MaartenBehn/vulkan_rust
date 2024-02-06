use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use std::ops::Mul;

use app::anyhow::{anyhow, bail, ensure, Result};
use app::glam::{ivec3, uvec3, IVec3, Mat3, Mat4, UVec3, Vec3};
use app::log;
use app::vulkan::ash::vk::ExtTransformFeedbackFn;
use dot_vox::Color;
use serde_json::Value;

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

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Pattern {
    pub prio: usize,
    pub id: NodeID,
    pub req: HashMap<IVec3, Vec<NodeID>>,
}

impl NodeController {
    pub fn new(voxel_loader: VoxelLoader, path: &str) -> Result<NodeController> {
        let (config_to_id, complex_pattern) = Self::load_config(path)?;
        let pattern = Self::make_pattern(&voxel_loader, config_to_id, complex_pattern)?;

        Ok(NodeController {
            nodes: voxel_loader.nodes,
            mats: voxel_loader.mats,
            blocks: voxel_loader.blocks,
            pattern: pattern,
        })
    }

    fn load_config(
        path: &str,
    ) -> Result<(
        Vec<Config>,
        HashMap<String, (Vec<(IVec3, String, Rot)>, usize)>,
    )> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let v: Value = serde_json::from_reader(reader)?;

        let config_to_id = v["config_ids"]
            .as_array()
            .unwrap()
            .into_iter()
            .map(|config| {
                let bools: Vec<bool> = config.as_str().unwrap().chars().map(|c| c == '1').collect();
                ensure!(bools.len() == 8, "Config string wrong size!");

                Ok(Config::from(bools))
            })
            .collect::<Result<Vec<_>>>()?;

        let complex_pattern = v["complex_patterns"]
            .as_object()
            .unwrap()
            .into_iter()
            .map(|(name, complex)| {
                let prio = complex["prio"].as_u64().unwrap() as usize;
                let xs = complex["x"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap() as i32);
                let ys = complex["y"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap() as i32);
                let zs = complex["z"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap() as i32);
                let names = complex["name"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap());
                let rot = complex["rot"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| Rot::from(v.as_u64().unwrap() as u8));

                if xs.len() == ys.len()
                    && ys.len() == zs.len()
                    && zs.len() == names.len()
                    && names.len() == rot.len()
                {
                    log::error!("Config file complex pattern not same length!");
                }

                let req: Vec<_> = xs
                    .zip(ys)
                    .zip(zs)
                    .zip(names)
                    .zip(rot)
                    .map(|((((x, y), z), name), rot)| (ivec3(x, y, z), name.to_owned(), rot))
                    .collect();

                (name.to_owned(), (req, prio))
            })
            .collect();

        Ok((config_to_id, complex_pattern))
    }

    fn make_pattern(
        voxel_loader: &VoxelLoader,
        config_to_id: Vec<Config>,
        complex_pattern: HashMap<String, (Vec<(IVec3, String, Rot)>, usize)>,
    ) -> Result<[Vec<Pattern>; 256]> {
        let mut patterns = std::array::from_fn(|_| Vec::new());
        patterns[0] = vec![Pattern::new(NodeID::none(), HashMap::new(), 0)];

        for block in voxel_loader.pattern.iter() {
            let name = &block.name;
            let parts: Vec<_> = name.split("_").collect();

            //let base = parts[0];
            let id = parts[1].parse::<usize>()?;

            let config = config_to_id[id - 1];
            let possibilities = config.get_possibilities();

            let (reqs, prio) = if parts.len() > 2 {
                ensure!(
                    complex_pattern.contains_key(name),
                    "Config file dose not contain the block name {name} !"
                );
                complex_pattern[name].to_owned()
            } else {
                (Vec::new(), 0)
            };

            for (c, rot) in possibilities.into_iter() {
                let p_reqs: Vec<_> = reqs
                    .iter()
                    .map(|(pos, name, p_rot)| {
                        let mat = Mat4::from_mat3(rot.into());
                        let pos1 = mat.transform_point3(pos.as_vec3());
                        let rot1 = rot.mul(*p_rot);
                        let node_index = voxel_loader
                            .pattern
                            .iter()
                            .find_map(|block| {
                                if block.name == *name {
                                    Some(block.node_index)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();

                        (pos1.as_ivec3(), NodeID::new(node_index, rot1))
                    })
                    .collect();

                let mut req_map = HashMap::new();
                for (pos, id) in p_reqs {
                    req_map.entry(pos).or_insert(Vec::new()).push(id);
                }

                let index: usize = c.into();
                patterns[index].push(Pattern::new(
                    NodeID::new(block.node_index, rot),
                    req_map,
                    prio,
                ));
            }
        }

        for pattern in patterns.iter_mut() {
            pattern.sort_by(|a, b| b.prio.cmp(&a.prio));
        }

        Ok(patterns)
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
    pub fn new(node_id: NodeID, req: HashMap<IVec3, Vec<NodeID>>, prio: usize) -> Self {
        Pattern {
            id: node_id,
            req: req,
            prio: prio,
        }
    }
}
