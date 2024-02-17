use crate::{
    pattern_config::{BlockConfig, Config},
    rotation::Rot,
    voxel_loader::VoxelLoader,
};
use app::anyhow::bail;
use app::glam::{ivec3, vec3, BVec3, Mat3, Mat4, Vec3};
use app::{
    anyhow::Result,
    glam::{uvec3, IVec3, UVec3},
    log,
};
use dot_vox::Color;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::BufReader;

pub type NodeIndex = usize;
pub type BlockIndex = usize;
pub type Voxel = u8;

pub const BLOCK_INDEX_EMPTY: BlockIndex = 0;
pub const BLOCK_INDEX_BASE: BlockIndex = 1;
pub const BLOCK_INDECIES_GENERAL: [BlockIndex; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
pub const BLOCK_INDECIES_OTHER: [BlockIndex; 7] = [2, 3, 4, 5, 6, 7, 8];

pub const NODE_INDEX_NONE: NodeIndex = NodeIndex::MAX;
pub const VOXEL_EMPTY: Voxel = 0;

pub const NODE_SIZE: UVec3 = uvec3(4, 4, 4);
pub const NODE_VOXEL_LENGTH: usize = (NODE_SIZE.x * NODE_SIZE.y * NODE_SIZE.z) as usize;

#[derive(Clone, Debug)]
pub struct NodeController {
    pub config_path: String,
    pub nodes: Vec<Node>,
    pub mats: [Material; 256],
    pub patterns: Vec<Vec<Pattern>>,
    pub blocks: Vec<Block>,
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
    pub nodes: Vec<NodeIndex>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Pattern {
    pub prio: usize,
    pub node: NodeID,
    pub block_req: HashMap<IVec3, Vec<BlockIndex>>,
    pub node_req: HashMap<IVec3, Vec<NodeIndex>>,
}

impl NodeController {
    pub fn new(voxel_loader: VoxelLoader, path: &str) -> Result<NodeController> {
        let patterns = Self::make_patterns(&voxel_loader, path)?;

        Ok(NodeController {
            config_path: path.to_owned(),
            nodes: voxel_loader.nodes,
            mats: voxel_loader.mats,
            blocks: voxel_loader.blocks,
            patterns: patterns,
        })
    }

    pub fn load(&mut self, voxel_loader: VoxelLoader) -> Result<()> {
        let patterns = Self::make_patterns(&voxel_loader, &self.config_path)?;

        self.nodes = voxel_loader.nodes;
        self.mats = voxel_loader.mats;
        self.blocks = voxel_loader.blocks;
        self.patterns = patterns;

        Ok(())
    }

    fn make_patterns(voxel_loader: &VoxelLoader, path: &str) -> Result<Vec<Vec<Pattern>>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let v: Value = serde_json::from_reader(reader)?;

        let mut patterns = Vec::new();
        for (i, (block_name, v)) in v["blocks"].as_object().unwrap().iter().enumerate() {
            let block = voxel_loader
                .blocks
                .iter()
                .find(|b| (**b).name == (*block_name))
                .unwrap();
            let node_index = block.nodes[i];

            let mut block_patterns: Vec<Pattern> = Vec::new();
            for v in v["nodes"].as_array().unwrap().iter() {
                let prio = v["prio"].as_u64().unwrap() as usize;

                let r = v["block_req"].as_array();
                let block_req: HashMap<_, _> = if r.is_some() {
                    r.unwrap()
                        .iter()
                        .map(|p| {
                            let pos_array = p["pos"].as_array().unwrap();
                            let pos = ivec3(
                                pos_array[0].as_i64().unwrap() as i32,
                                pos_array[1].as_i64().unwrap() as i32,
                                pos_array[2].as_i64().unwrap() as i32,
                            );

                            let blocks: Vec<_> = p["name"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|n| {
                                    let name = n.as_str().unwrap();
                                    let block_index = voxel_loader
                                        .blocks
                                        .iter()
                                        .position(|b| b.name == name)
                                        .unwrap();
                                    block_index
                                })
                                .collect();

                            (pos, blocks)
                        })
                        .collect()
                } else {
                    HashMap::new()
                };

                let r = v["node_req"].as_array();
                let node_req: HashMap<_, _> = if r.is_some() {
                    r.unwrap()
                        .iter()
                        .map(|p| {
                            let pos_array = p["pos"].as_array().unwrap();
                            let pos = ivec3(
                                pos_array[0].as_i64().unwrap() as i32,
                                pos_array[1].as_i64().unwrap() as i32,
                                pos_array[2].as_i64().unwrap() as i32,
                            );

                            let blocks: Vec<_> = p["name"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|n| {
                                    let name = n.as_str().unwrap();

                                    let parts: Vec<_> = name.split("_").collect();
                                    let node_type = parts[1].parse::<usize>().unwrap();

                                    let block = voxel_loader
                                        .blocks
                                        .iter()
                                        .find(|b| b.name == parts[0])
                                        .unwrap();
                                    let node_index = block.nodes[node_type];

                                    node_index
                                })
                                .collect();

                            (pos, blocks)
                        })
                        .collect()
                } else {
                    HashMap::new()
                };

                let pattern = Pattern::new(NodeID::from(node_index), block_req, node_req, prio);
                let permuations = Self::permutate_pattern(pattern);

                for pattern in permuations.into_iter() {
                    if block_patterns
                        .iter()
                        .find(|p| (**p).block_req == pattern.block_req)
                        .is_some()
                    {
                        continue;
                    }

                    block_patterns.push(pattern);
                }
            }

            patterns.push(block_patterns);
        }

        Ok(patterns)
    }

    fn permutate_pattern(pattern: Pattern) -> Vec<Pattern> {
        let rot_angles = [0.0, PI * 0.5];
        let base_rot = pattern.node.rot;

        let mut patterns: Vec<Pattern> = Vec::new();
        for rot_x in rot_angles {
            for rot_y in rot_angles {
                for rot_z in rot_angles {
                    let rot = base_rot.get_permutation(BVec3::FALSE, vec3(rot_x, rot_y, rot_z));
                    let mat: Mat4 = rot.into();

                    let block_reqs: HashMap<_, _> = pattern
                        .block_req
                        .iter()
                        .map(|(pos, indecies)| {
                            let new_pos = (*pos)
                                + ivec3(
                                    (rot_x != 0.0) as u8 as i32,
                                    (rot_y != 0.0) as u8 as i32,
                                    (rot_z != 0.0) as u8 as i32,
                                );
                            let rotated_pos = mat.transform_point3(new_pos.as_vec3());
                            (rotated_pos.round().as_ivec3(), indecies.to_owned())
                        })
                        .collect();

                    if patterns
                        .iter()
                        .find(|p| (**p).block_req == block_reqs)
                        .is_some()
                    {
                        continue;
                    }

                    patterns.push(Pattern::new(
                        NodeID::new(pattern.node.index, rot),
                        block_reqs,
                        pattern.node_req.to_owned(),
                        pattern.prio,
                    ))
                }
            }
        }

        patterns
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
    pub fn new(name: String, nodes: Vec<NodeIndex>) -> Self {
        Block { name, nodes }
    }
}

impl Pattern {
    pub fn new(
        node: NodeID,
        block_req: HashMap<IVec3, Vec<NodeIndex>>,
        node_req: HashMap<IVec3, Vec<NodeIndex>>,
        prio: usize,
    ) -> Self {
        Pattern {
            node,
            block_req,
            node_req,
            prio,
        }
    }
}
