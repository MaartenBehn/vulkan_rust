use crate::VulkanApp;

use super::mesh::Mesh;

impl VulkanApp{

    pub fn get_world_mesh() -> Mesh {
        Mesh::super_mesh(vec![
            //Mesh::floor(),
            Mesh::cube(),
            ])
    }
}