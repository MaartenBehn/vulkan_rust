use crate::node::{NodeIndex, NODE_INDEX_NONE};
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
    pub blocks: Vec<Block>,
}

impl VoxelLoader {
    pub fn new(path: &str) -> Result<VoxelLoader> {
        let r = dot_vox::load(path);
        let data = if r.is_err() {
            bail!("Could not load .vox file");
        } else {
            r.unwrap()
        };

        let mats = Self::load_materials(&data)?;
        let nodes = Self::load_models(&data)?;
        let blocks = Self::load_blocks(&data)?;

        let voxel_loader = Self {
            path: path.to_owned(),
            mats,
            nodes,
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

    fn load_blocks(data: &DotVoxData) -> Result<Vec<Block>> {
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
                        let block_name = parts[1];

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
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        let mut blocks: Vec<Block> = Vec::new();
        blocks.push(Block::new("Empty".to_owned(), Vec::new()));
        for (block_name, children_ids) in block_children_ids.into_iter() {
            let mut nodes = Vec::new();
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

                        let child = &data.scenes[*child_id as usize];
                        let model_id = match child {
                            SceneNode::Shape {
                                attributes: _,
                                models: m,
                            } => {
                                if m.is_empty() {
                                    bail!("Rule child model list is empty!");
                                }

                                m[0].model_id as usize
                            }
                            _ => bail!("Rule child is not Model!"),
                        };

                        let parts: Vec<_> = node_name.split("_").collect();
                        if parts.len() != 2 {
                            bail!("Block Child Name to short")
                        }

                        let r = parts[1].parse::<usize>();
                        if r.is_err() {
                            bail!("Block Child Name Number invalid")
                        }
                        let node_type = r.unwrap();

                        if nodes.len() <= node_type {
                            nodes.resize(node_type + 1, NODE_INDEX_NONE)
                        }
                        nodes[node_type] = model_id;
                    }
                    _ => {
                        bail!("Block Child is not Transform Node")
                    }
                };
            }

            blocks.push(Block::new(block_name, nodes));
        }

        Ok(blocks)
    }
}
