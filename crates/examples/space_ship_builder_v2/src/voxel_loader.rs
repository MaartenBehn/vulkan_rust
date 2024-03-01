use std::collections::HashMap;

use octa_force::anyhow::{bail, Result};
use octa_force::glam::{ivec3, uvec3, IVec3, UVec3};
use octa_force::log::{self, debug};
use dot_vox::{DotVoxData, Position, SceneNode};

use crate::math::to_1d;
use crate::node::{Block, Material, Node, NodeID, NodeIndex, NODE_SIZE, NODE_VOXEL_LENGTH};

pub struct VoxelLoader {
    pub path: String,
    pub mats: [Material; 256],
    pub nodes: Vec<Node>,
    pub blocks: Vec<Block>,
    pub pattern: Vec<Block>,
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
        let blocks = Self::load_blocks(&data, "Blocks")?;
        let pattern = Self::load_blocks(&data, "Pattern")?;

        let voxel_loader = Self {
            path,
            mats,
            nodes,
            blocks,
            pattern,
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

    fn load_blocks(data: &DotVoxData, name: &str) -> Result<Vec<Block>> {
        let r = data.scenes.iter().find_map(|node| match node {
            SceneNode::Transform {
                attributes: a,
                child: c,
                frames: f,
                layer_id: _,
            } => {
                if a.contains_key("_name") && a["_name"] == name {
                    let node = &data.scenes[*c as usize];

                    match node {
                        SceneNode::Group {
                            attributes: _,
                            children: cs,
                        } => Some(cs.to_owned()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        });

        let block_ids = if r.is_none() {
            bail!(format!("{name:} node not found!"));
        } else {
            r.unwrap()
        };

        let mut blocks = Vec::new();

        for id in block_ids.into_iter() {
            let node = &data.scenes[id as usize];

            match node {
                SceneNode::Transform {
                    attributes: a,
                    frames: _,
                    child: c,
                    layer_id: _,
                } => {
                    let name = if a.contains_key("_name") {
                        a["_name"].to_owned()
                    } else {
                        bail!("Node name not found!")
                    };

                    let node = &data.scenes[*c as usize];
                    let model_id = match node {
                        SceneNode::Shape {
                            attributes: _,
                            models: m,
                        } => {
                            if m.is_empty() {
                                bail!("Node child model list is empty!");
                            }

                            m[0].model_id
                        }
                        _ => bail!("Node child is not Model!"),
                    };

                    blocks.push(Block::new(name, model_id as NodeIndex));
                }
                _ => bail!("Node is not Transform!"),
            }
        }

        Ok(blocks)
    }
}
