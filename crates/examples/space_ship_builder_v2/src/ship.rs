use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::thread::panicking;
use std::time::Duration;

use index_queue::IndexQueue;
use octa_force::anyhow::*;
use octa_force::glam::*;
use octa_force::log;
use octa_force::vulkan::Context;

use crate::math::to_1d;
use crate::math::to_1d_i;
use crate::math::to_3d;
use crate::node::BlockIndex;
use crate::node::NodeController;
use crate::node::Pattern;
use crate::node::BLOCK_INDEX_NONE;
use crate::node::NODE_INDEX_NONE;
use crate::pattern_config::Config;
use crate::ship_mesh::ShipMesh;

pub type WaveIndex = usize;

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

pub struct Ship {
    pub block_size: UVec3,
    pub wave_size: UVec3,

    pub blocks: Vec<BlockIndex>,
    pub wave: Vec<Wave>,
    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,

    pub actions_per_tick: usize,
    pub full_tick: bool,

    pub mesh: ShipMesh,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Wave {
    pub possible_pattern: Vec<Pattern>,
    pub all_possible_pattern: Vec<Pattern>,
}

impl Ship {
    pub fn new(context: &Context, node_controller: &NodeController) -> Result<Ship> {
        let block_size = uvec3(10, 10, 10);
        let wave_size = block_size + uvec3(1, 1, 1);

        let max_block_index = (block_size.x * block_size.y * block_size.z) as usize;
        let max_wave_index = (wave_size.x * wave_size.y * wave_size.z) as usize;
        let mesh = ShipMesh::new(context, max_wave_index + 1)?;

        let mut ship = Ship {
            block_size,
            wave_size,
            blocks: vec![BLOCK_INDEX_NONE; max_block_index],
            wave: vec![Wave::new(node_controller); max_wave_index],
            to_propergate: IndexQueue::default(),
            to_collapse: IndexQueue::default(),
            actions_per_tick: 4,
            full_tick: false,

            mesh,
        };

        ship.place_block(uvec3(0, 0, 0), 0, node_controller)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    fn pos_in_bounds(pos: IVec3, size: UVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(size.as_ivec3()).all()
    }

    pub fn get_block(&self, pos: UVec3) -> Result<BlockIndex> {
        self.get_block_i(pos.as_ivec3())
    }

    pub fn get_block_i(&self, pos: IVec3) -> Result<BlockIndex> {
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

        log::info!("Place: {pos:?}");
        self.blocks[cell_index] = block_index;

        self.update_wave(pos, node_controller);

        while !self.to_collapse.is_empty() {
            self.to_collapse.pop_front();
        }

        Ok(())
    }

    pub fn get_wave_poses_of_block_pos(pos: IVec3) -> impl Iterator<Item = UVec3> {
        [
            pos + ivec3(0, 0, 0),
            pos + ivec3(1, 0, 0),
            pos + ivec3(0, 1, 0),
            pos + ivec3(1, 1, 0),
            pos + ivec3(0, 0, 1),
            pos + ivec3(1, 0, 1),
            pos + ivec3(0, 1, 1),
            pos + ivec3(1, 1, 1),
        ]
        .into_iter()
        .map(|pos| pos.as_uvec3())
    }

    pub fn get_block_poses_of_wave_pos(
        &mut self,
        pos: IVec3,
    ) -> impl Iterator<Item = (usize, UVec3)> + '_ {
        [
            pos + ivec3(0, 0, 0),
            pos + ivec3(-1, 0, 0),
            pos + ivec3(0, -1, 0),
            pos + ivec3(-1, -1, 0),
            pos + ivec3(0, 0, -1),
            pos + ivec3(-1, 0, -1),
            pos + ivec3(0, -1, -1),
            pos + ivec3(-1, -1, -1),
        ]
        .into_iter()
        .enumerate()
        .filter(|(_, pos)| Self::pos_in_bounds(*pos, self.block_size))
        .map(|(i, pos)| (i, pos.as_uvec3()))
    }

    pub fn get_neigbor_indices_of_wave_pos(
        &mut self,
        pos: IVec3,
    ) -> impl Iterator<Item = WaveIndex> + '_ {
        [
            pos + ivec3(-1, -1, -1),
            pos + ivec3(0, -1, -1),
            pos + ivec3(1, -1, -1),
            pos + ivec3(-1, 0, -1),
            pos + ivec3(0, 0, -1),
            pos + ivec3(1, 0, -1),
            pos + ivec3(-1, 1, -1),
            pos + ivec3(0, 1, -1),
            pos + ivec3(1, 1, -1),
            pos + ivec3(-1, -1, 0),
            pos + ivec3(0, -1, 0),
            pos + ivec3(1, -1, 0),
            pos + ivec3(-1, 0, 0),
            pos + ivec3(1, 0, 0),
            pos + ivec3(-1, 1, 0),
            pos + ivec3(0, 1, 0),
            pos + ivec3(1, 1, 0),
            pos + ivec3(-1, -1, 1),
            pos + ivec3(0, -1, 1),
            pos + ivec3(1, -1, 1),
            pos + ivec3(-1, 0, 1),
            pos + ivec3(0, 0, 1),
            pos + ivec3(1, 0, 1),
            pos + ivec3(-1, 1, 1),
            pos + ivec3(0, 1, 1),
            pos + ivec3(1, 1, 1),
        ]
        .into_iter()
        .filter(|pos| Self::pos_in_bounds(*pos, self.wave_size))
        .map(|pos| to_1d(pos.as_uvec3(), self.wave_size))
    }

    fn update_wave(&mut self, pos: UVec3, node_controller: &NodeController) {
        for wave_pos in Self::get_wave_poses_of_block_pos(pos.as_ivec3()) {
            let wave_index = to_1d(wave_pos, self.wave_size) as usize;

            let config = self.get_wave_config(wave_pos);
            let config_index: usize = config.into();

            let wave = &mut self.wave[wave_index];
            wave.all_possible_pattern = node_controller.pattern[config_index].to_owned();

            self.to_propergate.push_back(wave_index);
        }
    }

    fn get_wave_config(&mut self, wave_pos: UVec3) -> Config {
        let mut config = [false; 8];
        let block_poses: Vec<_> = self
            .get_block_poses_of_wave_pos(wave_pos.as_ivec3())
            .collect();
        for (i, block_pos) in block_poses {
            let block_index = self.get_block(block_pos).unwrap();
            config[i] = block_index != BLOCK_INDEX_NONE;
        }
        config.into()
    }

    pub fn tick(&mut self, delta_time: Duration) -> Result<()> {
        if self.to_propergate.is_empty() && self.to_collapse.is_empty() {
            return Ok(());
        }

        if self.full_tick {
            if delta_time < MIN_TICK_LENGTH && self.actions_per_tick < (usize::MAX / 2) {
                self.actions_per_tick *= 2;
            } else if delta_time > MAX_TICK_LENGTH && self.actions_per_tick > 4 {
                self.actions_per_tick /= 2;
            }
        }

        log::info!("Tick: {}", self.actions_per_tick);

        self.full_tick = true;
        for _ in 0..self.actions_per_tick {
            //self.print_ship();

            if !self.to_propergate.is_empty() {
                let index = self.to_propergate.pop_front().unwrap();
                self.propergate(index);
            } else if !self.to_collapse.is_empty() {
                let index = self.to_collapse.pop_front().unwrap();
                self.collapse(index);
            } else {
                self.full_tick = false;
                break;
            }
        }

        self.mesh.update(self.wave_size, &self.wave)?;

        Ok(())
    }

    fn propergate(&mut self, wave_index: WaveIndex) {
        let pos = to_3d(wave_index as u32, self.wave_size);
        let wave = self.wave[wave_index].to_owned();

        let old_pattern = wave.possible_pattern.to_owned();
        let mut patterns = wave.all_possible_pattern.to_owned();

        for i in (0..patterns.len()).rev() {
            let pattern = &patterns[i];
            if pattern.req.is_empty() {
                continue;
            }

            for (offset, node_id) in pattern.req.iter() {
                let req_pos = pos.as_ivec3() + *offset;
                if !Self::pos_in_bounds(req_pos, self.wave_size) {
                    continue;
                }
                let req_index = to_1d(req_pos.as_uvec3(), self.wave_size);

                let mut found = false;
                for test_pattern in self.wave[req_index].all_possible_pattern.iter() {
                    if node_id.contains(&test_pattern.id.index) {
                        found = true;
                        break;
                    }
                }

                if !found {
                    patterns.remove(i);
                    break;
                }
            }
        }

        if old_pattern != patterns {
            let to_collapse = self.to_collapse.to_owned();
            let neigbors: Vec<_> = self
                .get_neigbor_indices_of_wave_pos(pos.as_ivec3())
                .filter(|index| !to_collapse.contains(*index))
                .collect();
            for neigbor in neigbors {
                self.to_propergate.push_back(neigbor);
            }

            self.wave[wave_index].possible_pattern = patterns;
        }

        self.to_collapse.push_back(wave_index);
    }

    fn collapse(&mut self, wave_index: WaveIndex) {
        let pos = to_3d(wave_index as u32, self.wave_size);
        let wave = self.wave[wave_index].to_owned();

        let old_pattern = wave.possible_pattern.to_owned();
        let mut patterns = wave.possible_pattern.to_owned();

        for i in (0..patterns.len()).rev() {
            let pattern = &patterns[i];
            if pattern.req.is_empty() {
                continue;
            }

            for (offset, node_id) in pattern.req.iter() {
                let req_pos = pos.as_ivec3() + *offset;
                if Self::pos_in_bounds(req_pos, self.wave_size) {
                    let req_index = to_1d(req_pos.as_uvec3(), self.wave_size);
                    let index = self.wave[req_index].possible_pattern[0].id.index;
                    if node_id.contains(&index) {
                        continue;
                    }
                }

                patterns.remove(i);
                break;
            }
        }

        if old_pattern != patterns {
            let neigbors: Vec<_> = self
                .get_neigbor_indices_of_wave_pos(pos.as_ivec3())
                .collect();
            for neigbor in neigbors {
                self.to_collapse.push_back(neigbor);
            }

            self.wave[wave_index].possible_pattern = patterns;
        }
    }

    fn print_ship(&self) {
        log::info!("Ship: ");

        let mut text = "".to_owned();
        for z in 0..self.wave_size.z {
            log::info!("Z: {:?}", z);
            for x in 0..self.wave_size.x {
                text.push_str("|");
                for y in 0..self.wave_size.y {
                    let pos = uvec3(x, y, z);
                    let wave = self.get_wave(pos).unwrap();
                    let index = to_1d(pos, self.wave_size);

                    let mut t = "".to_owned();

                    for pattern in wave.possible_pattern.iter() {
                        if pattern.id.index == NODE_INDEX_NONE {
                            continue;
                        }

                        t.push_str(&format!("{:?} ", pattern.id.index));
                    }

                    if self.to_propergate.contains(index) {
                        t.push_str("p");
                    }

                    if self.to_collapse.contains(index) {
                        t.push_str("c");
                    }

                    text.push_str(&t);

                    for _ in (t.len())..12 {
                        text.push_str(" ");
                    }

                    text.push_str("|");
                }
                log::info!("{:?}", text);
                text.clear();
            }
        }
    }
}

impl Wave {
    pub fn new(node_controller: &NodeController) -> Self {
        Self {
            possible_pattern: node_controller.pattern[0].to_owned(),
            all_possible_pattern: node_controller.pattern[0].to_owned(),
        }
    }
}
