use crate::math::get_neighbors;
use crate::node::{BlockIndex, NodeID};
use crate::rotation::Rot;
use crate::voxel_loader::VoxelLoader;
use octa_force::glam::{ivec3, BVec3, IVec3, Mat3, Mat4};
use std::collections::HashMap;

pub struct Rules {
    pub node_rules: Vec<HashMap<IVec3, Vec<NodeID>>>,
    pub block_rules: Vec<Vec<(HashMap<IVec3, BlockIndex>, usize)>>,
    pub map_rules_index_to_node_id: Vec<NodeID>,

    pub affected_by_block: Vec<Vec<IVec3>>,
    pub affected_by_node: HashMap<NodeID, Vec<IVec3>>,
}

impl Rules {
    pub fn new(voxel_loader: &VoxelLoader) -> Self {
        let mut possible_node_neighbor_list: Vec<HashMap<IVec3, Vec<NodeID>>> = Vec::new();
        let mut possible_block_neighbor_list: Vec<Vec<(HashMap<IVec3, BlockIndex>, usize)>> =
            Vec::new();
        let mut node_id_index_map = Vec::new();

        for (pos, (node_id, prio)) in voxel_loader.node_positions.iter() {
            if node_id.is_none() {
                // Dont add empty Nodes
                continue;
            }

            // Find node_id index
            let r = node_id_index_map
                .iter()
                .position(|test_id| test_id == node_id);
            let node_id_index = if r.is_none() {
                possible_node_neighbor_list.push(HashMap::default());
                possible_block_neighbor_list.push(Vec::new());

                node_id_index_map.push(node_id.to_owned());
                node_id_index_map.len() - 1
            } else {
                r.unwrap()
            };

            // Neighbor Nodes
            for offset in get_neighbors() {
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

                let possible_ids = possible_node_neighbor_list[node_id_index]
                    .entry(neighbor_offset)
                    .or_insert(Vec::new());

                possible_ids.push(neighbor_node_id.unwrap().to_owned().0);
            }

            // Neighbor Blocks
            let block_pos = (pos.as_ivec3() / 2) * 2;
            let in_block_pos = pos.as_ivec3() % 2;

            let mut possible_block_neighbors = HashMap::default();
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
                possible_block_neighbors.insert(
                    block_neigbor_offset,
                    neighbor_block_index.unwrap().to_owned(),
                );
            }

            possible_block_neighbor_list[node_id_index].push((possible_block_neighbors, *prio));
        }

        let mut permutated_possible_node_neighbor_list: Vec<HashMap<IVec3, Vec<NodeID>>> =
            Vec::new();
        let mut permutated_possible_block_neighbor_list: Vec<
            Vec<(HashMap<IVec3, BlockIndex>, usize)>,
        > = Vec::new();
        let mut permutated_node_id_index_map = Vec::new();

        let mut affected_by_block = Vec::new();
        for _ in 0..voxel_loader.block_names.len() {
            affected_by_block.push(Vec::new())
        }

        let mut affected_by_node: HashMap<NodeID, Vec<IVec3>> = HashMap::default();

        let (rotations, flips) = Self::all_rotations_and_flips();
        for ((node_id, node_req), block_reqs) in node_id_index_map
            .iter()
            .zip(possible_node_neighbor_list.iter())
            .zip(possible_block_neighbor_list.iter())
        {
            // Nodes
            let flipped_rules = Self::flip_node_req(&node_id, node_req, &flips);

            for (flipped_node_id, flipped_req) in flipped_rules {
                let rotated_rules =
                    Self::rotate_node_req(&flipped_node_id, &flipped_req, &rotations);

                for (permutated_node_id, permutated_req) in rotated_rules {
                    // Find node_id index
                    let r = permutated_node_id_index_map
                        .iter()
                        .position(|test_id| *test_id == permutated_node_id);
                    let node_id_index = if r.is_none() {
                        permutated_possible_node_neighbor_list.push(HashMap::default());
                        permutated_possible_block_neighbor_list.push(Vec::new());

                        permutated_node_id_index_map.push(permutated_node_id.to_owned());
                        permutated_node_id_index_map.len() - 1
                    } else {
                        r.unwrap()
                    };

                    // Affected by node
                    let mut affected = affected_by_node
                        .entry(permutated_node_id)
                        .or_insert(Vec::new());

                    // Check if rule already was added
                    for (offset, new_ids) in permutated_req {
                        let ids = permutated_possible_node_neighbor_list[node_id_index]
                            .entry(offset)
                            .or_default();

                        for id in new_ids {
                            if !ids.contains(&id) {
                                ids.push(id);
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
                        let r = permutated_node_id_index_map
                            .iter()
                            .position(|test_id| *test_id == permutated_node_id);
                        let node_id_index = if r.is_none() {
                            permutated_possible_node_neighbor_list.push(HashMap::default());
                            permutated_possible_block_neighbor_list.push(Vec::new());

                            permutated_node_id_index_map.push(permutated_node_id.to_owned());
                            permutated_node_id_index_map.len() - 1
                        } else {
                            r.unwrap()
                        };

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

        Rules {
            node_rules: permutated_possible_node_neighbor_list,
            block_rules: permutated_possible_block_neighbor_list,
            map_rules_index_to_node_id: permutated_node_id_index_map,
            affected_by_block,
            affected_by_node,
        }
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
                .map(|(pos, indecies)| {
                    let flipped_pos = (*pos) * flip_a;
                    (flipped_pos, indecies.to_owned())
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
                .map(|(pos, indecies)| {
                    let rotated_pos = pos_mat.transform_point3(pos.as_vec3()).round().as_ivec3();
                    (rotated_pos, indecies.to_owned())
                })
                .collect();

            rotated_rules.push((NodeID::new(node_id.index, rotated_rot), rotated_req));
        }

        rotated_rules
    }

    fn flip_block_req(
        node_id: &NodeID,
        block_req: &HashMap<IVec3, BlockIndex>,
        flips: &Vec<BVec3>,
    ) -> Vec<(NodeID, HashMap<IVec3, BlockIndex>)> {
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

            let flippped_req: HashMap<_, _> = block_req
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
        block_req: &HashMap<IVec3, BlockIndex>,
        rotates: &Vec<BVec3>,
    ) -> Vec<(NodeID, HashMap<IVec3, BlockIndex>)> {
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

            let rotated_req: HashMap<_, _> = block_req
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
