use crate::rules::marching_cubes::MarchingCubes;
use crate::rules::solver::{Solver, SolverCacheIndex, SolverFunctions};
use crate::rules::{Prio, Rules};
use crate::world::block_object::{BlockObject, ChunkIndex};
use crate::world::data::block::{Block, BlockIndex, BlockNameIndex};
use crate::world::data::node::NodeID;
use crate::world::data::voxel_loader::VoxelLoader;
use log::info;
use octa_force::{anyhow::Result, glam::IVec3};

const STONE_BLOCK_NAME: &str = "Stone";
const STONE_MARCHING_CUBES_NAME: &str = "Stone-Marching-Cubes";

const MARCHING_CUBES_CACHE_INDEX: usize = 0;

pub struct StoneSolver {
    pub block_name_index: BlockNameIndex,
    pub marching_cubes: MarchingCubes,
}

impl Rules {
    pub fn make_stone(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        info!("Making Stone");

        let stone_block_name_index = self.block_names.len() as BlockNameIndex;
        self.block_names.push(STONE_BLOCK_NAME.to_owned());

        let marching_cubes = MarchingCubes::new(
            self,
            voxel_loader,
            STONE_MARCHING_CUBES_NAME,
            stone_block_name_index,
        )?;
        let stone_solver = StoneSolver {
            block_name_index: stone_block_name_index,
            marching_cubes,
        };

        self.solvers.push(Solver::Stone(stone_solver));

        info!("Making Stone Done");
        Ok(())
    }
}

impl SolverFunctions for StoneSolver {
    fn block_check_reset(
        &self,
        block_object: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        _: IVec3,
    ) -> Vec<SolverCacheIndex> {
        if block_object.chunks[chunk_index].block_names[block_index] == self.block_name_index {
            vec![MARCHING_CUBES_CACHE_INDEX]
        } else {
            vec![]
        }
    }

    fn block_check(
        &self,
        _: &mut BlockObject,
        _: BlockIndex,
        _: ChunkIndex,
        _: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex> {
        cache
    }

    fn get_block(
        &self,
        block_object: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        pos: IVec3,
        _: Vec<SolverCacheIndex>,
    ) -> (Block, Prio, usize) {
        if block_object.chunks[chunk_index].block_names[block_index] != self.block_name_index {
            return (Block::from_single_node_id(NodeID::empty()), Prio::Empty, 0);
        }

        let block = self.marching_cubes.get_block(block_object, pos);
        (block, Prio::MarchingCubes, MARCHING_CUBES_CACHE_INDEX)
    }

    fn get_block_from_cache_index(&self, _: usize) -> Block {
        Block::from_single_node_id(NodeID::empty())
    }
}
