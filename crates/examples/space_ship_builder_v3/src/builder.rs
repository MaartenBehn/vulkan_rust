use crate::{
    node::{BlockIndex, NodeController},
    ship::{Ship, SHIP_TYPE_BASE, SHIP_TYPE_BUILDER},
};
use app::{anyhow::Result, camera::Camera, controls::Controls, glam::UVec3, vulkan::Context};
use std::{mem, ops::Index, time::Duration};

const SCROLL_SPEED: f32 = 0.01;
const PLACE_SPEED: Duration = Duration::from_millis(100);

enum BuilderState {
    ON,
    OFF,
}

pub struct Builder {
    pub base_ship: Ship,
    pub build_ship: Ship,

    state: BuilderState,

    possible_blocks: Vec<BlockIndex>,
    block_to_build: usize,
    distance: f32,

    last_block_to_build: BlockIndex,
    last_pos: Option<UVec3>,
    last_action_time: Duration,
}

impl Builder {
    pub fn new(ship: Ship, context: &Context, node_controller: &NodeController) -> Result<Builder> {
        let mut possible_blocks = Vec::new();
        possible_blocks.push(
            node_controller
                .blocks
                .iter()
                .position(|b| b.name == "Base")
                .unwrap(),
        );
        possible_blocks.push(
            node_controller
                .blocks
                .iter()
                .position(|b| b.name == "Hull")
                .unwrap(),
        );
        possible_blocks.push(
            node_controller
                .blocks
                .iter()
                .position(|b| b.name == "Empty")
                .unwrap(),
        );

        Ok(Builder {
            build_ship: Ship::new(ship.block_size, context, node_controller, SHIP_TYPE_BUILDER)?,
            base_ship: ship,

            state: BuilderState::ON,
            block_to_build: 0,
            possible_blocks,
            distance: 3.0,

            last_block_to_build: 0,
            last_pos: None,
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
    ) -> Result<()> {
        match self.state {
            BuilderState::ON => {
                if controls.e && (self.last_action_time + PLACE_SPEED) < total_time {
                    self.last_action_time = total_time;

                    self.block_to_build += 1;
                    if self.block_to_build >= self.possible_blocks.len() {
                        self.block_to_build = 0;
                    }
                }

                self.distance -= controls.scroll_delta * SCROLL_SPEED;
                let pos = ((camera.position + camera.direction * self.distance) / 2.0)
                    .round()
                    .as_ivec3()
                    * 2;

                // Get the index of the block that could be placed
                let selected_block_index = self.base_ship.get_block_i(pos);
                let selected_pos = if selected_block_index.is_ok() {
                    Some(pos.as_uvec3())
                } else {
                    None
                };

                if self.last_pos != selected_pos || self.last_block_to_build != self.block_to_build
                {
                    self.last_pos = selected_pos;

                    // Reset the build ship to the state of the current ship.
                    self.build_ship.clone_from(&self.base_ship)?;

                    // If block index is valid.
                    if selected_pos.is_some() {
                        // Simulate placement of the block to create preview in build_ship.
                        self.build_ship.place_block(
                            selected_pos.unwrap() / 2,
                            self.possible_blocks[self.block_to_build],
                            node_controller,
                        )?;
                    }
                }
                self.last_block_to_build = self.block_to_build;

                if controls.left && (self.last_action_time + PLACE_SPEED) < total_time {
                    mem::swap(&mut self.base_ship, &mut self.build_ship);
                    self.base_ship.ship_type = SHIP_TYPE_BASE;
                    self.build_ship.ship_type = SHIP_TYPE_BUILDER;
                    self.last_block_to_build = usize::MAX;

                    self.last_action_time = total_time;
                }

                self.build_ship.tick(delta_time)?;
            }
            BuilderState::OFF => {}
        }

        self.base_ship.tick(delta_time)?;

        Ok(())
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        self.base_ship.on_node_controller_change(node_controller)?;
        self.build_ship.on_node_controller_change(node_controller)?;

        Ok(())
    }
}
