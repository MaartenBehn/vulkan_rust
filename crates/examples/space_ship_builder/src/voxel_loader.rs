use app::anyhow::{bail, Result};
use app::glam::{ivec3, uvec3, IVec3};
use dot_vox::{DotVoxData, SceneNode};

use crate::math::to_1d;
use crate::voxel::{Node, NODE_SIZE, NODE_VOXEL_LENGTH, Material};
use crate::rotation::Rot;

pub struct VoxelLoader {
    pub path: String,
    pub mats: [Material; 256],
    pub nodes: Vec<Node>,
    pub rules: Vec<(IVec3, Rot, u32)>,
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
                voxels[to_1d(uvec3(v.x as u32, v.y as u32, v.z as u32), NODE_SIZE)] = v.i
            }

            nodes.push(Node::new(voxels));
        }

        Ok(nodes)
    }

    fn load_rules(data: &DotVoxData) -> Result<Vec<(IVec3, Rot, u32)>> {
        let r = data.scenes.iter().find_map(|node| match node {
            SceneNode::Transform {
                attributes: a,
                child: c,
                frames: _,
                layer_id: _,
            } => {
                if a.contains_key("_name") && a["_name"] == "rules" {
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

        let rule_ids = if r.is_none() {
            bail!("Rules node not found!");
        } else {
            r.unwrap()
        };

        let mut rules = Vec::new();
        for id in rule_ids {
            let node = &data.scenes[id as usize];
            match node {
                SceneNode::Transform {
                    attributes: _,
                    frames: f,
                    child: c,
                    layer_id: _,
                } => {
                    let r = f[0].position();
                    let pos = if r.is_some() {
                        let d = r.unwrap();
                        ivec3(d.x - 4, d.y - 4, d.z - 4)
                    } else {
                        IVec3::ZERO
                    };

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

                    rules.push((pos, rot, model_id));
                }
                _ => bail!("Rule is not Transform!"),
            }
        }

        Ok(rules)
    }
}
