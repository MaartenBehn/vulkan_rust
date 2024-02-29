use crate::math::get_config;
use crate::node::NodeController;
use crate::ship::Ship;
use crate::{
    math::to_3d,
    node::NodeID,
    renderer::{self, Vertex},
    ship::Wave,
};
use app::{
    anyhow::Result,
    glam::{uvec3, vec3, BVec3, IVec3, UVec3},
    log,
    vulkan::{
        ash::vk, gpu_allocator::MemoryLocation, utils::create_gpu_only_buffer_from_data, Buffer,
        CommandBuffer, Context,
    },
};
use std::mem::{align_of, size_of};

pub struct ShipMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_counter: u32,
}

impl ShipMesh {
    pub fn new(context: &Context, ship: &Ship) -> Result<ShipMesh> {
        let max_index = (ship.wave_size.x * ship.wave_size.y * ship.wave_size.z) as usize;
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

        let index_buffer = create_gpu_only_buffer_from_data(
            context,
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

    pub fn update(&mut self, ship: &Ship, node_controller: &NodeController) -> Result<()> {
        let mut vertecies = Vec::new();
        self.index_counter = 0;

        // Nodes
        for (wave_index, wave) in ship.wave.iter().enumerate() {
            let wave_pos = to_3d(wave_index as u32, ship.wave_size).as_ivec3();
            let config = get_config(wave_pos);

            let pattern = &node_controller.patterns[config][wave.render_pattern];
            if pattern.node.is_none() {
                continue;
            }

            let (mut v, _) = Self::get_node_mesh(pattern.node, wave_pos);

            vertecies.append(&mut v);
            self.index_counter += 36;
        }

        self.vertex_buffer
            .copy_data_to_buffer_complex(&vertecies, 0, align_of::<Vertex>())?;

        Ok(())
    }

    pub fn get_node_mesh(node_id: NodeID, offset: IVec3) -> (Vec<Vertex>, Vec<u32>) {
        let v_pos = offset.as_vec3();

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
