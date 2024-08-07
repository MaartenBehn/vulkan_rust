use std::mem::{align_of, size_of};

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
    math::to_3d,
    node::{BlockIndex, NodeController, NodeID, BLOCK_INDEX_NONE},
    renderer::Vertex,
    ship::Wave,
};

pub struct ShipMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
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

        let mesh = ShipMesh {
            vertex_buffer,
            index_buffer,
            index_counter: 0,
        };

        Ok(mesh)
    }

    pub fn update(&mut self, size: UVec3, wave: &Vec<Wave>) -> Result<()> {
        let mut vertecies = Vec::new();
        self.index_counter = 0;

        // Blocks
        /*
        for (i, block_index) in blocks.iter().enumerate() {
            if *block_index == BLOCK_INDEX_NONE {
                continue;
            }

            let pos = to_3d(i as u32, size);
            let node_id = node_controller.blocks[*block_index].get_node_id();
            let (mut v, _) = Self::get_node_mesh(node_id, pos.as_ivec3(), 1.0, false);

            vertecies.append(&mut v);
            self.index_counter += 36;
        }
        */

        // Nodes
        for (i, wave) in wave.iter().enumerate() {
            if wave.possible_pattern.is_empty() || wave.possible_pattern[0].id.is_none() {
                continue;
            }

            let pos = to_3d(i as u32, size);
            let (mut v, _) =
                Self::get_node_mesh(wave.possible_pattern[0].id, pos.as_ivec3(), 1.0, true);

            vertecies.append(&mut v);
            self.index_counter += 36;
        }

        self.vertex_buffer
            .copy_data_to_buffer_complex(&vertecies, 0, align_of::<Vertex>())?;

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer) {
        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.bind_index_buffer(&self.index_buffer);
        buffer.draw_indexed(self.index_counter);
    }

    pub fn get_node_mesh(
        node_id: NodeID,
        offset: IVec3,
        opacity: f32,
        is_node: bool,
    ) -> (Vec<Vertex>, Vec<u32>) {
        let mut v_pos = offset.as_vec3();
        if is_node {
            v_pos -= vec3(0.5, 0.5, 0.5)
        }

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
