use crate::math::to_1d_i;
use crate::math::{get_neighbors, oct_positions, to_3d_i};
use crate::rules::{Prio, Rules};
use order::NodeOrderController;
use possible_blocks::PossibleBlocks;

use crate::render::compute_raytracing::compute_raytracing_data::ComputeRaytracingData;
use crate::render::parallax::node_parallax_mesh::{NodeParallaxMesh, RenderNode};
use crate::rules::empty::EMPTY_BLOCK_NAME_INDEX;
use crate::rules::solver::{SolverCacheIndex, SolverFunctions};
use crate::world::data::block::{BlockNameIndex, BLOCK_INDEX_EMPTY};
use crate::world::data::node::NodeID;
use collapse::Collapser;
use index_queue::IndexQueue;
use log::{debug, trace};
use octa_force::puffin_egui::puffin;
use octa_force::{glam::*, log};

pub mod collapse;
pub mod order;
pub mod possible_blocks;

pub type ChunkIndex = usize;
pub type CacheIndex = usize;

pub struct BlockObject {
    pub transform: Mat4,

    pub chunks: Vec<BlockChunk>,

    pub blocks_per_chunk: IVec3,
    pub block_length: usize,
    pub nodes_per_chunk: IVec3,
    pub nodes_length: usize,
    pub nodes_per_chunk_with_padding: IVec3,
    pub nodes_length_with_padding: usize,
    pub chunk_block_pos_mask: IVec3,
    pub in_chunk_block_pos_mask: IVec3,

    pub order_controller: NodeOrderController,
    pub to_reset: IndexQueue,
    pub was_reset: IndexQueue,
    pub to_propergate: IndexQueue,
    pub collapser: Collapser,
    pub is_collapsed: IndexQueue,

    pub builder_active: bool,
}

pub struct BlockChunk {
    pub pos: IVec3,
    pub block_names: Vec<BlockNameIndex>,
    pub blocks: Vec<PossibleBlocks>,
    pub node_id_bits: Vec<u32>,

    pub render_nodes: Vec<RenderNode>,
    pub parallax_data: Option<NodeParallaxMesh>,
    pub compute_raytracing_data: Option<ComputeRaytracingData>,
}

impl BlockObject {
    pub fn new(transform: Mat4, nodes_per_chunk_side: i32, num_block_names: usize) -> BlockObject {
        let block_size = nodes_per_chunk_side / 2;
        let blocks_per_chunk = IVec3::ONE * block_size;
        let block_length = blocks_per_chunk.element_product() as usize;

        let nodes_per_chunk = IVec3::ONE * nodes_per_chunk_side;
        let nodes_length = nodes_per_chunk.element_product() as usize;

        let nodes_per_chunk_with_padding = IVec3::ONE * (nodes_per_chunk_side + 2);
        let nodes_length_with_padding = nodes_per_chunk_with_padding.element_product() as usize;

        let chunk_pos_mask = IVec3::ONE * !(block_size - 1);
        let in_chunk_pos_mask = IVec3::ONE * (block_size - 1);

        let node_order_controller = NodeOrderController::new(num_block_names, nodes_length);

        BlockObject {
            transform,

            chunks: Vec::new(),

            blocks_per_chunk,
            block_length,

            nodes_per_chunk,
            nodes_length,

            nodes_per_chunk_with_padding,
            nodes_length_with_padding,

            chunk_block_pos_mask: chunk_pos_mask,
            in_chunk_block_pos_mask: in_chunk_pos_mask,

            order_controller: node_order_controller,

            to_reset: IndexQueue::default(),
            was_reset: IndexQueue::default(),
            to_propergate: IndexQueue::default(),
            collapser: Collapser::new(),
            is_collapsed: IndexQueue::default(),

            builder_active: false,
        }
    }

    pub fn place_block(&mut self, world_block_pos: IVec3, new_block_name_index: BlockNameIndex) {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let chunk_index = self.get_chunk_index_from_world_block_pos(world_block_pos);
        let block_index = self.get_block_index_from_world_block_pos(world_block_pos);
        let chunk = &mut self.chunks[chunk_index];

        let old_block_name_index = chunk.block_names[block_index];
        if old_block_name_index == new_block_name_index {
            return;
        }

        trace!("Place: {world_block_pos:?}");
        chunk.block_names[block_index] = new_block_name_index;

        let old_order = self.order_controller.pack_propergate_order(
            old_block_name_index,
            block_index,
            chunk_index,
        );
        let new_order = self.order_controller.pack_propergate_order(
            new_block_name_index,
            block_index,
            chunk_index,
        );

        self.to_reset.push_back(old_order);
        self.to_reset.push_back(new_order);

        // Resetting was_reset and is_collapsed
        {
            // Two Options: setting = new empty Queue or drain via while loop
            // A new empty Queue has long allocation times in later ticks so draining is better.
            #[cfg(debug_assertions)]
            puffin::profile_scope!("Drain_was_reset_and_is_collapsed");

            while self.was_reset.pop_front().is_some() {}
            while self.is_collapsed.pop_front().is_some() {}
        }

        let collapse_order = self
            .order_controller
            .pack_collapse_order(block_index, chunk_index);
        let neighbor_cache_len = self.chunks[chunk_index].blocks[block_index].get_num_caches();

        self.collapser
            .push_order(collapse_order, neighbor_cache_len);
    }

    pub fn get_block_name_from_world_block_pos(
        &mut self,
        world_block_pos: IVec3,
    ) -> BlockNameIndex {
        let chunk_index = self.get_chunk_index_from_world_block_pos(world_block_pos);
        let in_chunk_block_index = self.get_block_index_from_world_block_pos(world_block_pos);

        self.chunks[chunk_index].block_names[in_chunk_block_index]
    }

    pub fn get_cache_from_world_block_pos(
        &mut self,
        world_block_pos: IVec3,
        block_name_index: BlockNameIndex,
    ) -> &[SolverCacheIndex] {
        let chunk_index = self.get_chunk_index_from_world_block_pos(world_block_pos);
        let in_chunk_block_index = self.get_block_index_from_world_block_pos(world_block_pos);

        self.chunks[chunk_index].blocks[in_chunk_block_index].get_cache(block_name_index)
    }

    pub fn tick(&mut self, ticks: usize, rules: &Rules) -> (usize, Vec<ChunkIndex>) {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let mut changed_chunks = Vec::new();

        for i in 0..ticks {
            if !self.to_reset.is_empty() {
                self.reset(rules);
            } else if !self.to_propergate.is_empty() {
                self.propergate(rules);
            } else if !self.collapser.is_empty() {
                let changed_chunk = self.collapse(rules);

                if !changed_chunks.contains(&changed_chunk) {
                    changed_chunks.push(changed_chunk)
                }
            } else {
                return (ticks - i, changed_chunks);
            }
        }

        (0, changed_chunks)
    }

    fn reset(&mut self, rules: &Rules) {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let order = self.to_reset.pop_front().unwrap();
        let (block_name_index, block_index, chunk_index) =
            self.order_controller.unpack_propergate_order(order);
        let world_block_pos =
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

        let new_cache = rules.solvers[block_name_index as usize].block_check_reset(
            self,
            block_index,
            chunk_index,
            world_block_pos,
        );
        let old_cache = self.chunks[chunk_index].blocks[block_index].get_cache(block_name_index);
        if new_cache != old_cache {
            #[cfg(debug_assertions)]
            puffin::profile_scope!("Cache_was_changed");

            self.chunks[chunk_index].blocks[block_index].set_cache(block_name_index, &new_cache);

            if cfg!(debug_assertions) {
                {
                    #[cfg(debug_assertions)]
                    puffin::profile_scope!("Push_propergate_order");
                    self.to_propergate.push_back(order);
                }
                {
                    #[cfg(debug_assertions)]
                    puffin::profile_scope!("Push_was_reset_order");

                    // Takes sometimes very long
                    self.was_reset.push_back(order);
                }
            } else {
                self.to_propergate.push_back(order);
                self.was_reset.push_back(order);
            }

            for offset in get_neighbors() {
                #[cfg(debug_assertions)]
                puffin::profile_scope!("Push_Neighbor_Rest");

                let neighbor_world_pos = world_block_pos + offset;
                let neighbor_chunk_index =
                    self.get_chunk_index_from_world_block_pos(neighbor_world_pos);
                let neighbor_block_index =
                    self.get_block_index_from_world_block_pos(neighbor_world_pos);
                let neighbor_order = self.order_controller.pack_propergate_order(
                    block_name_index,
                    neighbor_block_index,
                    neighbor_chunk_index,
                );

                if !self.was_reset.contains(neighbor_order) {
                    self.to_reset.push_back(neighbor_order);
                }
            }
        }
    }

    fn propergate(&mut self, rules: &Rules) {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let order = self.to_propergate.pop_front().unwrap();
        let (block_name_index, block_index, chunk_index) =
            self.order_controller.unpack_propergate_order(order);
        let world_block_pos =
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

        let old_cache = self.chunks[chunk_index].blocks[block_index]
            .get_cache(block_name_index)
            .to_owned();
        let new_cache = rules.solvers[block_name_index as usize].block_check(
            self,
            block_index,
            chunk_index,
            world_block_pos,
            old_cache.to_owned(),
        );

        if new_cache != old_cache {
            self.chunks[chunk_index].blocks[block_index].set_cache(block_name_index, &new_cache);

            for offset in get_neighbors() {
                let neighbor_world_pos = world_block_pos + offset;
                let neighbor_chunk_index =
                    self.get_chunk_index_from_world_block_pos(neighbor_world_pos);
                let neighbor_block_index =
                    self.get_block_index_from_world_block_pos(neighbor_world_pos);

                let collapse_order = self
                    .order_controller
                    .pack_collapse_order(neighbor_block_index, neighbor_chunk_index);
                if self.is_collapsed.contains(collapse_order) {
                    continue;
                }

                let propergate_order = self.order_controller.pack_propergate_order(
                    block_name_index,
                    neighbor_block_index,
                    neighbor_chunk_index,
                );
                self.to_propergate.push_back(propergate_order);
            }
        }
    }

    fn collapse(&mut self, rules: &Rules) -> ChunkIndex {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let order = self.collapser.pop_order();
        let (block_index, chunk_index) = self.order_controller.unpack_collapse_order(order);
        let world_block_pos =
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

        // Get best Block
        let mut best_block = None;
        let mut best_prio = Prio::Zero;
        let mut best_block_name_index = EMPTY_BLOCK_NAME_INDEX;
        let mut best_cache_index = 0;
        for (block_name_index, solver) in rules.solvers.iter().enumerate() {
            let block_name_index = block_name_index as BlockNameIndex;

            let old_cache = self.chunks[chunk_index].blocks[block_index]
                .get_cache(block_name_index)
                .to_owned();
            let (block, prio, cache_index) =
                solver.get_block(self, block_index, chunk_index, world_block_pos, old_cache);

            if best_prio < prio {
                best_prio = prio;
                best_block = Some(block);
                best_block_name_index = block_name_index;
                best_cache_index = cache_index;
            }
        }

        // Update Cache to the chosen index
        self.chunks[chunk_index].blocks[block_index]
            .set_all_caches_with_one(best_block_name_index, best_cache_index);

        // Set node_id and render nodes
        let node_pos = self.get_node_pos_from_block_index(block_index);
        let indices: Vec<_> = oct_positions()
            .into_iter()
            .map(|offset| {
                let pos = node_pos + offset;
                let index = self.get_node_index_from_node_pos(pos);
                let index_with_padding = self.get_node_index_with_padding_from_node_pos(pos);
                (index, index_with_padding)
            })
            .collect();

        if best_block.is_some() {
            let block = best_block.unwrap();
            for (node_id, (index, index_with_padding)) in
                block.node_ids.into_iter().zip(indices.into_iter())
            {
                self.chunks[chunk_index].node_id_bits[index] = node_id.into();

                self.chunks[chunk_index].render_nodes[index_with_padding] =
                    RenderNode(node_id.is_some());
            }
        } else {
            for (index, index_with_padding) in indices.into_iter() {
                self.chunks[chunk_index].node_id_bits[index] = NodeID::empty().into();

                self.chunks[chunk_index].render_nodes[index_with_padding] = RenderNode(false);
            }
        }

        self.is_collapsed.push_back(order);

        for offset in get_neighbors() {
            let neighbor_world_pos = world_block_pos + offset;
            let neighbor_chunk_index =
                self.get_chunk_index_from_world_block_pos(neighbor_world_pos);
            let neighbor_block_index =
                self.get_block_index_from_world_block_pos(neighbor_world_pos);

            let collapse_order = self
                .order_controller
                .pack_collapse_order(neighbor_block_index, neighbor_chunk_index);
            if self.is_collapsed.contains(collapse_order) {
                continue;
            }

            let propergate_order = self.order_controller.pack_propergate_order(
                best_block_name_index,
                neighbor_block_index,
                neighbor_chunk_index,
            );
            self.to_propergate.push_back(propergate_order);

            if self.get_block_name_from_world_block_pos(neighbor_world_pos)
                == EMPTY_BLOCK_NAME_INDEX
            {
                continue;
            }

            let collapse_order = self
                .order_controller
                .pack_collapse_order(neighbor_block_index, neighbor_chunk_index);
            let neighbor_cache_len =
                self.chunks[neighbor_chunk_index].blocks[neighbor_block_index].get_num_caches();

            self.collapser
                .push_order(collapse_order, neighbor_cache_len);
        }

        chunk_index
    }

    pub fn add_chunk(&mut self, chunk_pos: IVec3) {
        let chunk = BlockChunk {
            pos: chunk_pos,
            block_names: vec![BLOCK_INDEX_EMPTY; self.block_length],
            blocks: vec![PossibleBlocks::default(); self.block_length],
            node_id_bits: vec![0; self.nodes_length],

            render_nodes: vec![RenderNode::default(); self.nodes_length_with_padding],
            parallax_data: None,
            compute_raytracing_data: None,
        };

        self.chunks.push(chunk)
    }

    pub fn has_chunk(&self, chunk_pos: IVec3) -> bool {
        self.chunks.iter().find(|c| c.pos == chunk_pos).is_some()
    }

    pub fn get_chunk_index_from_world_block_pos(&mut self, world_block_pos: IVec3) -> usize {
        let chunk_pos = self.get_chunk_node_pos_from_world_block_pos(world_block_pos);

        let r = self.chunks.iter().position(|c| c.pos == chunk_pos);
        let index = if r.is_none() {
            self.add_chunk(chunk_pos);
            self.chunks.len() - 1
        } else {
            r.unwrap()
        };

        index
    }

    pub fn get_chunk_node_pos_from_world_block_pos(&self, world_block_pos: IVec3) -> IVec3 {
        ((world_block_pos / self.blocks_per_chunk)
            - ivec3(
                (world_block_pos.x < 0) as i32,
                (world_block_pos.y < 0) as i32,
                (world_block_pos.z < 0) as i32,
            ))
            * self.nodes_per_chunk
    }

    pub fn get_chunk_node_pos_from_world_node_pos(&self, world_block_pos: IVec3) -> IVec3 {
        ((world_block_pos / self.nodes_per_chunk)
            - ivec3(
                (world_block_pos.x < 0) as i32,
                (world_block_pos.y < 0) as i32,
                (world_block_pos.z < 0) as i32,
            ))
            * self.nodes_per_chunk
    }

    pub fn get_chunk_block_pos_from_world_node_pos(&self, world_block_pos: IVec3) -> IVec3 {
        ((world_block_pos / self.nodes_per_chunk)
            - ivec3(
                (world_block_pos.x < 0) as i32,
                (world_block_pos.y < 0) as i32,
                (world_block_pos.z < 0) as i32,
            ))
            * self.blocks_per_chunk
    }

    pub fn get_chunk_block_pos_from_world_block_pos(&self, world_block_pos: IVec3) -> IVec3 {
        ((world_block_pos / self.blocks_per_chunk)
            - ivec3(
                (world_block_pos.x < 0) as i32,
                (world_block_pos.y < 0) as i32,
                (world_block_pos.z < 0) as i32,
            ))
            * self.blocks_per_chunk
    }

    pub fn get_block_pos_from_world_block_pos(&self, pos: IVec3) -> IVec3 {
        // TODO Clean up
        ivec3(
            if pos.x < 0 {
                (self.blocks_per_chunk.x + (pos.x % self.blocks_per_chunk.x))
                    % self.blocks_per_chunk.x
            } else {
                pos.x & self.in_chunk_block_pos_mask.x
            },
            if pos.y < 0 {
                (self.blocks_per_chunk.y + (pos.y % self.blocks_per_chunk.y))
                    % self.blocks_per_chunk.y
            } else {
                pos.y & self.in_chunk_block_pos_mask.y
            },
            if pos.z < 0 {
                (self.blocks_per_chunk.z + (pos.z % self.blocks_per_chunk.z))
                    % self.blocks_per_chunk.z
            } else {
                pos.z & self.in_chunk_block_pos_mask.z
            },
        )
    }

    pub fn get_block_index_from_world_block_pos(&self, world_block_pos: IVec3) -> usize {
        let block_pos = self.get_block_pos_from_world_block_pos(world_block_pos);
        to_1d_i(block_pos, self.blocks_per_chunk) as usize
    }

    pub fn get_world_block_pos_from_chunk_and_block_index(
        &self,
        block_index: usize,
        chunk_index: usize,
    ) -> IVec3 {
        let chunk_pos = self.chunks[chunk_index].pos / 2;
        let block_pos = to_3d_i(block_index as i32, self.blocks_per_chunk);

        chunk_pos + block_pos
    }

    pub fn get_world_node_pos_from_chunk_and_block_index(
        &self,
        block_index: usize,
        chunk_index: usize,
    ) -> IVec3 {
        self.get_node_pos_from_block_pos(
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index),
        )
    }

    pub fn get_node_pos_from_block_pos(&self, block_pos: IVec3) -> IVec3 {
        block_pos * 2
    }

    pub fn get_block_pos_from_node_pos(&self, node_pos: IVec3) -> IVec3 {
        (node_pos / 2)
            + (node_pos % 2)
                * ivec3(
                    (node_pos.x < 0) as i32,
                    (node_pos.y < 0) as i32,
                    (node_pos.z < 0) as i32,
                )
    }

    pub fn get_node_pos_from_block_index(&self, block_index: usize) -> IVec3 {
        let block_pos = to_3d_i(block_index as i32, self.blocks_per_chunk);
        let node_pos = self.get_node_pos_from_block_pos(block_pos);
        node_pos
    }

    pub fn get_node_index_from_node_pos(&self, node_pos: IVec3) -> usize {
        to_1d_i(node_pos, self.nodes_per_chunk) as usize
    }

    pub fn get_node_index_with_padding_from_node_pos(&self, node_pos: IVec3) -> usize {
        to_1d_i(node_pos + 1, self.nodes_per_chunk_with_padding) as usize
    }

    pub fn get_world_node_pos_from_chunk_and_node_index(
        &self,
        node_index: usize,
        chunk_index: usize,
    ) -> IVec3 {
        let chunk_pos = self.chunks[chunk_index].pos;
        let node_pos = to_3d_i(node_index as i32, self.nodes_per_chunk);

        chunk_pos + node_pos
    }

    pub fn get_node_index_plus_padding_from_node_index(&self, node_index: usize) -> usize {
        let node_pos = to_3d_i(node_index as i32, self.nodes_per_chunk);
        to_1d_i(node_pos + IVec3::ONE, self.nodes_per_chunk_with_padding) as usize
    }

    pub fn get_block_world_pos_from_block_index_and_chunk_pos(
        &self,
        block_index: usize,
        chunk_pos: IVec3,
    ) -> IVec3 {
        to_3d_i(block_index as i32, self.blocks_per_chunk) + chunk_pos
    }
}

#[test]
pub fn test_math() {
    let block_object = BlockObject::new(Mat4::IDENTITY, 16, 3);

    assert_eq!(
        block_object.get_chunk_node_pos_from_world_block_pos(ivec3(17, 17, 17)),
        ivec3(32, 32, 32)
    );
    assert_eq!(
        block_object.get_chunk_node_pos_from_world_block_pos(ivec3(-1, -1, -1)),
        ivec3(-16, -16, -16)
    );
    assert_eq!(
        block_object.get_chunk_node_pos_from_world_block_pos(ivec3(-8, -8, -8)),
        ivec3(-32, -32, -32)
    );
    assert_eq!(
        block_object.get_chunk_node_pos_from_world_block_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );

    assert_eq!(
        block_object.get_chunk_node_pos_from_world_node_pos(ivec3(17, 17, 17)),
        ivec3(16, 16, 16)
    );
    assert_eq!(
        block_object.get_chunk_node_pos_from_world_node_pos(ivec3(-1, -1, -1)),
        ivec3(-16, -16, -16)
    );
    assert_eq!(
        block_object.get_chunk_node_pos_from_world_node_pos(ivec3(-16, -16, -16)),
        ivec3(-32, -32, -32)
    );
    assert_eq!(
        block_object.get_chunk_node_pos_from_world_node_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );

    assert_eq!(
        block_object.get_chunk_block_pos_from_world_block_pos(ivec3(17, 17, 17)),
        ivec3(16, 16, 16)
    );
    assert_eq!(
        block_object.get_chunk_block_pos_from_world_block_pos(ivec3(-1, -1, -1)),
        ivec3(-8, -8, -8)
    );
    assert_eq!(
        block_object.get_chunk_block_pos_from_world_block_pos(ivec3(-8, -8, -8)),
        ivec3(-16, -16, -16)
    );
    assert_eq!(
        block_object.get_chunk_block_pos_from_world_block_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );

    assert_eq!(
        block_object.get_chunk_block_pos_from_world_node_pos(ivec3(17, 17, 17)),
        ivec3(8, 8, 8)
    );
    assert_eq!(
        block_object.get_chunk_block_pos_from_world_node_pos(ivec3(-1, -1, -1)),
        ivec3(-8, -8, -8)
    );
    assert_eq!(
        block_object.get_chunk_block_pos_from_world_node_pos(ivec3(-16, -16, -16)),
        ivec3(-16, -16, -16)
    );
    assert_eq!(
        block_object.get_chunk_block_pos_from_world_node_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );

    assert_eq!(
        block_object.get_block_pos_from_world_block_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );
    assert_eq!(
        block_object.get_block_pos_from_world_block_pos(ivec3(8, 8, 8)),
        ivec3(0, 0, 0)
    );
    assert_eq!(
        block_object.get_block_pos_from_world_block_pos(ivec3(-1, -1, -1)),
        ivec3(7, 7, 7)
    );
    assert_eq!(
        block_object.get_block_pos_from_world_block_pos(ivec3(-8, -8, -8)),
        ivec3(0, 0, 0)
    );
    assert_eq!(
        block_object.get_block_pos_from_world_block_pos(ivec3(-16, -16, -16)),
        ivec3(0, 0, 0)
    );
    assert_eq!(
        block_object.get_block_pos_from_world_block_pos(ivec3(-15, -15, -15)),
        ivec3(1, 1, 1)
    );

    assert_eq!(
        block_object.get_block_index_from_world_block_pos(ivec3(0, 0, 0)),
        0
    );
    assert_eq!(
        block_object.get_block_index_from_world_block_pos(ivec3(8, 8, 8)),
        0
    );
    assert_eq!(
        block_object.get_block_index_from_world_block_pos(ivec3(-8, -8, -8)),
        0
    );

    assert_eq!(
        block_object.get_block_index_from_world_block_pos(ivec3(1, 1, 1)),
        block_object.get_block_index_from_world_block_pos(ivec3(-7, -7, -7)),
    );

    assert_eq!(
        block_object.get_node_pos_from_block_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );
    assert_eq!(
        block_object.get_node_pos_from_block_pos(ivec3(1, 1, 1)),
        ivec3(2, 2, 2)
    );
    assert_eq!(
        block_object.get_node_pos_from_block_pos(ivec3(-1, -1, -1)),
        ivec3(-2, -2, -2)
    );

    assert_eq!(
        block_object.get_block_pos_from_node_pos(ivec3(0, 0, 0)),
        ivec3(0, 0, 0)
    );
    assert_eq!(
        block_object.get_block_pos_from_node_pos(ivec3(1, 2, 3)),
        ivec3(0, 1, 1)
    );
    assert_eq!(
        block_object.get_block_pos_from_node_pos(ivec3(-1, -2, -3)),
        ivec3(-1, -1, -2)
    );
}
