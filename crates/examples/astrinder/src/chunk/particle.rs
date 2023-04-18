
#[derive(Copy, Clone)]
pub struct Particle {
    pub material: u16,
    pub mass: u16,
}

impl Particle {
    pub fn new() -> Self {
        Self { 
            mass: 1,
            material: 1,
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self {  
            material: 0, 
            mass: 0,
        }
    }
}