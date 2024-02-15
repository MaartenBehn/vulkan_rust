use crate::{
    math::to_1d,
    node::{
        Block, Material, Node, NodeID, Pattern, BLOCK_INDEX_EMPTY, NODE_SIZE, NODE_VOXEL_LENGTH,
    },
    rotation::Rot,
};
use app::{
    anyhow::{bail, Result},
    glam::uvec3,
    log,
};
use dot_vox::{DotVoxData, SceneNode};
use std::collections::HashMap;
use crate::node::{NODE_INDEX_NONE, NodeIndex};

pub struct VoxelLoader {
    pub path: String,
    pub mats: [Material; 256],
    pub nodes: Vec<Node>,
    pub patterns: Vec<Pattern>,
    pub blocks: Vec<Block>,
}

impl VoxelLoader {
    pub fn new(path: String) -> Result<VoxelLoader> {
        let r = dot_vox::load(&path);
        let data = if r.is_err() {
            bail!("Could not load .vox file");
        } else {
            r.unwrap()
        };

        let mats = Self::load_materials(&data)?;
        let nodes = Self::load_models(&data)?;
        let (pattern, blocks) = Self::load_patterns_and_blocks(&data, &nodes)?;

        let voxel_loader = Self {
            path,
            mats,
            nodes,
            patterns: pattern,
            blocks,
        };

        Ok(voxel_loader)
    }

    fn load_materials(data: &DotVoxData) -> Result<[Material; 256]> {
        let mut mats = [Material::default(); 256];
        for (i, color) in data.palette.iter().enumerate() {
            mats[i] = color.into();
        }

        Ok(mats)
    }

    fn load_models(data: &DotVoxData) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        for model in data.models.iter() {
            let mut voxels = [0; NODE_VOXEL_LENGTH];
            for v in model.voxels.iter() {
                //let x = (NODE_SIZE.x - 1) - v.x as u32; // Flip x to match game axis system.
                //let y = (NODE_SIZE.y - 1) - v.y as u32; // Flip x to match game axis system.

                voxels[to_1d(uvec3(v.x as u32, v.y as u32, v.z as u32), NODE_SIZE)] = v.i
            }

            nodes.push(Node::new(voxels));
        }

        Ok(nodes)
    }

    fn load_patterns_and_blocks(data: &DotVoxData, nodes: &Vec<Node>) -> Result<(Vec<Pattern>, Vec<Block>)> {
        let root_children_ids = match &data.scenes[1] {
            SceneNode::Group {
                attributes: _,
                children,
            } => children.to_owned(),
            _ => {
                unreachable!()
            }
        };

        let mut block_children_ids = Vec::new();
        let mut pattern_children_ids = Vec::new();
        for root_child_id in root_children_ids.into_iter() {
            let root_child = &data.scenes[root_child_id as usize];

            match root_child {
                SceneNode::Transform {
                    attributes,
                    frames: _,
                    child: child_id,
                    layer_id: _,
                } => {
                    let pattern_name = if attributes.contains_key("_name") {
                        attributes["_name"].to_owned()
                    } else {
                        continue;
                    };

                    let parts: Vec<_> = pattern_name.split("_").collect();

                    if parts.len() < 2 {
                        continue;
                    }

                    if parts[0] == "B" {
                        let block_name  = parts[1];

                        let child = &data.scenes[*child_id as usize];
                        match child {
                            SceneNode::Group {
                                attributes: _,
                                children,
                            } => block_children_ids
                                .push((block_name.to_owned(), children.to_owned())),
                            _ => {
                                unreachable!()
                            }
                        }
                    } else if parts[0] == "P" {
                        let child = &data.scenes[*child_id as usize];
                        match child {
                            SceneNode::Group {
                                attributes: _,
                                children,
                            } => pattern_children_ids
                                .push((pattern_name.to_owned(), children.to_owned())),
                            _ => {
                                unreachable!()
                            }
                        }
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        let mut blocks: Vec<Block> = vec![Block::default(); 9];
        let general_blocks = ["Empty", "Base", "Other2", "Other3", "Other3", "Other4", "Other5", "Other6", "Other7"];

        let mut found_nodes = vec![false; nodes.len()];
        for (block_name, children_ids) in block_children_ids.into_iter() {

            let mut general_node_indices = [NODE_INDEX_NONE, NODE_INDEX_NONE, NODE_INDEX_NONE, NODE_INDEX_NONE];
            for child_id in children_ids.into_iter() {
                let child = &data.scenes[child_id as usize];

                match child {
                    SceneNode::Transform {
                        attributes,
                        frames,
                        child: child_id,
                        layer_id: _,
                    } => {
                        let node_name = if attributes.contains_key("_name") {
                            attributes["_name"].to_owned()
                        } else {
                            bail!("Block Child has no name")
                        };

                        let parts: Vec<_> = node_name.split("_").collect();

                        if parts.len() != 2 {
                            bail!("Block Child Name to short")
                        }

                        let r = parts[1].parse::<u32>();
                        if r.is_err() {
                            bail!("Block Child Name Number invalid")
                        }

                        let node_type = r.unwrap();
                        if node_type == 0 || node_type > 4 {
                            continue
                        }

                        let child = &data.scenes[*child_id as usize];
                        let model_id = match child {
                            SceneNode::Shape {
                                attributes: _,
                                models: m,
                            } => {
                                if m.is_empty() {
                                    bail!("Rule child model list is empty!");
                                }

                                m[0].model_id
                            }
                            _ => bail!("Rule child is not Model!"),
                        };

                        general_node_indices[(node_type - 1) as usize] = model_id as NodeIndex;
                        found_nodes[model_id as usize] = true;
                    }
                    _ => {
                        bail!("Block Child is not Transform Node")
                    }
                };
            }

            let r = general_blocks.iter().position(|n| (**n) == block_name);
            if r.is_some() {
                blocks[r.unwrap()] = Block::new(block_name, general_node_indices);
            } else {
                blocks.push(Block::new(block_name, general_node_indices));
            }
        }

        let mut double_error_nodes = Vec::new();
        for (i, found_node) in found_nodes.into_iter().enumerate() {
            if i != 0 && !found_node {
                log::warn!("Node: {i} was not used in Blocks!");
                double_error_nodes.push(i);
            }
        }

        let mut patterns = Vec::new();
        let mut print_rot = true;
        for (name, children_ids) in pattern_children_ids.into_iter() {
            if children_ids.len() != 8 {
                bail!("{} has not 8 children!", name)
            }

            let mut node_ids = [NodeID::default(); 8];
            let mut block_indices = [0; 8];

            for child_id in children_ids.into_iter() {
                let child = &data.scenes[child_id as usize];

                match child {
                    SceneNode::Transform {
                        attributes,
                        frames,
                        child: child_id,
                        layer_id: _,
                    } => {
                        let node_name = if attributes.contains_key("_name") {
                            attributes["_name"].to_owned()
                        } else {
                            bail!("Pattern Child has no name")
                        };

                        let parts: Vec<_> = node_name.split("_").collect();

                        if parts.len() != 2 {
                            bail!("Pattern Child Name to short")
                        }

                        let r = parts[1].parse::<u32>();
                        if r.is_err() {
                            bail!("Pattern Child Name Number invalid")
                        }

                        let node_type = r.unwrap();
                        if node_type > 4 {
                            bail!("Node type to big")
                        }

                        let pos = frames[0].position().unwrap();
                        let node_pos =
                            (pos.x > 0) as u8 + ((pos.y > 0) as u8) * 2 + ((pos.z > 0) as u8) * 4;

                        let rot = frames[0]
                            .attributes
                            .iter()
                            .find_map(|(key, val)| {
                                if key == "_r" {
                                    let r = val.parse::<u8>();
                                    if r.is_ok() {
                                        if print_rot {
                                            log::info!("Rot: {}", r.clone().unwrap());
                                            print_rot = false;
                                        }

                                        Some(Rot::from_magica(r.unwrap()))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(Rot::IDENTITY);

                        let child = &data.scenes[*child_id as usize];
                        let model_id = match child {
                            SceneNode::Shape {
                                attributes: _,
                                models: m,
                            } => {
                                if m.is_empty() {
                                    bail!("Rule child model list is empty!");
                                }

                                m[0].model_id
                            }
                            _ => bail!("Rule child is not Model!"),
                        };

                        let r = double_error_nodes.iter().find(|id| (**id) == model_id as usize);
                        if r.is_some() {
                            log::warn!("Double Node {model_id} in Pattern {name}");
                        }

                        let r = blocks
                            .iter()
                            .position(|block| block.name == parts[0]);

                        let block_index = if node_type == 0 {
                            BLOCK_INDEX_EMPTY
                        } else if r.is_some() {
                            r.unwrap()
                        } else {
                            bail!("Unknown Block in Pattern");
                        };

                        block_indices[node_pos as usize] = block_index;
                        node_ids[node_pos as usize] = NodeID::new(model_id as usize, rot);
                    }
                    _ => {
                        bail!("Pattern Child is not Transform Node")
                    }
                };
            }

            patterns.push(Pattern::new(
                block_indices.into(),
                node_ids,
                HashMap::new(),
                0,
            ))
        }

        Ok((patterns, blocks))
    }
}
