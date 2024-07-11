use crate::debug::DebugController;
use crate::math::rotation::Rot;
use crate::math::to_1d_i;
use crate::render::parallax::chunk::{ParallaxData, RenderNode};
use crate::render::parallax::renderer::ParallaxRenderer;
use crate::rules::Rules;
use crate::world::data::node::NodeID;
use log::info;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::IVec3;
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::time::{Duration, Instant};

pub const NODES_DEBUG_SIZE: i32 = 4;
const INPUT_INTERVAL: Duration = Duration::from_millis(100);

pub struct DebugNodesRenderer {
    mesh: ParallaxMesh,
    index: usize,
    last_input: Instant,
}

impl DebugNodesRenderer {
    pub fn new(image_len: usize) -> Self {
        let size = IVec3::ONE * NODES_DEBUG_SIZE;
        DebugNodesRenderer {
            mesh: ParallaxMesh::new(image_len, size, size),
            index: 1,
            last_input: Instant::now(),
        }
    }

    pub fn update_controls(&mut self, controls: &Controls, rules: &Rules) {
        if controls.t && self.last_input.elapsed() > INPUT_INTERVAL {
            self.last_input = Instant::now();

            self.index = (self.index + 1) % rules.nodes.len();

            info!("Node Index: {}", self.index)
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
            let new_chunk = ParallaxData::new_from_data(
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

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        renderer: &ParallaxRenderer,
        image_index: usize,
    ) {
        renderer
            .render_mesh(buffer, image_index, &self.mesh)
            .unwrap()
    }
}

impl DebugController {
    pub fn update_nodes(
        &mut self,

        rules: &Rules,
        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.nodes_renderer.update_controls(controls, rules);

        let (node_id_bits, render_nodes) =
            self.get_nodes_node_id_bits(self.nodes_renderer.mesh.size);

        self.nodes_renderer.update_renderer(
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

    fn get_nodes_node_id_bits(&mut self, size: IVec3) -> (Vec<u32>, Vec<RenderNode>) {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];
        let mut render_nodes = vec![RenderNode(false); (size + 2).element_product() as usize];
        let middle_pos = size / 2;
        let middle_index = to_1d_i(middle_pos, size) as usize;
        let middle_index_with_padding = to_1d_i(middle_pos + 1, size + 2) as usize;

        node_debug_node_id_bits[middle_index] =
            NodeID::new(self.nodes_renderer.index, Rot::IDENTITY).into();
        render_nodes[middle_index_with_padding] = RenderNode(true);

        (node_debug_node_id_bits, render_nodes)
    }
}
