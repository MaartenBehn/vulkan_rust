use std::mem::size_of;
use crate::vulkan::fs;
use super::VulkanApp;
use ash::vk; 


#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
    coords: [f32; 2],
}

impl Vertex {
    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as _)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        let position_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let color_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(12)
            .build();
        let coords_desc = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(24)
            .build();
        [position_desc, color_desc, coords_desc]
    }
}

impl VulkanApp{

    pub fn load_model() -> (Vec<Vertex>, Vec<u32>) {
        log::debug!("Loading model.");
        let mut cursor = fs::load("models/chalet.obj");
        let (models, _) = tobj::load_obj_buf(
            &mut cursor,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            |_| Ok((vec![], ahash::AHashMap::new())),
        )
        .unwrap();

        let mesh = &models[0].mesh;
        let positions = mesh.positions.as_slice();
        let coords = mesh.texcoords.as_slice();
        let vertex_count = mesh.positions.len() / 3;

        let mut vertices = Vec::with_capacity(vertex_count);
        for i in 0..vertex_count {
            let x = positions[i * 3];
            let y = positions[i * 3 + 1];
            let z = positions[i * 3 + 2];
            let u = coords[i * 2];
            let v = coords[i * 2 + 1];

            let vertex = Vertex {
                pos: [x, y, z],
                color: [1.0, 1.0, 1.0],
                coords: [u, v],
            };
            vertices.push(vertex);
        }

        (vertices, mesh.indices.clone())
    }
}