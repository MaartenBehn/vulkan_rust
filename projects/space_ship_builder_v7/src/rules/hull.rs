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
        let mut node_ids = vec![];
        let max_hull_node = 8;
        for i in 0..=max_hull_node {
            let node_id = rules.load_node(&format!("Hull-{i}"), voxel_loader)?;
            node_ids.push(node_id);
        }

        rules
            .block_previews
            .push(Block::from_single_node_id(node_ids[0]));

        let block_reqs = vec![
            (
                node_ids[0],
                HULL0,
                vec![
                    (ivec3(0, 0, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                ],
            ),
            (
                node_ids[1],
                HULL1,
                vec![
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                ],
            ),
            (
                node_ids[2],
                HULL2,
                vec![
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                ],
            ),
            (
                node_ids[3],
                HULL3,
                vec![
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                    (ivec3(0, 0, 1), self.block_name_index),
                ],
            ),
            (
                node_ids[4],
                HULL4,
                vec![
                    (ivec3(-2, -2, -1), self.block_name_index),
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                ],
            ),
            (
                node_ids[5],
                HULL5,
                vec![
                    (ivec3(-2, -2, -1), self.block_name_index),
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                    (ivec3(0, 0, 1), self.block_name_index),
                ],
            ),
            (
                node_ids[7],
                HULL6,
                vec![
                    (ivec3(-2, -2, -1), self.block_name_index),
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                    (ivec3(-2, 0, 1), self.block_name_index),
                    (ivec3(0, 0, 1), self.block_name_index),
                ],
            ),
            (
                node_ids[8],
                HULL7,
                vec![
                    (ivec3(-2, -2, -1), self.block_name_index),
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                    (ivec3(-2, 0, 1), self.block_name_index),
                    (ivec3(0, 0, 1), self.block_name_index),
                    (ivec3(0, -2, 1), self.block_name_index),
                ],
            ),
            (
                node_ids[6],
                HULL8,
                vec![
                    (ivec3(-2, -2, -1), self.block_name_index),
                    (ivec3(-2, 0, -1), self.block_name_index),
                    (ivec3(0, -2, -1), self.block_name_index),
                    (ivec3(0, 0, -1), self.block_name_index),
                    (ivec3(-2, -2, 1), self.block_name_index),
                    (ivec3(-2, 0, 1), self.block_name_index),
                    (ivec3(0, -2, 1), self.block_name_index),
                    (ivec3(0, 0, 1), self.block_name_index),
                ],
            ),
        ];

        let permutated_block_req = permutate_block_req(&block_reqs, rules);
        let blocks = create_all_possible_blocks(&permutated_block_req);

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

fn create_all_possible_blocks(
    block_reqs: &[(NodeID, Prio, Vec<(IVec3, BlockNameIndex)>)],
) -> Vec<(Block, Vec<(IVec3, BlockNameIndex)>)> {
    let in_block_positions = oct_positions();

    let mut i = 0;
    let mut indices = [block_reqs.len() - 1; 8];
    let mut current_blocks = [Block::default(); 8];
    let mut current_reqs: [Vec<(IVec3, BlockNameIndex)>; 8] = [
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    ];
    let mut current_prio = Prio::HULL10;
    let mut blocks: Vec<(Block, _)> = vec![];

    let mut n = 0;
    loop {
        let index = indices[i];

        let mut accpted = true;

        let mut reqs = vec![];
        let mut found = vec![false; current_reqs[i].len()];

        for (offset, id) in &block_reqs[index].2 {
            let pos = *offset - in_block_positions[i];

            if pos % 2 != IVec3::ZERO {
                accpted = false;
                break;
            }

            if !current_reqs[i].is_empty() {
                let test_id = current_reqs[i]
                    .iter()
                    .position(|(test_offset, _)| *test_offset == pos);

                if test_id.is_none() {
                    accpted = false;
                    break;
                }

                let req_index = test_id.unwrap();

                if current_reqs[i][req_index].1 != *id {
                    accpted = false;
                    break;
                }

                found[req_index] = true;
            }

            reqs.push((pos, id.to_owned()))
        }

        accpted &= found.iter().all(|b| *b);

        if accpted {
            if i == 7 {
                current_blocks[i].node_ids[i] = block_reqs[index].0;

                let mut found = false;
                for (test_block, test_req) in blocks.iter() {
                    if test_block.node_ids == current_blocks[7].node_ids {
                        found = true;
                        break;
                    }
                }

                if !found {
                    blocks.push((current_blocks[7], current_reqs[7].to_owned()));
                    debug!("Added: {:?}", current_blocks[7])
                }

                current_blocks[7] = current_blocks[6];
                current_reqs[7] = current_reqs[6].to_owned();

                indices[7] -= 1;
            } else {
                current_blocks[i + 1] = current_blocks[i].to_owned();
                current_blocks[i + 1].node_ids[i] = block_reqs[index].0;
                current_reqs[i + 1] = reqs;

                i += 1;
            }
        } else {
            indices[i] -= 1;

            if indices[0] == 0 {
                break;
            } else if indices[i] == 0 {
                indices[i] = block_reqs.len() - 1;
                indices[i - 1] -= 1;
                i -= 1;
            }
        }

        n += 1;
        if n >= 10000000 {
            debug!("{:?} {}", indices, blocks.len());
            n = 0;
        }
    }

    blocks
}
