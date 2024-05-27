use crate::math::get_neighbors;
use crate::node::{BlockIndex, Node, NodeID, NODE_INDEX_ANY, NODE_INDEX_EMPTY};
use crate::rotation::Rot;
use crate::voxel_loader::{VoxelLoader, IGNORE_PRIO};
use octa_force::glam::{ivec3, BVec3, IVec3, Mat3, Mat4};
use std::collections::HashMap;

const NODE_ID_MAP_INDEX_NONE: usize = NODE_INDEX_EMPTY;
const NODE_ID_MAP_INDEX_ANY: usize = NODE_INDEX_ANY;

pub struct Rules {
    pub map_rules_index_to_node_id: Vec<Vec<NodeID>>,

    pub node_rules: Vec<HashMap<IVec3, Vec<NodeID>>>,
    pub block_rules: Vec<Vec<(Vec<(IVec3, BlockIndex)>, usize)>>,

    pub affected_by_block: Vec<Vec<IVec3>>,
}

impl Rules {
    pub fn new(voxel_loader: &VoxelLoader) -> Self {
        let mut possible_node_neighbor_list: Vec<HashMap<IVec3, Vec<NodeID>>> = Vec::new();
        let mut possible_block_neighbor_list: Vec<Vec<(Vec<(IVec3, BlockIndex)>, usize)>> =
            Vec::new();
        let mut node_id_index_map = Vec::new();

        for (pos, (node_id, prio)) in voxel_loader.node_positions.iter() {
            if node_id.is_empty() || *prio == IGNORE_PRIO {
                continue;
            }

            // Find node_id index
            let (node_id_index, new_node_id) =
                Self::find_node_index(&mut node_id_index_map, node_id, voxel_loader);
            if new_node_id {
                possible_node_neighbor_list.push(HashMap::default());
                possible_block_neighbor_list.push(Vec::new());
            }

            // Neighbor Nodes
            for offset in get_neighbors() {
                if offset == IVec3::ZERO {
                    continue;
                }

                let neighbor_offset = offset;
                let neighbor_pos = pos.as_ivec3() + neighbor_offset;

                if neighbor_pos.is_negative_bitmask() != 0 {
                    continue;
                }

                let neighbor_node_id = voxel_loader.node_positions.get(&neighbor_pos.as_uvec3());
                if neighbor_node_id.is_none() {
                    // No Node at neighbor pos
                    continue;
                }

                let (mapped_req_index, new_node_id) = Self::find_node_index(
                    &mut node_id_index_map,
                    &neighbor_node_id.unwrap().0,
                    voxel_loader,
                );
                if new_node_id {
                    possible_node_neighbor_list.push(HashMap::default());
                    possible_block_neighbor_list.push(Vec::new());
                }

                let mapped_req_id = if mapped_req_index == NODE_ID_MAP_INDEX_NONE {
                    NodeID::empty()
                } else if mapped_req_index == NODE_ID_MAP_INDEX_ANY {
                    NodeID::any()
                } else {
                    node_id_index_map[mapped_req_index][0]
                };

                let possible_ids = possible_node_neighbor_list[node_id_index]
                    .entry(neighbor_offset)
                    .or_insert(Vec::new());

                if !possible_ids.contains(&mapped_req_id) {
                    possible_ids.push(mapped_req_id);
                }
            }

            // Neighbor Blocks
            let block_pos = (pos.as_ivec3() / 2) * 2;
            let in_block_pos = pos.as_ivec3() % 2;

            let mut possible_block_neighbors = Vec::new();
            for offset in get_neighbors() {
                let neighbor_offset = offset * 2;
                let neighbor_pos = block_pos + neighbor_offset;

                if neighbor_pos.is_negative_bitmask() != 0 {
                    continue;
                }

                let neighbor_block_index =
                    voxel_loader.block_positions.get(&neighbor_pos.as_uvec3());
                if neighbor_block_index.is_none() {
                    continue;
                }

                let block_neigbor_offset = neighbor_offset - in_block_pos;
                possible_block_neighbors.push((
                    block_neigbor_offset,
                    neighbor_block_index.unwrap().to_owned(),
                ));
            }

            possible_block_neighbor_list[node_id_index].push((possible_block_neighbors, *prio));
        }

        let mut permutated_possible_node_neighbor_list: Vec<HashMap<IVec3, Vec<NodeID>>> =
            Vec::new();
        let mut permutated_possible_block_neighbor_list: Vec<
            Vec<(Vec<(IVec3, BlockIndex)>, usize)>,
        > = Vec::new();
        let mut permutated_node_id_index_map = Vec::new();

        let mut affected_by_block = Vec::new();
        for _ in 0..voxel_loader.block_names.len() {
            affected_by_block.push(Vec::new())
        }

        let (rotations, flips) = Self::all_rotations_and_flips();
        for ((node_ids, node_req), block_reqs) in node_id_index_map
            .iter()
            .zip(possible_node_neighbor_list.iter())
            .zip(possible_block_neighbor_list.iter())
        {
            let node_id = &node_ids[0];

            // Nodes
            let flipped_rules = Self::flip_node_req(node_id, node_req, &flips);

            for (flipped_node_id, flipped_req) in flipped_rules {
                let rotated_rules =
                    Self::rotate_node_req(&flipped_node_id, &flipped_req, &rotations);

                for (permutated_node_id, permutated_req) in rotated_rules {
                    // Find node_id index
                    let (node_id_index, new_node_id) = Self::find_node_index(
                        &mut permutated_node_id_index_map,
                        &permutated_node_id,
                        voxel_loader,
                    );
                    if new_node_id {
                        permutated_possible_node_neighbor_list.push(HashMap::default());
                        permutated_possible_block_neighbor_list.push(Vec::new());
                    }

                    // Check if rule already was added
                    for (offset, req_ids) in permutated_req {
                        let inv_offset = offset * -1;
                        for req_id in req_ids.iter() {
                            let (mapped_req_index, new_node_id) = Self::find_node_index(
                                &mut permutated_node_id_index_map,
                                req_id,
                                voxel_loader,
                            );
                            if new_node_id {
                                permutated_possible_node_neighbor_list.push(HashMap::default());
                                permutated_possible_block_neighbor_list.push(Vec::new());
                            }

                            let mapped_req_id = if mapped_req_index == NODE_ID_MAP_INDEX_NONE {
                                NodeID::empty()
                            } else if mapped_req_index == NODE_ID_MAP_INDEX_ANY {
                                NodeID::any()
                            } else {
                                permutated_node_id_index_map[mapped_req_index][0]
                            };

                            let ids = permutated_possible_node_neighbor_list[node_id_index]
                                .entry(offset)
                                .or_default();
                            if !ids.contains(&mapped_req_id) {
                                ids.push(mapped_req_id);
                            }
                        }
                    }
                }
            }

            // Blocks
            for (req, prio) in block_reqs {
                let flipped_rules = Self::flip_block_req(&node_id, req, &flips);

                for (flipped_node_id, flipped_req) in flipped_rules {
                    let rotated_rules =
                        Self::rotate_block_req(&flipped_node_id, &flipped_req, &rotations);

                    for (permutated_node_id, permutated_req) in rotated_rules {
                        // Find node_id index

                        let (node_id_index, new_node_id) = Self::find_node_index(
                            &mut permutated_node_id_index_map,
                            &permutated_node_id,
                            voxel_loader,
                        );
                        if new_node_id {
                            permutated_possible_node_neighbor_list.push(HashMap::default());
                            permutated_possible_block_neighbor_list.push(Vec::new());
                        }

                        // Affected Blocks
                        for (offset, block_index) in permutated_req.iter() {
                            let inv_offset = *offset * -1;
                            if !affected_by_block[*block_index].contains(&inv_offset) {
                                affected_by_block[*block_index].push(inv_offset);
                            }
                        }

                        // Check if rule already was added
                        let mut found = false;
                        for (test_req, _) in
                            permutated_possible_block_neighbor_list[node_id_index].iter()
                        {
                            if *test_req == permutated_req {
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            permutated_possible_block_neighbor_list[node_id_index]
                                .push((permutated_req, *prio));
                        }
                    }
                }
            }
        }

        for possible_ids in permutated_possible_node_neighbor_list.iter_mut() {
            for (_, ids) in possible_ids.iter_mut() {
                let mut has_any = false;
                let mut has_empty = false;

                for id in ids.iter() {
                    has_empty |= id.is_empty();
                    has_any |= id.is_any();
                }

                if has_any {
                    ids.clear();
                    ids.push(NodeID::any());

                    if has_empty {
                        ids.push(NodeID::empty());
                    }
                }
            }
        }

        Rules {
            node_rules: permutated_possible_node_neighbor_list,
            block_rules: permutated_possible_block_neighbor_list,
            map_rules_index_to_node_id: permutated_node_id_index_map,
            affected_by_block,
        }
    }

    fn find_node_index(
        map_rules_index_to_node_id: &mut Vec<Vec<NodeID>>,
        node_id: &NodeID,
        voxel_loader: &VoxelLoader,
    ) -> (usize, bool) {
        if node_id.is_empty() {
            return (NODE_ID_MAP_INDEX_NONE, false);
        }

        if node_id.is_any() {
            return (NODE_ID_MAP_INDEX_ANY, false);
        }

        let r = map_rules_index_to_node_id
            .iter()
            .position(|test_ids| test_ids.contains(&node_id));

        let mut index = 0;
        let mut new_node_id = false;
        if r.is_none() {
            let mut found = false;
            for (i, ids) in map_rules_index_to_node_id.iter_mut().enumerate() {
                if Node::is_duplicate_node_id(&ids[0], node_id, voxel_loader) {
                    ids.push(node_id.to_owned());
                    index = i;
                    found = true;
                    break;
                }
            }

            if !found {
                map_rules_index_to_node_id.push(vec![node_id.to_owned()]);
                new_node_id = true;
                index = map_rules_index_to_node_id.len() - 1;
            }
        } else {
            index = r.unwrap()
        };

        (index, new_node_id)
    }

    fn all_rotations_and_flips() -> (Vec<BVec3>, Vec<BVec3>) {
        let all = vec![
            BVec3::new(false, false, false),
            BVec3::new(true, false, false),
            BVec3::new(false, true, false),
            BVec3::new(true, true, false),
            BVec3::new(false, false, true),
            BVec3::new(true, false, true),
            BVec3::new(false, true, true),
            BVec3::new(true, true, true),
        ];

        (all.to_owned(), all)
    }

    fn flip_node_req(
        node_id: &NodeID,
        node_req: &HashMap<IVec3, Vec<NodeID>>,
        flips: &Vec<BVec3>,
    ) -> Vec<(NodeID, HashMap<IVec3, Vec<NodeID>>)> {
        let mut rotated_rules = Vec::new();

        for flip in flips.iter() {
            let flip_a = ivec3(
                if flip.x { -1 } else { 1 },
                if flip.y { -1 } else { 1 },
                if flip.z { -1 } else { 1 },
            );

            let flipped_rot = node_id.rot.flip(flip.to_owned());

            let flippped_req: HashMap<_, _> = node_req
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
        node_req: &HashMap<IVec3, Vec<NodeID>>,
        rotates: &Vec<BVec3>,
    ) -> Vec<(NodeID, HashMap<IVec3, Vec<NodeID>>)> {
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

            let rotated_req: HashMap<_, _> = node_req
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

    fn flip_block_req(
        node_id: &NodeID,
        block_req: &Vec<(IVec3, BlockIndex)>,
        flips: &Vec<BVec3>,
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
        block_req: &Vec<(IVec3, BlockIndex)>,
        rotates: &Vec<BVec3>,
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
}
