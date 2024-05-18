#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode};

use crate::math::{get_neigbor_offsets, get_packed_index, to_3d_i};
use crate::node::{Node, NodeID, PatternIndex, EMPYT_PATTERN_INDEX};
use crate::ship_mesh::RenderNode;
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::{BlockIndex, NodeController, Pattern, BLOCK_INDEX_EMPTY},
    ship_mesh::ShipMesh,
};
use index_queue::IndexQueue;
use octa_force::log::debug;
use octa_force::{anyhow::*, glam::*, log, vulkan::Context};
use std::collections::{HashMap, VecDeque};
use std::iter;
use std::mem::size_of;
use std::ops::Mul;
use std::time::Duration;

pub type ChunkIndex = usize;
pub type WaveIndex = usize;

pub struct Ship<
    const BS: i32,   // Bock size
    const WS: i32,   // Wave size
    const PS: u32,   // Wave size + Padding
    const BL: usize, // Bock array len
    const WL: usize, // Wave array len
    const PL: usize, // Wave with Padding array len
> {
    pub chunks: Vec<ShipChunk<BS, WS, PS, BL, WL, PL>>,

    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,
}

pub struct ShipChunk<
    const BS: i32,   // Bock size
    const WS: i32,   // Wave size
    const PS: u32,   // Wave size + Padding
    const BL: usize, // Bock array len
    const WL: usize, // Wave array len
    const PL: usize, // Wave with Padding array len
> {
    pub pos: IVec3,
    pub blocks: [BlockIndex; BL],
    pub wave: [Wave; WL],
    pub node_id_bits: [u32; WL],
    pub node_voxels: [RenderNode; PL],
}

#[derive(Clone, Debug, Default)]
pub struct Wave {
    pub possible_patterns: Vec<PatternIndex>,
    pub dependent_waves: IndexQueue,
}

impl<
        const BS: i32,
        const WS: i32,
        const PS: u32,
        const BL: usize,
        const WL: usize,
        const PL: usize,
    > Ship<BS, WS, PS, BL, WL, PL>
{
    pub fn new() -> Result<Ship<BS, WS, PS, BL, WL, PL>> {
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
        let in_chunk_index = to_1d_i(in_chunk_pos, IVec3::ONE * WS) as usize;
        Ok(in_chunk_index + WL * chunk_index)
    }

    pub fn get_wave_index_from_wave_pos(&self, wave_pos: IVec3) -> Result<usize> {
        let chunk_pos = self.get_chunk_pos_of_wave_pos(wave_pos);
        let in_chunk_pos = self.get_in_chunk_pos_of_wave_pos(wave_pos);
        self.get_wave_index(chunk_pos, in_chunk_pos)
    }

    pub fn place_block(
        &mut self,
        block_pos: IVec3,
        block_index: BlockIndex,
        node_controller: &NodeController,
    ) -> Result<()> {
        let chunk_pos = self.get_chunk_pos_of_block_pos(block_pos);
        let in_chunk_pos = self.get_in_chunk_pos_of_wave_pos(block_pos);
        let chunk_index = self.get_chunk_index(chunk_pos)?;
        let in_chunk_index = to_1d_i(in_chunk_pos, IVec3::ONE * BS) as usize;

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

                if debug_controller.mode == DebugMode::WFCSkip {
                    let chunk_index = wave_index / WL;
                    let in_chunk_wave_index = wave_index % WL;
                    last_wave_something =
                        self.chunks[chunk_index].node_id_bits[in_chunk_wave_index] != 0;
                }
                continue;
            }

            if !self.to_collapse.is_empty() {
                let wave_index = self.to_collapse.pop_front().unwrap();
                self.collapse(wave_index, node_controller, debug_controller)?;
                changed = true;

                if debug_controller.mode == DebugMode::WFCSkip {
                    let chunk_index = wave_index / WL;
                    let in_chunk_wave_index = wave_index % WL;
                    last_wave_something =
                        self.chunks[chunk_index].node_id_bits[in_chunk_wave_index] != 0;
                }
                continue;
            }

            full = false;
            break;
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
        let chunk_index = wave_index / WL;
        let in_chunk_wave_index = wave_index % WL;

        let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, IVec3::ONE * WS);
        let chunk_pos = self.chunks[chunk_index].pos;
        let wave_pos = in_chunk_wave_pos + chunk_pos * WS;

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
                let req_chunk_pos = self.get_chunk_pos_of_wave_pos(req_wave_pos);
                let req_chunk_index = self.get_chunk_index(req_chunk_pos);

                if req_chunk_index.is_err() {
                    return indecies.contains(&BLOCK_INDEX_EMPTY);
                }
                debug_assert!((req_wave_pos % 2) == IVec3::ONE);

                let req_block_pos = get_block_pos_of_wave_pos(req_wave_pos);
                let req_in_chunk_pos = self.get_in_chunk_pos_of_block_pos(req_block_pos);
                let index = to_1d_i(req_in_chunk_pos, IVec3::ONE * BS) as usize;
                let block_index = self.chunks[req_chunk_index.unwrap()].blocks[index];
                indecies.contains(&block_index)
            });

            if accepted {
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
        let chunk_index = wave_index / WL;
        let in_chunk_wave_index = wave_index % WL;

        let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, IVec3::ONE * WS);
        let chunk_pos = self.chunks[chunk_index].pos;
        let wave_pos = in_chunk_wave_pos + chunk_pos * WS;

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

            let accepted = true; /*= get_neigbor_offsets().iter().all(|&offset| {
                                     let req_wave_pos = wave_pos + offset;
                                     let req_chunk_pos = self.get_chunk_pos_of_wave_pos(req_wave_pos);
                                     let req_in_chunk_pos = self.get_in_chunk_pos_of_wave_pos(req_wave_pos);
                                     let req_chunk_index = self.get_chunk_index(req_chunk_pos);

                                     if req_chunk_index.is_err() {
                                         return false;
                                     }

                                     let req_config = get_config(req_wave_pos);
                                     let req_wave_index = to_1d_i(req_in_chunk_pos, IVec3::ONE * WS) as usize;

                                     let possible_pattern_index = self.chunks[req_chunk_index.unwrap()].wave
                                         [req_wave_index]
                                         .possible_patterns
                                         .last();

                                     if possible_pattern_index.is_none() {
                                         return false;
                                     }

                                     true
                                 }); */

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

        let node_index = to_1d_i(in_chunk_wave_pos, IVec3::ONE * WS) as usize;
        chunk.node_id_bits[node_index] = pattern.node.into();

        let node_index_with_padding =
            to_1d_i(in_chunk_wave_pos + IVec3::ONE, IVec3::ONE * PS as i32) as usize;
        chunk.node_voxels[node_index_with_padding] = RenderNode(pattern.node.is_some());

        if possible_patterns_changed {
            while !chunk.wave[in_chunk_wave_index].dependent_waves.is_empty() {
                let index = chunk.wave[in_chunk_wave_index]
                    .dependent_waves
                    .pop_front()
                    .unwrap();
                self.to_collapse.push_back(index);
            }
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn debug_show_wave(&self, debug_controller: &mut DebugController) {
        for chunk in self.chunks.iter() {
            debug_controller.add_cube(
                (chunk.pos * WS).as_vec3(),
                ((chunk.pos + IVec3::ONE) * WS).as_vec3(),
                vec4(1.0, 0.0, 0.0, 1.0),
            );
        }

        let mut to_propergate = self.to_propergate.to_owned();

        let mut i = 0;
        while !to_propergate.is_empty() {
            let wave_index = to_propergate.pop_front().unwrap();

            let chunk_index = wave_index / WL;
            let in_chunk_wave_index = wave_index % WL;

            let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, IVec3::ONE * WS);
            let chunk_pos = self.chunks[chunk_index].pos;
            let wave_pos = in_chunk_wave_pos + chunk_pos * WS;

            let wave = &self.chunks[chunk_index].wave[in_chunk_wave_index];
            let lines =
                iter::once(format!("p {} {} {}", wave_pos.x, wave_pos.y, wave_pos.z).to_owned())
                    .chain(wave.possible_patterns.iter().map(|p| p.to_string()))
                    .collect();
            debug_controller.add_text(lines, wave_pos.as_vec3() + vec3(0.0, 1.0, 0.0));

            if i == 0 {
                debug_controller.add_cube(
                    wave_pos.as_vec3(),
                    wave_pos.as_vec3() + Vec3::ONE,
                    vec4(0.0, 0.0, 1.0, 1.0),
                );
            }

            i += 1;
        }

        let mut to_collpase = self.to_collapse.to_owned();

        let mut i = 0;
        while !to_collpase.is_empty() {
            let wave_index = to_collpase.pop_front().unwrap();
            let chunk_index = wave_index / WL;
            let in_chunk_wave_index = wave_index % WL;

            let in_chunk_wave_pos = to_3d_i(in_chunk_wave_index as i32, IVec3::ONE * WS);
            let chunk_pos = self.chunks[chunk_index].pos;
            let wave_pos = in_chunk_wave_pos + chunk_pos * WS;

            let wave = &self.chunks[chunk_index].wave[in_chunk_wave_index];
            let lines =
                iter::once(format!("c {} {} {}", wave_pos.x, wave_pos.y, wave_pos.z).to_owned())
                    .chain(wave.possible_patterns.iter().map(|p| p.to_string()))
                    .collect();
            debug_controller.add_text(lines, wave_pos.as_vec3() + vec3(0.0, 1.0, 0.0));

            if i == 0 {
                debug_controller.add_cube(
                    wave_pos.as_vec3(),
                    wave_pos.as_vec3() + Vec3::ONE,
                    vec4(0.0, 1.0, 0.0, 1.0),
                );
            }

            i += 1;
        }
    }

    pub fn on_node_controller_change(&mut self, node_controller: &NodeController) -> Result<()> {
        for chunk_index in 0..self.chunks.len() {
            self.chunks[chunk_index].wave = std::array::from_fn(|_| Wave::default());

            let chunk_pos = self.chunks[chunk_index].pos * BS;
            for x in 0..BS as i32 {
                for y in 0..BS as i32 {
                    for z in 0..BS as i32 {
                        let pos = ivec3(x, y, z) + chunk_pos;
                        self.update_wave(pos, node_controller)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_chunk_pos_of_block_pos(&self, pos: IVec3) -> IVec3 {
        (pos / BS) - ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
    }

    pub fn get_in_chunk_pos_of_block_pos(&self, pos: IVec3) -> IVec3 {
        pos % BS
    }

    pub fn get_chunk_pos_of_wave_pos(&self, pos: IVec3) -> IVec3 {
        (pos / WS) - ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
    }

    pub fn get_in_chunk_pos_of_wave_pos(&self, pos: IVec3) -> IVec3 {
        pos % WS
    }
}

pub fn pos_in_bounds(pos: IVec3, size: IVec3) -> bool {
    pos.cmpge(IVec3::ZERO).all() && pos.cmplt(size).all()
}

pub fn get_wave_pos_of_block_pos(pos: IVec3) -> IVec3 {
    pos * 2
}

pub fn get_block_pos_of_wave_pos(pos: IVec3) -> IVec3 {
    pos / 2
}

pub fn get_config(pos: IVec3) -> usize {
    let c = (pos % 2).abs();
    (c.x + (c.y << 1) + (c.z << 2)) as usize
}

impl<
        const BS: i32,
        const WS: i32,
        const PS: u32,
        const BL: usize,
        const WL: usize,
        const PL: usize,
    > ShipChunk<BS, WS, PS, BL, WL, PL>
{
    pub fn new(pos: IVec3) -> ShipChunk<BS, WS, PS, BL, WL, PL> {
        ShipChunk {
            pos,
            blocks: [BLOCK_INDEX_EMPTY; BL],
            wave: std::array::from_fn(|_| Wave::default()),
            node_id_bits: [0; WL],
            node_voxels: [RenderNode(false); PL],
        }
    }
}
