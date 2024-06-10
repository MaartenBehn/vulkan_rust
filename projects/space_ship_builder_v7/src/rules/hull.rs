use crate::math::{all_bvec3s, all_sides_dirs, get_all_poses, get_neighbors, oct_positions, to_1d};
use crate::node::{NodeID, NODE_VOXEL_LENGTH, VOXEL_EMPTY};
use crate::rotation::Rot;
use crate::rules::block::{Block, BlockNameIndex};
use crate::rules::solver::Solver;
use crate::rules::Prio::{
    HULL0, HULL1, HULL10, HULL2, HULL3, HULL4, HULL5, HULL6, HULL7, HULL8, HULL9,
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

pub struct HullSolver {
    pub block_name_index: usize,
    pub base_blocks: Vec<(Vec<IVec3>, Block)>,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        debug!("Making Hull");

        let hull_block_name_index = self.block_names.len();
        self.block_names.push(HULL_BLOCK_NAME.to_owned());

        let mut hull_solver = HullSolver {
            block_name_index: hull_block_name_index,
            base_blocks: vec![],
        };

        hull_solver.add_base_blocks(self, voxel_loader)?;

        self.solvers.push(Box::new(hull_solver));

        debug!("Making Hull Done");
        Ok(())
    }
}

impl Solver for HullSolver {
    fn block_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<BlockNameIndex> {
        todo!()
    }
}

impl HullSolver {
    fn add_base_blocks(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let hull_reqs = vec![
            vec![],
            vec![ivec3(-1, 0, 0), ivec3(1, 0, 0)],
            vec![ivec3(1, 0, 0)],
            vec![ivec3(-1, 0, 0), ivec3(0, -1, 0)],
            vec![ivec3(-1, 0, 0), ivec3(0, -1, 0), ivec3(0, 0, 1)],
            vec![ivec3(-1, 0, 0), ivec3(1, 0, 0), ivec3(0, -1, 0)],
            vec![
                ivec3(-1, 0, 0),
                ivec3(1, 0, 0),
                ivec3(0, -1, 0),
                ivec3(0, 0, 1),
            ],
            vec![
                ivec3(-1, 0, 0),
                ivec3(1, 0, 0),
                ivec3(0, -1, 0),
                ivec3(0, 1, 0),
                ivec3(0, 0, 1),
            ],
            vec![
                ivec3(-1, 0, 0),
                ivec3(1, 0, 0),
                ivec3(0, -1, 0),
                ivec3(0, 1, 0),
                ivec3(0, 0, -1),
                ivec3(0, 0, 1),
            ],
        ];

        let mut base_blocks = vec![];
        for (i, req) in hull_reqs.into_iter().enumerate() {
            let block = rules
                .load_block_from_node_folder(&format!("{HULL_BASE_NAME_PART}-{i}"), voxel_loader)?;

            base_blocks.push((req, block));
        }

        let mut rotated_base_blocks = permutate_base_blocks(&base_blocks, rules);
        self.base_blocks.append(&mut rotated_base_blocks);

        Ok(())
    }
}

fn permutate_base_blocks(
    blocks: &[(Vec<IVec3>, Block)],
    rules: &mut Rules,
) -> Vec<(Vec<IVec3>, Block)> {
    let mut rotated_blocks = vec![];
    for (reqs, block) in blocks.iter() {
        for rot in Rot::IDENTITY.get_all_permutations() {
            let mat: Mat4 = rot.into();
            let rotated_reqs: Vec<_> = reqs
                .iter()
                .map(|req| mat.transform_vector3((*req).as_vec3()).round().as_ivec3())
                .collect();

            let rotated_block = block.rotate(rot, rules);

            rotated_blocks.push((rotated_reqs, rotated_block))
        }
    }

    rotated_blocks
}
