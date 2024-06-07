use crate::rules::block::BlockNameIndex;
use crate::rules::Rules;
use octa_force::glam::{vec3, IVec3};
use octa_force::{anyhow::Result, camera::Camera, controls::Controls};
use std::time::Duration;

use crate::ship::data::ShipData;

const SCROLL_SPEED: f32 = 0.01;
const PLACE_SPEED: Duration = Duration::from_millis(100);

pub struct ShipBuilder {
    possible_blocks: Vec<BlockNameIndex>,
    block_to_build: usize,
    distance: f32,

    last_action_time: Duration,

    last_pos: IVec3,
    last_block_to_build: usize,
    last_block_index: Option<usize>,
}

impl ShipBuilder {
    pub fn new(rules: &Rules) -> ShipBuilder {
        let mut possible_blocks = Vec::new();
        possible_blocks.push(
            rules
                .block_names
                .iter()
                .position(|name| name == "Empty")
                .unwrap(),
        );
        possible_blocks.push(
            rules
                .block_names
                .iter()
                .position(|name| name == "Hull")
                .unwrap(),
        );

        ShipBuilder {
            block_to_build: 1,
            possible_blocks,
            distance: 3.0,

            last_action_time: Duration::default(),
            last_pos: IVec3::ZERO,
            last_block_to_build: BlockNameIndex::MAX,
            last_block_index: None,
        }
    }

    pub fn update(
        &mut self,
        data: &mut ShipData,

        controls: &Controls,
        camera: &Camera,
        rules: &Rules,
        total_time: Duration,
    ) -> Result<()> {
        if controls.e && (self.last_action_time + PLACE_SPEED) < total_time {
            self.last_action_time = total_time;

            self.block_to_build = (self.block_to_build + 1) % self.possible_blocks.len();
        }
        self.distance -= controls.scroll_delta * SCROLL_SPEED;

        let pos = (((camera.position + camera.direction * self.distance) - vec3(1.0, 1.0, 1.0))
            / 2.0)
            .round()
            .as_ivec3();

        if self.last_pos != pos || self.last_block_to_build != self.block_to_build {
            if self.last_block_index.is_some() {
                data.place_block(self.last_pos, self.last_block_index.unwrap(), rules);
            }

            // Update last
            self.last_block_to_build = self.block_to_build;
            self.last_pos = pos;
            self.last_block_index = Some(data.get_block_at_pos(pos));

            // Place new Block
            let block_index = self.possible_blocks[self.block_to_build];
            data.place_block(pos, block_index, rules);
        }

        if controls.left && (self.last_action_time + PLACE_SPEED) < total_time {
            self.last_action_time = total_time;
            self.last_block_index = None;
        }

        Ok(())
    }

    pub fn on_rules_changed(&mut self) {
        self.last_block_to_build = BlockNameIndex::MAX;
    }
}
