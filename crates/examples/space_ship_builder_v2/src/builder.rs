use std::mem::size_of;

use app::{
    anyhow::Result,
    camera::Camera,
    controls::Controls,
    glam::IVec3,
    log,
    vulkan::{ash::vk, gpu_allocator::MemoryLocation, Buffer, CommandBuffer, Context},
};

use crate::{
    node::{BlockIndex, NodeController, NodeID},
    renderer::Vertex,
    rotation::Rot,
    ship::Ship,
    ship_mesh::ShipMesh,
};

const MAX_BUILDER_VERTECIES: usize = 8;
const MAX_BUILDER_INDICES: usize = 36;
const SCROLL_SPEED: f32 = 0.01;

type BuilderState = u32;
const STATE_OFF: BuilderState = 0;
const STATE_ON: BuilderState = 1;

pub struct Builder {
    state: BuilderState,
    current_block_index: BlockIndex,

    distance: f32,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

impl Builder {
    pub fn new(context: &Context) -> Result<Builder> {
        let vertex_buffer = context.create_buffer(
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Vertex>() * MAX_BUILDER_VERTECIES) as _,
        )?;

        let index_buffer = context.create_buffer(
            vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Vertex>() * MAX_BUILDER_INDICES) as _,
        )?;

        Ok(Builder {
            state: STATE_ON,
            current_block_index: 0,

            distance: 3.0,

            vertex_buffer,
            index_buffer,
        })
    }

    pub fn update(
        &mut self,
        controls: &Controls,
        camera: &Camera,
        ship: &mut Ship,
        node_controller: &NodeController,
    ) -> Result<()> {
        if self.state == STATE_ON {
            self.distance -= controls.scroll_delta * SCROLL_SPEED;

            let pos = (camera.position + camera.direction * self.distance)
                .round()
                .as_ivec3();

            let node_id = node_controller.blocks[self.current_block_index].get_node_id();
            let (vertecies, indecies) = ShipMesh::get_node_mesh(node_id, pos, 0.5);

            self.vertex_buffer
                .copy_data_to_buffer(vertecies.as_slice())?;

            self.index_buffer.copy_data_to_buffer(indecies.as_slice())?;

            let ship_node = ship.get_block_i(pos);

            if ship_node.is_ok() && controls.left {
                ship.place_node(pos.as_uvec3(), self.current_block_index, node_controller)?;
            }

            if controls.q {
                self.current_block_index += 1;
                if self.current_block_index >= node_controller.blocks.len() {
                    self.current_block_index = 0;
                }
            }
        }

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer) {
        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.bind_index_buffer(&self.index_buffer);
        buffer.draw_indexed(MAX_BUILDER_INDICES as u32);
    }
}
