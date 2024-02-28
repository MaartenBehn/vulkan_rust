use crate::math::get_packed_index;
use crate::node::{Node, NodeID, PatternIndex};
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::{BlockIndex, NodeController, Pattern, BLOCK_INDEX_EMPTY},
    pattern_config::{BlockConfig, Config},
    ship_mesh::ShipMesh,
};
use app::{
    anyhow::*,
    glam::*,
    log,
    vulkan::{ash::extensions::khr::RayTracingMaintenance1, Context},
};
use index_queue::IndexQueue;
use std::collections::{HashMap, VecDeque};
use std::mem::size_of;
use std::time::Duration;

pub type WaveIndex = usize;
pub type ShipType = u32;
pub const SHIP_TYPE_BASE: ShipType = 0;
pub const SHIP_TYPE_BUILD: ShipType = 1;

pub const REQ_STATE_SIZE: usize = 10000;
pub const PATTERN_STATE_SIZE: usize = 10000;

pub struct Ship {
    pub block_size: UVec3,
    pub wave_size: UVec3,

    pub blocks: Vec<BlockIndex>,
    pub wave: Vec<Wave>,
    pub tasks: VecDeque<UVec3>,
    pub task_counter: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Wave {
    pub current_pattern: PatternIndex,
    pub req_state: [u8; REQ_STATE_SIZE],
    pub pattern_state: [u8; PATTERN_STATE_SIZE],
}

impl Ship {
    pub fn new(
        block_size: UVec3,
        context: &Context,
        node_controller: &NodeController,
    ) -> Result<Ship> {
        let wave_size = block_size * 2;
        let max_block_index = (block_size.x * block_size.y * block_size.z) as usize;
        let max_wave_index = (wave_size.x * wave_size.y * wave_size.z) as usize;

        let mut ship = Ship {
            block_size,
            wave_size,
            blocks: vec![BLOCK_INDEX_EMPTY; max_block_index],
            wave: vec![Wave::new(node_controller); max_wave_index],
            tasks: VecDeque::new(),
            task_counter: 0,
        };

        let size = size_of::<Ship>()
            + size_of::<PatternIndex>() * max_block_index
            + size_of::<Wave>() * max_wave_index;
        log::info!("Ship size {:?} MB", size as f32 / 1000000.0);

        //ship.place_block(uvec3(0, 0, 0), 1, node_controller)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    pub(crate) fn pos_in_bounds(pos: IVec3, size: UVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(size.as_ivec3()).all()
    }

    pub fn get_block(&self, pos: UVec3) -> Result<usize> {
        self.get_block_i(pos.as_ivec3())
    }

    pub fn get_block_i(&self, pos: IVec3) -> Result<usize> {
        if !Self::pos_in_bounds(pos, self.block_size) {
            bail!("Pos not in ship")
        }

        let index = to_1d_i(pos, self.block_size.as_ivec3());
        Ok(self.blocks[index as usize])
    }

    pub fn get_wave(&self, pos: UVec3) -> Result<&Wave> {
        self.get_wave_i(pos.as_ivec3())
    }

    pub fn get_wave_i(&self, pos: IVec3) -> Result<&Wave> {
        if !Self::pos_in_bounds(pos, self.wave_size) {
            bail!("Wave Pos not in ship")
        }

        let index = to_1d_i(pos, self.wave_size.as_ivec3());
        Ok(&self.wave[index as usize])
    }

    pub fn fill_all(
        &mut self,
        block_index: BlockIndex,
        node_controller: &NodeController,
    ) -> Result<()> {
        for x in 0..self.block_size.x {
            for y in 0..self.block_size.y {
                for z in 0..self.block_size.z {
                    self.place_block(uvec3(x, y, z), block_index, node_controller)?;
                }
            }
        }

        Ok(())
    }

    pub fn place_block(
        &mut self,
        pos: UVec3,
        block_index: BlockIndex,
        node_controller: &NodeController,
    ) -> Result<()> {
        let cell_index = to_1d(pos, self.block_size);
        if self.blocks[cell_index] == block_index {
            return Ok(());
        }

        //log::info!("Place: {pos:?}");
        self.blocks[cell_index] = block_index;

        self.tasks.push_back(pos);

        Ok(())
    }

    pub fn get_wave_pos_of_block_pos(pos: IVec3) -> IVec3 {
        pos * 2 + IVec3::ONE
    }

    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        node_controller: &NodeController,
    ) -> Result<bool> {
        let mut full = true;
        for _ in 0..actions_per_tick {
            if !self.tasks.is_empty() {
                let block_pos = self.tasks.front().unwrap().as_ivec3();
                let block_index = self.get_block_i(block_pos).unwrap();
                let (offset, pattern_indecies) = &node_controller.req_poses[self.task_counter];

                let req_pos = Self::get_wave_pos_of_block_pos(block_pos) - offset.to_owned();

                if Self::pos_in_bounds(req_pos, self.wave_size) {
                    let index = to_1d(req_pos.as_uvec3(), self.wave_size);

                    for (i, &pattern_index) in pattern_indecies.iter().enumerate() {
                        let pattern = &node_controller.patterns[pattern_index];

                        if !pattern.block_req.is_empty() {
                            let req_block_indecies = pattern.block_req.get(&offset).unwrap();

                            let found = req_block_indecies.contains(&block_index);

                            self.wave[index].set_req_state(offset, i, found, node_controller);
                            self.wave[index].set_pattern_state(
                                offset,
                                pattern_index,
                                found,
                                node_controller,
                            );
                        }

                        if self.wave[index].current_pattern < pattern_index
                            && self.wave[index].all_pattern_state(pattern_index, node_controller)
                        {
                            self.wave[index].current_pattern = pattern_index;
                        } else if self.wave[index].current_pattern == pattern_index
                            && !self.wave[index].all_pattern_state(pattern_index, node_controller)
                        {
                            // Search all the patterns from this to next pattern in pattern_indecies for a vaild one
                            let mut found = false;
                            if i < pattern_indecies.len() - 1 {
                                for test_index in
                                    ((pattern_indecies[i + 1] + 1)..pattern_index).rev()
                                {
                                    if self.wave[index]
                                        .all_pattern_state(test_index, node_controller)
                                    {
                                        self.wave[index].current_pattern = test_index;
                                        found = true;
                                        break;
                                    }
                                }

                                if !found {
                                    self.wave[index].current_pattern = pattern_indecies[i + 1]
                                }
                            }
                        }
                    }
                }

                self.task_counter += 1;
                if self.task_counter >= node_controller.req_poses.len() {
                    self.tasks.pop_front();
                    self.task_counter = 0;
                }
            } else {
                full = false;
                break;
            }
        }

        Ok(full)
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        let max_wave_index = (self.wave_size.x * self.wave_size.y * self.wave_size.z) as usize;
        self.wave = vec![Wave::new(node_controller); max_wave_index];

        for x in 0..self.block_size.x {
            for y in 0..self.block_size.y {
                for z in 0..self.block_size.z {
                    let pos = uvec3(x, y, z);
                    self.tasks.push_back(pos);
                }
            }
        }

        Ok(())
    }
}

impl Wave {
    pub fn new(node_controller: &NodeController) -> Self {
        let mut wave = Wave {
            current_pattern: 0,
            req_state: [0; REQ_STATE_SIZE],
            pattern_state: [0; PATTERN_STATE_SIZE],
        };

        for (i, p) in node_controller.patterns.iter().enumerate() {
            for (pos, indecies) in p.block_req.iter() {
                if indecies.contains(&BLOCK_INDEX_EMPTY) {
                    wave.set_pattern_state(pos, i, true, node_controller);
                }
            }
        }

        wave
    }

    pub fn get_req_state(
        &self,
        pos: &IVec3,
        index: usize,
        node_controller: &NodeController,
    ) -> bool {
        let index = node_controller.req_state_lookup[pos][index];
        let (i, j) = get_packed_index(index);
        (self.req_state[i] & j) == 1
    }

    pub fn set_req_state(
        &mut self,
        pos: &IVec3,
        index: usize,
        value: bool,
        node_controller: &NodeController,
    ) {
        let index = node_controller.req_state_lookup[pos][index];
        let (i, j) = get_packed_index(index);

        let mask = self.req_state[i] & !j;
        self.req_state[i] = mask + j * (value as u8);
    }

    pub fn get_pattern_state(
        &self,
        pos: &IVec3,
        index: PatternIndex,
        node_controller: &NodeController,
    ) -> bool {
        let index = node_controller.pattern_state_lookup[index][pos];
        let (i, j) = get_packed_index(index);
        (self.pattern_state[i] & j) == 1
    }

    pub fn set_pattern_state(
        &mut self,
        pos: &IVec3,
        index: PatternIndex,
        value: bool,
        node_controller: &NodeController,
    ) {
        let index = node_controller.pattern_state_lookup[index][pos];
        let (i, j) = get_packed_index(index);

        let mask = self.pattern_state[i] & !j;
        self.pattern_state[i] = mask + j * (value as u8);
    }

    pub fn all_pattern_state(&self, index: PatternIndex, node_controller: &NodeController) -> bool {
        for (_, &index) in node_controller.pattern_state_lookup[index].iter() {
            let (i, j) = get_packed_index(index);
            if (self.pattern_state[i] & j) != 1 {
                return false;
            }
        }
        return true;
    }
}
