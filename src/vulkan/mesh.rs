use crate::VulkanApp;
use crate::vulkan::vertex::Vertex;

pub struct Mesh{
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>
}

impl Mesh {
    pub fn new() -> Self {
        Self { vertices: vec![], indices: vec![] }
    }


    pub fn super_mesh(mehes: Vec<Mesh>) -> Mesh{
        let mut super_mesh = Mesh::new();

        let mut v = 0;
        let mut i = 0;
        for mesh in mehes {
            super_mesh.vertices.extend(&mesh.vertices);
            super_mesh.indices.extend(&mesh.indices);

            let vl = mesh.vertices.len();
            let il = mesh.indices.len();
            for j in 0..il {
                super_mesh.indices[i + j] += v as u32;
            }

            v += vl;
            i += il;
        }

        super_mesh
    }


    pub fn plane() -> Mesh {
        let vertices: Vec<Vertex> = vec![
            Vertex {
                pos: [-1.0, -1.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                pos: [1.0, -1.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                pos: [-1.0, 1.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                pos: [1.0, 1.0, 0.0],
                color: [1.0, 1.0, 0.0],
            }
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 3, 2, 1];
        Mesh { vertices, indices }
    }

    pub fn floor() -> Mesh {
        let vertices: Vec<Vertex> = vec![
            Vertex {
                pos: [-10.0, 0.0, -10.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                pos: [-10.0, 0.0, 10.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                pos: [10.0, 0.0, -10.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                pos: [10.0, 0.0, 10.0],
                color: [1.0, 1.0, 0.0],
            }
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 3, 2, 1];
        Mesh { vertices, indices }
    }
}