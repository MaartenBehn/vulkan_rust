use crate::math::{all_sides_dirs, get_all_poses, get_neighbors, oct_positions, to_1d};
use crate::node::{NodeID, NODE_VOXEL_LENGTH, VOXEL_EMPTY};
use crate::rotation::Rot;
use crate::rules::block::Block;
use crate::rules::empty::EMPTY_BLOCK_NAME_INDEX;
use crate::rules::solver::{Solver, SolverCacheIndex};
use crate::rules::Prio::{
    HULL_BASE0, HULL_BASE1, HULL_BASE2, HULL_BASE3, HULL_BASE4, HULL_BASE5, HULL_BASE6, HULL_BASE7,
    HULL_BASE8, HULL_FILL0, HULL_FILL1,
};
use crate::rules::{Prio, Rules};
use crate::ship::data::{CacheIndex, ShipData};
use crate::voxel_loader::VoxelLoader;
use log::{debug, error, warn};
use octa_force::anyhow::bail;
use octa_force::glam::{uvec3, UVec3};
use octa_force::{
    anyhow::Result,
    glam::{ivec3, BVec3, IVec3, Mat3, Mat4},
};
use std::ops::Deref;

const HULL_CACHE_NONE: CacheIndex = CacheIndex::MAX;
const HULL_BLOCK_NAME: &str = "Hull";
const HULL_BASE_NAME_PART: &str = "Hull-Base";
const HULL_MULTI_NAME_PART: &str = "Hull-Multi";
const HULL_MULTI_BLOCK: &str = "Block";
const HULL_MULTI_FOLDER: &str = "Req-Folder";
const HULL_MULTI_MULTI: &str = "Req-Multi";

pub struct HullSolver {
    pub block_name_index: usize,
    pub basic_blocks: Vec<(Vec<IVec3>, Block, Prio)>,
    pub multi_blocks: Vec<(Vec<(IVec3, Vec<Block>)>, Block, Prio)>,

    #[cfg(debug_assertions)]
    pub debug_basic_blocks: Vec<(Vec<IVec3>, Block, Prio)>,

    #[cfg(debug_assertions)]
    pub debug_multi_blocks: Vec<(Vec<(IVec3, Vec<Block>)>, Block, Prio)>,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        debug!("Making Hull");

        let hull_block_name_index = self.block_names.len();
        self.block_names.push(HULL_BLOCK_NAME.to_owned());

        let mut hull_solver = HullSolver {
            block_name_index: hull_block_name_index,
            basic_blocks: vec![],
            multi_blocks: vec![],

            #[cfg(debug_assertions)]
            debug_basic_blocks: vec![],

            #[cfg(debug_assertions)]
            debug_multi_blocks: vec![],
        };

        hull_solver.add_base_blocks(self, voxel_loader)?;
        hull_solver.add_multi_blocks(self, voxel_loader)?;

        self.solvers.push(Box::new(hull_solver));

        debug!("Making Hull Done");
        Ok(())
    }
}

impl Solver for HullSolver {
    fn block_check_reset(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex> {
        let mut cache = vec![];
        cache.append(&mut self.get_basic_blocks(ship, world_block_pos));
        cache.append(&mut self.get_multi_blocks_reset(ship, world_block_pos));
        cache
    }

    fn block_check(
        &self,
        ship: &mut ShipData,
        chunk_index: usize,
        node_index: usize,
        world_node_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex> {
        let mut new_cache = vec![];
        for index in cache {
            if index < self.basic_blocks.len() {
                new_cache.push(index);
            } else {
                if self.keep_multi_block(ship, world_node_pos, index) {
                    new_cache.push(index);
                }
            }
        }

        new_cache
    }

    fn get_block(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> (Block, Prio) {
        let mut best_block = Block::from_single_node_id(NodeID::empty());
        let mut best_prio = Prio::BASE;

        for index in cache {
            if index < self.basic_blocks.len() {
                let (_, block, prio) = &self.basic_blocks[index];
                if best_prio < *prio {
                    best_block = *block;
                    best_prio = *prio;
                }
            } else {
                let (_, block, prio) = &self.multi_blocks[index - self.basic_blocks.len()];
                if best_prio < *prio {
                    best_block = *block;
                    best_prio = *prio;
                }
            }
        }

        (best_block, best_prio)
    }
}

impl HullSolver {
    fn add_base_blocks(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let hull_reqs = vec![(vec![], HULL_BASE0)];

        let mut base_blocks = vec![];
        for (i, (req, prio)) in hull_reqs.into_iter().enumerate() {
            let block = rules
                .load_block_from_node_folder(&format!("{HULL_BASE_NAME_PART}-{i}"), voxel_loader)?;

            base_blocks.push((req, block, prio));
        }

        let mut rotated_base_blocks = permutate_base_blocks(&base_blocks, rules);
        self.basic_blocks.append(&mut rotated_base_blocks);

        #[cfg(debug_assertions)]
        self.debug_basic_blocks.append(&mut base_blocks);

        Ok(())
    }

    fn add_multi_blocks(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let mut blocks = vec![];
        let mut multi_req_block = vec![];

        let num = 1;
        for i in 0..num {
            let (models, rot) =
                voxel_loader.get_name_folder(&format!("{HULL_MULTI_NAME_PART}-{i}"))?;

            if rot != Rot::IDENTITY {
                bail!("Multi Block Rot should be IDENTITY");
            }

            for (name, index, rot, pos) in models {
                if name.contains(HULL_MULTI_BLOCK) {
                    let block = rules.load_block_from_multi_node_by_index(index, voxel_loader)?;
                    let rotated_block = block.rotate(rot, rules);
                    blocks.push((rotated_block, pos))
                } else if name.contains(HULL_MULTI_FOLDER) {
                    let req_block = rules.load_block_from_node_folder(&name, voxel_loader)?;
                    let rotated_block = req_block.rotate(rot, rules);
                    multi_req_block.push((rotated_block, pos))
                } else if name.contains(HULL_MULTI_MULTI) {
                    let req_block =
                        rules.load_block_from_multi_node_by_index(index, voxel_loader)?;
                    let rotated_block = req_block.rotate(rot, rules);
                    multi_req_block.push((rotated_block, pos))
                } else {
                    error!("{} not reconized", name)
                }
            }
        }

        let mut multi_blocks: Vec<(Vec<(IVec3, Vec<Block>)>, Block, Prio)> = vec![];
        for (block, pos) in blocks {
            let mut empty_reqs = vec![];
            let mut add = false;
            let reqs = multi_blocks
                .iter_mut()
                .find_map(|(reqs, test_block, _)| {
                    if *test_block == block {
                        Some(reqs)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    add = true;
                    &mut empty_reqs
                });

            for offset in get_neighbors() {
                let neighbor_pos = pos + offset * 8;

                for (block, test_pos) in multi_req_block.iter() {
                    if neighbor_pos == *test_pos {
                        let blocks = reqs.iter_mut().find_map(|(test_offset, blocks)| {
                            let add_blocks = false;
                            if *test_offset == offset {
                                Some(blocks)
                            } else {
                                None
                            }
                        });

                        if blocks.is_some() {
                            blocks.unwrap().push(*block);
                        } else {
                            reqs.push((offset, vec![*block]));
                        }
                    }
                }
            }

            if add {
                multi_blocks.push((empty_reqs, block, Prio::HULL_MULTI))
            }
        }

        let mut rotated_multi_blocks = permutate_multi_blocks(&multi_blocks, rules);
        self.multi_blocks.append(&mut rotated_multi_blocks);

        #[cfg(debug_assertions)]
        self.debug_multi_blocks.append(&mut multi_blocks);

        Ok(())
    }

    fn get_basic_blocks(
        &self,
        ship: &mut ShipData,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex> {
        let block_name_index = ship.get_block_name_from_world_block_pos(world_block_pos);
        if block_name_index != self.block_name_index {
            return vec![];
        }

        let mut best_block_index = None;
        let mut best_prio = Prio::BASE;

        for (i, (reqs, _, prio)) in self.basic_blocks.iter().enumerate() {
            let mut pass = true;
            for offset in reqs {
                let req_world_block_pos = world_block_pos + *offset;
                let block_name_index =
                    ship.get_block_name_from_world_block_pos(req_world_block_pos);

                if block_name_index != self.block_name_index {
                    pass = false;
                    break;
                }
            }

            if pass && best_prio < *prio {
                best_block_index = Some(i);
                best_prio = *prio;
            }
        }

        return if best_block_index.is_some() {
            vec![best_block_index.unwrap()]
        } else {
            vec![]
        };
    }

    fn get_multi_blocks_reset(
        &self,
        ship: &mut ShipData,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex> {
        let mut cache = vec![];
        for (i, (reqs, _, _)) in self.multi_blocks.iter().enumerate() {
            let mut pass = true;
            for (req_pos, req_blocks) in reqs {
                let req_world_block_pos = world_block_pos + *req_pos;
                let block_name_index =
                    ship.get_block_name_from_world_block_pos(req_world_block_pos);

                let mut ok = false;
                for req_block in req_blocks {
                    let req_empty = *req_block == Block::from_single_node_id(NodeID::empty());
                    //let req_base = *req_block == self.basic_blocks[0].1;

                    if !req_empty && block_name_index == self.block_name_index {
                        ok = true;
                        break;
                    }
                    if req_empty && block_name_index == EMPTY_BLOCK_NAME_INDEX {
                        ok = true;
                        break;
                    }
                }

                if !ok {
                    pass = false;
                    break;
                }
            }

            if pass {
                cache.push(i + self.basic_blocks.len())
            }
        }

        cache
    }

    fn keep_multi_block(
        &self,
        ship: &mut ShipData,
        world_block_pos: IVec3,
        cache_index: CacheIndex,
    ) -> bool {
        let (reqs, _, _) = &self.multi_blocks[cache_index - self.basic_blocks.len()];

        let mut pass = true;
        for (req_pos, req_blocks) in reqs {
            let req_world_block_pos = world_block_pos + *req_pos;
            let cache =
                ship.get_cache_from_world_block_pos(req_world_block_pos, self.block_name_index);

            let mut ok = false;
            for req_block in req_blocks {
                if *req_block == Block::from_single_node_id(NodeID::empty()) {
                    continue;
                }

                for index in cache {
                    let test_block = self.get_block_from_cache_index(*index);

                    if *req_block == test_block {
                        ok = true;
                        break;
                    }
                }
            }

            if !ok {
                pass = false;
                break;
            }
        }

        pass
    }

    pub fn get_block_from_cache_index(&self, index: usize) -> Block {
        return if index < self.basic_blocks.len() {
            self.basic_blocks[index].1
        } else {
            self.multi_blocks[index - self.basic_blocks.len()].1
        };
    }
}

fn permutate_base_blocks(
    blocks: &[(Vec<IVec3>, Block, Prio)],
    rules: &mut Rules,
) -> Vec<(Vec<IVec3>, Block, Prio)> {
    let mut rotated_blocks = vec![];
    for (reqs, block, prio) in blocks.iter() {
        for rot in Rot::IDENTITY.get_all_permutations() {
            let mat: Mat4 = rot.into();
            let rotated_reqs: Vec<_> = reqs
                .iter()
                .map(|req| mat.transform_vector3((*req).as_vec3()).round().as_ivec3())
                .collect();

            let rotated_block = block.rotate(rot, rules);

            let mut found = false;
            for (_, test_block, _) in rotated_blocks.iter() {
                if *test_block == rotated_block {
                    found = true;
                    break;
                }
            }

            if !found {
                rotated_blocks.push((rotated_reqs, rotated_block, *prio))
            }
        }
    }

    rotated_blocks
}

fn permutate_multi_blocks(
    blocks: &[(Vec<(IVec3, Vec<Block>)>, Block, Prio)],
    rules: &mut Rules,
) -> Vec<(Vec<(IVec3, Vec<Block>)>, Block, Prio)> {
    let mut rotated_blocks = vec![];
    for (reqs, block, prio) in blocks.iter() {
        for rot in Rot::IDENTITY.get_all_permutations() {
            let mat: Mat4 = rot.into();
            let rotated_reqs: Vec<_> = reqs
                .iter()
                .map(|(req_pos, req_blocks)| {
                    let rotated_pos = mat
                        .transform_vector3((*req_pos).as_vec3())
                        .round()
                        .as_ivec3();
                    let rotated_blocks = req_blocks.iter().map(|b| b.rotate(rot, rules)).collect();
                    (rotated_pos, rotated_blocks)
                })
                .collect();

            let rotated_block = block.rotate(rot, rules);

            let mut found = false;
            for (_, test_block, _) in rotated_blocks.iter() {
                if *test_block == rotated_block {
                    found = true;
                    break;
                }
            }

            if !found {
                rotated_blocks.push((rotated_reqs, rotated_block, *prio))
            }
        }
    }

    rotated_blocks
}
