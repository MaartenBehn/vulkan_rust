#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode};

use crate::math::{get_packed_index, to_3d_i};
use crate::node::{Node, NodeID, PatternIndex, EMPYT_PATTERN_INDEX};
use crate::ship_mesh::RenderNode;
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
use std::ops::Mul;
use std::time::Duration;

pub type ChunkIndex = usize;
pub type WaveIndex = usize;
pub type ShipType = u32;
pub const SHIP_TYPE_BASE: ShipType = 0;
pub const SHIP_TYPE_BUILD: ShipType = 1;

pub const CHUNK_BLOCK_SIZE: IVec3 = ivec3(8, 8, 8);
pub const CHUNK_WAVE_SIZE: IVec3 = ivec3(16, 16, 16);
pub const CHUNK_WAVE_SIZE_WITH_PADDING: IVec3 = ivec3(18, 18, 18);
pub const CHUNK_BLOCK_LEN: usize =
    (CHUNK_BLOCK_SIZE.x * CHUNK_BLOCK_SIZE.y * CHUNK_BLOCK_SIZE.z) as usize;
pub const CHUNK_WAVE_LEN: usize =
    (CHUNK_WAVE_SIZE.x * CHUNK_WAVE_SIZE.y * CHUNK_WAVE_SIZE.z) as usize;
pub const CHUNK_WAVE_WITH_PADDING_LEN: usize = (CHUNK_WAVE_SIZE_WITH_PADDING.x
    * CHUNK_WAVE_SIZE_WITH_PADDING.y
    * CHUNK_WAVE_SIZE_WITH_PADDING.z) as usize;

pub struct Ship {
    pub chunks: Vec<ShipChunk>,

    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,
}

pub struct ShipChunk {
    pub pos: IVec3,
    pub blocks: [BlockIndex; CHUNK_BLOCK_LEN],
    pub wave: [Wave; CHUNK_WAVE_LEN],
    pub render_nodes: [u32; CHUNK_WAVE_LEN],
    pub render_nodes_with_padding: [RenderNode; CHUNK_WAVE_WITH_PADDING_LEN],
}

#[derive(Clone, Debug, Default)]
pub struct Wave {
    pub possible_patterns: Vec<PatternIndex>,
    pub dependent_waves: IndexQueue,
}

impl Ship {
    pub fn new() -> Result<Ship> {
        let chunk = ShipChunk::new(IVec3::ZERO);

        let mut ship = Ship {
            chunks: vec![chunk],
            to_propergate: IndexQueue::default(),
            to_collapse: IndexQueue::default(),
        };

        /*
        let size = size_of::<Ship>()
            + size_of::<PatternIndex>() * max_block_index
            + size_of::<Wave>() * max_wave_index;
        log::info!("Ship size {:?} MB", size as f32 / 1000000.0);
         */

        //ship.place_block(uvec3(0, 0, 0), 1, node_controller)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    pub fn has_chunk(&self, chunk_pos: IVec3) -> bool {
        chunk_pos == IVec3::ZERO
    }

    pub fn get_chunk_index(&self, chunk_pos: IVec3) -> Result<usize> {
        if !self.has_chunk(chunk_pos) {
            bail!("Chunk not found!");
        }

        Ok(0)
    }

    pub fn get_wave_index(&self, chunk_pos: IVec3, in_chunk_pos: IVec3) -> Result<usize> {
        let chunk_index = self.get_chunk_index(chunk_pos)?;
        let in_chunk_index = to_1d_i(in_chunk_pos, CHUNK_WAVE_SIZE) as usize;
        Ok(in_chunk_index + CHUNK_WAVE_LEN * chunk_index)
    }

    pub fn get_wave_index_from_wave_pos(&self, wave_pos: IVec3) -> Result<usize> {
        let chunk_pos = get_chunk_pos_of_wave_pos(wave_pos);
        let in_chunk_pos = get_in_chunk_pos_of_wave_pos(wave_pos);
        self.get_wave_index(chunk_pos, in_chunk_pos)
    }

    pub fn place_block(
        &mut self,
        block_pos: IVec3,
        block_index: BlockIndex,
        node_controller: &NodeController,
    ) -> Result<()> {
        let chunk_pos = get_chunk_pos_of_block_pos(block_pos);
        let in_chunk_pos = get_in_chunk_pos_of_wave_pos(block_pos);
        let chunk_index = self.get_chunk_index(chunk_pos)?;
        let in_chunk_index = to_1d_i(in_chunk_pos, CHUNK_BLOCK_SIZE) as usize;

        let chunk = &mut self.chunks[chunk_index];

        if chunk.blocks[in_chunk_index] == block_index {
            return Ok(());
        }

        log::info!("Place: {block_pos:?}");
        chunk.blocks[in_chunk_index] = block_index;
        self.update_wave(block_pos, node_controller)?;

        while !self.to_collapse.is_empty() {
            self.to_collapse.pop_front();
        }

        Ok(())
    }

    fn update_wave(&mut self, block_pos: IVec3, node_controller: &NodeController) -> Result<()> {
        for &pos in node_controller.affected_poses.iter() {
            let req_pos = get_wave_pos_of_block_pos(block_pos) + pos;
            let wave_index = self.get_wave_index_from_wave_pos(req_pos);

            if wave_index.is_err() {
                continue;
            }

            self.to_propergate.push_back(wave_index.unwrap());
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        node_controller: &NodeController,
        debug_controller: &mut DebugController,
    ) -> Result<(bool, Vec<ChunkIndex>, bool)> {
        let mut full = true;
        let mut last_wave_something = false;
        let mut changed = false;

        for _ in 0..actions_per_tick {
            if !self.to_propergate.is_empty() {
                let wave_index = self.to_propergate.pop_front().unwrap();
                self.propergate(wave_index, node_controller, debug_controller)?;

                if debug_controller.mode == DebugMode::WFC {
                    let chunk_index = wave_index / CHUNK_WAVE_LEN;
                    let in_chunk_wave_index = wave_index % CHUNK_WAVE_LEN;
                    last_wave_something =
                        self.chunks[chunk_index].render_nodes[in_chunk_wave_index] != 0;
                }
                continue;
            }

            if !self.to_collapse.is_empty() {
                let wave_index = self.to_collapse.pop_front().unwrap();
                self.collapse(wave_index, node_controller, debug_controller)?;
                changed = true;

                if debug_controller.mode == DebugMode::WFC {
                    let chunk_index = wave_index / CHUNK_WAVE_LEN;
                    let in_chunk_wave_index = wave_index % CHUNK_WAVE_LEN;
                    last_wave_something =
                        self.chunks[chunk_index].render_nodes[in_chunk_wave_index] != 0;
                }
                continue;
            }

            full = false;
            break;
        }

        if debug_controller.mode == DebugMode::WFC {
            self.debug_show_wave(debug_controller);
        }

        let changed_chunks = if changed { vec![0] } else { Vec::new() };
        Ok((full, changed_chunks, last_wave_something))
    }

    #[cfg(not(debug_assertions))]
    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        node_controller: &NodeController,
    ) -> Result<(bool, Vec<WaveIndex>)> {
        let mut full = true;
        let mut changed_waves = Vec::new();
        for _ in 0..actions_per_tick {
            if !self.to_propergate.is_empty() {
                let wave_index = self.to_propergate.pop_front().unwrap();
                self.propergate(wave_index, node_controller)?;

                continue;
            }

            if !self.to_collapse.is_empty() {
                let wave_index = self.to_collapse.pop_front().unwrap();
                self.collapse(wave_index, node_controller)?;
                changed_waves.push(wave_index);

                continue;
            }

            full = false;
            break;
        }

        Ok((full, changed_waves))
    }

    fn propergate(
        &mut self,
        wave_index: WaveIndex,
        node_controller: &NodeController,
        #[cfg(debug_assertions)] debug_controller: &mut DebugController,
    ) -> Result<()> {
        let chunk_index = wave_index / CHUNK_WAVE_LEN;
        let in_chunk_wave_index = wave_index % CHUNK_WAVE_LEN;

        let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, CHUNK_WAVE_SIZE);
        let chunk_pos = self.chunks[chunk_index].pos;
        let wave_pos = in_chunk_wave_pos + chunk_pos * CHUNK_WAVE_SIZE;

        let config = get_config(wave_pos);

        #[cfg(debug_assertions)]
        if debug_controller.mode == DebugMode::WFC {
            debug_controller.add_cube(
                wave_pos.as_vec3(),
                (wave_pos + IVec3::ONE).as_vec3(),
                vec4(0.0, 0.0, 1.0, 1.0),
            );
        }

        self.chunks[chunk_index].wave[in_chunk_wave_index]
            .possible_patterns
            .clear();
        for (pattern_index, pattern) in node_controller.patterns[config].iter().enumerate() {
            let accepted = pattern.block_req.iter().all(|(&offset, indecies)| {
                let req_wave_pos = wave_pos + offset;
                let req_chunk_pos = get_chunk_pos_of_wave_pos(req_wave_pos);
                let req_chunk_index = self.get_chunk_index(req_chunk_pos);

                if req_chunk_index.is_err() {
                    return indecies.contains(&BLOCK_INDEX_EMPTY);
                }
                debug_assert!((req_wave_pos % 2) == IVec3::ONE);

                let req_block_pos = get_block_pos_of_wave_pos(req_wave_pos);
                let req_in_chunk_pos = get_in_chunk_pos_of_block_pos(req_block_pos);
                let index = to_1d_i(req_in_chunk_pos, CHUNK_BLOCK_SIZE) as usize;
                let block_index = self.chunks[req_chunk_index.unwrap()].blocks[index];
                indecies.contains(&block_index)
            });

            if accepted {
                for (&offset, _) in pattern.node_req.iter() {
                    let req_wave_pos = wave_pos + offset;
                    let req_wave_index = self.get_wave_index_from_wave_pos(req_wave_pos).unwrap();

                    if !self.to_collapse.contains(req_wave_index) {
                        self.to_propergate.push_back(req_wave_index);
                    }
                }

                self.chunks[chunk_index].wave[in_chunk_wave_index]
                    .possible_patterns
                    .push(pattern_index);
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
        let chunk_index = wave_index / CHUNK_WAVE_LEN;
        let in_chunk_wave_index = wave_index % CHUNK_WAVE_LEN;

        let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, CHUNK_WAVE_SIZE);
        let chunk_pos = self.chunks[chunk_index].pos;
        let wave_pos = in_chunk_wave_pos + chunk_pos * CHUNK_WAVE_SIZE;

        let config = get_config(wave_pos);

        #[cfg(debug_assertions)]
        if debug_controller.mode == DebugMode::WFC {
            debug_controller.add_cube(
                wave_pos.as_vec3(),
                (wave_pos + IVec3::ONE).as_vec3(),
                vec4(0.0, 1.0, 0.0, 1.0),
            );
        }

        let old_possible_pattern_size = self.chunks[chunk_index].wave[in_chunk_wave_index]
            .possible_patterns
            .len();
        for (i, pattern_index) in self.chunks[chunk_index].wave[in_chunk_wave_index]
            .possible_patterns
            .to_owned()
            .into_iter()
            .enumerate()
            .rev()
        {
            let pattern = &node_controller.patterns[config][pattern_index];

            let accepted = pattern.node_req.iter().all(|(&offset, indecies)| {
                let req_wave_pos = wave_pos + offset;
                let req_chunk_pos = get_chunk_pos_of_wave_pos(req_wave_pos);
                let req_in_chunk_pos = get_in_chunk_pos_of_block_pos(req_wave_pos);
                let req_chunk_index = self.get_chunk_index(req_chunk_pos);

                if req_chunk_index.is_err() {
                    return false;
                }

                let req_config = get_config(req_wave_pos);
                let req_wave_index = to_1d_i(req_in_chunk_pos, CHUNK_WAVE_SIZE) as usize;

                let found = self.chunks[req_chunk_index.unwrap()].wave[req_wave_index]
                    .possible_patterns
                    .iter()
                    .all(|&possible_pattern_index| {
                        let possible_pattern =
                            &node_controller.patterns[req_config][possible_pattern_index];
                        indecies.contains(&possible_pattern.node.index)
                    });

                found
            });

            if !accepted {
                self.chunks[chunk_index].wave[in_chunk_wave_index]
                    .possible_patterns
                    .remove(i);
            }
        }

        let chunk = &mut self.chunks[chunk_index];

        let possible_patterns = &chunk.wave[in_chunk_wave_index].possible_patterns;
        let possible_patterns_changed = possible_patterns.len() != old_possible_pattern_size;
        let new_render_pattern = possible_patterns[possible_patterns.len() - 1];

        let pattern = &node_controller.patterns[config][new_render_pattern];

        let node_index = to_1d_i(in_chunk_wave_pos, CHUNK_WAVE_SIZE) as usize;
        chunk.render_nodes[node_index] = pattern.node.into();

        let node_index_with_padding =
            to_1d_i(in_chunk_wave_pos + IVec3::ONE, CHUNK_WAVE_SIZE_WITH_PADDING) as usize;
        chunk.render_nodes_with_padding[node_index_with_padding] =
            RenderNode(pattern.node.is_some());

        if possible_patterns_changed {
            while !chunk.wave[in_chunk_wave_index].dependent_waves.is_empty() {
                let index = chunk.wave[in_chunk_wave_index]
                    .dependent_waves
                    .pop_front()
                    .unwrap();
                self.to_collapse.push_back(index);
            }
        }

        for (&offset, _) in pattern.node_req.iter() {
            let req_pos = wave_pos + offset;
            let req_chunk_pos = get_chunk_pos_of_wave_pos(req_pos);
            let req_in_chunk_pos = get_in_chunk_pos_of_wave_pos(req_pos);

            let req_chunk_index = self.get_chunk_index(req_chunk_pos).unwrap();
            let req_index = to_1d_i(req_in_chunk_pos, CHUNK_WAVE_SIZE) as usize;
            let req_wave_index = self
                .get_wave_index(req_chunk_pos, req_in_chunk_pos)
                .unwrap();

            self.chunks[req_chunk_index].wave[req_index]
                .dependent_waves
                .push_back(req_wave_index);

            if possible_patterns_changed {
                self.to_collapse.push_back(req_wave_index);
            }
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn debug_show_wave(&mut self, debug_controller: &mut DebugController) {
        let mut to_propergate = self.to_propergate.to_owned();
        while !to_propergate.is_empty() {
            let wave_index = to_propergate.pop_front().unwrap();

            let chunk_index = wave_index / CHUNK_WAVE_LEN;
            let in_chunk_wave_index = wave_index % CHUNK_WAVE_LEN;

            let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, CHUNK_WAVE_SIZE);
            let chunk_pos = self.chunks[chunk_index].pos;
            let wave_pos = in_chunk_wave_pos + chunk_pos * CHUNK_WAVE_SIZE;

            let wave = &self.chunks[chunk_index].wave[in_chunk_wave_index];
            let lines = iter::once("p".to_owned())
                .chain(wave.possible_patterns.iter().map(|p| p.to_string()))
                .collect();
            debug_controller.add_text(lines, wave_pos.as_vec3());
        }

        let mut to_collpase = self.to_collapse.to_owned();
        while !to_collpase.is_empty() {
            let wave_index = to_collpase.pop_front().unwrap();
            let chunk_index = wave_index / CHUNK_WAVE_LEN;
            let in_chunk_wave_index = wave_index % CHUNK_WAVE_LEN;

            let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, CHUNK_WAVE_SIZE);
            let chunk_pos = self.chunks[chunk_index].pos;
            let wave_pos = in_chunk_wave_pos + chunk_pos * CHUNK_WAVE_SIZE;

            let wave = &self.chunks[chunk_index].wave[in_chunk_wave_index];
            let lines = iter::once("c".to_owned())
                .chain(wave.possible_patterns.iter().map(|p| p.to_string()))
                .collect();
            debug_controller.add_text(lines, wave_pos.as_vec3());
        }
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        for chunk_index in 0..self.chunks.len() {
            self.chunks[chunk_index].wave = std::array::from_fn(|_| Wave::default());

            let chunk_pos = self.chunks[chunk_index].pos * CHUNK_BLOCK_SIZE;
            for x in 0..CHUNK_BLOCK_SIZE.x {
                for y in 0..CHUNK_BLOCK_SIZE.y {
                    for z in 0..CHUNK_BLOCK_SIZE.z {
                        let pos = ivec3(x, y, z) + chunk_pos;
                        self.update_wave(pos, node_controller)?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn pos_in_bounds(pos: IVec3, size: IVec3) -> bool {
    pos.cmpge(IVec3::ZERO).all() && pos.cmplt(size).all()
}

pub fn get_chunk_pos_of_block_pos(pos: IVec3) -> IVec3 {
    (pos / CHUNK_BLOCK_SIZE.x) - ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
}

pub fn get_in_chunk_pos_of_block_pos(pos: IVec3) -> IVec3 {
    pos % CHUNK_BLOCK_SIZE.x
}

pub fn get_chunk_pos_of_wave_pos(pos: IVec3) -> IVec3 {
    (pos / CHUNK_WAVE_SIZE.x) - ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
}

pub fn get_in_chunk_pos_of_wave_pos(pos: IVec3) -> IVec3 {
    pos % CHUNK_WAVE_SIZE.x
}

pub fn get_wave_pos_of_block_pos(pos: IVec3) -> IVec3 {
    pos * 2 - IVec3::ONE
}

pub fn get_block_pos_of_wave_pos(pos: IVec3) -> IVec3 {
    pos / 2
}

pub fn get_config(pos: IVec3) -> usize {
    let c = (pos % 2).abs();
    (c.x + (c.y << 1) + (c.z << 2)) as usize
}

impl ShipChunk {
    pub fn new(pos: IVec3) -> ShipChunk {
        ShipChunk {
            pos,
            blocks: [BLOCK_INDEX_EMPTY; CHUNK_BLOCK_LEN],
            wave: std::array::from_fn(|_| Wave::default()),
            render_nodes: [0; CHUNK_WAVE_LEN],
            render_nodes_with_padding: [RenderNode(false); CHUNK_WAVE_WITH_PADDING_LEN],
        }
    }
}
