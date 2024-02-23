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
    pub patterns: Vec<Pattern>,
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
        let r = Self::make_patterns(&voxel_loader, path);
        let patterns = if r.is_err() {
            log::error!("{}", r.err().unwrap());
            Vec::new()
        } else {
            r.unwrap()
        };

        Ok(NodeController {
            config_path: path.to_owned(),
            nodes: voxel_loader.nodes,
            mats: voxel_loader.mats,
            blocks: voxel_loader.blocks,
            patterns: patterns,
        })
    }

    pub fn load(&mut self, voxel_loader: VoxelLoader) -> Result<()> {
        let r = Self::make_patterns(&voxel_loader, &self.config_path);
        let patterns = if r.is_err() {
            log::error!("{}", r.err().unwrap());
            return Ok(());
        } else {
            r.unwrap()
        };

        self.nodes = voxel_loader.nodes;
        self.mats = voxel_loader.mats;
        self.blocks = voxel_loader.blocks;
        self.patterns = patterns;

        Ok(())
    }

    fn make_patterns(voxel_loader: &VoxelLoader, path: &str) -> Result<Vec<Pattern>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let v: Value = serde_json::from_reader(reader)?;

        let mut patterns = Vec::new();
        let v = v["blocks"].as_object().unwrap();
        for block in voxel_loader.blocks.iter() {
            if v.contains_key(&block.name) {
                let v = v[&block.name].as_object().unwrap();

                for v in v["nodes"].as_array().unwrap().iter() {
                    let node_type = v["type"].as_u64().unwrap() as usize;
                    if node_type >= block.nodes.len() {
                        bail!("Config Nodetype {node_type} not in voxel file!");
                    }
                    let node_index = block.nodes[node_type];

                    let prio = v["prio"].as_u64().unwrap() as usize;

                    let r = v["flip"].as_array();
                    let flip: Vec<_> = if r.is_some() {
                        r.unwrap()
                            .iter()
                            .map(|v| {
                                let x = v.as_array().unwrap()[0].as_u64().unwrap() == 1;
                                let y = v.as_array().unwrap()[1].as_u64().unwrap() == 1;
                                let z = v.as_array().unwrap()[2].as_u64().unwrap() == 1;
                                BVec3::new(x, y, z)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let r = v["rotate"].as_array();
                    let rotate: Vec<_> = if r.is_some() {
                        r.unwrap()
                            .iter()
                            .map(|v| {
                                let x = v.as_array().unwrap()[0].as_u64().unwrap() as u32;
                                let y = v.as_array().unwrap()[1].as_u64().unwrap() as u32;
                                let z = v.as_array().unwrap()[2].as_u64().unwrap() as u32;
                                uvec3(x, y, z)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

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

                                let nodes: Vec<_> = p["name"]
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

                                (pos, nodes)
                            })
                            .collect()
                    } else {
                        HashMap::new()
                    };

                    let pattern = Pattern::new(NodeID::from(node_index), block_req, node_req, prio);
                    let mut permuations = Self::permutate_pattern(&pattern, flip, rotate);
                    patterns.push(pattern);
                    patterns.append(&mut permuations);
                }
            }
        }

        patterns.sort_by(|p1, p2| p2.prio.cmp(&p1.prio));
        Ok(patterns)
    }

    fn permutate_pattern(
        pattern: &Pattern,
        flips: Vec<BVec3>,
        rotates: Vec<UVec3>,
    ) -> Vec<Pattern> {
        let mut patterns: Vec<Pattern> = Vec::new();
        let base_rot = Rot::default();

        let mut base_flipped = Self::flip_pattern(pattern, &flips);
        patterns.append(&mut base_flipped);

        let rots = [0.0, PI * 0.5, PI * 1.5];
        for rotate in rotates {
            let mat = Mat4::from_mat3(base_rot.into());
            let mat_x = Mat4::from_rotation_x(rots[rotate.x as usize]);
            let mat_y = Mat4::from_rotation_y(rots[rotate.y as usize]);
            let mat_z = Mat4::from_rotation_z(rots[rotate.z as usize]);
            let rot_mat = mat_x * mat_y * mat_z;
            let rotated_rot: Rot = Mat3::from_mat4(rot_mat * mat).into();

            let rotated_block_req: HashMap<_, _> = pattern
                .block_req
                .iter()
                .map(|(pos, indecies)| {
                    let rotated_pos = rot_mat.transform_point3(pos.as_vec3()).round().as_ivec3();
                    (rotated_pos, indecies.to_owned())
                })
                .collect();

            let rotated_pattern = Pattern::new(
                NodeID::new(pattern.node.index, rotated_rot),
                rotated_block_req,
                pattern.node_req.to_owned(),
                pattern.prio,
            );

            let mut rotated_flipped = Self::flip_pattern(&rotated_pattern, &flips);
            patterns.push(rotated_pattern);
            patterns.append(&mut rotated_flipped);
        }

        patterns
    }

    fn flip_pattern(pattern: &Pattern, flips: &Vec<BVec3>) -> Vec<Pattern> {
        let mut patterns: Vec<Pattern> = Vec::new();

        for flip in flips.iter() {
            let flip_a = ivec3(
                if flip.x { -1 } else { 1 },
                if flip.y { -1 } else { 1 },
                if flip.z { -1 } else { 1 },
            );
            let flip_b = ivec3(
                if flip.x { 1 } else { 0 },
                if flip.y { 1 } else { 0 },
                if flip.z { 1 } else { 0 },
            );
            let rot = pattern.node.rot.flip(flip.to_owned());

            let block_req: HashMap<_, _> = pattern
                .block_req
                .iter()
                .map(|(pos, indecies)| {
                    let flipped_pos = ((*pos) - flip_b) * flip_a;
                    (flipped_pos, indecies.to_owned())
                })
                .collect();

            patterns.push(Pattern::new(
                NodeID::new(pattern.node.index, rot),
                block_req,
                pattern.node_req.to_owned(),
                pattern.prio,
            ))
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
        block_req: HashMap<IVec3, Vec<BlockIndex>>,
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
