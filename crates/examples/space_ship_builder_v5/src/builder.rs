use crate::ship_mesh::ShipMesh;
use crate::{node::BlockIndex, ship::Ship};
use std::collections::HashMap;

use crate::rules::Rules;
use crate::ship::CHUNK_SIZE;
use crate::voxel_loader::VoxelLoader;
use index_queue::IndexQueue;
use octa_force::glam::{vec3, IVec3};
use octa_force::vulkan::{DescriptorPool, DescriptorSetLayout};
use octa_force::{anyhow::Result, camera::Camera, controls::Controls, vulkan::Context};
use std::time::Duration;

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode};
use crate::node::BLOCK_INDEX_EMPTY;

const SCROLL_SPEED: f32 = 0.01;
const PLACE_SPEED: Duration = Duration::from_millis(100);

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

const DEBUG_COLLAPSE_SPEED: Duration = Duration::from_millis(100);

pub struct Builder {
    pub ship: Ship,
    pub base_ship_mesh: ShipMesh,
    pub build_ship_mesh: ShipMesh,

    pub build_blocks: HashMap<IVec3, BlockIndex>,

    possible_blocks: Vec<BlockIndex>,
    block_to_build: usize,
    distance: f32,

    actions_per_tick: usize,
    full_tick: bool,

    last_block_to_build: BlockIndex,
    last_pos: IVec3,
    last_action_time: Duration,

    changed_chunks: IndexQueue,
}

impl Builder {
    pub fn new(images_count: usize, voxel_loader: &VoxelLoader, rules: &Rules) -> Result<Builder> {
        let mut possible_blocks = Vec::new();
        possible_blocks.push(
            voxel_loader
                .block_names
                .iter()
                .position(|name| name == "Empty")
                .unwrap(),
        );
        possible_blocks.push(
            voxel_loader
                .block_names
                .iter()
                .position(|name| name == "Hull")
                .unwrap(),
        );

        let ship = Ship::new(CHUNK_SIZE, rules)?;
        let base_ship_mesh = ShipMesh::new(images_count, CHUNK_SIZE as u32 * 2)?;
        let build_ship_mesh = ShipMesh::new(images_count, CHUNK_SIZE as u32 * 2)?;

        Ok(Builder {
            ship,
            base_ship_mesh,
            build_ship_mesh,

            build_blocks: HashMap::new(),

            block_to_build: 1,
            possible_blocks,
            distance: 3.0,

            actions_per_tick: 4,
            full_tick: false,

            last_block_to_build: 0,
            last_pos: IVec3::ZERO,
            last_action_time: Duration::ZERO,

            changed_chunks: IndexQueue::default(),
        })
    }

    pub fn update(
        &mut self,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,

        controls: &Controls,
        camera: &Camera,
        voxel_loader: &VoxelLoader,
        rules: &Rules,
        delta_time: Duration,
        total_time: Duration,

        #[cfg(debug_assertions)] debug_controller: &mut DebugController,
    ) -> Result<()> {
        #[cfg(debug_assertions)]
        let d = debug_controller.mode != DebugMode::WFC;
        #[cfg(not(debug_assertions))]
        let d = true;

        if d {
            if self.full_tick
                && delta_time < MIN_TICK_LENGTH
                && self.actions_per_tick < (usize::MAX / 2)
            {
                self.actions_per_tick *= 2;
            } else if delta_time > MAX_TICK_LENGTH && self.actions_per_tick > 4 {
                self.actions_per_tick /= 2;
            }
        }

        if controls.e && (self.last_action_time + PLACE_SPEED) < total_time {
            self.last_action_time = total_time;

            self.block_to_build += 1;
            if self.block_to_build >= self.possible_blocks.len() {
                self.block_to_build = 0;
            }
        }

        self.distance -= controls.scroll_delta * SCROLL_SPEED;

        #[cfg(debug_assertions)]
        let d = debug_controller.mode == DebugMode::OFF || controls.lshift;
        #[cfg(not(debug_assertions))]
        let d = true;

        if d {
            let pos = (((camera.position + camera.direction * self.distance)
                - vec3(1.0, 1.0, 1.0))
                / 2.0)
                .round()
                .as_ivec3();

            if self.last_pos != pos || self.last_block_to_build != self.block_to_build {
                let chunk_index = self
                    .ship
                    .get_chunk_index(self.ship.get_node_pos_from_block_pos(pos));
                if chunk_index.is_ok() {
                    // Undo the last placement.
                    let last_block_index = *self
                        .build_blocks
                        .get(&self.last_pos)
                        .unwrap_or(&BLOCK_INDEX_EMPTY);

                    self.ship
                        .place_block(self.last_pos, last_block_index, rules)?;

                    // Simulate placement of the block to create preview in build_ship.
                    let block_index = self.possible_blocks[self.block_to_build];
                    let _ = self.ship.place_block(pos, block_index, rules)?;

                    // If block index is valid.
                    self.last_block_to_build = self.block_to_build;
                    self.last_pos = pos;
                }
            }
        }

        if controls.left && (self.last_action_time + PLACE_SPEED) < total_time {
            self.last_action_time = total_time;
            self.build_blocks
                .insert(self.last_pos, self.possible_blocks[self.block_to_build]);

            self.base_ship_mesh.update_from_mesh(
                &self.build_ship_mesh,
                image_index,
                context,
                descriptor_layout,
                descriptor_pool,
            )?;
        }

        let mut changed_chunks = Vec::new();

        #[cfg(debug_assertions)]
        if debug_controller.mode == DebugMode::WFC || debug_controller.mode == DebugMode::WFCSkip {
            if controls.t && (self.last_action_time + DEBUG_COLLAPSE_SPEED) < total_time {
                self.last_action_time = total_time;

                let mut full = true;
                loop {
                    debug_controller.line_renderer.vertecies.clear();
                    debug_controller.text_renderer.texts.clear();
                    let (f, c) = self.ship.tick(1, rules)?;
                    full &= f;
                    changed_chunks = c;

                    if debug_controller.mode == DebugMode::WFC || !f {
                        break;
                    }
                }

                log::info!("BUILDER: TICK FULL {:?}", full);
            }
        } else {
            (self.full_tick, changed_chunks) = self.ship.tick(self.actions_per_tick, rules)?;
        }

        #[cfg(not(debug_assertions))]
        {
            (self.full_tick, changed_chunks) = self.ship.tick(self.actions_per_tick, rules)?;
        }

        self.build_ship_mesh.update(
            &self.ship,
            changed_chunks,
            image_index,
            context,
            descriptor_layout,
            descriptor_pool,
        )?;

        #[cfg(debug_assertions)]
        {
            self.ship.show_debug(debug_controller);
        }

        Ok(())
    }

    pub fn on_node_controller_change(&mut self) -> Result<()> {
        self.last_block_to_build = BlockIndex::MAX;

        Ok(())
    }
}
