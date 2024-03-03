#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode};

use crate::math::{get_config, get_packed_index};
use crate::node::{Node, NodeID, PatternIndex, EMPYT_PATTERN_INDEX};
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::{BlockIndex, NodeController, Pattern, BLOCK_INDEX_EMPTY},
    ship_mesh::ShipMesh,
};
use index_queue::IndexQueue;
use octa_force::{
    anyhow::*,
    glam::*,
    log,
    vulkan::{ash::extensions::khr::RayTracingMaintenance1, Context},
};
use std::collections::{HashMap, VecDeque};
use std::iter;
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
    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,
}

#[derive(Clone, Debug)]
pub struct Wave {
    pub render_pattern: PatternIndex,
    pub possible_patterns: Vec<PatternIndex>,
    pub dependent_waves: IndexQueue,
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
            wave: vec![Wave::new(); max_wave_index],
            to_propergate: IndexQueue::default(),
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
        self.update_wave(pos, node_controller)?;

        while !self.to_collapse.is_empty() {
            self.to_collapse.pop_front();
        }

        Ok(())
    }

    fn update_wave(&mut self, block_pos: UVec3, node_controller: &NodeController) -> Result<()> {
        for &pos in node_controller.affected_poses.iter() {
            let req_pos = Self::get_wave_pos_of_block_pos(block_pos.as_ivec3()) + pos;
            if !Self::pos_in_bounds(req_pos, self.wave_size) {
                continue;
            }

            let index = to_1d_i(req_pos, self.wave_size.as_ivec3()) as usize;
            //let config = get_config(req_pos);

            //self.wave[index].current_pattern = node_controller.patterns[config].len() - 1;

            self.to_propergate.push_back(index);
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        node_controller: &NodeController,
        debug_controller: &mut DebugController,
    ) -> Result<(bool, bool)> {
        let mut full = true;
        let mut last_wave_something = false;
        for _ in 0..actions_per_tick {
            if !self.to_propergate.is_empty() {
                let wave_index = self.to_propergate.pop_front().unwrap();
                self.propergate(wave_index, node_controller, debug_controller)?;

                if debug_controller.mode == DebugMode::WFC {
                    last_wave_something =
                        self.wave[wave_index].render_pattern != EMPYT_PATTERN_INDEX;
                }

                continue;
            }

            if !self.to_collapse.is_empty() {
                let wave_index = self.to_collapse.pop_front().unwrap();
                self.collapse(wave_index, node_controller, debug_controller)?;

                if debug_controller.mode == DebugMode::WFC {
                    last_wave_something =
                        self.wave[wave_index].render_pattern != EMPYT_PATTERN_INDEX;
                }

                continue;
            }

            full = false;
            break;
        }

        if debug_controller.mode == DebugMode::WFC {
            self.debug_show_wave(debug_controller);
        }

        Ok((full, last_wave_something))
    }

    #[cfg(not(debug_assertions))]
    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        node_controller: &NodeController,
    ) -> Result<bool> {
        let mut full = true;
        for _ in 0..actions_per_tick {
            if !self.to_propergate.is_empty() {
                let wave_index = self.to_propergate.pop_front().unwrap();
                self.propergate(wave_index, node_controller)?;

                continue;
            }

            if !self.to_collapse.is_empty() {
                let wave_index = self.to_collapse.pop_front().unwrap();
                self.collapse(wave_index, node_controller)?;

                continue;
            }

            full = false;
            break;
        }

        Ok(full)
    }

    fn propergate(
        &mut self,
        wave_index: WaveIndex,
        node_controller: &NodeController,
        #[cfg(debug_assertions)] debug_controller: &mut DebugController,
    ) -> Result<()> {
        let wave_pos = to_3d(wave_index as u32, self.wave_size).as_ivec3();
        let config = get_config(wave_pos);

        #[cfg(debug_assertions)]
        if debug_controller.mode == DebugMode::WFC {
            debug_controller.add_cube(
                wave_pos.as_vec3(),
                (wave_pos + IVec3::ONE).as_vec3(),
                vec4(0.0, 0.0, 1.0, 1.0),
            );
        }

        self.wave[wave_index].possible_patterns.clear();
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
                for (&offset, _) in pattern.node_req.iter() {
                    let req_pos = wave_pos + offset;
                    let req_index = to_1d_i(req_pos, self.wave_size.as_ivec3()) as usize;

                    if !self.to_collapse.contains(wave_index) {
                        self.to_propergate.push_back(req_index);
                    }
                }

                self.wave[wave_index].possible_patterns.push(pattern_index);
            }
        }

        self.to_collapse.push_back(wave_index);

        Ok(())
    }

    fn collapse(
        &mut self,
        wave_index: WaveIndex,
        node_controller: &NodeController,
        #[cfg(debug_assertions)] debug_controller: &mut DebugController,
    ) -> Result<()> {
        let wave_pos = to_3d(wave_index as u32, self.wave_size).as_ivec3();
        let config = get_config(wave_pos);

        #[cfg(debug_assertions)]
        if debug_controller.mode == DebugMode::WFC {
            debug_controller.add_cube(
                wave_pos.as_vec3(),
                (wave_pos + IVec3::ONE).as_vec3(),
                vec4(0.0, 1.0, 0.0, 1.0),
            );
        }

        let old_possible_pattern_size = self.wave[wave_index].possible_patterns.len();
        for (i, pattern_index) in self.wave[wave_index]
            .possible_patterns
            .to_owned()
            .into_iter()
            .enumerate()
            .rev()
        {
            let pattern = &node_controller.patterns[config][pattern_index];

            let accepted =
                pattern.node_req.iter().all(|(&offset, indecies)| {
                    let req_pos = wave_pos + offset;
                    let req_config = get_config(req_pos);

                    if !Self::pos_in_bounds(req_pos, self.wave_size) {
                        return false;
                    }

                    let req_index = to_1d_i(req_pos, self.wave_size.as_ivec3()) as usize;
                    let found = self.wave[req_index].possible_patterns.iter().all(
                        |&possible_pattern_index| {
                            let possible_pattern =
                                &node_controller.patterns[req_config][possible_pattern_index];
                            indecies.contains(&possible_pattern.node.index)
                        },
                    );

                    found
                });

            if !accepted {
                self.wave[wave_index].possible_patterns.remove(i);
            }
        }

        let possible_patterns = &self.wave[wave_index].possible_patterns;
        let possible_patterns_changed = possible_patterns.len() != old_possible_pattern_size;
        let new_render_pattern = possible_patterns[possible_patterns.len() - 1];

        self.wave[wave_index].render_pattern = new_render_pattern;

        let pattern = &node_controller.patterns[config][new_render_pattern];
        for (&offset, _) in pattern.node_req.iter() {
            let req_pos = wave_pos + offset;

            debug_assert!(Self::pos_in_bounds(req_pos, self.wave_size));

            let index = to_1d_i(req_pos, self.wave_size.as_ivec3()) as usize;
            self.wave[index].dependent_waves.push_back(wave_index);

            if possible_patterns_changed {
                self.to_collapse.push_back(index);
            }
        }

        if possible_patterns_changed {
            while !self.wave[wave_index].dependent_waves.is_empty() {
                let index = self.wave[wave_index].dependent_waves.pop_front().unwrap();
                self.to_collapse.push_back(index);
            }
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn debug_show_wave(&mut self, debug_controller: &mut DebugController) {
        let mut to_propergate = self.to_propergate.to_owned();
        while !to_propergate.is_empty() {
            let index = to_propergate.pop_front().unwrap();
            let wave_pos = to_3d(index as u32, self.wave_size).as_ivec3();

            let wave = &self.wave[index];
            let lines = iter::once("p".to_owned())
                .chain(wave.possible_patterns.iter().map(|p| p.to_string()))
                .collect();
            debug_controller.add_text(lines, wave_pos.as_vec3());
        }

        let mut to_collpase = self.to_collapse.to_owned();
        while !to_collpase.is_empty() {
            let index = to_collpase.pop_front().unwrap();
            let wave_pos = to_3d(index as u32, self.wave_size).as_ivec3();

            let wave = &self.wave[index];
            let lines = iter::once("c".to_owned())
                .chain(wave.possible_patterns.iter().map(|p| p.to_string()))
                .collect();
            debug_controller.add_text(lines, wave_pos.as_vec3());
        }
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        let max_wave_index = (self.wave_size.x * self.wave_size.y * self.wave_size.z) as usize;
        self.wave = vec![Wave::new(); max_wave_index];

        for x in 0..self.block_size.x {
            for y in 0..self.block_size.y {
                for z in 0..self.block_size.z {
                    let pos = uvec3(x, y, z);
                    self.update_wave(pos, node_controller)?;
                }
            }
        }

        Ok(())
    }
}

impl Wave {
    pub fn new() -> Self {
        Wave {
            render_pattern: 0,
            possible_patterns: Vec::new(),
            dependent_waves: IndexQueue::default(),
        }
    }
}
