use crate::world::block_object::BlockObject;
use octa_force::glam::IVec3;

pub struct Region {
    pub pos: IVec3,
    pub loaded_objects: Vec<BlockObject>,
}

impl Region {
    pub fn new(pos: IVec3) -> Region {
        Region {
            pos,
            loaded_objects: vec![],
        }
    }
}
