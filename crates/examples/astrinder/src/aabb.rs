use app::glam::{IVec2};

#[derive(Copy, Clone, Debug, Default)]
pub struct AABB {
    pub min: IVec2,
    pub max: IVec2,
}

#[allow(dead_code)]
impl AABB {
    pub fn new(min: IVec2, max: IVec2) -> Self {
        Self { min, max }
    }

    pub fn is_inside(&self, pos: IVec2) -> bool {
        self.min.x <= pos.x && self.min.y <= pos.y 
        && pos.x <= self.max.x && pos.y <= self.max.y
    }

    pub fn extend(&mut self, pos: IVec2) {
        if self.min.x > pos.x || self.min.y > pos.y {
            self.min = pos;
        }
        else if pos.x > self.max.x || pos.y > self.max.y {
            self.max = pos;
        }
    }

    pub fn size(&self) -> IVec2 {
        self.max - self.min
    }
}

