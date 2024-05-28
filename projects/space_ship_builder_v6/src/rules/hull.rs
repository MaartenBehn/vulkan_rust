use crate::math::all_sides_dirs;
use crate::node::NodeID;
use crate::rules;
use crate::rules::block_preview::BlockPreview;
use crate::rules::solver::Solver;
use crate::rules::Rules;
use crate::ship::Ship;
use crate::voxel_loader::VoxelLoader;
use log::{debug, info};
use octa_force::glam::IVec3;
use std::process::id;

pub struct HullSolver {
    pub node_ids: Vec<NodeID>,
    pub node_reqs: Vec<Vec<(IVec3, Vec<NodeID>)>>,
}

impl Rules {
    pub fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> octa_force::anyhow::Result<()> {
        debug!("Making Hull");

        self.block_names.push("Hull".to_owned());

        let mut node_ids = vec![];
        let max_hull_node = 8;
        for i in 1..=max_hull_node {
            let node_id = self.add_node(&format!("Hull-{i}"), voxel_loader)?;
            node_ids.push(node_id);
        }

        self.block_previews
            .push(BlockPreview::from_single_node_id(node_ids[0]));

        let hull_solver = HullSolver::new(node_ids, self);
        self.solvers.push(Box::new(hull_solver));

        debug!("Making Hull Done");
        Ok(())
    }
}

impl HullSolver {
    pub fn new(node_ids: Vec<NodeID>, rules: &mut Rules) -> Self {
        let node_reqs = get_matching_sides_reqs(&node_ids, rules);

        Self {
            node_ids,
            node_reqs,
        }
    }
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
                        let new_node_id = rules
                            .get_duplicate_node_id(NodeID::new(test_node_id.index, permutated_rot));

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

impl Solver for HullSolver {
    fn block_check(&mut self, ship: &Ship, node_pos: IVec3, node_index: usize, chunk_index: usize) {
        todo!()
    }

    fn node_check(&mut self, ship: &Ship, node_pos: IVec3, node_index: usize, chunk_index: usize) {
        todo!()
    }
}
