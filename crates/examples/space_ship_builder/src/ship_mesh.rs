use std::{future::IntoFuture, mem::size_of};

use app::{
    anyhow::Result,
    glam::{uvec3, vec3, vec4, IVec3, UVec3, Vec3, Vec4},
    vulkan::{ash::vk, gpu_allocator::MemoryLocation, Buffer, CommandBuffer, Context},
};

use crate::{
    math::{to_1d, to_3d},
    node::NodeID,
    renderer::Vertex,
    ship::{self, Cell, Ship},
};

pub const MAX_VERTECIES: usize = 40000;
pub const MAX_INDICES: usize = 6000;

pub struct ShipMesh {
    pub vertecies: Vec<Vertex>,
    pub indecies: Vec<u32>,
    pub index_counter: u32,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

impl ShipMesh {
    pub fn new(context: &Context, ship: &Ship) -> Result<ShipMesh> {
        let vertex_buffer = context.create_buffer(
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Vertex>() * MAX_VERTECIES) as _,
        )?;

        let index_buffer = context.create_buffer(
            vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Vertex>() * MAX_INDICES) as _,
        )?;

        let mut mesh = ShipMesh {
            vertecies: Vec::new(),
            indecies: Vec::new(),
            index_counter: 0,

            vertex_buffer,
            index_buffer,
        };

        mesh.update(ship)?;

        Ok(mesh)
    }

    pub fn update(&mut self, ship: &Ship) -> Result<()> {
        self.vertecies.clear();
        self.indecies.clear();
        self.index_counter = 0;

        /*
        for (i, node) in ship.cells.iter().enumerate() {
            if node.id.index == 0 {
                continue;
            }

            let pos = to_3d(i as u32, ship.size);
            let (mut vertices, indecies) = Self::get_node_mesh(node.id, pos.as_ivec3(), 0.9, 1.0);

            for i in indecies {
                self.indecies.push(i + self.index_counter);
            }

            self.index_counter += vertices.len() as u32;
            self.vertecies.append(&mut vertices);
        }*/

        type ChunkShape = block_mesh::ndshape::RuntimeShape<u32, 3>;
        let mut buffer = block_mesh::GreedyQuadsBuffer::new(ship.cells.len());
        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;
        block_mesh::greedy_quads(
            &ship.cells,
            &ChunkShape::new([ship.size.x, ship.size.y, ship.size.z]),
            [0; 3],
            [ship.size.x - 1, ship.size.y - 1, ship.size.z - 1],
            &faces,
            &mut buffer,
        );

        for (i, group) in buffer.quads.groups.iter().enumerate() {
            for quad in group.iter() {
                let pos = uvec3(quad.minimum[0], quad.minimum[1], quad.minimum[2]);
                let cell = ship.cells[to_1d(pos, ship.size)];

                let v = cell.id.index as f32 / 20.0;
                let color = vec4(v, v, v, 1.0);

                let vertecies = faces[i].quad_mesh_positions(&quad, 1.0);
                let indecies = faces[i].quad_mesh_indices(self.index_counter);

                self.index_counter += vertecies.len() as u32;

                let color = [
                    vec4(1.0, 0.0, 0.0, 1.0),
                    vec4(0.0, 1.0, 0.0, 1.0),
                    vec4(0.0, 0.0, 1.0, 1.0),
                    vec4(1.0, 1.0, 1.0, 1.0),
                ];
                for (i, pos) in vertecies.iter().enumerate() {
                    self.vertecies
                        .push(Vertex::new(vec3(pos[0], pos[1], pos[2]), color[i]))
                }

                self.indecies.extend(indecies);
            }
        }

        self.vertex_buffer
            .copy_data_to_buffer(self.vertecies.as_slice())?;

        self.index_buffer
            .copy_data_to_buffer(self.indecies.as_slice())?;

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer) {
        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.bind_index_buffer(&self.index_buffer);
        buffer.draw_indexed(self.indecies.len() as u32);
    }

    pub fn get_node_mesh(
        node_id: NodeID,
        offset: IVec3,
        size: f32,
        opacity: f32,
    ) -> (Vec<Vertex>, Vec<u32>) {
        let v_pos = offset.as_vec3();
        let v = node_id.index as f32 / 20.0;
        let color = vec4(v, v, v, 1.0);
        let vertices = vec![
            Vertex::new(vec3(-0.5, -0.5, -0.5) * size + v_pos, color),
            Vertex::new(vec3(0.5, -0.5, -0.5) * size + v_pos, color),
            Vertex::new(vec3(-0.5, 0.5, -0.5) * size + v_pos, color),
            Vertex::new(vec3(0.5, 0.5, -0.5) * size + v_pos, color),
            Vertex::new(vec3(-0.5, -0.5, 0.5) * size + v_pos, color),
            Vertex::new(vec3(0.5, -0.5, 0.5) * size + v_pos, color),
            Vertex::new(vec3(-0.5, 0.5, 0.5) * size + v_pos, color),
            Vertex::new(vec3(0.5, 0.5, 0.5) * size + v_pos, color),
        ];

        let indecies: Vec<u32> = vec![
            0, 2, 1, 3, 1, 2, //
            6, 4, 5, 5, 7, 6, //
            0, 1, 4, 1, 5, 4, //
            1, 3, 5, 3, 7, 5, //
            2, 6, 3, 3, 6, 7, //
            0, 6, 2, 6, 0, 4,
        ];

        (vertices, indecies)
    }
}
