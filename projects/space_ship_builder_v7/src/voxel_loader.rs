use crate::math::to_1d;
use crate::node::{Material, Node, NODE_SIZE, NODE_VOXEL_LENGTH};
use crate::rotation::Rot;
use dot_vox::{DotVoxData, SceneNode};
use octa_force::anyhow::{anyhow, bail, Result};
use octa_force::glam::{uvec3, UVec3};

pub struct VoxelLoader {
    pub path: String,
    pub data: DotVoxData,
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

    pub fn load_materials(&self) -> [Material; 256] {
        let mut mats = [Material::default(); 256];
        for (i, color) in self.data.palette.iter().enumerate() {
            mats[i] = color.into();
        }

        mats
    }

    pub fn load_node_model(&self, model_index: usize) -> Result<Node> {
        let model = &self.data.models[model_index];

        let size = uvec3(model.size.x, model.size.y, model.size.z);
        if size != NODE_SIZE {
            bail!("Node Model of size {} is not of size {}.", size, NODE_SIZE);
        }

        let mut voxels = [0; NODE_VOXEL_LENGTH];
        for v in model.voxels.iter() {
            let pos = uvec3(v.x as u32, v.y as u32, v.z as u32);
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
            let pos = uvec3(v.x as u32, v.y as u32, v.z as u32);
            let model_pos = pos / NODE_SIZE;
            let in_model_pos = pos % NODE_SIZE;

            let node_index = to_1d(model_pos, nodes_size);
            let voxel_index = to_1d(in_model_pos, NODE_SIZE);
            nodes[node_index].voxels[voxel_index] = v.i;
        }

        Ok((nodes_size, nodes))
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
                            Rot::from(r.unwrap().as_str()).from_magica()
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
}
