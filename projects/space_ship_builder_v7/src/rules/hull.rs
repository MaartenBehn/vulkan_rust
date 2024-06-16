use crate::math::{all_sides_dirs, get_all_poses, get_neighbors, oct_positions, to_1d};
use crate::node::{NodeID, NODE_VOXEL_LENGTH, VOXEL_EMPTY};
use crate::rotation::Rot;
use crate::rules::block::Block;
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
use std::collections::HashMap;

const HULL_CACHE_NONE: CacheIndex = CacheIndex::MAX;
const HULL_BLOCK_NAME: &str = "Hull";
const HULL_BASE_NAME_PART: &str = "Hull-Base";
const HULL_FILL_NAME_PART: &str = "Hull-Fill";

pub struct HullSolver {
    pub block_name_index: usize,
    pub basic_blocks: Vec<(Vec<IVec3>, Block, Prio)>,

    #[cfg(debug_assertions)]
    pub debug_basic_blocks: Vec<(Vec<IVec3>, Block, Prio)>,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        debug!("Making Hull");

        let hull_block_name_index = self.block_names.len();
        self.block_names.push(HULL_BLOCK_NAME.to_owned());

        let mut hull_solver = HullSolver {
            block_name_index: hull_block_name_index,
            basic_blocks: vec![],

            #[cfg(debug_assertions)]
            debug_basic_blocks: vec![],
        };

        hull_solver.add_base_blocks(self, voxel_loader)?;
        hull_solver.add_fill_blocks(self, voxel_loader)?;

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
        self.get_base_blocks(ship, world_block_pos)
    }

    fn block_check(
        &self,
        ship: &mut ShipData,
        chunk_index: usize,
        node_index: usize,
        world_node_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex> {
        cache
    }

    fn get_block(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> (Block, Prio) {
        let (base_block, base_prio) = if !cache.is_empty() && cache[0] < self.basic_blocks.len() {
            (self.basic_blocks[cache[0]].1, self.basic_blocks[cache[0]].2)
        } else {
            (Block::from_single_node_id(NodeID::empty()), Prio::BASE)
        };

        (base_block, base_prio)
    }
}

impl HullSolver {
    fn add_base_blocks(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let hull_reqs = vec![
            (vec![], Prio::HULL_BASE0),
            (vec![ivec3(-1, 0, 0), ivec3(1, 0, 0)], Prio::HULL_BASE2),
            (vec![ivec3(1, 0, 0)], Prio::HULL_BASE1),
            (vec![ivec3(-1, 0, 0), ivec3(0, -1, 0)], Prio::HULL_BASE3),
            (
                vec![ivec3(-1, 0, 0), ivec3(0, -1, 0), ivec3(0, 0, 1)],
                Prio::HULL_BASE4,
            ),
            (
                vec![ivec3(-1, 0, 0), ivec3(1, 0, 0), ivec3(0, -1, 0)],
                Prio::HULL_BASE5,
            ),
            (
                vec![
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(0, -1, 0),
                    ivec3(0, 0, 1),
                ],
                Prio::HULL_BASE6,
            ),
            (
                vec![
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(0, -1, 0),
                    ivec3(0, 1, 0),
                    ivec3(0, 0, 1),
                ],
                Prio::HULL_BASE7,
            ),
            (
                vec![
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(0, -1, 0),
                    ivec3(0, 1, 0),
                    ivec3(0, 0, -1),
                    ivec3(0, 0, 1),
                ],
                Prio::HULL_BASE8,
            ),
        ];

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

    fn add_fill_blocks(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let hull_reqs = vec![
            (
                vec![ivec3(-1, 0, 0), ivec3(0, -1, 0), ivec3(-1, -1, 0)],
                Prio::HULL_FILL0,
            ),
            (
                vec![
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(0, -1, 0),
                    ivec3(-1, -1, 0),
                    ivec3(1, -1, 0),
                ],
                Prio::HULL_FILL1,
            ),
            (
                vec![
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(1, 1, 0),
                ],
                Prio::HULL_FILL4,
            ),
            (
                vec![
                    ivec3(-1, 0, 0),
                    ivec3(0, -1, 0),
                    ivec3(-1, -1, 0),
                    ivec3(0, 0, 1),
                    ivec3(-1, 0, 1),
                    ivec3(0, -1, 1),
                    ivec3(-1, -1, 1),
                ],
                Prio::HULL_FILL2,
            ),
            (
                vec![
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(1, 1, 0),
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                    ivec3(1, 1, -1),
                ],
                Prio::HULL_FILL8,
            ),
            (
                vec![
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                ],
                Prio::HULL_FILL3,
            ),
            (
                vec![
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                ],
                Prio::HULL_FILL5,
            ),
            (
                vec![
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                ],
                Prio::HULL_FILL7,
            ),
            (
                vec![
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                    ivec3(1, 1, -1),
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(1, 1, 0),
                    ivec3(-1, -1, 1),
                    ivec3(0, -1, 1),
                    ivec3(1, -1, 1),
                    ivec3(-1, 0, 1),
                    ivec3(0, 0, 1),
                    ivec3(1, 0, 1),
                    ivec3(-1, 1, 1),
                    ivec3(0, 1, 1),
                    ivec3(1, 1, 1),
                ],
                Prio::HULL_FILL13,
            ),
            (
                vec![
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                    ivec3(1, 1, -1),
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(1, 1, 0),
                    ivec3(-1, -1, 1),
                    ivec3(0, -1, 1),
                    ivec3(1, -1, 1),
                    ivec3(-1, 0, 1),
                    ivec3(0, 0, 1),
                    ivec3(1, 0, 1),
                    ivec3(-1, 1, 1),
                    ivec3(0, 1, 1),
                ],
                Prio::HULL_FILL12,
            ),
            (
                vec![
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                    ivec3(1, 1, -1),
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(1, 1, 0),
                    ivec3(-1, -1, 1),
                    ivec3(0, -1, 1),
                    ivec3(1, -1, 1),
                    ivec3(-1, 0, 1),
                    ivec3(0, 0, 1),
                    ivec3(1, 0, 1),
                ],
                Prio::HULL_FILL11,
            ),
            (
                vec![
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                    ivec3(1, 1, -1),
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(1, 1, 0),
                    ivec3(-1, -1, 1),
                    ivec3(0, -1, 1),
                    ivec3(-1, 0, 1),
                    ivec3(0, 0, 1),
                ],
                Prio::HULL_FILL10,
            ),
            (
                vec![
                    ivec3(-1, -1, -1),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                    ivec3(-1, -1, 0),
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(-1, -1, 1),
                    ivec3(0, -1, 1),
                    ivec3(-1, 0, 1),
                    ivec3(0, 0, 1),
                ],
                Prio::HULL_FILL9,
            ),
            (
                vec![
                    ivec3(0, -1, 0),
                    ivec3(1, -1, 0),
                    ivec3(-1, 0, 0),
                    ivec3(1, 0, 0),
                    ivec3(-1, 1, 0),
                    ivec3(0, 1, 0),
                    ivec3(0, -1, -1),
                    ivec3(1, -1, -1),
                    ivec3(-1, 0, -1),
                    ivec3(0, 0, -1),
                    ivec3(1, 0, -1),
                    ivec3(-1, 1, -1),
                    ivec3(0, 1, -1),
                ],
                Prio::HULL_FILL6,
            ),
        ];

        let mut base_blocks = vec![];
        for (i, (req, prio)) in hull_reqs.into_iter().enumerate() {
            let block = rules
                .load_block_from_node_folder(&format!("{HULL_FILL_NAME_PART}-{i}"), voxel_loader)?;

            base_blocks.push((req, block, prio));
        }

        let mut rotated_base_blocks = permutate_base_blocks(&base_blocks, rules);
        self.basic_blocks.append(&mut rotated_base_blocks);

        #[cfg(debug_assertions)]
        self.debug_basic_blocks.append(&mut base_blocks);

        Ok(())
    }

    fn get_base_blocks(
        &self,
        ship: &mut ShipData,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex> {
        let block_name_index = ship.get_block_from_world_block_pos(world_block_pos);
        if block_name_index != self.block_name_index {
            return vec![];
        }

        let mut best_block_index = None;
        let mut best_prio = Prio::BASE;

        for (i, (reqs, _, prio)) in self.basic_blocks.iter().enumerate() {
            let mut pass = true;
            for offset in reqs {
                let req_world_block_pos = world_block_pos + *offset;
                let block_name_index = ship.get_block_from_world_block_pos(req_world_block_pos);

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
