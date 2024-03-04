use crate::math::{to_1d, to_1d_i};
use crate::ship_mesh::ShipMesh;
use crate::{
    node::{BlockIndex, NodeController},
    ship::Ship,
};
use std::collections::HashMap;

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode};

use crate::node::BLOCK_INDEX_EMPTY;
use crate::ship::{
    get_chunk_pos_of_block_pos, get_in_chunk_pos_of_block_pos, get_in_chunk_pos_of_wave_pos,
    WaveIndex, CHUNK_BLOCK_LEN,
};
use index_queue::IndexQueue;
use octa_force::glam::{vec3, vec4, IVec3, Vec3};
use octa_force::{
    anyhow::Result, camera::Camera, controls::Controls, glam::UVec3, log, vulkan::Context,
};
use std::time::Duration;

const SCROLL_SPEED: f32 = 0.01;
const PLACE_SPEED: Duration = Duration::from_millis(100);

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

const DEBUG_COLLAPSE_SPEED: Duration = Duration::from_millis(100);

pub struct Builder {
    pub ship: Ship,
    pub build_blocks: HashMap<IVec3, BlockIndex>,

    pub base_ship_mesh: ShipMesh,
    pub build_ship_mesh: ShipMesh,

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
    pub fn new(ship: Ship, context: &Context, node_controller: &NodeController) -> Result<Builder> {
        let mut possible_blocks = Vec::new();
        possible_blocks.push(
            node_controller
                .blocks
                .iter()
                .position(|b| b.name == "Empty")
                .unwrap(),
        );
        possible_blocks.push(
            node_controller
                .blocks
                .iter()
                .position(|b| b.name == "Hull")
                .unwrap(),
        );

        Ok(Builder {
            build_blocks: HashMap::new(),
            base_ship_mesh: ShipMesh::new(context, &ship).unwrap(),
            build_ship_mesh: ShipMesh::new(context, &ship).unwrap(),
            ship,

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
        context: &Context,
        controls: &Controls,
        camera: &Camera,
        node_controller: &NodeController,
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
                let chunk_pos = get_chunk_pos_of_block_pos(pos);
                if self.ship.has_chunk(chunk_pos) {
                    // Undo the last placement.
                    let last_block_index = *self
                        .build_blocks
                        .get(&self.last_pos)
                        .unwrap_or(&BLOCK_INDEX_EMPTY);
                    self.ship
                        .place_block(self.last_pos, last_block_index, node_controller)?;

                    // Simulate placement of the block to create preview in build_ship.
                    let block_index = self.possible_blocks[self.block_to_build];
                    let _ = self.ship.place_block(pos, block_index, node_controller)?;

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

            //self.base_ship_mesh.update(&self.ship, context)?;
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
                    let (f, c, last_some) = self.ship.tick(1, node_controller, debug_controller)?;
                    full &= f;
                    changed_chunks = c;

                    if debug_controller.mode == DebugMode::WFC || !f || last_some {
                        break;
                    }
                }

                log::info!("BUILDER: TICK FULL {:?}", full);
            }

            if debug_controller.mode == DebugMode::WFC {
                debug_controller.add_text(vec!["WFC".to_owned()], vec3(-1.0, 0.0, 0.0))
            } else {
                debug_controller.add_text(vec!["WFC Skip".to_owned()], vec3(-1.0, 0.0, 0.0))
            }

            self.ship.debug_show_wave(debug_controller);
        } else {
            (self.full_tick, changed_chunks, _) =
                self.ship
                    .tick(self.actions_per_tick, node_controller, debug_controller)?;
        }

        #[cfg(not(debug_assertions))]
        {
            (self.full_tick, changed_chunks) =
                self.ship.tick(self.actions_per_tick, node_controller)?;
        }

        self.build_ship_mesh
            .update(&self.ship, changed_chunks, context)?;

        Ok(())
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        self.ship.on_node_controller_change(node_controller)?;
        self.last_block_to_build = BlockIndex::MAX;

        Ok(())
    }
}
