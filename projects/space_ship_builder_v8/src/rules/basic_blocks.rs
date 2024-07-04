use crate::rotation::Rot;
use crate::rules::block::{Block, BlockIndex};
use crate::rules::solver::SolverCacheIndex;
use crate::rules::Prio::HullBase;
use crate::rules::{Prio, Rules};
use crate::ship::data::ShipData;
use crate::voxel_loader::VoxelLoader;
use octa_force::anyhow::Result;
use octa_force::glam::{IVec3, Mat4};
use octa_force::puffin_egui::puffin;

#[derive(Clone, Debug)]
pub struct BasicBlocks {
    blocks: Vec<(Vec<IVec3>, Block, Prio)>,

    #[cfg(debug_assertions)]
    pub debug_basic_blocks: Vec<(Vec<IVec3>, Block, Prio)>,
}

impl BasicBlocks {
    pub fn new(
        rules: &mut Rules,
        voxel_loader: &VoxelLoader,
        base_name_part: &str,
        folder: bool,
    ) -> Result<Self> {
        let hull_reqs = vec![(vec![], HullBase)];

        let mut base_blocks = vec![];
        for (i, (req, prio)) in hull_reqs.into_iter().enumerate() {
            let block = if folder {
                rules.load_block_from_node_folder(&format!("{base_name_part}-{i}"), voxel_loader)?
            } else {
                rules.load_block_from_multi_node_by_name(
                    &format!("{base_name_part}-{i}"),
                    voxel_loader,
                )?
            };

            base_blocks.push((req, block, prio));
        }

        let mut rotated_base_blocks = permutate_base_blocks(&base_blocks, rules);

        Ok(BasicBlocks {
            blocks: rotated_base_blocks,
            #[cfg(debug_assertions)]
            debug_basic_blocks: base_blocks,
        })
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn has_index(&self, index: usize) -> bool {
        index < self.blocks.len()
    }

    pub fn get_block(&self, index: usize) -> &(Vec<IVec3>, Block, Prio) {
        &self.blocks[index]
    }

    pub fn get_possible_blocks(
        &self,
        ship: &mut ShipData,
        world_block_pos: IVec3,
        block_name_index: BlockIndex,
    ) -> Vec<SolverCacheIndex> {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let test_block_name_index = ship.get_block_name_from_world_block_pos(world_block_pos);
        if test_block_name_index != block_name_index {
            return vec![];
        }

        let mut best_block_index = None;
        let mut best_prio = Prio::Zero;

        for (i, (reqs, _, prio)) in self.blocks.iter().enumerate() {
            let mut pass = true;
            for offset in reqs {
                let req_world_block_pos = world_block_pos + *offset;
                let test_block_name_index =
                    ship.get_block_name_from_world_block_pos(req_world_block_pos);

                if test_block_name_index != block_name_index {
                    pass = false;
                    break;
                }
            }

            if pass && best_prio < *prio {
                best_block_index = Some(i);
                best_prio = *prio;
            }
        }

        return if best_block_index.is_some() {
            vec![best_block_index.unwrap()]
        } else {
            vec![]
        };
    }
}

fn permutate_base_blocks(
    blocks: &[(Vec<IVec3>, Block, Prio)],
    rules: &mut Rules,
) -> Vec<(Vec<IVec3>, Block, Prio)> {
    let mut rotated_blocks = vec![];
    for (reqs, block, prio) in blocks.iter() {
        for rot in Rot::IDENTITY.get_all_permutations() {
            let mat: Mat4 = rot.into();
            let rotated_reqs: Vec<_> = reqs
                .iter()
                .map(|req| mat.transform_vector3((*req).as_vec3()).round().as_ivec3())
                .collect();

            let rotated_block = block.rotate(rot, rules);

            let mut found = false;
            for (_, test_block, _) in rotated_blocks.iter() {
                if *test_block == rotated_block {
                    found = true;
                    break;
                }
            }

            if !found {
                rotated_blocks.push((rotated_reqs, rotated_block, *prio))
            }
        }
    }

    rotated_blocks
}
