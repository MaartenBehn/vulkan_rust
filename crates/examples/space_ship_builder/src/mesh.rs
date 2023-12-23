use std::future::IntoFuture;

use app::{
    anyhow::Result,
    glam::{vec3, UVec3, Vec3},
};

use crate::{
    math::to_3d,
    ship::{Node, Ship},
};

pub const MAX_VERTECIES: usize = 1000;
pub const MAX_INDICES: usize = 5000;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Vec3,
}

pub struct Mesh {
    pub vertecies: Vec<Vertex>,
    pub indecies: Vec<u32>,
    pub index_counter: u32,
}

impl Mesh {
    pub fn from_ship(ship: &Ship) -> Result<Mesh> {
        let mut mesh = Mesh {
            vertecies: Vec::new(),
            indecies: Vec::new(),
            index_counter: 0,
        };

        for (i, node) in ship.nodes.iter().enumerate() {
            if node.id == 0 {
                continue;
            }

            let pos = to_3d(i as u32, ship.size);
            mesh.add_node(pos, node)
        }

        Ok(mesh)
    }

    fn add_node(&mut self, pos: UVec3, node: &Node) {
        let node_colors = [
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.5, 1.0),
            vec3(1.0, 0.0, 0.5),
        ];

        let v_pos = pos.as_vec3();
        let color = node_colors[node.id];
        let mut vertices = vec![
            Vertex {
                position: vec3(0.0, -0.0, 0.0) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.9, 0.0, 0.0) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.0, 0.9, 0.0) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.9, 0.9, 0.0) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.0, 0.0, 0.9) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.9, 0.0, 0.9) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.0, 0.9, 0.9) + v_pos,
                color,
            },
            Vertex {
                position: vec3(0.9, 0.9, 0.9) + v_pos,
                color,
            },
        ];

        let indecies = [
            0, 1, 2, 3, 2, 1, //
            6, 5, 4, 5, 6, 7, //
            0, 4, 1, 1, 4, 5, //
            1, 5, 3, 3, 5, 7, //
            2, 3, 6, 3, 7, 6, //
            0, 2, 6, 6, 4, 0,
        ];
        for i in indecies {
            self.indecies.push(i + self.index_counter);
        }

        self.index_counter += vertices.len() as u32;
        self.vertecies.append(&mut vertices);
    }
}
