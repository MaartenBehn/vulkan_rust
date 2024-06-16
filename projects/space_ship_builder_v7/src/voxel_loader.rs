use crate::math::to_1d;
use crate::node::{Material, Node, NODE_SIZE, NODE_VOXEL_LENGTH};
use crate::rotation::Rot;
use dot_vox::{DotVoxData, SceneNode};
use octa_force::anyhow::{anyhow, bail, Result};
use octa_force::glam::{ivec3, uvec3, IVec3, UVec3};
use std::process::id;

pub struct VoxelLoader {
    pub path: String,
    pub data: DotVoxData,
}

enum ModelBlockKind {
    Folder(usize),
    Multi(usize),
}

impl VoxelLoader {
    pub fn new(path: &str) -> Result<VoxelLoader> {
        let r = dot_vox::load(path);
        let data = if r.is_err() {
            bail!("Could not load .vox file");
        } else {
            r.unwrap()
        };

        let voxel_loader = Self {
            path: path.to_owned(),
            data,
        };

        Ok(voxel_loader)
    }

    pub fn reload(&mut self) -> Result<()> {
        let r = dot_vox::load(&self.path);
        let data = if r.is_err() {
            bail!("Could not reload .vox file");
        } else {
            r.unwrap()
        };
        self.data = data;

        Ok(())
    }

    pub fn load_materials(&self) -> [Material; 256] {
        let mut mats = [Material::default(); 256];
        for (i, color) in self.data.palette.iter().enumerate() {
            mats[i] = color.into();
        }

        mats
    }

    pub fn find_model(&self, name: &str) -> Result<(usize, Rot)> {
        self.data
            .scenes
            .iter()
            .find_map(|n| match n {
                SceneNode::Transform {
                    attributes,
                    child,
                    frames,
                    ..
                } => {
                    if attributes.get("_name").is_some_and(|s| s == name) {
                        let model_id = match &self.data.scenes[*child as usize] {
                            SceneNode::Shape { models, .. } => Some(models[0].model_id as usize),
                            _ => None,
                        };

                        if model_id.is_none() {
                            return None;
                        }

                        let r = frames[0].attributes.get("_r");
                        let rot = if r.is_some() {
                            Rot::from(r.unwrap().as_str())
                        } else {
                            Rot::IDENTITY
                        };
                        Some((model_id.unwrap(), rot))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .ok_or(anyhow!("No node or model found for {name}."))
    }

    pub fn load_node_model(&self, model_index: usize) -> Result<Node> {
        let model = &self.data.models[model_index];

        let size = uvec3(model.size.x, model.size.y, model.size.z);
        if size != NODE_SIZE {
            bail!("Node Model of size {} is not of size {}.", size, NODE_SIZE);
        }

        let mut voxels = [0; NODE_VOXEL_LENGTH];
        for v in model.voxels.iter() {
            let x = v.x as u32;
            let y = v.y as u32;
            let z = v.z as u32;

            let pos = uvec3(x, y, z);
            voxels[to_1d(pos, NODE_SIZE)] = v.i;
        }

        Ok(Node::new(voxels))
    }

    pub fn load_multi_node_model(&self, model_index: usize) -> Result<(UVec3, Vec<Node>)> {
        let model = &self.data.models[model_index];

        let size = uvec3(model.size.x, model.size.y, model.size.z);
        if size % NODE_SIZE != UVec3::ZERO {
            bail!(
                "Node Model with size {} is not multiple of size {}.",
                size,
                NODE_SIZE
            );
        }

        let nodes_size = size / NODE_SIZE;
        let mut nodes =
            vec![Node::new([0; NODE_VOXEL_LENGTH]); nodes_size.element_product() as usize];

        for v in model.voxels.iter() {
            let x = v.x as u32;
            let y = v.y as u32;
            let z = v.z as u32;

            let pos = uvec3(x, y, z);
            let model_pos = pos / NODE_SIZE;
            let in_model_pos = pos % NODE_SIZE;

            let node_index = to_1d(model_pos, nodes_size);
            let voxel_index = to_1d(in_model_pos, NODE_SIZE);
            nodes[node_index].voxels[voxel_index] = v.i;
        }

        Ok((nodes_size, nodes))
    }

    pub fn load_node_folder_models(&self, name: &str) -> Result<(UVec3, Vec<(Node, Rot, UVec3)>)> {
        let (model_ids, rot) = self.get_model_folder(name)?;
        if rot != Rot::default() {
            bail!("Folder should not be rotated!")
        }

        let mut max = IVec3::ZERO;
        let mut min = IVec3::ZERO;
        let mut nodes = vec![];
        for (id, rot, pos) in model_ids.into_iter() {
            let node = self.load_node_model(id)?;
            nodes.push((node, rot, pos));
            max = ivec3(
                i32::max(max.x, pos.x),
                i32::max(max.y, pos.y),
                i32::max(max.z, pos.z),
            );
            min = ivec3(
                i32::min(min.x, pos.x),
                i32::min(min.y, pos.y),
                i32::min(min.z, pos.z),
            );
        }

        let size = (max - min).as_uvec3();
        let mut final_nodes = vec![];
        for (node, rot, pos) in nodes.into_iter() {
            final_nodes.push((node, rot, (pos - min).as_uvec3()))
        }

        Ok((size, final_nodes))
    }

    pub fn get_model_folder(&self, name: &str) -> Result<(Vec<(usize, Rot, IVec3)>, Rot)> {
        self.data
            .scenes
            .iter()
            .find_map(|n| match n {
                SceneNode::Transform {
                    attributes,
                    child,
                    frames,
                    ..
                } => {
                    if attributes.get("_name").is_some_and(|s| s == name) {
                        let child_ids = match &self.data.scenes[*child as usize] {
                            SceneNode::Group { children, .. } => children
                                .iter()
                                .map(|child| match &self.data.scenes[*child as usize] {
                                    SceneNode::Transform {
                                        frames,
                                        child,
                                        attributes,
                                        ..
                                    } => {
                                        let model_id = match &self.data.scenes[*child as usize] {
                                            SceneNode::Shape { models, .. } => {
                                                Some(models[0].model_id as usize)
                                            }
                                            _ => None,
                                        }
                                        .unwrap();

                                        let r = frames[0].attributes.get("_r");
                                        let rot = if r.is_some() {
                                            Rot::from(r.unwrap().as_str())
                                        } else {
                                            Rot::IDENTITY
                                        };

                                        let p = frames[0].position().unwrap();
                                        let pos = ivec3(p.x, p.y, p.z);

                                        Some((model_id, rot, pos))
                                    }
                                    _ => None,
                                })
                                .collect(),
                            _ => None,
                        };

                        if child_ids.is_none() {
                            return None;
                        }

                        let r = frames[0].attributes.get("_r");
                        let rot = if r.is_some() {
                            Rot::from(r.unwrap().as_str())
                        } else {
                            Rot::IDENTITY
                        };
                        Some((child_ids.unwrap(), rot))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .ok_or(anyhow!("No node or model found for {name}."))
    }

    pub fn get_name_folder(&self, name: &str) -> Result<(Vec<(String, Rot, IVec3)>, Rot)> {
        self.data
            .scenes
            .iter()
            .find_map(|n| match n {
                SceneNode::Transform {
                    attributes,
                    child,
                    frames,
                    ..
                } => {
                    if attributes.get("_name").is_some_and(|s| s == name) {
                        let child_ids = match &self.data.scenes[*child as usize] {
                            SceneNode::Group { children, .. } => children
                                .iter()
                                .map(|child| match &self.data.scenes[*child as usize] {
                                    SceneNode::Transform {
                                        frames, attributes, ..
                                    } => {
                                        let r = frames[0].attributes.get("_r");
                                        let rot = if r.is_some() {
                                            Rot::from(r.unwrap().as_str())
                                        } else {
                                            Rot::IDENTITY
                                        };

                                        let p = frames[0].position().unwrap();
                                        let pos = ivec3(p.x, p.y, p.z);

                                        let name = attributes.get("_name").unwrap().to_owned();

                                        Some((name, rot, pos))
                                    }
                                    _ => None,
                                })
                                .collect(),
                            _ => None,
                        };

                        if child_ids.is_none() {
                            return None;
                        }

                        let r = frames[0].attributes.get("_r");
                        let rot = if r.is_some() {
                            Rot::from(r.unwrap().as_str())
                        } else {
                            Rot::IDENTITY
                        };
                        Some((child_ids.unwrap(), rot))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .ok_or(anyhow!("No node or model found for {name}."))
    }
}
