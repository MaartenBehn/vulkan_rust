use app::anyhow::*;
use app::glam::*;
use app::log;
use app::vulkan::Context;

use crate::math::to_1d;
use crate::math::to_1d_i;
use crate::node::BlockIndex;
use crate::node::NodeController;
use crate::node::NodeID;
use crate::node::BLOCK_INDEX_NONE;
use crate::node::NODE_INDEX_NONE;
use crate::pattern_config::Config;
use crate::ship_mesh::ShipMesh;

pub struct Ship {
    pub size: UVec3,
    pub max_index: isize,
    pub blocks: Vec<BlockIndex>,
    pub nodes: Vec<NodeID>,

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
            nodes: vec![NodeID::default(); ((size.x + 1) * (size.y + 1) * (size.z + 1)) as usize],
            mesh,
        };

        ship.place_node(uvec3(5, 5, 5), 0, node_controller)?;

        Ok(ship)
    }

    fn pos_in_bounds(&self, pos: IVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(self.size.as_ivec3()).all()
    }

    pub fn get_block_u(&self, pos: UVec3) -> Result<BlockIndex> {
        self.get_block_i(pos.as_ivec3())
    }

    pub fn get_block_i(&self, pos: IVec3) -> Result<BlockIndex> {
        if !self.pos_in_bounds(pos) {
            bail!("Pos not in ship")
        }

        let index = to_1d_i(pos, self.size.as_ivec3());
        Ok(self.blocks[index as usize])
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

        self.update_nodes(pos, node_controller);

        self.mesh
            .update(self.size, &self.blocks, &self.nodes, node_controller)?;

        Ok(())
    }

    pub fn get_neigbors(pos: IVec3) -> [IVec3; 8] {
        [
            pos + ivec3(0, 0, 0),
            pos + ivec3(1, 0, 0),
            pos + ivec3(0, 1, 0),
            pos + ivec3(1, 1, 0),
            pos + ivec3(0, 0, 1),
            pos + ivec3(1, 0, 1),
            pos + ivec3(0, 1, 1),
            pos + ivec3(1, 1, 1),
        ]
    }

    fn update_nodes(&mut self, pos: UVec3, node_controller: &NodeController) {
        let neigbors = Self::get_neigbors(pos.as_ivec3());

        for neigbor in neigbors {
            let config = self.get_node_config(neigbor);

            let bools: [bool; 8] = config.into();
            log::debug!("{:?}", bools);

            let index: usize = config.into();
            let patterns = &node_controller.pattern[index];

            let node_index = to_1d_i(neigbor, self.size.as_ivec3() + ivec3(1, 1, 1)) as usize;

            self.nodes[node_index] = if patterns.is_empty() {
                NodeID::default()
            } else {
                patterns[0].id
            }
        }
    }

    fn get_node_config(&mut self, node_pos: IVec3) -> Config {
        let blocks = Self::get_neigbors(node_pos - ivec3(1, 1, 1));

        let mut config = [false; 8];
        for (i, block) in blocks.iter().enumerate() {
            if block.is_negative_bitmask() != 0 || block.cmpge(self.size.as_ivec3()).any() {
                continue;
            }

            let block = self.get_block_i(*block).unwrap();
            config[i] = block != BLOCK_INDEX_NONE;
        }
        config.into()
    }
}
