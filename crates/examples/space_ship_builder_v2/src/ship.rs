use std::collections::VecDeque;
use std::time::Duration;

use app::anyhow::*;
use app::glam::*;
use app::log;
use app::vulkan::Context;

use crate::math::to_1d;
use crate::math::to_1d_i;
use crate::node;
use crate::node::BlockIndex;
use crate::node::NodeController;
use crate::node::NodeID;
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
    pub nodes: Vec<NodeID>,
    pub to_propergate: VecDeque<WaveIndex>,

    pub actions_per_tick: usize,

    pub mesh: ShipMesh,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Wave {
    possible_pattern: Vec<Pattern>,
    all_possible_pattern: Vec<Pattern>,
}

impl Ship {
    pub fn new(context: &Context, node_controller: &NodeController) -> Result<Ship> {
        let block_size = uvec3(10, 10, 10);
        let wave_size = block_size + uvec3(1, 1, 1);

        let max_block_index = (wave_size.x * wave_size.y * wave_size.z + 1) as usize;
        let max_wave_index = (wave_size.x * wave_size.y * wave_size.z + 1) as usize;
        let mesh = ShipMesh::new(context, max_wave_index)?;

        let mut ship = Ship {
            block_size,
            wave_size,
            blocks: vec![BLOCK_INDEX_NONE; max_block_index],
            wave:  vec![Wave::new(node_controller);max_wave_index],
            nodes: vec![NodeID::default(); max_wave_index],
            to_propergate: VecDeque::new(),
            actions_per_tick: 4,

            mesh,
        };

        ship.place_block(uvec3(5, 5, 5), 0, node_controller)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    fn block_pos_in_bounds(&self, pos: IVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(self.block_size.as_ivec3()).all()
    }

    pub fn get_block_u(&self, pos: UVec3) -> Result<BlockIndex> {
        self.get_block_i(pos.as_ivec3())
    }

    pub fn get_block_i(&self, pos: IVec3) -> Result<BlockIndex> {
        if !self.block_pos_in_bounds(pos) {
            bail!("Pos not in ship")
        }

        let index = to_1d_i(pos, self.block_size.as_ivec3());
        Ok(self.blocks[index as usize])
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
        log::info!("Place: {pos:?}");

        let cell_index = to_1d(pos, self.block_size);
        self.blocks[cell_index] = block_index;

        self.update_nodes(pos, node_controller);

        self.mesh
            .update(self.wave_size, &self.blocks, &self.nodes, node_controller)?;

        Ok(())
    }

    pub fn get_neigbors(pos: IVec3) -> [IVec3; 8] {
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
    }

    fn update_nodes(&mut self, pos: UVec3, node_controller: &NodeController) {
        let neigbors = Self::get_neigbors(pos.as_ivec3());

        for neigbor in neigbors {
            let node_index = to_1d_i(neigbor, self.wave_size.as_ivec3()) as usize;

            let config = self.get_node_config(neigbor);
            let index: usize = config.into();
            
            self.wave[node_index].all_possible_pattern = node_controller.pattern[index];
            self.wave[node_index].possible_pattern = node_controller.pattern[index];

            self.to_propergate.push_back(node_index);
        }
    }

    fn get_node_config(&mut self, node_pos: IVec3) -> Config {
        let blocks = Self::get_neigbors(node_pos - ivec3(1, 1, 1));

        let mut config = [false; 8];
        for (i, block) in blocks.iter().enumerate() {
            if block.is_negative_bitmask() != 0 || block.cmpge(self.block_size.as_ivec3()).any() {
                continue;
            }

            let block = self.get_block_i(*block).unwrap();
            config[i] = block != BLOCK_INDEX_NONE;
        }
        config.into()
    }

    pub fn tick(&mut self, deltatime: Duration) {
        if self.to_propergate.is_empty() {
            return;
        }

        if deltatime < MIN_TICK_LENGTH && self.actions_per_tick < usize::MAX / 2 {
            self.actions_per_tick *= 2;
        } else if deltatime > MAX_TICK_LENGTH && self.actions_per_tick > 4 {
            self.actions_per_tick /= 2;
        }


    }

    fn propergate(&mut self, wave_index: WaveIndex) {
        let mut wave = self.wave[wave_index];

        for (i, pattern) in wave.possible_pattern.iter().rev().enumerate() {
            if pattern.req.is_empty() {
                continue;
            }

            for (pos, node_id) in pattern.req.iter() {
                let req_index = to_1d(*pos, self.wave_size);

                let mut found = false;
                for test_pattern in self.wave[req_index].possible_pattern.iter() {
                    if test_pattern.id == *node_id {
                        found = true;
                        break;
                    }
                }

                if !found {
                    wave.possible_pattern.remove(i);
                    break;
                }
            }
        }
        self.wave[wave_index] = wave;

    }

   
}


impl Wave {
    pub fn new(node_controller: &NodeController) -> Self {
        Self { 
            possible_pattern: node_controller.pattern[0],
            all_possible_pattern: node_controller.pattern[0],
        }
    }
}