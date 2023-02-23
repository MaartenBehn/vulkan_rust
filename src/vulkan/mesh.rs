use crate::vulkan::vertex::Vertex;
use cgmath::{Vector4, Vector3, vec4, Matrix4, Zero};
use super::{transform::Transform};

#[derive(Clone)]
pub struct Mesh{
    vertices: Vec<Vertex>,
    indices: Vec<u32>,

    pub transform: Transform,
    last_transform_matrix: Matrix4<f32>,
    transformed_vertices: Vec<Vertex>,

    child_meshes: Vec<Mesh>,
    is_child_mesh: bool,
    needs_parent_mesh_update: bool,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
       Self { 
            vertices, 
            indices, 
            transform: Transform::default(), 
            last_transform_matrix: Matrix4::zero(),
            transformed_vertices: vec![], 
            child_meshes: vec![],
            is_child_mesh: true,
            needs_parent_mesh_update: true,
        }
    }

    pub fn new_parent_mesh(mehes: Vec<Mesh>) -> Mesh{
        Self { 
            vertices: vec![], 
            indices: vec![], 
            transform: Transform::default(), 
            last_transform_matrix: Matrix4::zero(),
            transformed_vertices:  vec![], 
            child_meshes: mehes,
            is_child_mesh: false,
            needs_parent_mesh_update: true,
        }
    }

    fn generate_mesh(&mut self){
        if self.is_child_mesh || !self.needs_parent_mesh_update{
            return;
        }

        self.vertices = vec![];
        self.indices = vec![];

        let mut v = 0;
        let mut i = 0;
        for mesh in &mut self.child_meshes {
            self.vertices.extend(mesh.get_transformed_vertices());
            self.indices.extend(mesh.get_indices());

            let vl = mesh.vertices.len();
            let il = mesh.indices.len();
            for j in 0..il {
                self.indices[i + j] += v as u32;
            }

            v += vl;
            i += il;
        }

        self.needs_parent_mesh_update = false;

    }

    pub fn get_base_vertices(&mut self) -> &Vec<Vertex> {
        self.generate_mesh();
        &self.vertices
    }

    pub fn get_transformed_vertices(&mut self) -> &Vec<Vertex> {
        self.generate_mesh();

        if self.last_transform_matrix != self.transform.get_matrix() {
            self.transformed_vertices = self.vertices.to_vec();
            for (i, vertex) in self.vertices.iter().enumerate() {
                let new_pos = self.transform.get_matrix() * vec4(vertex.pos[0], vertex.pos[1], vertex.pos[2], 1.0 );

                self.transformed_vertices[i].pos[0] = new_pos[0];
                self.transformed_vertices[i].pos[1] = new_pos[1];
                self.transformed_vertices[i].pos[2] = new_pos[2];
            }
            self.last_transform_matrix = self.transform.get_matrix();
        }

        &self.transformed_vertices
    }

    pub fn get_indices(&mut self) -> &Vec<u32> {
        self.generate_mesh();
        &self.indices
    }

    // Mesh Models
    pub fn plane(double_sided: bool ) -> Mesh {
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
        
        let mut indices: Vec<u32> = vec![0, 1, 2, 3, 2, 1];
        if double_sided {
            indices.extend(vec![2, 1, 0, 1, 2, 3])
        }

        Mesh::new(vertices, indices )
    }

    pub fn floor(double_sided: bool ) -> Mesh {
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

        let mut indices: Vec<u32> = vec![0, 1, 2, 3, 2, 1];
        if double_sided {
            indices.extend(vec![2, 1, 0, 1, 2, 3])
        }

        Mesh::new(vertices, indices )
    }

    pub fn cube(double_sided: bool ) -> Mesh {
        let vertices: Vec<Vertex> = vec![
            Vertex {
                pos: [-1.0, -1.0, -1.0],
                color: [0.0, 0.0, 0.0],
            },
            Vertex {
                pos: [-1.0, -1.0, 1.0],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                pos: [-1.0, 1.0, -1.0],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                pos: [-1.0, 1.0, 1.0],
                color: [0.0, 1.0, 1.0],
            },
            Vertex {
                pos: [1.0, -1.0, -1.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                pos: [1.0, -1.0, 1.0],
                color: [1.0, 0.0, 0.0],
            },
            Vertex {
                pos: [1.0, 1.0, -1.0],
                color: [1.0, 1.0, 0.0],
            },
            Vertex {
                pos: [1.0, 1.0, 1.0],
                color: [1.0, 1.0, 1.0],
            }
        ];

        let mut indices: Vec<u32> = vec![
            0, 1, 3, 0, 3, 2,
            0, 5, 1, 0, 4, 5,
            1, 7, 3, 1, 5, 7, 
            3, 6, 2, 3, 7, 6, 
            2, 4, 0, 2, 6, 4, 
            4, 7, 5, 4, 6, 7,
            ];
        if double_sided {
            indices.extend(vec![
                3, 1, 0, 2, 3, 0,
                1, 5, 0, 5, 4, 0,
                3, 7, 1, 7, 5, 1, 
                2, 6, 3, 6, 7, 3, 
                0, 4, 2, 4, 6, 2, 
                5, 7, 4, 7, 6, 4,
                ]);
        }  

        Mesh::new(vertices, indices )
    }
}

impl Default for Mesh {
    // An Empty Mesh
    fn default() -> Self {
        Self::new(vec![], vec![])
    }
}