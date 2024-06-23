use std::iter;
use crate::debug::DebugController;
use crate::math::{oct_positions, to_1d_i};
use crate::rules::hull::HullSolver;
use crate::ship::data::ShipData;
use crate::ship::mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship::renderer::{ShipRenderer, RENDER_MODE_BASE};
use crate::ship::ShipManager;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{IVec3, ivec3};
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::time::{Duration, Instant};
use crate::rules::block::{BlockNameIndex};
use crate::ship::possible_blocks::PossibleBlocks;

const INPUT_INTERVAL: Duration = Duration::from_millis(100);

pub struct CollapseLogRenderer {
    mesh: ShipMesh,
    last_input: Instant,
    
    last_blocks_names: Vec<BlockNameIndex>,
    block_log: Vec<Vec<PossibleBlocks>>,
    log_index: usize,
    cache_index: usize,
}

impl CollapseLogRenderer {
    pub fn new(image_len: usize, ship_data: &ShipData) -> Self {
        CollapseLogRenderer {
            mesh: ShipMesh::new(
                image_len,
                ship_data.nodes_per_chunk,
                ship_data.nodes_per_chunk,
            ),
            last_input: Instant::now(),
            last_blocks_names: vec![],
            block_log: vec![],
            log_index: 0,
            cache_index: 0,
        }
    }

    pub fn on_enable(&self, ship_manager: &mut ShipManager) {
        ship_manager.update_actions_per_tick = false;
        ship_manager.actions_per_tick = 1;
    }

    pub fn on_disable(&self, ship_manager: &mut ShipManager) {
        ship_manager.update_actions_per_tick = true;
        ship_manager.actions_per_tick = 4;
    }
    
    pub fn update_log(&mut self, ship_data: &ShipData) {
        if ship_data.chunks[0].block_names != self.last_blocks_names {
            self.block_log = vec![];
        }
        
        if ship_data.chunks[0].blocks != *self.block_log.last().unwrap() {
            self.block_log.push(ship_data.chunks[0].blocks.to_owned());
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
    pub fn update_collapse_log_debug(
        &mut self,

        ship_data: &ShipData,
        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.collapse_log_renderer.update_log(ship_data);
        
        let (node_id_bits, render_nodes) =
            self.get_collapse_log_node_id_bits(self.collapse_log_renderer.mesh.size, ship_data);

        self.collapse_log_renderer.update_renderer(
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

    fn get_collapse_log_node_id_bits(
        &mut self,
        size: IVec3,
        ship_data: &ShipData,
    ) -> (Vec<u32>, Vec<RenderNode>) {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];
        let mut render_nodes = vec![RenderNode(false); (size + 2).element_product() as usize];
        
        for x in 0..size.x {
            for y in 0..size.y {
                for z in 0..size.z {
                    let index = ship_data.get_node_index_from_node_pos(ivec3(x, y, z));
                    
                    let caches: Vec<_> = self.collapse_log_renderer.block_log[self.collapse_log_renderer.log_index][index]
                        .get_all_caches()
                        .into_iter()
                        .map(|(block_name, cache)| {
                            iter::repeat(block_name).zip(cache.into_iter())
                        })
                        .flatten()
                        .collect();
                    
                    let (block_name, cache_index) = caches[self.collapse_log_renderer.cache_index % caches.len()];
                    
                    
                }
            }
        }
        

        (node_debug_node_id_bits, render_nodes)
    }
}
