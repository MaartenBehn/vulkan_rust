use crate::node::NodeID;
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
use std::collections::HashMap;
use std::time::Duration;

pub type WaveIndex = usize;
pub type ShipType = u32;

pub const SHIP_TYPE_BASE: ShipType = 0;
pub const SHIP_TYPE_BUILDER: ShipType = 1;

pub struct Ship {
    pub ship_type: ShipType,

    pub block_size: UVec3,
    pub wave_size: UVec3,

    pub blocks: Vec<BlockIndex>,
    pub wave: Vec<Wave>,
    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,

    pub mesh: ShipMesh,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Wave {
    pub possible_pattern: Vec<Pattern>,
    pub all_possible_pattern: Vec<Pattern>,
}

impl Ship {
    pub fn new(
        block_size: UVec3,
        context: &Context,
        node_controller: &NodeController,
        ship_type: ShipType,
    ) -> Result<Ship> {
        let wave_size = block_size * 2;
        let max_block_index = (block_size.x * block_size.y * block_size.z) as usize;
        let max_wave_index = (wave_size.x * wave_size.y * wave_size.z) as usize;
        let mesh = ShipMesh::new(context, max_wave_index * 8)?;

        let mut ship = Ship {
            ship_type,
            block_size,
            wave_size,
            blocks: vec![BLOCK_INDEX_EMPTY; max_block_index],
            wave: vec![Wave::default(); max_wave_index],
            to_propergate: IndexQueue::default(),
            to_collapse: IndexQueue::default(),

            mesh,
        };

        //ship.place_block(uvec3(0, 0, 0), 1, node_controller)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    fn pos_in_bounds(pos: IVec3, size: UVec3) -> bool {
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

        log::info!("Place: {pos:?}");
        self.blocks[cell_index] = block_index;

        self.update_wave(pos, node_controller);

        while !self.to_collapse.is_empty() {
            self.to_collapse.pop_front();
        }

        Ok(())
    }

    pub fn get_wave_poses_of_block_pos(pos: IVec3) -> impl Iterator<Item = UVec3> {
        let p = pos * 2;
        [
            p + ivec3(0, 0, 0),
            p + ivec3(1, 0, 0),
            p + ivec3(0, 1, 0),
            p + ivec3(1, 1, 0),
            p + ivec3(0, 0, 1),
            p + ivec3(1, 0, 1),
            p + ivec3(0, 1, 1),
            p + ivec3(1, 1, 1),
        ]
        .into_iter()
        .map(|pos| pos.as_uvec3())
    }

    pub fn get_block_poses_of_wave_pos(
        &mut self,
        pos: IVec3,
    ) -> impl Iterator<Item = (usize, UVec3)> + '_ {
        let p = pos / 2;
        [
            p + ivec3(0, 0, 0),
            p + ivec3(-1, 0, 0),
            p + ivec3(0, -1, 0),
            p + ivec3(-1, -1, 0),
            p + ivec3(0, 0, -1),
            p + ivec3(-1, 0, -1),
            p + ivec3(0, -1, -1),
            p + ivec3(-1, -1, -1),
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

    fn update_wave(&mut self, block_pos: UVec3, node_controller: &NodeController) {
        for wave_pos in Self::get_wave_poses_of_block_pos(block_pos.as_ivec3()) {
            let wave_index = to_1d(wave_pos, self.wave_size) as usize;
            let config = (wave_pos % 2).cmpeq(UVec3::ZERO);

            let wave = Wave::new(config, node_controller);
            self.wave[wave_index] = wave;

            self.to_propergate.push_back(wave_index);
        }
    }

    pub fn tick(&mut self, actions_per_tick: usize) -> Result<bool> {
        if self.to_propergate.is_empty() && self.to_collapse.is_empty() {
            return Ok(false);
        }

        let mut full_tick = true;
        for _ in 0..actions_per_tick {
            if !self.to_propergate.is_empty() {
                let index = self.to_propergate.pop_front().unwrap();
                self.propergate(index);
            } else if !self.to_collapse.is_empty() {
                let index = self.to_collapse.pop_front().unwrap();
                self.collapse(index);
            } else {
                full_tick = false;
                break;
            }
        }

        self.mesh.update(self.wave_size, &self.wave)?;

        Ok(full_tick)
    }

    fn propergate(&mut self, wave_index: WaveIndex) {
        let pos = to_3d(wave_index as u32, self.wave_size);
        let wave = self.wave[wave_index].to_owned();

        let old_pattern = wave.possible_pattern.to_owned();
        let mut patterns = wave.all_possible_pattern.to_owned();
        for i in (0..patterns.len()).rev() {
            let pattern = patterns[i].to_owned();
            if pattern.block_req.is_empty() {
                continue;
            }

            for (offset, block_indecies) in pattern.block_req.iter() {
                let mut found = false;

                let req_pos = pos.as_ivec3() - *offset;
                let block_pos = req_pos / 2;
                if (req_pos % 2).cmpeq(IVec3::ONE).all()
                    && Self::pos_in_bounds(block_pos, self.block_size)
                {
                    let req_index = to_1d(block_pos.as_uvec3(), self.block_size);

                    for index in block_indecies.iter() {
                        if self.blocks[req_index] == *index {
                            found = true;
                            break;
                        }
                    }
                } else if block_indecies.contains(&BLOCK_INDEX_EMPTY) {
                    found = true;
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
            let pattern = patterns[i].to_owned();
            if pattern.node_req.is_empty() {
                continue;
            }

            for (offset, node_indecies) in pattern.node_req.iter() {
                let req_pos = pos.as_ivec3() + *offset;

                let mut found = false;
                if Self::pos_in_bounds(req_pos, self.wave_size) {
                    let req_index = to_1d(req_pos.as_uvec3(), self.wave_size);
                    for index in node_indecies.iter() {
                        if self.wave[req_index].possible_pattern.len() > 0
                            && self.wave[req_index].possible_pattern[0].node.index == *index
                        {
                            found = true;
                            break;
                        }
                    }
                }

                if !found {
                    patterns.remove(i);
                    break;
                }
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

                    /*
                    for pattern in wave.possible_pattern.iter() {
                        if pattern.id.index == NODE_INDEX_NONE {
                            continue;
                        }

                        t.push_str(&format!("{:?} ", pattern.id.index));
                    }
                    */

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

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        let max_wave_index = (self.wave_size.x * self.wave_size.y * self.wave_size.z) as usize;
        for i in 0..max_wave_index {
            self.wave[i].possible_pattern = node_controller.patterns.to_owned();
            self.wave[i].all_possible_pattern = node_controller.patterns.to_owned();
        }

        for x in 0..self.block_size.x {
            for y in 0..self.block_size.y {
                for z in 0..self.block_size.z {
                    let pos = uvec3(x, y, z);
                    self.update_wave(pos, node_controller);
                }
            }
        }

        Ok(())
    }

    pub fn clone_from(&mut self, other: &Ship) -> Result<()> {
        debug_assert!(self.block_size == other.block_size);

        self.blocks.clone_from(&other.blocks);
        self.wave.clone_from(&other.wave);
        self.to_propergate.clone_from(&other.to_propergate);
        self.to_collapse.clone_from(&other.to_collapse);

        self.mesh.update(self.wave_size, &self.wave)?;

        Ok(())
    }
}

impl Wave {
    pub fn new(config: BVec3, node_controller: &NodeController) -> Self {
        Self {
            possible_pattern: Vec::new(),
            all_possible_pattern: node_controller.patterns.to_owned(),
        }
    }
}
