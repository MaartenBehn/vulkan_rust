use std::{mem::size_of};
use super::VulkanApp;
use ash::vk; 
use rand_distr::{UnitSphere, Distribution};

use rand::*;

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

    pub fn plane_model() -> (Vec<Vertex>, Vec<u32>){
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

    pub fn voronoi_sphere_model() -> (Vec<Vertex>, Vec<u32>){

        let mut rng = rand::thread_rng();

        let mut points: Vec<spherical_voronoi::Point> = Vec::with_capacity(20);
        for _ in 0..points.capacity() {
            points.push(spherical_voronoi::Point::from(UnitSphere.sample(&mut rng)));
        }

        let mut s:Sphere = Sphere{
            vertecies: Vec::new(),
            edges: Vec::new(),
            cell_indecies: Vec::new(),
        };

        spherical_voronoi::build(&points, &mut s);

        let mut indices: Vec<u32> = Vec::new();
       
        for i in 0..s.cell_indecies.len() {
            let mut cell_edges: Vec<&[usize; 2]> = Vec::new();
            
            for ele in &s.edges {
                if s.cell_indecies[i].contains(&ele[0]) && s.cell_indecies[i].contains(&ele[1]) && !cell_edges.contains(&ele) {
                    cell_edges.push(ele);
                }
            }

            let mut ring: Vec<usize> = Vec::new();
            ring.push(cell_edges[0][0]);
            ring.push(cell_edges[0][1]);

            let mut avalable: Vec<usize> = Vec::with_capacity(cell_edges.len()-1);
            for i in 1..cell_edges.len() {
                avalable.push(i);
            }

            for _ in 2..cell_edges.len() {

                let d = ring[ring.len() -1 ];
                for k in 0..avalable.len() {

                    if d == cell_edges[avalable[k]][0] {
                        ring.push(cell_edges[avalable[k]][1]);
                        avalable.remove(k);
                        break;
                    }
                    else if d == cell_edges[avalable[k]][1] {
                        ring.push(cell_edges[avalable[k]][0]);
                        avalable.remove(k);
                        break;
                    }
                }
            }

            for i in 1..ring.len() -1 {
                indices.push(ring[0] as u32);
                indices.push(ring[i] as u32);
                indices.push(ring[i+1] as u32);

                indices.push(ring[0] as u32);
                indices.push(ring[i+1] as u32);
                indices.push(ring[i] as u32);
            }
            indices.push(ring[0] as u32);
            indices.push(ring[1] as u32);
            indices.push(ring[ring.len()-1] as u32);

            indices.push(ring[0] as u32);
            indices.push(ring[ring.len()-1] as u32);
            indices.push(ring[1] as u32);
        }

        (s.vertecies, indices)
    }
}

struct Sphere{
    vertecies: Vec<Vertex>,
    edges: Vec<[usize; 2]>,
    cell_indecies: Vec<Vec<usize>>
}

impl spherical_voronoi::Visitor for Sphere {
    fn vertex(&mut self, point: spherical_voronoi::Point, cells: [usize; 3]) {

        let mut rng = rand::thread_rng();
        self.vertecies.push(Vertex{ 
            pos: [point.x as f32, point.y as f32, point.z as f32,], 
            color: [rng.gen(), rng.gen(), rng.gen()] 
        });

        for ele in cells {
            self.cell_indecies[ele].push(self.vertecies.len() - 1);
        }
    }

    fn edge(&mut self, vertices: [usize; 2]) {
        self.edges.push(vertices);
    }

    fn cell(&mut self) {
        self.cell_indecies.push(Vec::new());
    }
}