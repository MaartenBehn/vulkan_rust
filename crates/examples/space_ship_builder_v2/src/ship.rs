use app::anyhow::*;
use app::glam::*;
use app::log;
use app::vulkan::Context;

use crate::math::to_1d;
use crate::math::to_1d_i;
use crate::node::Block;
use crate::node::BlockIndex;
use crate::node::NodeController;
use crate::node::NodeIndex;
use crate::node::BLOCK_INDEX_NONE;
use crate::ship_mesh::ShipMesh;

pub struct Ship {
    pub size: UVec3,
    pub max_index: isize,
    pub blocks: Vec<BlockIndex>,

    pub mesh: ShipMesh,
}

impl Ship {
    pub fn new(context: &Context, node_controller: &NodeController) -> Result<Ship> {
        let size = uvec3(10, 10, 10);
        let max_index = (size.x * size.y * size.z) as usize;

        let mesh = ShipMesh::new(context, max_index)?;

        let mut ship = Ship {
            size,
            max_index: max_index as isize,
            blocks: vec![BLOCK_INDEX_NONE; (size.x * size.y * size.z) as usize],
            mesh,
        };

        ship.place_node(uvec3(5, 5, 5), 0, node_controller)?;

        Ok(ship)
    }

    fn pos_in_bounds(&self, pos: IVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(self.size.as_ivec3()).all()
    }

    pub fn get_block_u(&self, pos: UVec3) -> Result<&BlockIndex> {
        self.get_block_i(pos.as_ivec3())
    }

    pub fn get_block_i(&self, pos: IVec3) -> Result<&BlockIndex> {
        if !self.pos_in_bounds(pos) {
            bail!("Pos not in ship")
        }

        let index = to_1d_i(pos, self.size.as_ivec3());
        Ok(&self.blocks[index as usize])
    }

    pub fn place_node(
        &mut self,
        pos: UVec3,
        block_index: BlockIndex,
        node_controller: &NodeController,
    ) -> Result<()> {
        log::info!("Place: {pos:?}");

        let cell_index = to_1d(pos, self.size);
        self.blocks[cell_index] = block_index;

        self.mesh.update(self.size, &self.blocks, node_controller)?;

        Ok(())
    }
}
