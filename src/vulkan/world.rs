use cgmath::vec3;

use crate::VulkanApp;

use super::mesh::Mesh;

impl VulkanApp{

    pub fn get_world_mesh() -> Mesh {
        let mut cube = Mesh::cube(true);
        cube.transform.set_position(vec3(5.0, 1.0, 0.0));

        let world = Mesh::new_parent_mesh(vec![
            Mesh::floor(true),
            cube,
            ]);
        
        let mut worlds = vec![];
        for i in 0..100{
            let mut new_world = world.clone();
            new_world.transform.set_position(vec3(20.0 * i as f32, 0.0, 0.0));
            if i % 2 != 0{
                new_world.transform.set_scale(vec3(-1.0, 1.0, 1.0));
            }
            worlds.push(new_world);
        }

        Mesh::new_parent_mesh(worlds)
    }
}