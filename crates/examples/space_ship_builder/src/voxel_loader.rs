use std::collections::HashMap;

use dot_vox::{DotVoxData, Position, SceneNode};
use octa_force::anyhow::{bail, Result};
use octa_force::glam::{ivec3, uvec3, IVec3, UVec3};
use octa_force::log::{self, debug};

use crate::math::to_1d;
use crate::node::{Material, Node, NodeID, NODE_SIZE, NODE_VOXEL_LENGTH};
use crate::rotation::Rot;

pub struct VoxelLoader {
    pub path: String,
    pub mats: [Material; 256],
    pub nodes: Vec<Node>,
    pub rules: HashMap<UVec3, NodeID>,
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
        let rules = Self::load_rules(&data)?;

        let mut voxel_loader = Self {
            path,
            mats,
            nodes,
            rules,
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
                let x = (NODE_SIZE.x - 1) - v.x as u32; // Flip x to match game axis system.
                let y = (NODE_SIZE.y - 1) - v.y as u32; // Flip x to match game axis system.

                voxels[to_1d(uvec3(x as u32, y as u32, v.z as u32), NODE_SIZE)] = v.i
            }

            nodes.push(Node::new(voxels));
        }

        Ok(nodes)
    }

    fn load_rules(data: &DotVoxData) -> Result<HashMap<UVec3, NodeID>> {
        let r = data.scenes.iter().find_map(|node| match node {
            SceneNode::Transform {
                attributes: a,
                child: c,
                frames: f,
                layer_id: _,
            } => {
                if a.contains_key("_name") && a["_name"] == "rules" {
                    let node = &data.scenes[*c as usize];

                    let p = f[0].position().unwrap_or(Position::from((0, 0, 0)));
                    let pos = ivec3(p.x, p.y, p.z);

                    match node {
                        SceneNode::Group {
                            attributes: _,
                            children: cs,
                        } => Some((cs.to_owned(), pos)),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        });

        let (rule_ids, root_pos) = if r.is_none() {
            bail!("Rules node not found!");
        } else {
            r.unwrap()
        };

        let mut rules = HashMap::new();

        for id in rule_ids {
            let node = &data.scenes[id as usize];
            match node {
                SceneNode::Transform {
                    attributes: a,
                    frames: f,
                    child: c,
                    layer_id: _,
                } => {
                    let r = f[0].position();
                    let d = r.unwrap_or(Position { x: 0, y: 0, z: 0 });
                    let p = ivec3(d.x, d.y, d.z) + root_pos;
                    let fp = (p - ivec3(4, 4, 4)) / 8;
                    let tp = (p - ivec3(4, 4, 4)) % 8;

                    if fp.cmplt(IVec3::ZERO).any() || tp.cmpne(IVec3::ZERO).any() {
                        log::warn!("Skipping Rule at {d:?}");
                        continue;
                    }
                    let pos = fp.as_uvec3();

                    let rot = f[0]
                        .attributes
                        .iter()
                        .find_map(|(key, val)| {
                            if key == "_r" {
                                let r = val.parse::<u8>();
                                if r.is_ok() {
                                    Some(r.unwrap().into())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .unwrap_or(Rot::IDENTITY);

                    let node = &data.scenes[*c as usize];
                    let model_id = match node {
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

                    rules.insert(
                        pos,
                        NodeID {
                            rot,
                            index: model_id as usize,
                        },
                    );
                }
                _ => bail!("Rule is not Transform!"),
            }
        }

        Ok(rules)
    }
}
