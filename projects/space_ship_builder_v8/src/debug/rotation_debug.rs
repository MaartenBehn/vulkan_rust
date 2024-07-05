use crate::debug::line_renderer::DebugLine;
use crate::debug::DebugController;
use crate::math::rotation::Rot;
use crate::math::to_1d_i;
use crate::render::mesh::{Mesh, MeshChunk, RenderNode};
use crate::render::mesh_renderer::{MeshRenderer, RENDER_MODE_BASE};
use crate::world::data::node::NodeID;
use log::{debug, info};
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{vec3, vec4, IVec3, Mat4, Vec3};
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::time::{Duration, Instant};

pub const ROTATION_DEBUG_SIZE: i32 = 1;

const INPUT_INTERVAL: Duration = Duration::from_millis(100);

pub struct RotationRenderer {
    mesh: Mesh,
    node_id: NodeID,
    last_input: Instant,
}

impl RotationRenderer {
    pub fn new(image_len: usize, test_node_id: NodeID) -> Self {
        let size = IVec3::ONE * crate::debug::hull_basic::HULL_BASE_DEBUG_SIZE;

        RotationRenderer {
            mesh: Mesh::new(image_len, size, size),
            node_id: test_node_id,
            last_input: Instant::now(),
        }
    }

    pub fn update_controls(&mut self, controls: &Controls) {
        if controls.t && self.last_input.elapsed() > INPUT_INTERVAL {
            self.last_input = Instant::now();

            let rot = self.node_id.rot;
            let mut num: u8 = rot.into();
            loop {
                num = (num + 1) % 127;

                if <u8 as TryInto<Rot>>::try_into(num).is_err() {
                    continue;
                }
                self.node_id.rot = num.try_into().unwrap();
                info!("Rot: {}", num);
                break;
            }
        }
    }

    fn update_renderer(
        &mut self,

        controls: &Controls,

        node_id_bits: &Vec<u32>,
        render_nodes: &Vec<RenderNode>,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.update_controls(controls);

        // Buffers from the last swapchain iteration are being dropped
        self.mesh.to_drop_buffers[image_index].clear();

        if !self.mesh.chunks.is_empty() {
            self.mesh.chunks[0].update_from_data(
                node_id_bits,
                &render_nodes,
                context,
                &mut self.mesh.to_drop_buffers[image_index],
            )?;
        } else {
            let new_chunk = MeshChunk::new_from_data(
                IVec3::ZERO,
                self.mesh.size,
                self.mesh.render_size,
                node_id_bits,
                render_nodes,
                self.mesh.to_drop_buffers.len(),
                context,
                descriptor_layout,
                descriptor_pool,
            )?;
            if new_chunk.is_some() {
                self.mesh.chunks.push(new_chunk.unwrap())
            }
        }

        Ok(())
    }

    pub fn render(&mut self, buffer: &CommandBuffer, renderer: &MeshRenderer, image_index: usize) {
        renderer.render(buffer, image_index, RENDER_MODE_BASE, &self.mesh)
    }
}

impl DebugController {
    pub fn update_rotation_debug(
        &mut self,

        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        let num: u8 = self.rotation_renderer.node_id.rot.into();
        debug!("{:?}", num);

        let (node_id_bits, render_nodes) =
            self.get_rotation_debug_node_id_bits(self.rotation_renderer.mesh.size);

        self.rotation_renderer.update_renderer(
            controls,
            &node_id_bits,
            &render_nodes,
            image_index,
            context,
            descriptor_layout,
            descriptor_pool,
        )?;

        let mat: Mat4 = self.rotation_renderer.node_id.rot.into();

        let x = vec3(1.0, 0.0, 0.0);
        let y = vec3(0.0, 1.0, 0.0);
        let z = vec3(0.0, 0.0, 1.0);

        let rx = mat.transform_vector3(x);
        let ry = mat.transform_vector3(y);
        let rz = mat.transform_vector3(z);

        self.add_lines(vec![
            DebugLine::new(Vec3::ZERO, rx, vec4(1.0, 0.0, 0.0, 1.0)),
            DebugLine::new(Vec3::ZERO, ry, vec4(0.0, 1.0, 0.0, 1.0)),
            DebugLine::new(Vec3::ZERO, rz, vec4(0.0, 0.0, 1.0, 1.0)),
        ]);

        self.text_renderer.push_texts()?;
        self.line_renderer.push_lines()?;

        Ok(())
    }

    fn get_rotation_debug_node_id_bits(&mut self, size: IVec3) -> (Vec<u32>, Vec<RenderNode>) {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];
        let mut render_nodes = vec![RenderNode(false); (size + 2).element_product() as usize];
        let middle_pos = IVec3::ZERO;
        let index = to_1d_i(middle_pos, size) as usize;
        node_debug_node_id_bits[index] = self.rotation_renderer.node_id.into();

        let padded_index = to_1d_i(middle_pos + 1, size + 2) as usize;
        render_nodes[padded_index] = RenderNode(true);

        (node_debug_node_id_bits, render_nodes)
    }
}
