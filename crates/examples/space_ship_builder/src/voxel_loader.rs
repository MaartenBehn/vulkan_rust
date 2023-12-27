use app::anyhow::{bail, format_err, Result};
use app::glam::{ivec2, ivec3, mat3, IVec3, Mat3};
use app::log;
use dot_vox::{DotVoxData, SceneGroup, SceneNode};

use crate::Rotation::Rot;

pub struct VoxelLoader {
    pub path: String,
}

impl VoxelLoader {
    pub fn new(path: String) -> Result<()> {
        let r = dot_vox::load(&path);
        let model = if r.is_err() {
            bail!("Could not load .vox file");
        } else {
            r.unwrap()
        };

        let mut voxel_loader = Self { path };

        voxel_loader.load_models(&model)?;
        let rules = voxel_loader.load_rules(&model)?;

        Ok(())
    }

    fn load_models(&mut self, model: &DotVoxData) -> Result<()> {
        Ok(())
    }

    fn load_rules(&mut self, model: &DotVoxData) -> Result<Vec<(IVec3, Rot, u32)>> {
        let r = model.scenes.iter().find_map(|node| match node {
            SceneNode::Transform {
                attributes: a,
                child: c,
                frames: _,
                layer_id: _,
            } => {
                if a.contains_key("_name") && a["_name"] == "rules" {
                    let node = &model.scenes[*c as usize];
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
            let node = &model.scenes[id as usize];
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

                    let node = &model.scenes[*c as usize];
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
