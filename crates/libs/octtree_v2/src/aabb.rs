use app::glam::IVec3;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct AABB {
    pub min: IVec3,
    pub max: IVec3,
}

impl AABB {
    pub fn new(min: IVec3, max: IVec3) -> AABB {
        AABB { min, max }
    }

    pub fn collide(&self, aabb: &AABB) -> bool {
        (self.min.x <= aabb.max.x && self.max.x >= aabb.min.x) &&
        (self.min.y <= aabb.max.y && self.max.y >= aabb.min.y) &&
        (self.min.z <= aabb.max.z && self.max.z >= aabb.min.z)
    }
}