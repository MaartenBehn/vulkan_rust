use crate::math::to_1d;
use crate::ship_mesh::ShipMesh;
use crate::{
    node::{BlockIndex, NodeController},
    ship::Ship,
};

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode};

use octa_force::glam::{vec4, Vec3};
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
    pub build_blocks: Vec<BlockIndex>,

    pub base_ship_mesh: ShipMesh,
    pub build_ship_mesh: ShipMesh,

    possible_blocks: Vec<BlockIndex>,
    block_to_build: usize,
    distance: f32,

    actions_per_tick: usize,
    full_tick: bool,

    last_block_to_build: BlockIndex,
    last_pos: UVec3,
    last_action_time: Duration,
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
            build_blocks: ship.blocks.to_owned(),
            base_ship_mesh: ShipMesh::new(context, &ship).unwrap(),
            build_ship_mesh: ShipMesh::new(context, &ship).unwrap(),
            ship,

            block_to_build: 1,
            possible_blocks,
            distance: 3.0,

            actions_per_tick: 4,
            full_tick: false,

            last_block_to_build: 0,
            last_pos: UVec3::ZERO,
            last_action_time: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
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
        let d = debug_controller.mode != DebugMode::WFC || controls.lshift;
        #[cfg(not(debug_assertions))]
        let d = true;
        if d {
            let pos = ((camera.position + camera.direction * self.distance) / 2.0)
                .round()
                .as_ivec3();

            // Get the index of the block that could be placed
            let selected_pos = if Ship::pos_in_bounds(pos, self.ship.block_size) {
                Some(pos.as_uvec3())
            } else {
                None
            };

            if Some(self.last_pos) != selected_pos
                || self.last_block_to_build != self.block_to_build
            {
                // Undo the last placement.
                let last_block_index = to_1d(self.last_pos, self.ship.block_size);
                self.ship.place_block(
                    self.last_pos,
                    self.build_blocks[last_block_index],
                    node_controller,
                )?;

                // If block index is valid.
                if selected_pos.is_some() {
                    self.last_block_to_build = self.block_to_build;
                    self.last_pos = selected_pos.unwrap();

                    // Simulate placement of the block to create preview in build_ship.
                    self.ship.place_block(
                        selected_pos.unwrap(),
                        self.possible_blocks[self.block_to_build],
                        node_controller,
                    )?;
                }
            }
        }

        if controls.left && (self.last_action_time + PLACE_SPEED) < total_time {
            self.build_blocks = self.ship.blocks.to_owned();
            self.last_action_time = total_time;

            self.base_ship_mesh.update(&self.ship, node_controller)?;
        }

        #[cfg(debug_assertions)]
        if debug_controller.mode == DebugMode::WFC
            && controls.t
            && (self.last_action_time + DEBUG_COLLAPSE_SPEED) < total_time
        {
            self.last_action_time = total_time;

            let mut full = true;
            loop {
                debug_controller.line_renderer.vertecies.clear();
                debug_controller.text_renderer.texts.clear();
                let (f, last_some) = self.ship.tick(1, node_controller, debug_controller)?;
                full &= f;

                if !f || last_some {
                    break;
                }
            }

            if !full {
                debug_controller.add_cube(
                    Vec3::ZERO,
                    Vec3::ONE * self.ship.wave_size.as_vec3(),
                    vec4(1.0, 0.0, 0.0, 1.0),
                );
                debug_controller.add_text("Done".to_owned(), Vec::new(), Vec3::ZERO)
            }

            log::info!("BUILDER: TICK FULL {:?}", full);
        } else if debug_controller.mode != DebugMode::WFC {
            let (full, _) =
                self.ship
                    .tick(self.actions_per_tick, node_controller, debug_controller)?;
            self.full_tick = full;

            debug_controller.add_cube(
                Vec3::ZERO,
                Vec3::ONE * self.ship.wave_size.as_vec3(),
                vec4(1.0, 0.0, 0.0, 1.0),
            );
        }

        #[cfg(not(debug_assertions))]
        {
            self.full_tick = self.ship.tick(self.actions_per_tick, node_controller)?;
        }

        self.build_ship_mesh.update(&self.ship, node_controller)?;

        Ok(())
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        self.ship.blocks = self.build_blocks.to_owned();
        self.ship.on_node_controller_change(node_controller)?;
        self.last_block_to_build = BlockIndex::MAX;

        Ok(())
    }
}
