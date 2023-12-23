pub const PARTICLE_RADIUS: f32 = 10.0;

#[derive(Copy, Clone)]
pub struct Particle {
    pub material: u32,
    pub mass: f32,
}

impl Particle {
    pub fn new() -> Self {
        Self {
            mass: 1.0,
            material: 1,
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            material: 0,
            mass: 0.0,
        }
    }
}
