use app::glam::{UVec2, IVec2, Vec2, ivec2};

use super::{Chunk, particle::Particle};



#[allow(dead_code)]
impl Chunk {
    pub fn new_cube(pos: Vec2, rot: f32, size: UVec2) -> Self {

        let mut particles = Vec::new();

        for x in 0..size.x {
            for y in 0..size.y {

                let hex = UVec2::new(x, y);
                particles.push((Particle::new(), hex.as_ivec2()))
            }
        }

        Self::new(pos, rot, particles)
    }

    pub fn new_circle(pos: Vec2, rot: f32, layers: u32) -> Self {

        let mut particles = Vec::new();

        let hex_dirs = [
            IVec2::new(-1, 1),
            IVec2::new(-1, 0),
            IVec2::new(0, -1),

            IVec2::new(1, -1),
            IVec2::new(1, 0),
            IVec2::new(0, 1),
            ];

        let mut hex = IVec2::ZERO;
        particles.push((Particle::new(), hex));

        for layer in 1..=layers {
            hex = ivec2(layer as i32, 0);

            for dir in 0..6 {
                for _ in 0..layer {
                    hex += hex_dirs[dir];
                    particles.push((Particle::new(), hex));
                }
            }
        }

        Self::new(pos, rot, particles)
    }
}