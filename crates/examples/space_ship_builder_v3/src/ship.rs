use crate::math::{get_config, get_packed_index};
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
    pub to_collapse: IndexQueue,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Wave {
    pub current_pattern: PatternIndex,
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
            to_collapse: IndexQueue::default(),
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

    pub fn get_wave_pos_of_block_pos(pos: IVec3) -> IVec3 {
        pos * 2 - IVec3::ONE
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

        log::info!("Place: {pos:?}");
        self.blocks[cell_index] = block_index;
        self.propergate(pos, node_controller)?;

        Ok(())
    }

    fn propergate(&mut self, block_pos: UVec3, node_controller: &NodeController) -> Result<()> {
        for &pos in node_controller.affected_poses.iter() {
            let req_pos = Self::get_wave_pos_of_block_pos(block_pos.as_ivec3()) + pos;
            if !Self::pos_in_bounds(req_pos, self.wave_size) {
                continue;
            }

            let index = to_1d_i(req_pos, self.wave_size.as_ivec3()) as usize;
            let config = get_config(req_pos);

            self.wave[index].current_pattern = node_controller.patterns[config].len() - 1;

            if !self.to_collapse.contains(index) {
                self.to_collapse.push_back(index);
            }
        }

        Ok(())
    }

    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        node_controller: &NodeController,
    ) -> Result<bool> {
        let mut full = true;
        for _ in 0..actions_per_tick {
            if self.to_collapse.is_empty() {
                full = false;
                break;
            }

            let wave_index = self.to_collapse.pop_front().unwrap();
            let wave_pos = to_3d(wave_index as u32, self.wave_size).as_ivec3();
            let config = get_config(wave_pos);

            self.wave[wave_index].current_pattern = 0;
            for (pattern_index, pattern) in node_controller.patterns[config].iter().enumerate() {
                let accepted = pattern.block_req.iter().all(|(&offset, indecies)| {
                    let req_pos = wave_pos + offset;

                    if !Self::pos_in_bounds(req_pos, self.wave_size) {
                        return indecies.contains(&BLOCK_INDEX_EMPTY);
                    }

                    debug_assert!((req_pos % 2) == IVec3::ONE);

                    let block_pos = req_pos / 2;
                    let index = to_1d_i(block_pos, self.block_size.as_ivec3()) as usize;
                    let block_index = self.blocks[index];
                    indecies.contains(&block_index)
                });

                if accepted {
                    self.wave[wave_index].current_pattern = pattern_index;
                }
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
                    self.propergate(pos, node_controller)?;
                }
            }
        }

        Ok(())
    }
}

impl Wave {
    pub fn new(node_controller: &NodeController) -> Self {
        Wave { current_pattern: 0 }
    }
}
