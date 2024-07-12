use crate::debug::hull_basic::{DebugHullBasicRenderer, HULL_BASE_DEBUG_SIZE};
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
    data: ParallaxData,
    index: usize,
    last_input: Instant,
}

impl DebugNodesRenderer {
    pub fn new(
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
        num_frames: usize,
    ) -> Result<Self> {
        let size = IVec3::ONE * NODES_DEBUG_SIZE;
        Ok(DebugNodesRenderer {
            data: ParallaxData::new(
                IVec3::ZERO,
                size,
                size.element_product() as usize,
                num_frames,
                context,
                descriptor_layout,
                descriptor_pool,
            )?,
            index: 1,
            last_input: Instant::now(),
        })
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

        node_id_bits: &[u32],
        render_nodes: &[RenderNode],

        image_index: usize,
        context: &Context,
        renderer: &mut ParallaxRenderer,
    ) -> Result<()> {
        let size = IVec3::ONE * HULL_BASE_DEBUG_SIZE;
        self.data.update(
            size,
            node_id_bits,
            render_nodes,
            context,
            &mut renderer.to_drop_buffers[image_index],
        )?;

        Ok(())
    }

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        renderer: &ParallaxRenderer,
        image_index: usize,
    ) {
        renderer.render_data(buffer, image_index, &self.data)
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
            self.get_nodes_node_id_bits(self.nodes_renderer.data.size);

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
