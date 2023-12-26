use std::{future::IntoFuture, mem::size_of};

use app::{
    anyhow::Result,
    glam::{vec3, vec4, IVec3, UVec3, Vec3, Vec4},
    vulkan::{ash::vk, gpu_allocator::MemoryLocation, Buffer, CommandBuffer, Context},
};

use crate::{
    math::to_3d,
    renderer::Vertex,
    ship::{Node, NodeID, Ship},
};

pub const MAX_VERTECIES: usize = 10000;
pub const MAX_INDICES: usize = 50000;

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

        for (i, node) in ship.nodes.iter().enumerate() {
            if node.id == 0 {
                continue;
            }

            let pos = to_3d(i as u32, ship.size);
            let (mut vertices, indecies) = Self::get_node_mesh(node.id, pos.as_ivec3(), 0.9, 1.0);

            for i in indecies {
                self.indecies.push(i + self.index_counter);
            }

            self.index_counter += vertices.len() as u32;
            self.vertecies.append(&mut vertices);
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
        let node_colors = [
            vec4(1.0, 0.0, 0.0, opacity),
            vec4(1.0, 0.0, 0.0, opacity),
            vec4(0.0, 0.0, 1.0, opacity),
            vec4(0.0, 1.0, 0.0, opacity),
        ];

        let v_pos = offset.as_vec3();
        let color = node_colors[node_id];
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
