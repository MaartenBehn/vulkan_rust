use crate::math::{all_bvec3s, all_sides_dirs, get_all_poses, get_neighbors, oct_positions, to_1d};
use crate::node::{BlockIndex, NodeID, BLOCK_INDEX_EMPTY, NODE_VOXEL_LENGTH, VOXEL_EMPTY};
use crate::rotation::Rot;
use crate::rules::block_preview::BlockPreview;
use crate::rules::solver::{push_in_block_affected_nodes, Solver};
use crate::rules::Prio::{
    HULL0, HULL1, HULL10, HULL2, HULL3, HULL4, HULL5, HULL6, HULL7, HULL8, HULL9,
};
use crate::rules::{Prio, Rules};
use crate::ship::data::{CacheIndex, ShipData};
use crate::ship::possible_nodes::NodeData;
use crate::voxel_loader::VoxelLoader;
use log::debug;
use octa_force::anyhow::bail;
use octa_force::glam::{uvec3, UVec3};
use octa_force::{
    anyhow::Result,
    glam::{ivec3, BVec3, IVec3, Mat3, Mat4},
};
use std::collections::HashMap;

const HULL_CACHE_NONE: CacheIndex = CacheIndex::MAX;

pub struct HullSolver {
    pub block_index: usize,
    pub block_reqs: Vec<(NodeData, Vec<(IVec3, BlockIndex)>)>,
    pub node_reqs: Vec<(NodeID, Vec<(IVec3, Vec<NodeID>)>)>,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        debug!("Making Hull");

        let hull_block_index = self.block_names.len();
        self.block_names.push("Hull".to_owned());

        let mut hull_solver = HullSolver {
            block_index: hull_block_index,
            block_reqs: vec![],
            node_reqs: vec![],
        };

        hull_solver.add_base_nodes(self, voxel_loader)?;
        hull_solver.add_multi(self, voxel_loader)?;

        self.solvers.push(Box::new(hull_solver));

        debug!("Making Hull Done");
        Ok(())
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
    ) -> Vec<NodeData> {
        self.block_level(ship, world_node_pos)
    }

    fn node_check_reset(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData> {
        self.node_level(ship, node_index, chunk_index, world_node_pos, true)
    }

    fn node_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData> {
        self.node_level(ship, node_index, chunk_index, world_node_pos, false)
    }
}

impl HullSolver {
    fn add_base_nodes(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let mut node_ids = vec![];
        let max_hull_node = 8;
        for i in 0..=max_hull_node {
            let node_id = rules.load_node(&format!("Hull-{i}"), voxel_loader)?;
            node_ids.push(node_id);
        }

        rules
            .block_previews
            .push(BlockPreview::from_single_node_id(node_ids[0]));

        let block_reqs = vec![
            (
                NodeData::new(node_ids[0], HULL0, HULL_CACHE_NONE),
                vec![(ivec3(0, 0, -1), self.block_index)],
            ),
            (
                NodeData::new(node_ids[1], HULL1, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[2], HULL2, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[3], HULL3, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                    (ivec3(0, 0, 1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[4], HULL4, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, -2, -1), self.block_index),
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[5], HULL5, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, -2, -1), self.block_index),
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                    (ivec3(0, 0, 1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[7], HULL6, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, -2, -1), self.block_index),
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                    (ivec3(-2, 0, 1), self.block_index),
                    (ivec3(0, 0, 1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[8], HULL7, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, -2, -1), self.block_index),
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                    (ivec3(-2, 0, 1), self.block_index),
                    (ivec3(0, 0, 1), self.block_index),
                    (ivec3(0, -2, 1), self.block_index),
                ],
            ),
            (
                NodeData::new(node_ids[6], HULL8, HULL_CACHE_NONE),
                vec![
                    (ivec3(-2, -2, -1), self.block_index),
                    (ivec3(-2, 0, -1), self.block_index),
                    (ivec3(0, -2, -1), self.block_index),
                    (ivec3(0, 0, -1), self.block_index),
                    (ivec3(-2, -2, 1), self.block_index),
                    (ivec3(-2, 0, 1), self.block_index),
                    (ivec3(0, -2, 1), self.block_index),
                    (ivec3(0, 0, 1), self.block_index),
                ],
            ),
        ];

        let mut permutated_block_reqs = permutate_block_req(&block_reqs, rules);
        self.block_reqs.append(&mut permutated_block_reqs);

        Ok(())
    }

    fn add_multi(&mut self, rules: &mut Rules, voxel_loader: &VoxelLoader) -> Result<()> {
        let max_hull_node = 0;
        for i in 0..=max_hull_node {
            let name = format!("Hull-Multi-{i}");
            let (size, node_ids) = rules.load_multi_node(&name, voxel_loader)?;

            if (size % 2) != UVec3::ZERO {
                bail!("The node size multi node model {name} needs to multiple of 2.")
            }

            let filled = Self::get_multi_nodes_filled(size, &node_ids, rules);
            let blocks = Self::get_multi_blocks_ids(size, &filled, self.block_index);

            let mut block_reqs = vec![];
            let mut node_reqs = vec![];
            for pos in get_all_poses(size) {
                let index = to_1d(pos, size);
                let node_id = &node_ids[index];

                if node_id.is_empty() {
                    continue;
                }

                // Blocks
                let block_pos = (pos.as_ivec3() / 2);
                let in_block_pos = pos.as_ivec3() % 2;

                let mut possible_block_neighbors = vec![];
                for offset in get_neighbors() {
                    let neighbor_pos = block_pos + offset;

                    if neighbor_pos.is_negative_bitmask() != 0
                        || neighbor_pos.cmpge((size / 2).as_ivec3()).any()
                    {
                        continue;
                    }

                    let neighbor_block_index = to_1d(neighbor_pos.as_uvec3(), size / 2);
                    let neighbor_block = blocks[neighbor_block_index];

                    let block_neigbor_offset = (offset * 2) - in_block_pos;

                    possible_block_neighbors.push((block_neigbor_offset, neighbor_block));
                }

                // Nodes
                let mut possible_node_neighbors: HashMap<IVec3, Vec<NodeID>> = HashMap::default();
                for offset in get_neighbors() {
                    if offset == IVec3::ZERO {
                        continue;
                    }

                    let neighbor_pos = pos.as_ivec3() + offset;

                    if neighbor_pos.is_negative_bitmask() != 0
                        || neighbor_pos.cmpge(size.as_ivec3()).any()
                    {
                        continue;
                    }

                    let neighbor_node_id = node_ids[to_1d(neighbor_pos.as_uvec3(), size)];
                    let ids = possible_node_neighbors.entry(offset).or_insert(vec![]);

                    if !ids.contains(&neighbor_node_id) {
                        ids.push(neighbor_node_id);
                    }
                }

                block_reqs.push((
                    NodeData::new(node_id.to_owned(), HULL10, HULL_CACHE_NONE),
                    possible_block_neighbors,
                ));

                node_reqs.push((
                    node_id.to_owned(),
                    possible_node_neighbors.into_iter().collect(),
                ));
            }

            let mut permutated_block_reqs = permutate_block_req(&block_reqs, rules);
            let mut permutated_node_reqs = permutate_node_req(&node_reqs, rules);

            // Link Node reqs in Block reqs
            for (data, _) in permutated_block_reqs.iter_mut() {
                for (i, (node_id, _)) in permutated_node_reqs.iter().enumerate() {
                    if data.id == *node_id {
                        data.cache_index = i;
                    }
                }
            }

            self.block_reqs.append(&mut permutated_block_reqs);
            self.node_reqs.append(&mut permutated_node_reqs);
        }

        Ok(())
    }

    fn get_multi_nodes_filled(size: UVec3, node_ids: &[NodeID], rules: &Rules) -> Vec<bool> {
        let mut filled = vec![];
        for pos in get_all_poses(size) {
            let index = to_1d(pos, size);
            let node = &rules.nodes[node_ids[index].index];

            let mut count = 0;
            for voxel in node.voxels {
                count += u8::from(voxel != VOXEL_EMPTY);
            }

            filled.push(count as usize >= NODE_VOXEL_LENGTH / 2);
        }

        filled
    }

    fn get_multi_blocks_ids(
        size: UVec3,
        filled: &[bool],
        block_index: BlockIndex,
    ) -> Vec<BlockIndex> {
        let mut block_ids = vec![];

        let block_size = size / 2;
        for block_pos in get_all_poses(block_size) {
            let pos = block_pos * 2;

            let mut count = 0;
            for offset in oct_positions() {
                let node_pos = pos + offset.as_uvec3();
                let node_index = to_1d(node_pos, size);

                count += u8::from(filled[node_index]);
            }

            if count >= 4 {
                block_ids.push(block_index);
            } else {
                block_ids.push(BLOCK_INDEX_EMPTY);
            }
        }

        block_ids
    }

    fn block_level(&self, ship: &mut ShipData, pos: IVec3) -> Vec<NodeData> {
        let mut new_ids = Vec::new();
        for (node_data, block_reqs) in self.block_reqs.iter() {
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
                new_ids.push(node_data.to_owned());
            }
        }
        new_ids
    }

    fn node_level(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        pos: IVec3,
        reset: bool,
    ) -> Vec<NodeData> {
        let mut node_datas = if reset {
            ship.chunks[chunk_index].base_nodes[node_index]
                .get_node_ids(self.block_index)
                .to_owned()
        } else {
            ship.chunks[chunk_index].nodes[node_index]
                .get_node_ids(self.block_index)
                .to_owned()
        };

        node_datas = node_datas
            .into_iter()
            .filter(|data| {
                if data.cache_index == HULL_CACHE_NONE {
                    return true;
                }

                let (node_id, node_req) = &self.node_reqs[data.cache_index];

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
                        ship.chunks[test_chunk_index].base_nodes[test_node_index]
                            .get_node_ids(self.block_index)
                    } else {
                        ship.chunks[test_chunk_index].nodes[test_node_index]
                            .get_node_ids(self.block_index)
                    };

                    let mut found = false;
                    if test_nodes.is_empty() && req_ids_contains_empty {
                        found = true;
                    } else if req_ids_contains_any {
                        found = test_nodes.iter().any(|data| !data.id.is_empty())
                    } else {
                        for test_data in test_nodes.iter() {
                            if req_ids.contains(&test_data.id) {
                                found = true;
                                break;
                            }
                        }
                    }

                    node_accepted &= found;
                }

                node_accepted
            })
            .collect();

        node_datas
    }
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

fn permutate_block_req(
    block_reqs: &[(NodeData, Vec<(IVec3, BlockIndex)>)],
    rules: &mut Rules,
) -> Vec<(NodeData, Vec<(IVec3, BlockIndex)>)> {
    let rotations = all_bvec3s();
    let flips = all_bvec3s();

    let mut permutated_block_reqs: Vec<(NodeData, Vec<(IVec3, BlockIndex)>)> = Vec::new();

    for (data, block_reqs) in block_reqs.iter() {
        let flipped_rules = flip_block_req(&data.id, block_reqs, &flips);

        for (flipped_node_id, flipped_req) in flipped_rules {
            let rotated_rules = rotate_block_req(&flipped_node_id, &flipped_req, &rotations);

            for (permutated_node_id, permutated_req) in rotated_rules {
                let node_id = rules.get_duplicate_node_id(permutated_node_id);

                let mut added = false;
                for (test_data, test_reqs) in permutated_block_reqs.iter() {
                    if node_id != test_data.id {
                        continue;
                    }

                    if *test_reqs == permutated_req {
                        added = true;
                        break;
                    }
                }

                if !added {
                    permutated_block_reqs.push((
                        NodeData::new(permutated_node_id, data.prio, data.cache_index),
                        permutated_req,
                    ));
                }
            }
        }
    }

    permutated_block_reqs
}

fn flip_node_req(
    node_id: &NodeID,
    node_req: &[(IVec3, Vec<NodeID>)],
    flips: &[BVec3],
) -> Vec<(NodeID, Vec<(IVec3, Vec<NodeID>)>)> {
    let mut rotated_rules = Vec::new();

    for flip in flips.iter() {
        let flip_a = ivec3(
            if flip.x { -1 } else { 1 },
            if flip.y { -1 } else { 1 },
            if flip.z { -1 } else { 1 },
        );

        let flipped_rot = node_id.rot.flip(flip.to_owned());

        let flippped_req: Vec<_> = node_req
            .iter()
            .map(|(pos, ids)| {
                let flipped_pos = (*pos) * flip_a;
                let flipped_ids = ids
                    .iter()
                    .map(|id| {
                        let flipped_rot = id.rot.flip(flip.to_owned());
                        NodeID::new(id.index, flipped_rot)
                    })
                    .collect();
                (flipped_pos, flipped_ids)
            })
            .collect();

        rotated_rules.push((NodeID::new(node_id.index, flipped_rot), flippped_req))
    }

    rotated_rules
}

fn rotate_node_req(
    node_id: &NodeID,
    node_req: &[(IVec3, Vec<NodeID>)],
    rotates: &[BVec3],
) -> Vec<(NodeID, Vec<(IVec3, Vec<NodeID>)>)> {
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

        let rotated_req: Vec<_> = node_req
            .iter()
            .map(|(pos, ids)| {
                let rotated_pos = pos_mat.transform_point3(pos.as_vec3()).round().as_ivec3();
                let rotated_ids = ids
                    .iter()
                    .map(|id| {
                        let mat = Mat4::from_mat3(id.rot.into());
                        let roteted_rot: Rot = Mat3::from_mat4(mat * rot_mat).into();
                        NodeID::new(id.index, roteted_rot)
                    })
                    .collect();
                (rotated_pos, rotated_ids)
            })
            .collect();

        rotated_rules.push((NodeID::new(node_id.index, rotated_rot), rotated_req));
    }

    rotated_rules
}

fn permutate_node_req(
    block_reqs: &[(NodeID, Vec<(IVec3, Vec<NodeID>)>)],
    rules: &mut Rules,
) -> Vec<(NodeID, Vec<(IVec3, Vec<NodeID>)>)> {
    let rotations = all_bvec3s();
    let flips = all_bvec3s();

    let mut permutated_node_reqs: Vec<(NodeID, Vec<(IVec3, Vec<NodeID>)>)> = Vec::new();

    for (node_id, node_reqs) in block_reqs.iter() {
        let flipped_rules = flip_node_req(node_id, node_reqs, &flips);

        for (flipped_node_id, flipped_req) in flipped_rules {
            let rotated_rules = rotate_node_req(&flipped_node_id, &flipped_req, &rotations);

            for (permutated_node_id, mut permutated_req) in rotated_rules.into_iter() {
                // Find node_id index
                let permutated_node_id = rules.get_duplicate_node_id(permutated_node_id);

                let mut added = false;
                for (test_id, test_reqs) in permutated_node_reqs.iter_mut() {
                    if permutated_node_id != *test_id {
                        continue;
                    }

                    for (pos, ids) in permutated_req.iter() {
                        let mut req_pos_found = false;
                        for (test_req_pos, test_req_ids) in test_reqs.iter_mut() {
                            if *test_req_pos == *pos {
                                for id in ids {
                                    let id = rules.get_duplicate_node_id(*id);

                                    if !test_req_ids.contains(&id) {
                                        test_req_ids.push(id);
                                    }
                                }

                                req_pos_found = true;
                                break;
                            }
                        }

                        if !req_pos_found {
                            test_reqs.push((pos.to_owned(), ids.to_owned()))
                        }
                    }

                    added = true;
                    break;
                }

                if !added {
                    permutated_node_reqs.push((permutated_node_id, permutated_req))
                }
            }
        }
    }

    permutated_node_reqs
}

/*
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

 */
