use std::{
    future::IntoFuture,
    mem::{align_of, size_of},
};

use octa_force::{
    anyhow::Result,
    glam::{uvec3, vec3, vec4, BVec3, IVec3, UVec3, Vec3, Vec4},
    log,
    vulkan::{
        ash::vk, gpu_allocator::MemoryLocation,  Buffer,
        CommandBuffer, Context,
    },
};

use crate::{
    math::{to_1d, to_3d},
    node::NodeID,
    renderer::Vertex,
    ship::{self, Cell, Ship},
};

pub struct ShipMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub vertex_counter: usize,
    pub index_counter: u32,
}

impl ShipMesh {
    pub fn new(context: &Context, max_index: usize) -> Result<ShipMesh> {
        let vertex_size = size_of::<Vertex>() * 8 * max_index;
        log::info!(
            "Ship Vertex Buffer: {:?} MB",
            vertex_size as f32 / 1000000.0
        );

        let vertex_buffer = context.create_buffer(
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            vertex_size as _,
        )?;

        let mut indecies: Vec<u32> = Vec::with_capacity(36 * max_index);
        let mut j = 0;
        for _ in 0..max_index {
            indecies.append(&mut vec![
                0 + j,
                2 + j,
                1 + j,
                3 + j,
                1 + j,
                2 + j,
                6 + j,
                4 + j,
                5 + j,
                5 + j,
                7 + j,
                6 + j,
                0 + j,
                1 + j,
                4 + j,
                1 + j,
                5 + j,
                4 + j,
                1 + j,
                3 + j,
                5 + j,
                3 + j,
                7 + j,
                5 + j,
                2 + j,
                6 + j,
                3 + j,
                3 + j,
                6 + j,
                7 + j,
                0 + j,
                6 + j,
                2 + j,
                6 + j,
                0 + j,
                4 + j,
            ]);

            j += 8;
        }

        let index_size = size_of::<u32>() * indecies.len();
        log::info!("Ship Index Buffer: {:?} MB", index_size as f32 / 1000000.0);

        let index_buffer = context.create_gpu_only_buffer_from_data(
            vk::BufferUsageFlags::INDEX_BUFFER,
            &indecies,
        )?;

        let mut mesh = ShipMesh {
            vertex_buffer,
            index_buffer,
            vertex_counter: 0,
            index_counter: 0,
        };

        Ok(mesh)
    }

    pub fn reset(&mut self) {
        self.vertex_counter = 0;
        self.index_counter = 0;
    }

    pub fn update(
        &mut self,
        cells: &Vec<Cell>,
        size: UVec3,
        changed_indcies: &[usize],
    ) -> Result<()> {
        let last_vertices = self.vertex_counter;

        let mut vertecies = Vec::new();
        for i in changed_indcies.iter() {
            let cell = &cells[*i];
            if cell.id.is_none() {
                continue;
            }

            let pos = to_3d(*i as u32, size);
            let (mut v, _) = Self::get_node_mesh(cell.id, pos.as_ivec3(), 1.0);

            vertecies.append(&mut v);

            self.vertex_counter += 8;
            self.index_counter += 36;
        }

        self.vertex_buffer.copy_data_to_buffer_complex(
            &vertecies,
            last_vertices,
            align_of::<Vertex>(),
        )?;

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer) {
        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.bind_index_buffer(&self.index_buffer);
        buffer.draw_indexed(self.index_counter);
    }

    pub fn get_node_mesh(node_id: NodeID, offset: IVec3, opacity: f32) -> (Vec<Vertex>, Vec<u32>) {
        let v_pos = offset.as_vec3();
        let v = node_id.index as f32 / 20.0;
        let color = vec4(v, v, v, opacity);

        let node_id_bits: u32 = node_id.into();
        let vertices = vec![
            Vertex::new(
                vec3(-0.5, -0.5, -0.5) + v_pos,
                BVec3::new(false, false, false),
                node_id_bits,
            ),
            Vertex::new(
                vec3(0.5, -0.5, -0.5) + v_pos,
                BVec3::new(true, false, false),
                node_id_bits,
            ),
            Vertex::new(
                vec3(-0.5, 0.5, -0.5) + v_pos,
                BVec3::new(false, true, false),
                node_id_bits,
            ),
            Vertex::new(
                vec3(0.5, 0.5, -0.5) + v_pos,
                BVec3::new(true, true, false),
                node_id_bits,
            ),
            Vertex::new(
                vec3(-0.5, -0.5, 0.5) + v_pos,
                BVec3::new(false, false, true),
                node_id_bits,
            ),
            Vertex::new(
                vec3(0.5, -0.5, 0.5) + v_pos,
                BVec3::new(true, false, true),
                node_id_bits,
            ),
            Vertex::new(
                vec3(-0.5, 0.5, 0.5) + v_pos,
                BVec3::new(false, true, true),
                node_id_bits,
            ),
            Vertex::new(
                vec3(0.5, 0.5, 0.5) + v_pos,
                BVec3::new(true, true, true),
                node_id_bits,
            ),
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
