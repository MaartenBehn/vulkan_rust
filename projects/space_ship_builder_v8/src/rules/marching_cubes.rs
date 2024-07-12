use crate::math::rotation::Rot;
use crate::math::{get_neighbors, oct_positions, oct_positions_with_minus};
use crate::rules::empty::EMPTY_BLOCK_NAME_INDEX;
use crate::rules::Rules;
use crate::world::block_object::BlockObject;
use crate::world::data::block::{Block, BlockNameIndex};
use crate::world::data::node::NodeID;
use crate::world::data::voxel_loader::VoxelLoader;
use log::{debug, trace, warn};
use octa_force::anyhow::bail;
use octa_force::anyhow::Result;
use octa_force::glam::{ivec3, vec3, IVec3, Mat4};

pub struct MarchingCubes {
    block_name_index: BlockNameIndex,
    node_reqs: Vec<(NodeReq, NodeID)>,
}

impl MarchingCubes {
    pub fn new(
        rules: &mut Rules,
        voxel_loader: &VoxelLoader,
        folder_name: &str,
        block_name_index: BlockNameIndex,
    ) -> Result<Self> {
        let nodes = rules.load_nodes_in_folder(folder_name, voxel_loader)?;

        let configs = [
            ("01", [0, 0, 0, 0, 0, 0, 0, 0]),
            ("11", [0, 0, 0, 1, 0, 0, 0, 0]),
            ("21", [0, 0, 1, 1, 0, 0, 0, 0]),
            ("22", [0, 1, 1, 0, 0, 0, 0, 0]),
            ("23", [0, 0, 0, 1, 0, 1, 0, 0]),
            ("24", [0, 0, 0, 1, 1, 0, 0, 0]),
            ("31", [0, 1, 1, 1, 0, 0, 0, 0]),
            ("32", [0, 0, 1, 1, 0, 1, 0, 0]),
            ("33", [0, 1, 1, 0, 1, 0, 0, 0]),
            ("41", [1, 1, 1, 1, 0, 0, 0, 0]),
            ("42", [1, 0, 1, 1, 0, 1, 0, 0]),
            ("43", [0, 1, 1, 1, 0, 0, 0, 1]),
            ("44", [0, 0, 1, 1, 1, 1, 0, 0]),
            ("45", [1, 1, 0, 1, 1, 0, 0, 0]),
            ("46", [1, 0, 0, 1, 0, 1, 1, 0]),
            ("51", [1, 1, 1, 1, 0, 1, 0, 0]),
            ("52", [0, 1, 1, 1, 0, 1, 1, 0]),
            ("53", [1, 1, 0, 1, 0, 1, 1, 0]),
            ("61", [1, 1, 1, 1, 0, 0, 1, 1]),
            ("62", [1, 1, 1, 1, 0, 1, 1, 0]),
            ("63", [1, 1, 1, 0, 0, 1, 1, 1]),
            ("71", [1, 1, 1, 1, 0, 1, 1, 1]),
            ("81", [1, 1, 1, 1, 1, 1, 1, 1]),
        ];

        let mut node_reqs: Vec<(NodeReq, NodeID)> = vec![];
        for (node_id, _, name) in nodes.into_iter() {
            let name_parts: Vec<_> = name.split('-').collect();

            let config = configs.iter().find(|(name, _)| name_parts[1] == *name);
            if config.is_none() {
                bail!("Name Part 1 of {} is not a valid config name.", name);
            }
            let (_, config_reqs) = config.unwrap();

            let reqs: Vec<_> = config_reqs
                .iter()
                .zip(oct_positions().into_iter())
                .map(|(val, offset)| (offset, *val != 0))
                .collect();

            // Permutate Node Req
            for rot in Rot::IDENTITY.get_all_permutations() {
                let mat: Mat4 = rot.into();
                let rotated_reqs: Vec<_> = reqs
                    .iter()
                    .map(|(offset, val)| {
                        let p = vec3(
                            if offset.x == 1 { 1.0 } else { -1.0 },
                            if offset.y == 1 { 1.0 } else { -1.0 },
                            if offset.z == 1 { 1.0 } else { -1.0 },
                        );
                        let rotated_p = mat.transform_vector3(p).round().as_ivec3();
                        let rotated_offset = ivec3(
                            if rotated_p.x == 1 { 1 } else { 0 },
                            if rotated_p.y == 1 { 1 } else { 0 },
                            if rotated_p.z == 1 { 1 } else { 0 },
                        );

                        (rotated_offset, *val)
                    })
                    .collect();

                let rotated_node_id = NodeID::new(node_id.index, node_id.rot * rot);
                let rotated_node_id = rules.get_duplicate_node_id(rotated_node_id);

                let node_req = NodeReq::from(rotated_reqs);
                let res = node_reqs
                    .binary_search_by(|(test_node_req, _)| test_node_req.0.cmp(&node_req.0));

                if res.is_err() {
                    let insert_index = res.err().unwrap();
                    node_reqs.insert(insert_index, (node_req, rotated_node_id))
                }
            }
        }

        for i in 0..u8::MAX {
            if node_reqs
                .iter()
                .find(|(node_req, _)| node_req.0 == i)
                .is_none()
            {
                let req: Vec<_> = NodeReq(i).into_iter().collect();

                warn!("{req:?} missing.");
            }
        }

        Ok(MarchingCubes {
            block_name_index,
            node_reqs,
        })
    }

    pub fn get_block(&self, block_object: &mut BlockObject, world_block_pos: IVec3) -> Block {
        //debug!("World Block Pos: {world_block_pos:?}");

        let node_pos = block_object.get_node_pos_from_block_pos(world_block_pos);

        let node_ids = oct_positions().map(|offset| {
            let pos = node_pos + offset;

            /*
                      |
                      |
                  X---x---X
                  |   |   |
            ------x---x---x-----
                  |   | / |
                  X---x---O
                      |
                      |
             */

            let test_reqs = oct_positions().map(|offset| {
                let req_pos = pos + offset - 1;

                let in_block = (req_pos % 2).cmpeq(IVec3::ZERO);

                if in_block.all() {
                    let block_pos = req_pos / 2;
                    let req_block_name_index =
                        block_object.get_block_name_from_world_block_pos(block_pos);

                    req_block_name_index != EMPTY_BLOCK_NAME_INDEX
                } else {
                    let mut test_offset = |test_offset: IVec3| {
                        let block_pos = (req_pos + test_offset) / 2;
                        let req_block_name_index =
                            block_object.get_block_name_from_world_block_pos(block_pos);
                        req_block_name_index != EMPTY_BLOCK_NAME_INDEX
                    };

                    // TODO Clean up
                    if !in_block.x && !in_block.y && !in_block.z {
                        return (test_offset(ivec3(1, 1, 1)) && test_offset(ivec3(-1, -1, -1)))
                            || (test_offset(ivec3(1, 1, -1)) && test_offset(ivec3(-1, -1, 1)))
                            || (test_offset(ivec3(1, -1, -1)) && test_offset(ivec3(-1, 1, 1)))
                            || (test_offset(ivec3(1, -1, 1)) && test_offset(ivec3(-1, 1, -1)));
                    }

                    if !in_block.x && !in_block.y {
                        return (test_offset(ivec3(1, 1, 0)) && test_offset(ivec3(-1, -1, 0)))
                            || (test_offset(ivec3(1, -1, 0)) && test_offset(ivec3(-1, 1, 0)));
                    }

                    if !in_block.x && !in_block.z {
                        return (test_offset(ivec3(1, 0, 1)) && test_offset(ivec3(-1, 0, -1)))
                            || (test_offset(ivec3(1, 0, -1)) && test_offset(ivec3(-1, 0, 1)));
                    }

                    if !in_block.y && !in_block.z {
                        return (test_offset(ivec3(0, 1, 1)) && test_offset(ivec3(0, -1, -1)))
                            || (test_offset(ivec3(0, 1, -1)) && test_offset(ivec3(0, -1, 1)));
                    }

                    if !in_block.x {
                        return test_offset(ivec3(1, 0, 0)) && test_offset(ivec3(-1, 0, 0));
                    }

                    if !in_block.y {
                        return test_offset(ivec3(0, 1, 0)) && test_offset(ivec3(0, -1, 0));
                    }

                    if !in_block.z {
                        return test_offset(ivec3(0, 0, 1)) && test_offset(ivec3(0, 0, -1));
                    }

                    false
                }
            });

            let test_node_req = NodeReq::from(test_reqs);

            self.node_reqs[test_node_req.0 as usize].1
        });

        Block::from_node_ids(node_ids)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct NodeReq(u8);

impl From<Vec<(IVec3, bool)>> for NodeReq {
    fn from(value: Vec<(IVec3, bool)>) -> Self {
        let mut node_req = NodeReq(0);

        for (offset, val) in value.into_iter() {
            #[cfg(debug_assertions)]
            let mut found = false;

            for (i, test_offset) in oct_positions().into_iter().enumerate() {
                if test_offset == offset {
                    node_req.0 |= (val as u8) << i;

                    #[cfg(debug_assertions)]
                    {
                        found = true;
                    }

                    break;
                }
            }

            #[cfg(debug_assertions)]
            if !found {
                panic!("Invalid Vector as input.")
            }
        }

        node_req
    }
}

impl From<[bool; 8]> for NodeReq {
    fn from(value: [bool; 8]) -> Self {
        let mut node_req = NodeReq(0);

        for (i, val) in value.into_iter().enumerate() {
            node_req.0 |= (val as u8) << i;
        }

        node_req
    }
}

pub struct NodeReqIterator {
    node_req: NodeReq,
    mask: u8,
}

impl IntoIterator for NodeReq {
    type Item = bool;
    type IntoIter = NodeReqIterator;

    fn into_iter(self) -> Self::IntoIter {
        NodeReqIterator {
            node_req: self,
            mask: 1,
        }
    }
}

impl Iterator for NodeReqIterator {
    type Item = bool;
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }

        let result = (self.node_req.0 & self.mask) != 0;

        self.mask = self.mask << 1;
        Some(result)
    }
}
