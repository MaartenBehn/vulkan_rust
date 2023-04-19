use app::glam::{Vec2};

#[derive(Copy, Clone, Debug, Default)]
pub struct AABB {
    pub min: Vec2,
    pub max: Vec2,
}

#[allow(dead_code)]
impl AABB {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn is_inside(&self, pos: Vec2) -> bool {
        self.min.x <= pos.x && self.min.y <= pos.y && 
        pos.x <= self.max.x && pos.y <= self.max.y
    }

    pub fn is_inside_aabb(&self, other: AABB) -> bool {
        self.min.x <= other.min.x && self.min.y <= other.min.y && 
        other.max.x <= self.max.x && other.max.y <= self.max.y
    }

    pub fn collides(&self, other: AABB) -> bool {
        self.min.x <= other.max.x && other.min.x <= self.max.x && 
        self.min.y <= other.max.y && other.min.y <= self.max.y
    }

    pub fn extend(&mut self, pos: Vec2) {
        if self.min.x > pos.x || self.min.y > pos.y {
            self.min = pos;
        }
        else if pos.x > self.max.x || pos.y > self.max.y {
            self.max = pos;
        }
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }
}

