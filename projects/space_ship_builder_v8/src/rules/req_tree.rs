use crate::rules::Prio;
use crate::world::data::block::Block;
use crate::world::data::node::NodeID;
use log::{debug, info};
use octa_force::glam::IVec3;

#[derive(Clone, Debug, Default)]
pub struct BroadReqTree {
    pub nodes: Vec<BroadReqTreeNode>,
    pub leafs: Vec<Vec<usize>>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct BroadReqTreeNode {
    pub offset: IVec3,
    pub positive_child: usize,
    pub negative_child: usize,
    pub positive_leaf: bool,
    pub negative_leaf: bool,
}

#[derive(Clone)]
struct BuildNode {
    ids: Vec<usize>,
    positive_child: Option<usize>,
    negative_child: Option<usize>,
    level: usize,
}

impl BroadReqTree {
    pub fn new(req_list: &[(Vec<(IVec3, Vec<Block>)>, Block, Prio)], index_offset: usize) -> Self {
        let offset_usage = Self::get_offset_usage(req_list);

        let mut build_nodes = vec![BuildNode {
            ids: (0..req_list.len()).collect(),
            negative_child: None,
            positive_child: None,
            level: 0,
        }];
        let mut current_build_node = 0;

        loop {
            if current_build_node % 1000 == 0 {
                debug!("Building Broad Req Tree Helper ... {current_build_node}");
            }

            if current_build_node >= build_nodes.len() {
                break;
            }

            let build_node = build_nodes[current_build_node].to_owned();

            if build_node.level >= offset_usage.len() {
                current_build_node += 1;
                continue;
            }

            let offset = offset_usage[build_node.level].0;

            let mut empty_ids = vec![];
            let mut some_ids = vec![];

            for &i in build_node.ids.iter() {
                let mut hast_offset = false;
                let mut empty_pass = false;
                let mut some_pass = false;

                for (req_offset, req_blocks) in req_list[i].0.iter() {
                    if *req_offset == offset {
                        hast_offset = true;
                        for req_block in req_blocks {
                            let req_empty =
                                *req_block == Block::from_single_node_id(NodeID::empty());

                            if !req_empty {
                                some_pass = true;
                            }
                            if req_empty {
                                empty_pass = true;
                            }
                        }
                    }
                }

                if empty_pass || !hast_offset {
                    empty_ids.push(i);
                }

                if some_pass || !hast_offset {
                    some_ids.push(i);
                }
            }

            if (!empty_ids.is_empty() || !some_ids.is_empty()) && empty_ids != some_ids {
                build_nodes[current_build_node].negative_child = Some(build_nodes.len());
                build_nodes.push(BuildNode {
                    ids: empty_ids,
                    positive_child: None,
                    negative_child: None,
                    level: build_node.level + 1,
                });

                build_nodes[current_build_node].positive_child = Some(build_nodes.len());
                build_nodes.push(BuildNode {
                    ids: some_ids,
                    positive_child: None,
                    negative_child: None,
                    level: build_node.level + 1,
                });
                current_build_node += 1;
            } else if !empty_ids.is_empty() && empty_ids == some_ids {
                build_nodes[current_build_node].level = build_node.level + 1;
            } else {
                current_build_node += 1;
            }
        }

        let mut nodes = vec![BroadReqTreeNode::default()];
        let mut leafs = vec![vec![]];
        let mut current_node = 0;
        let mut map_node_to_build_node = vec![0];

        loop {
            if current_node % 1000 == 0 {
                debug!("Building Broad Req Tree ... {current_node}");
            }

            if current_node >= nodes.len() {
                break;
            }

            let build_node = &build_nodes[map_node_to_build_node[current_node]];
            nodes[current_node].offset = offset_usage[build_node.level].0;

            assert!(build_node.positive_child.is_some() && build_node.negative_child.is_some());

            let negative_build_node = &build_nodes[build_node.negative_child.unwrap()];
            if negative_build_node.positive_child.is_none()
                || negative_build_node.negative_child.is_none()
            {
                nodes[current_node].negative_leaf = true;

                if negative_build_node.ids.is_empty() {
                    nodes[current_node].negative_child = 0;
                } else {
                    let new_leaf: Vec<usize> = negative_build_node
                        .ids
                        .iter()
                        .map(|i| i + index_offset)
                        .collect();

                    let leaf_index = leafs.iter().position(|l| **l == new_leaf);
                    if leaf_index.is_some() {
                        nodes[current_node].negative_child = leaf_index.unwrap();
                    } else {
                        nodes[current_node].negative_child = leafs.len();
                        leafs.push(new_leaf);
                    }
                }
            } else {
                nodes[current_node].negative_child = nodes.len();
                nodes.push(BroadReqTreeNode::default());
                map_node_to_build_node.push(build_node.negative_child.unwrap());
            }

            let positive_build_node = &build_nodes[build_node.positive_child.unwrap()];
            if positive_build_node.positive_child.is_none()
                || positive_build_node.negative_child.is_none()
            {
                nodes[current_node].positive_leaf = true;

                if positive_build_node.ids.is_empty() {
                    nodes[current_node].positive_child = 0;
                } else {
                    let new_leaf: Vec<usize> = positive_build_node
                        .ids
                        .iter()
                        .map(|i| i + index_offset)
                        .collect();

                    let leaf_index = leafs.iter().position(|l| **l == new_leaf);
                    if leaf_index.is_some() {
                        nodes[current_node].positive_child = leaf_index.unwrap();
                    } else {
                        nodes[current_node].positive_child = leafs.len();
                        leafs.push(new_leaf);
                    }
                }
            } else {
                nodes[current_node].positive_child = nodes.len();
                nodes.push(BroadReqTreeNode::default());
                map_node_to_build_node.push(build_node.positive_child.unwrap());
            }

            current_node += 1;
        }

        info!(
            "Broad Req Tree has {} nodes and {} leafs",
            nodes.len(),
            leafs.len()
        );

        BroadReqTree { nodes, leafs }
    }

    fn get_offset_usage(
        req_list: &[(Vec<(IVec3, Vec<Block>)>, Block, Prio)],
    ) -> Vec<(IVec3, usize)> {
        let mut offsets = vec![];

        for (reqs, _, _) in req_list {
            for (offset, _) in reqs {
                let counter = offsets.iter_mut().find_map(|(test_offset, counter)| {
                    if test_offset == offset {
                        Some(counter)
                    } else {
                        None
                    }
                });

                if counter.is_some() {
                    *counter.unwrap() += 1;
                } else {
                    offsets.push((*offset, 1))
                }
            }
        }

        offsets.sort_by(|(_, c1), (_, c2)| c2.cmp(c1));
        offsets
    }
}
