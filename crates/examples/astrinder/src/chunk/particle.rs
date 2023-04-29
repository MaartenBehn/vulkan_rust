use app::glam::Vec2;


#[derive(Copy, Clone, PartialEq)]
pub struct Particle {
    pub material: u32,
    pub mass: u32,
    pub velocity: Vec2,
}

impl Particle {
    pub fn new() -> Self {
        Self { 
            mass: 1,
            material: 1,
            velocity: Vec2::ZERO,
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self {  
            material: 0, 
            mass: 0,
            velocity: Vec2::ZERO,
        }
    }
}

