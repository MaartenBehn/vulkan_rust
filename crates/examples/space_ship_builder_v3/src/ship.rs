use crate::node::{NodeID, PatternIndex};
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
    pub tasks: VecDeque<UVec3>,
    pub task_counter: usize,

    pub mesh: ShipMesh,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Wave {
    pub current_pattern: PatternIndex,
    pub req_state: HashMap<IVec3, Vec<bool>>,
    pub pattern_state: Vec<HashMap<IVec3, bool>>,
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
            wave: vec![Wave::new(node_controller); max_wave_index],
            tasks: VecDeque::new(),
            task_counter: 0,

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

                    let mut changed = false;
                    for (i, (&pattern_index, state)) in pattern_indecies
                        .iter()
                        .zip(self.wave[index].req_state[&offset].to_owned().into_iter())
                        .enumerate()
                    {
                        let pattern = &node_controller.patterns[pattern_index];

                        if !pattern.block_req.is_empty() {
                            let req_block_indecies = pattern.block_req.get(&offset).unwrap();

                            let found = req_block_indecies.contains(&block_index);
                            self.wave[index].req_state.get_mut(&offset).unwrap()[i] = found;
                            self.wave[index].pattern_state[pattern_index]
                                .insert(offset.to_owned(), found);
                        }

                        if !changed
                            && self.wave[index].pattern_state[pattern_index]
                                .iter()
                                .all(|(_, &s)| s)
                        {
                            self.wave[index].current_pattern = pattern_index;
                            changed = true;
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

        self.mesh
            .update(self.wave_size, &self.wave, node_controller)?;

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

    pub fn clone_from(&mut self, other: &Ship, node_controller: &NodeController) -> Result<()> {
        debug_assert!(self.block_size == other.block_size);

        self.blocks.clone_from(&other.blocks);
        self.wave.clone_from(&other.wave);
        self.tasks = VecDeque::new();
        self.task_counter = 0;

        self.mesh
            .update(self.wave_size, &self.wave, node_controller)?;

        Ok(())
    }
}

impl Wave {
    pub fn new(node_controller: &NodeController) -> Self {
        let req_state = node_controller
            .req_poses
            .iter()
            .map(|(pos, patterns)| (pos.to_owned(), patterns.iter().map(|_| false).collect()))
            .collect();

        let pattern_state = node_controller
            .patterns
            .iter()
            .map(|p| {
                p.block_req
                    .iter()
                    .map(|(pos, _)| (pos.to_owned(), false))
                    .collect()
            })
            .collect();

        Self {
            current_pattern: 0,
            req_state,
            pattern_state,
        }
    }
}
