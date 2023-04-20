use app::glam::{UVec2, IVec2, ivec2};
use noise::{core::perlin::perlin_2d, permutationtable::PermutationTable};

use super::{Chunk, particle::Particle, transform::Transform};

#[allow(dead_code)]
impl Chunk {
    pub fn new_cube(trans: Transform, vel_trans: Transform, size: UVec2) -> Self {

        let mut particles = Vec::new();

        for x in 0..size.x {
            for y in 0..size.y {

                let hex = UVec2::new(x, y);
                particles.push((Particle::new(), hex.as_ivec2()))
            }
        }

        Self::new(trans, vel_trans, particles)
    }

    pub fn new_hexagon(trans: Transform, vel_trans: Transform, layers: usize) -> Self {

        let points = hexagon_points(layers);
        let mut particles = Vec::new();
        for point in points {
            particles.push((Particle::new(), point))
        }

        Self::new(trans, vel_trans, particles)
    }

    pub fn new_noise_hexagon(trans: Transform, vel_trans: Transform, layers: usize) -> Self {
        let points = hexagon_points(layers);
        let mut particles = Vec::new();

        let hasher = PermutationTable::new(3);

        for point in points {
            if perlin_2d((point.as_dvec2() * 0.1).into(), &hasher) < 0.0 {
                particles.push((Particle::new(), point))
            }
        }

        Self::new(trans, vel_trans, particles)
    }
}

fn hexagon_points(layers: usize) -> Vec<IVec2> {

    let mut points = Vec::new();

    let hex_dirs = [
        IVec2::new(-1, 1),
        IVec2::new(-1, 0),
        IVec2::new(0, -1),

        IVec2::new(1, -1),
        IVec2::new(1, 0),
        IVec2::new(0, 1),
        ];

    let mut hex = IVec2::ZERO;
    points.push(hex);

    for layer in 1..=layers {
        hex = ivec2(layer as i32, 0);

        for dir in 0..6 {
            for _ in 0..layer {
                hex += hex_dirs[dir];
                points.push(hex);
            }
        }
    }

    points
}