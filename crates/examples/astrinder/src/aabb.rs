use app::glam::UVec2;

#[derive(Copy, Clone, Debug, Default)]
pub struct AABB {
    pub min: UVec2,
    pub max: UVec2,
}

#[allow(dead_code)]
impl AABB {
    pub fn new(min: UVec2, max: UVec2) -> Self {
        Self { min, max }
    }

    pub fn is_inside(&self, pos: UVec2) -> bool {
        self.min.x <= pos.x && self.min.y <= pos.y 
        && pos.x <= self.max.x && pos.y <= self.max.y
    }

    pub fn extend(&mut self, pos: UVec2) {
        if self.min.x > pos.x || self.min.y > pos.y {
            self.min = pos;
        }
        else if pos.x > self.max.x || pos.y > self.max.y {
            self.max = pos;
        }
    }

    pub fn size(&self) -> UVec2 {
        self.max - self.min
    }
}

