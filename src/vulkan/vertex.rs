use std::mem::size_of;
use crate::vulkan::fs;
use super::VulkanApp;
use ash::vk; 

use cgmath::num_traits::ToPrimitive;
use delaunator::*;


#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
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

    pub fn cube_model() -> (Vec<Vertex>, Vec<u32>){
        let vertices: Vec<Vertex> = vec![
            Vertex {
                pos: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                pos: [1.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                pos: [0.0, 1.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                pos: [1.0, 1.0, 0.0],
                color: [1.0, 1.0, 0.0],
            }
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 3, 2, 1];
        (vertices, indices)
    }

    pub fn voronoi_model() -> (Vec<Vertex>, Vec<u32>) {
        let points = vec![
            Point { x: 0., y: 0. },
            Point { x: 1., y: 0. },
            Point { x: 1., y: 1. },
            Point { x: 0., y: 1. },
        ];
        
        let result = triangulate(&points);
        
        println!("{:?}", result.triangles); // [0, 2, 1, 0, 3, 2]

        let mut vertices: Vec<Vertex> = Vec::with_capacity(points.capacity());
        for ele in points {
            vertices.push(Vertex{
                pos: [ele.x as f32, ele.y as f32, 0.0],
                color:  [1.0, 1.0, 1.0],
            });
        }

        let mut indicies: Vec<u32> = Vec::with_capacity(result.triangles.capacity());
        for ele in result.triangles {
            indicies.push(ele as u32);
        }

        (vertices, indicies)
    }
}