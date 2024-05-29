use crate::math::{all_bvec3s, all_sides_dirs};
use crate::node::{BlockIndex, NodeID};
use crate::rotation::Rot;
use crate::rules::block_preview::BlockPreview;
use crate::rules::solver::{push_in_block_affected_nodes, Solver};
use crate::rules::Prio::{HULL0, HULL1, HULL2, HULL3, HULL4, HULL5, HULL6, HULL7, HULL8};
use crate::rules::{Prio, Rules};
use crate::ship::data::ShipData;
use crate::voxel_loader::VoxelLoader;
use log::debug;
use octa_force::glam::{ivec3, BVec3, IVec3, Mat3, Mat4};

pub struct HullSolver {
    pub block_index: usize,
    pub block_reqs: Vec<(NodeID, Prio, Vec<(IVec3, BlockIndex)>)>,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> octa_force::anyhow::Result<()> {
        debug!("Making Hull");

        self.block_names.push("Hull".to_owned());

        let mut node_ids = vec![];
        let max_hull_node = 8;
        for i in 0..=max_hull_node {
            let node_id = self.add_node(&format!("Hull-{i}"), voxel_loader)?;
            node_ids.push(node_id);
        }

        self.block_previews
            .push(BlockPreview::from_single_node_id(node_ids[0]));

        let hull_solver = HullSolver::new(node_ids, self, self.solvers.len());
        self.solvers.push(Box::new(hull_solver));

        debug!("Making Hull Done");
        Ok(())
    }
}

impl HullSolver {
    pub fn new(node_ids: Vec<NodeID>, rules: &mut Rules, block_index: usize) -> Self {
        let block_reqs = Self::get_block_reqs(&node_ids, rules, block_index);

        Self {
            block_index,
            block_reqs,
        }
    }
}

impl Solver for HullSolver {
    fn push_block_affected_nodes(&self, ship: &mut ShipData, block_pos: IVec3) {
        push_in_block_affected_nodes(ship, block_pos, self.block_index);
    }

    fn block_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<(NodeID, Prio)> {
        self.block_level(ship, world_node_pos)
    }

    fn node_check_reset(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<(NodeID, Prio)> {
        ship.chunks[chunk_index].base_nodes[node_index]
            .get_node_ids(self.block_index)
            .to_owned()
    }

    fn node_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<(NodeID, Prio)> {
        ship.chunks[chunk_index].nodes[node_index]
            .get_node_ids(self.block_index)
            .to_owned()
    }
}

impl HullSolver {
    fn get_block_reqs(
        node_ids: &[NodeID],
        rules: &mut Rules,
        hull_block_index: usize,
    ) -> Vec<(NodeID, Prio, Vec<(IVec3, BlockIndex)>)> {
        let block_reqs = vec![
            (vec![(ivec3(0, 0, -1), hull_block_index)], HULL0),
            (
                vec![
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                ],
                HULL1,
            ),
            (
                vec![
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                ],
                HULL2,
            ),
            (
                vec![
                    (ivec3(-2, -2, -1), hull_block_index),
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                ],
                HULL3,
            ),
            (
                vec![
                    (ivec3(-2, -2, -1), hull_block_index),
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                ],
                HULL4,
            ),
            (
                vec![
                    (ivec3(-2, -2, -1), hull_block_index),
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                    (ivec3(0, 0, 1), hull_block_index),
                ],
                HULL5,
            ),
            (
                vec![
                    (ivec3(-2, -2, -1), hull_block_index),
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                    (ivec3(-2, -2, 1), hull_block_index),
                    (ivec3(-2, 0, 1), hull_block_index),
                    (ivec3(0, -2, 1), hull_block_index),
                    (ivec3(0, 0, 1), hull_block_index),
                ],
                HULL8,
            ),
            (
                vec![
                    (ivec3(-2, -2, -1), hull_block_index),
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                    (ivec3(0, -2, 1), hull_block_index),
                    (ivec3(0, 0, 1), hull_block_index),
                ],
                HULL6,
            ),
            (
                vec![
                    (ivec3(-2, -2, -1), hull_block_index),
                    (ivec3(-2, 0, -1), hull_block_index),
                    (ivec3(0, -2, -1), hull_block_index),
                    (ivec3(0, 0, -1), hull_block_index),
                    (ivec3(-2, 0, 1), hull_block_index),
                    (ivec3(0, -2, 1), hull_block_index),
                    (ivec3(0, 0, 1), hull_block_index),
                ],
                HULL7,
            ),
        ];

        let rotations = all_bvec3s();
        let flips = all_bvec3s();

        let mut permutated_block_reqs: Vec<(NodeID, Prio, Vec<(IVec3, BlockIndex)>)> = Vec::new();

        for (node_id, (block_reqs, prio)) in node_ids.iter().zip(block_reqs.iter()) {
            let flipped_rules = Self::flip_block_req(&node_id, block_reqs, &flips);

            for (flipped_node_id, flipped_req) in flipped_rules {
                let rotated_rules =
                    Self::rotate_block_req(&flipped_node_id, &flipped_req, &rotations);

                for (permutated_node_id, permutated_req) in rotated_rules {
                    let node_id = rules.get_duplicate_node_id(permutated_node_id);

                    let mut added = false;
                    for (test_node_id, _, test_reqs) in permutated_block_reqs.iter() {
                        if node_id != *test_node_id {
                            continue;
                        }

                        if *test_reqs == permutated_req {
                            added = true;
                            break;
                        }
                    }

                    if !added {
                        permutated_block_reqs.push((node_id, *prio, permutated_req));
                    }
                }
            }
        }

        permutated_block_reqs
    }

    fn flip_block_req(
        node_id: &NodeID,
        block_req: &[(IVec3, BlockIndex)],
        flips: &[BVec3],
    ) -> Vec<(NodeID, Vec<(IVec3, BlockIndex)>)> {
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
        block_req: &[(IVec3, BlockIndex)],
        rotates: &[BVec3],
    ) -> Vec<(NodeID, Vec<(IVec3, BlockIndex)>)> {
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

    pub fn get_matching_sides_reqs(
        node_ids: &[NodeID],
        rules: &mut Rules,
    ) -> Vec<Vec<(IVec3, Vec<NodeID>)>> {
        let mut node_reqs_list = vec![];

        for node_id in node_ids {
            let mut node_reqs: Vec<(IVec3, Vec<NodeID>)> = vec![];

            let node = rules.nodes[node_id.index].to_owned();

            for test_node_id in node_ids {
                let test_node = rules.nodes[node_id.index].to_owned();

                for permutated_rot in test_node_id.rot.get_all_permutations() {
                    for side in all_sides_dirs() {
                        if node.shares_side_voxels(node_id.rot, &test_node, permutated_rot, side) {
                            let new_node_id = rules.get_duplicate_node_id(NodeID::new(
                                test_node_id.index,
                                permutated_rot,
                            ));

                            let index = node_reqs
                                .iter()
                                .position(|(test_pos, ids)| *test_pos == side);

                            if index.is_some() {
                                if !node_reqs[index.unwrap()].1.contains(&new_node_id) {
                                    node_reqs[index.unwrap()].1.push(new_node_id)
                                }
                            } else {
                                node_reqs.push((side, vec![new_node_id]))
                            }
                        }
                    }
                }
            }

            node_reqs_list.push(node_reqs);
        }

        node_reqs_list
    }

    fn block_level(&self, ship: &mut ShipData, pos: IVec3) -> Vec<(NodeID, Prio)> {
        let mut new_ids = Vec::new();
        for (node_id, prio, block_reqs) in self.block_reqs.iter() {
            let mut accepted = true;

            for (offset, id) in block_reqs.iter() {
                let test_pos = pos + *offset;

                // If the offset does not aling with the node just ignore it.
                if (test_pos % 2) != IVec3::ZERO {
                    accepted &= false;
                    continue;
                }

                let test_chunk_index = ship.get_chunk_index_from_node_pos(test_pos);
                let test_block_index = ship.get_block_index(test_pos);

                let index = ship.chunks[test_chunk_index].blocks[test_block_index].to_owned();

                // Check if the Block at the pos is in the allowed id.
                accepted &= *id == index;
            }

            if accepted {
                new_ids.push((node_id.to_owned(), prio.to_owned()));
            }
        }
        new_ids
    }

    /*
    fn node_level(&mut self, ship: &mut ShipData, pos: IVec3, reset: bool) -> Vec<NodeID> {

        let mut new_nodes = vec![];
        for (node_id, node_req) in self.node_ids.iter().zip(self.node_reqs.iter()) {

            let mut node_accepted = true;
            for (offset, req_ids) in node_req.iter() {
                let test_pos = pos + *offset;

                let test_chunk_index = ship.get_chunk_index_from_node_pos(test_pos);
                let test_node_index = ship.get_node_index(test_pos);

                let mut req_ids_contains_empty = false;
                let mut req_ids_contains_any = false;
                for req_node in req_ids {
                    if req_node.is_empty() {
                        req_ids_contains_empty = true;
                    }
                    if req_node.is_any() {
                        req_ids_contains_any = true;
                    }
                    if req_ids_contains_empty && req_ids_contains_any {
                        break;
                    }
                }

                let test_nodes = if reset {
                    &ship.chunks[test_chunk_index].base_nodes[test_node_index].get_node_ids(self.block_index)
                } else {
                    &ship.chunks[test_chunk_index].nodes[test_node_index].get_node_ids(self.block_index)
                };

                let mut found = false;
                if test_nodes.is_empty() && req_ids_contains_empty {
                    found = true;
                } else if req_ids_contains_any {
                    found = test_nodes.iter().any(|node| !node.is_empty())
                } else {
                    for test_id in test_nodes.iter() {
                        if req_ids.contains(&test_id) {
                            found = true;
                            break;
                        }
                    }
                }

                node_accepted &= found;
            }

            if node_accepted {
                new_nodes.push(node_id.to_owned());
            }
        }

        new_nodes
    }
     */
}
