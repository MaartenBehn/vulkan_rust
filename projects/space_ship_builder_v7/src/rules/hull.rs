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

pub struct HullSolver {
    pub block_name_index: usize,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        debug!("Making Hull");

        let hull_block_name_index = self.block_names.len();
        self.block_names.push("Hull".to_owned());

        let mut hull_solver = HullSolver {
            block_name_index: hull_block_name_index,
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
        let max_hull_node = 8;
        for i in 0..=max_hull_node {
            let block = rules.load_block_from_node_folder(&format!("Hull-Base-{i}"), voxel_loader)?;
        }

        Ok(())
    }
}

fn flip_block_req(
    node_id: &NodeID,
    block_req: &[(IVec3, BlockNameIndex)],
    flips: &[BVec3],
) -> Vec<(NodeID, Vec<(IVec3, BlockNameIndex)>)> {
    let mut rotated_rules = Vec::new();

    for flip in flips.iter() {
        let flip_a = ivec3(
            if flip.x { -1 } else { 1 },
            if flip.y { -1 } else { 1 },
            if flip.z { -1 } else { 1 },
        );
        let flip_b = ivec3(
            if flip.x { 1 } else { 0 },
            if flip.y { 1 } else { 0 },
            if flip.z { 1 } else { 0 },
        );
        let flipped_rot = node_id.rot.flip(flip.to_owned());

        let flippped_req: Vec<_> = block_req
            .iter()
            .map(|(pos, indecies)| {
                let flipped_pos = ((*pos) + flip_b) * flip_a;
                (flipped_pos, indecies.to_owned())
            })
            .collect();

        rotated_rules.push((NodeID::new(node_id.index, flipped_rot), flippped_req))
    }

    rotated_rules
}

fn rotate_block_req(
    node_id: &NodeID,
    block_req: &[(IVec3, BlockNameIndex)],
    rotates: &[BVec3],
) -> Vec<(NodeID, Vec<(IVec3, BlockNameIndex)>)> {
    let mut rotated_rules = Vec::new();

    for &rotate in rotates {
        let mat = Mat4::from_mat3(node_id.rot.into());

        let rot_mat_x = if rotate.x {
            Mat4::from_mat3(Mat3::from_cols_array(&[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0,
            ]))
        } else {
            Mat4::IDENTITY
        };
        let rot_mat_y = if rotate.y {
            Mat4::from_mat3(Mat3::from_cols_array(&[
                0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0,
            ]))
        } else {
            Mat4::IDENTITY
        };
        let rot_mat_z = if rotate.z {
            Mat4::from_mat3(Mat3::from_cols_array(&[
                0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]))
        } else {
            Mat4::IDENTITY
        };

        // Yeah, glsl rotation order is different to glam, so I just create two different matrices.
        // I know ... but it's the simplest fix.
        let pos_mat = rot_mat_x * rot_mat_y * rot_mat_z;
        let rot_mat = rot_mat_z * rot_mat_y * rot_mat_x;

        let rotated_rot: Rot = Mat3::from_mat4(mat * rot_mat).into();

        let rotated_req: Vec<_> = block_req
            .iter()
            .map(|(pos, indecies)| {
                let rotated_pos = pos_mat.transform_point3(pos.as_vec3()).round().as_ivec3();
                (rotated_pos, indecies.to_owned())
            })
            .collect();

        rotated_rules.push((NodeID::new(node_id.index, rotated_rot), rotated_req));
    }

    rotated_rules
}

fn permutate_block_req(
    block_reqs: &[(NodeID, Prio, Vec<(IVec3, BlockNameIndex)>)],
    rules: &mut Rules,
) -> Vec<(NodeID, Prio, Vec<(IVec3, BlockNameIndex)>)> {
    let rotations = all_bvec3s();
    let flips = all_bvec3s();

    let mut permutated_block_reqs: Vec<(NodeID, Prio, Vec<(IVec3, BlockNameIndex)>)> = Vec::new();

    for (node_id, prio, block_reqs) in block_reqs.iter() {
        let flipped_rules = flip_block_req(node_id, block_reqs, &flips);

        for (flipped_node_id, flipped_req) in flipped_rules {
            let rotated_rules = rotate_block_req(&flipped_node_id, &flipped_req, &rotations);

            for (permutated_node_id, permutated_req) in rotated_rules {
                let permutated_node_id = rules.get_duplicate_node_id(permutated_node_id);

                let mut added = false;
                for (test_id, _, test_reqs) in permutated_block_reqs.iter() {
                    if permutated_node_id != *test_id {
                        continue;
                    }

                    if *test_reqs == permutated_req {
                        added = true;
                        break;
                    }
                }

                if !added {
                    permutated_block_reqs.push((
                        permutated_node_id,
                        prio.to_owned(),
                        permutated_req,
                    ));
                }
            }
        }
    }

    permutated_block_reqs
}
