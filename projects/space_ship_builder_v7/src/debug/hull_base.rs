use crate::debug::line_renderer::DebugLine;
use crate::debug::DebugController;
use crate::math::{oct_positions, to_1d_i};
use crate::rules::hull::HullSolver;
use crate::ship::mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship::renderer::{ShipRenderer, RENDER_MODE_BASE};
use log::info;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, vec3, vec4, IVec3, Mat4, Vec3};
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::time::{Duration, Instant};

pub const HULL_BASE_DEBUG_SIZE: i32 = 4;
const INPUT_INTERVAL: Duration = Duration::from_millis(100);

pub struct DebugHullBaseRenderer {
    mesh: ShipMesh,
    index: usize,
    last_input: Instant,
}

impl DebugHullBaseRenderer {
    pub fn new(image_len: usize) -> Self {
        let size = IVec3::ONE * HULL_BASE_DEBUG_SIZE;
        DebugHullBaseRenderer {
            mesh: ShipMesh::new(image_len, size, size),
            index: 0,
            last_input: Instant::now(),
        }
    }

    pub fn update_controls(&mut self, controls: &Controls, hull_solver: &HullSolver) {
        if controls.t && self.last_input.elapsed() > INPUT_INTERVAL {
            self.last_input = Instant::now();

            self.index = (self.index + 1) % hull_solver.base_blocks.len();

            info!("Base Hull Block: {}", self.index)
        }
    }

    fn update_renderer(
        &mut self,

        node_id_bits: &Vec<u32>,
        render_nodes: &Vec<RenderNode>,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
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

    pub fn render(&mut self, buffer: &CommandBuffer, renderer: &ShipRenderer, image_index: usize) {
        renderer.render(buffer, image_index, RENDER_MODE_BASE, &self.mesh)
    }
}

impl DebugController {
    pub fn update_hull_base(
        &mut self,

        hull_solver: &HullSolver,
        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.renderer_hull_base
            .update_controls(controls, hull_solver);

        let (node_id_bits, render_nodes) =
            self.get_hull_base_node_id_bits(self.renderer_hull_base.mesh.size, hull_solver);

        self.renderer_hull_base.update_renderer(
            &node_id_bits,
            &render_nodes,
            image_index,
            context,
            descriptor_layout,
            descriptor_pool,
        )?;

        self.text_renderer.push_texts()?;
        self.line_renderer.push_lines()?;

        Ok(())
    }

    fn get_hull_base_node_id_bits(
        &mut self,
        size: IVec3,
        hull_solver: &HullSolver,
    ) -> (Vec<u32>, Vec<RenderNode>) {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];
        let mut render_nodes = vec![RenderNode(false); (size + 2).element_product() as usize];
        let middle_pos = size / 2;

        let (reqs, block) = &hull_solver.base_blocks[self.renderer_hull_base.index];
        for (j, offset) in oct_positions().iter().enumerate() {
            let node_pos = middle_pos + *offset;
            let node_index = to_1d_i(node_pos, size) as usize;

            node_debug_node_id_bits[node_index] = block.node_ids[j].into();

            let node_pos_plus_padding = node_pos + 1;
            let node_index_plus_padding = to_1d_i(node_pos_plus_padding, size + 2) as usize;
            render_nodes[node_index_plus_padding] = RenderNode(true);

            let mat: Mat4 = block.node_ids[j].rot.into();
            let offset = block.node_ids[j].rot.rot_offset().as_vec3();

            let x = vec3(1.0, 0.0, 0.0);
            let y = vec3(0.0, 1.0, 0.0);
            let z = vec3(0.0, 0.0, 1.0);

            let p = node_pos.as_vec3() + offset;
            let rx = mat.transform_vector3(x) * 0.5 + p;
            let ry = mat.transform_vector3(y) * 0.5 + p;
            let rz = mat.transform_vector3(z) * 0.5 + p;

            self.add_lines(vec![
                DebugLine::new(p, rx, vec4(1.0, 0.0, 0.0, 1.0)),
                DebugLine::new(p, ry, vec4(0.0, 1.0, 0.0, 1.0)),
                DebugLine::new(p, rz, vec4(0.0, 0.0, 1.0, 1.0)),
            ]);
        }

        for req_offset in reqs {
            let pos = middle_pos + *req_offset * 2;

            self.add_cube(pos.as_vec3(), (pos + 2).as_vec3(), vec4(0.0, 1.0, 0.0, 1.0));
        }

        (node_debug_node_id_bits, render_nodes)
    }
}
