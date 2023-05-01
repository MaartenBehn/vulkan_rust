
#[derive(Copy, Clone, PartialEq)]
pub struct Particle {
    pub material: u32,
    pub mass: u32,
}

impl Particle {
    pub fn new(mass: u32, material: u32) -> Self {
        Self { 
            mass,
            material,
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

