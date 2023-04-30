
#[derive(Copy, Clone, PartialEq)]
pub struct Particle {
    pub material: u32,
    pub mass: u32,
    pub connections: [f32; 3],
}

impl Particle {
    pub fn new() -> Self {
        Self { 
            mass: 1,
            material: 1,
            connections: [1.0, 1.0, 1.0],
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self {  
            material: 0, 
            mass: 0,
            connections: [1.0, 1.0, 1.0],
        }
    }
}

