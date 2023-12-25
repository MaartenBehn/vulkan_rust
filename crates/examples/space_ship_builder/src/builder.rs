use app::camera::Camera;

use crate::{ship::{NodeID, ID_BEAM}, mesh::Mesh};

const MAX_BUILDER_VERTECIES: usize = 4;
const MAX_BUILDER_INDICES:   usize = 36;

type BUILDER_STATE = u32;
const STATE_OFF: BUILDER_STATE = 0;
const STATE_ON: BUILDER_STATE = 1;

pub struct Builder {
    state: BUILDER_STATE,
    current_node: NodeID,
    distance: f32, 

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

impl Builder {
    pub fn new() -> Builder {
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

        Builder {
            state: STATE_ON,
            current_node: ID_BEAM, 
            distance: 1.0,

            vertex_buffer,
            index_buffer,
        }
    }

    pub fn update(&mut self, camera: &Camera, mesh: &Mesh) {
        if self.state == STATE_ON {
            let pos = (camera.position + camera.direction * self.distance).as_uvec3();

            let node = 
            let mesh = mesh.get_node_mesh(node, offset)
        }
    }
}