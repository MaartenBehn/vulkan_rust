use octa_force::glam::IVec3;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct AABB {
    pub min: IVec3,
    pub max: IVec3,
}

impl AABB {
    pub fn new(min: IVec3, max: IVec3) -> AABB {
        AABB { min, max }
    }

    pub fn collide(&self, aabb: &AABB) -> bool {
        (self.min[0] <= aabb.max[0] && self.max[0] >= aabb.min[0]) &&
        (self.min[1] <= aabb.max[1] && self.max[1] >= aabb.min[1]) &&
        (self.min[2] <= aabb.max[2] && self.max[2] >= aabb.min[2])
    }
}