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
        let (pattern, blocks) = Self::load_patterns(&data)?;

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

    fn load_patterns(data: &DotVoxData) -> Result<(Vec<Pattern>, Vec<Block>)> {
        let root_children_ids = match &data.scenes[1] {
            SceneNode::Group {
                attributes: _,
                children,
            } => children.to_owned(),
            _ => {
                unreachable!()
            }
        };

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

                    if parts.len() < 2 || parts[0] != "P" {
                        continue;
                    }

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
                _ => {
                    continue;
                }
            }
        }

        let mut patterns = Vec::new();
        let mut blocks: Vec<Block> = Vec::new();
        blocks.push(Block::new("Empty".to_owned()));
        blocks.push(Block::new("Base".to_owned()));
        blocks.push(Block::new("Other2".to_owned()));
        blocks.push(Block::new("Other3".to_owned()));
        blocks.push(Block::new("Other4".to_owned()));
        blocks.push(Block::new("Other5".to_owned()));
        blocks.push(Block::new("Other6".to_owned()));
        blocks.push(Block::new("Other7".to_owned()));

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

                        let r = blocks
                            .iter()
                            .enumerate()
                            .find(|(_, block)| (**block).name == parts[0]);
                        let block_index = if node_type == 0 {
                            BLOCK_INDEX_EMPTY
                        } else if r.is_none() {
                            blocks.push(Block::new(parts[0].to_owned()));
                            blocks.len() - 1
                        } else {
                            r.unwrap().0
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
