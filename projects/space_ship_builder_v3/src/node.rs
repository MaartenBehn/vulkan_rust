use crate::{rotation::Rot, voxel_loader::VoxelLoader};
use octa_force::anyhow::bail;
use octa_force::glam::ivec3;

use octa_force::glam::BVec3;
use octa_force::glam::Mat3;
use octa_force::glam::Mat4;

use crate::ship::{get_config, Ship};
use dot_vox::Color;
use octa_force::{
    anyhow::Result,
    glam::{uvec3, IVec3, UVec3},
    log,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::Hash;
use std::io::BufReader;

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

#[derive(Clone, Debug)]
pub struct NodeController {
    pub config_path: String,
    pub nodes: Vec<Node>,
    pub mats: [Material; 256],
    pub blocks: Vec<Block>,

    pub patterns: [Vec<Pattern>; 8],
    pub affected_poses: Vec<IVec3>,
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
}

impl NodeController {
    pub fn new(voxel_loader: VoxelLoader, path: &str) -> Result<NodeController> {
        let r = Self::make_patterns(&voxel_loader, path);
        let (patterns, affected_poses) = if r.is_err() {
            log::error!("{}", r.err().unwrap());
            (
                [
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ],
                Vec::new(),
            )
        } else {
            r.unwrap()
        };

        Ok(NodeController {
            config_path: path.to_owned(),
            nodes: voxel_loader.nodes,
            mats: voxel_loader.mats,
            blocks: voxel_loader.blocks,
            patterns,
            affected_poses,
        })
    }

    pub fn load(&mut self, voxel_loader: VoxelLoader) -> Result<()> {
        let r = Self::make_patterns(&voxel_loader, &self.config_path);
        let (patterns, affected_poses) = if r.is_err() {
            log::error!("{}", r.err().unwrap());
            return Ok(());
        } else {
            r.unwrap()
        };

        self.nodes = voxel_loader.nodes;
        self.mats = voxel_loader.mats;
        self.blocks = voxel_loader.blocks;
        self.patterns = patterns;
        self.affected_poses = affected_poses;

        Ok(())
    }

    fn make_patterns(
        voxel_loader: &VoxelLoader,
        path: &str,
    ) -> Result<([Vec<Pattern>; 8], Vec<IVec3>)> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let v: Value = serde_json::from_reader(reader)?;

        let mut named_patterns = HashMap::new();
        let mut pattern_list: Vec<Pattern> = Vec::new();
        let mut permutations_patterns = Vec::new();
        let v = v["blocks"].as_object().unwrap();
        for block in voxel_loader.blocks.iter() {
            if v.contains_key(&block.name) {
                let v = v[&block.name].as_object().unwrap();

                for v in v["patterns"].as_array().unwrap().iter() {
                    // Required fields
                    let node_type = v["type"].as_u64().unwrap() as usize;
                    if node_type >= block.nodes.len() {
                        bail!("Config Nodetype {node_type} not in voxel file!");
                    }
                    let node_index = block.nodes[node_type];

                    let prio = v["prio"].as_u64().unwrap() as usize;

                    // Optional fields
                    let r = v["name"].as_str();
                    let name = if r.is_some() {
                        r.unwrap().to_owned()
                    } else {
                        "".to_owned()
                    };

                    let r = v["flip"].as_array();
                    let mut flip: Vec<_> = if r.is_some() {
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
                    let mut rotate: Vec<_> = if r.is_some() {
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

                    let r = v["copy"].as_object();
                    let copy = if r.is_some() {
                        let name = r.unwrap()["name"].as_str().unwrap().to_owned();
                        let offset_array = r.unwrap()["offset"].as_array().unwrap();
                        let offset = ivec3(
                            offset_array[0].as_i64().unwrap() as i32,
                            offset_array[1].as_i64().unwrap() as i32,
                            offset_array[2].as_i64().unwrap() as i32,
                        );

                        Some((name, offset))
                    } else {
                        None
                    };

                    if name != "" {
                        let index = pattern_list.len();
                        named_patterns.insert(name, (index, flip.to_owned(), rotate.to_owned()));
                    }

                    let mut pattern = Pattern::new(NodeID::from(node_index), prio, block_req);
                    if copy.is_some() {
                        let (copy_name, offset) = copy.unwrap();

                        if copy_name != "" {
                            let r = named_patterns.get(&copy_name);

                            if r.is_some() {
                                let (copy_index, copy_flip, copy_rotate) = r.unwrap();
                                let copy_pattern = &pattern_list[*copy_index];

                                for &f in copy_flip.iter() {
                                    flip.push(f);
                                }

                                for &r in copy_rotate.iter() {
                                    rotate.push(r);
                                }

                                for (&copy_pos, copy_indecies) in copy_pattern.block_req.iter() {
                                    let new_pos = copy_pos - offset;

                                    if !pattern.block_req.contains_key(&new_pos) {
                                        pattern.block_req.insert(new_pos, Vec::new());
                                    }

                                    pattern
                                        .block_req
                                        .get_mut(&new_pos)
                                        .unwrap()
                                        .append(&mut copy_indecies.to_owned());
                                }
                            }
                        }
                    }

                    let mut permuations = Self::permutate_pattern(&pattern, flip, rotate);
                    permutations_patterns.append(&mut permuations);
                    pattern_list.push(pattern);
                }
            }
        }

        pattern_list.push(Pattern::new(NodeID::none(), 0, HashMap::new()));
        pattern_list.append(&mut permutations_patterns);
        log::info!("{:?} Patterns created.", pattern_list.len());

        let mut patterns = [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let mut affected_poses = HashSet::new();
        for pattern in pattern_list.into_iter() {
            let mut config = None;
            for (&pos, _) in pattern.block_req.iter() {
                affected_poses.insert(pos);

                let new_config = get_config(pos);
                if config.is_none() {
                    config = Some(new_config);
                } else {
                    if config.unwrap() != new_config {
                        log::info!("Pattern has two different configs.");
                    }
                }
            }

            if config.is_none() {
                patterns.iter_mut().for_each(|l| l.push(pattern.to_owned()));
            } else {
                patterns[7 - config.unwrap()].push(pattern);
            }
        }

        patterns.iter_mut().for_each(|l| {
            l.sort_by(|p1, p2| p1.prio.cmp(&p2.prio));
        });

        Ok((patterns, affected_poses.into_iter().collect()))
    }

    fn permutate_pattern(
        pattern: &Pattern,
        flips: Vec<BVec3>,
        rotates: Vec<BVec3>,
    ) -> Vec<Pattern> {
        let mut patterns: Vec<Pattern> = Vec::new();

        let mut flipped_patterns = Self::flip_pattern(pattern, &flips);
        patterns.append(&mut flipped_patterns);

        let mut rotated_patterns = Self::rotate_pattern(pattern, &rotates);

        for rotated_pattern in rotated_patterns.iter() {
            let mut flipped_patterns = Self::flip_pattern(rotated_pattern, &flips);
            patterns.append(&mut flipped_patterns);
        }

        patterns.append(&mut rotated_patterns);

        patterns
    }

    fn rotate_pattern(pattern: &Pattern, rotates: &Vec<BVec3>) -> Vec<Pattern> {
        let mut patterns: Vec<Pattern> = Vec::new();

        for &rotate in rotates {
            let mat = Mat4::from_mat3(pattern.node.rot.into());

            let rot_mat_x = if rotate.x {
                Mat4::from_mat3(Mat3::from_cols_array(&[
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0,
                ]))
            } else {
                Mat4::IDENTITY
            };
            let rot_mat_y = if rotate.y {
                Mat4::from_mat3(Mat3::from_cols_array(&[
                    0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0,
                ]))
            } else {
                Mat4::IDENTITY
            };
            let rot_mat_z = if rotate.z {
                Mat4::from_mat3(Mat3::from_cols_array(&[
                    0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ]))
            } else {
                Mat4::IDENTITY
            };

            // Yeah, glsl rotation order is different to glam, so I just create two different matrices.
            // I know ... but it's the simplest fix.
            let pos_mat = rot_mat_x * rot_mat_y * rot_mat_z;
            let rot_mat = rot_mat_z * rot_mat_y * rot_mat_x;

            let rotated_rot: Rot = Mat3::from_mat4(mat * rot_mat).into();

            let rotated_block_req: HashMap<_, _> = pattern
                .block_req
                .iter()
                .map(|(pos, indecies)| {
                    let rotated_pos = pos_mat.transform_point3(pos.as_vec3()).round().as_ivec3();
                    (rotated_pos, indecies.to_owned())
                })
                .collect();

            let rotated_pattern = Pattern::new(
                NodeID::new(pattern.node.index, rotated_rot),
                pattern.prio,
                rotated_block_req,
            );

            patterns.push(rotated_pattern);
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
                pattern.prio,
                block_req,
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
    pub fn new(node: NodeID, prio: usize, block_req: HashMap<IVec3, Vec<BlockIndex>>) -> Self {
        Pattern {
            node,
            prio,
            block_req,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flip_pattern() {
        let mut block_req = HashMap::new();
        block_req.insert(ivec3(2, 2, 2), vec![0]);

        let pattern = Pattern::new(NodeID::new(1, Rot::default()), 2, block_req);

        let flips = vec![
            BVec3::new(true, false, false),
            BVec3::new(false, true, false),
            BVec3::new(true, true, false),
            BVec3::new(false, false, true),
            BVec3::new(true, false, true),
            BVec3::new(false, true, true),
            BVec3::new(true, true, true),
        ];
        let block_reqs = vec![
            ivec3(-1, 2, 2),
            ivec3(2, -1, 2),
            ivec3(-1, -1, 2),
            ivec3(2, 2, -1),
            ivec3(-1, 2, -1),
            ivec3(2, -1, -1),
            ivec3(-1, -1, -1),
        ];

        let flipped_patterns = NodeController::flip_pattern(&pattern, &flips);

        for (flipped_pattern, block_req) in flipped_patterns.into_iter().zip(block_reqs.into_iter())
        {
            assert!(flipped_pattern.block_req.contains_key(&block_req));
        }
    }
}
