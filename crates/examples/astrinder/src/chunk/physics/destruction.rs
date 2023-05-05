use app::glam::{ivec2, vec2, IVec2, Vec2};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::chunk::chunk::Chunk;
use crate::chunk::particle::Particle;
use crate::chunk::{ChunkController, IdCounter, CHUNK_PART_SIZE};
use crate::math::transform::{self, Transform};
use crate::math::{hex_in_chunk_frame, hex_to_coord, hex_to_particle_index};
use crate::settings::Settings;

use super::PhysicsController;

const BREAK_PATTERN_SIZE: usize = 100;

pub enum FallOffFunc {
    Linear(f32),
    Quadratic(f32, f32),
    Qubic(f32, f32, f32),
}

pub struct DestructionSolver {
    pub patterns: Vec<BreakPattern>,
}

impl DestructionSolver {
    pub fn new() -> Self {
        let patterns = vec![BreakPattern::new(
            100,
            FallOffFunc::Qubic(50.0, 0.0, 0.0),
            2,
        )];

        Self { patterns }
    }
}

#[derive(Clone, Debug)]
pub struct BreakPattern {
    points_len: usize,
    grid: Vec<u32>,
}

impl BreakPattern {
    pub fn new(points_num: usize, fall_off_func: FallOffFunc, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut points = Vec::with_capacity(points_num);

        for _ in 0..points_num {
            let rand_point = vec2(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0));

            let point = match fall_off_func {
                FallOffFunc::Linear(a) => apply_linear_fall_off(rand_point, a),
                FallOffFunc::Quadratic(a, b) => apply_quadratic_fall_off(rand_point, a, b),
                FallOffFunc::Qubic(a, b, c) => apply_quibic_fall_off(rand_point, a, b, c),
            };

            points.push(point)
        }

        let mut grid = Vec::new();
        let half_size = (BREAK_PATTERN_SIZE / 2) as i32;

        grid.resize(BREAK_PATTERN_SIZE * BREAK_PATTERN_SIZE, u32::default());
        for x in 0..BREAK_PATTERN_SIZE as i32 {
            for y in 0..BREAK_PATTERN_SIZE as i32 {
                let hex = ivec2(x, y);
                let pos = hex_to_coord(hex - half_size);

                let mut closest_index = 0;
                let mut closest_dist = f32::MAX;

                for (i, point) in points.iter().enumerate() {
                    let dist = pos.distance(*point);

                    if dist < closest_dist {
                        closest_index = i;
                        closest_dist = dist;
                    }
                }

                grid[(hex.x * BREAK_PATTERN_SIZE as i32 + hex.y) as usize] = closest_index as u32;
            }
        }

        Self {
            points_len: points.len(),
            grid,
        }
    }
}

impl ChunkController {
    pub fn check_break(&mut self) {
        while let Ok(contact_force_event) = self.physics_controller.contact_force_recv.try_recv() {
            if contact_force_event.total_force_magnitude < self.settings.min_destruction_force {
                continue;
            }

            let rigid_body_set = &self.physics_controller.rigid_body_set;
            let collider_set = &self.physics_controller.collider_set;

            let handle0 = collider_set.get(contact_force_event.collider1);
            let handle1 = collider_set.get(contact_force_event.collider2);

            if handle0.is_none() || handle1.is_none() {
                continue;
            }

            let rb0 = &rigid_body_set[handle0.unwrap().parent().unwrap()];
            let rb1 = &rigid_body_set[handle1.unwrap().parent().unwrap()];

            let chunk0_index = rb0.user_data as usize;
            let chunk1_index = rb1.user_data as usize;

            self.break_chunk(0, chunk0_index, ivec2(0, 0));

            self.break_chunk(0, chunk1_index, ivec2(0, 0));
        }
    }

    pub fn break_chunk(&mut self, pattern_index: usize, chunk_index: usize, start_hex: IVec2) {
        let mut new_particles = Vec::new();
        let pattern = &self.physics_controller.destruction_solver.patterns[pattern_index];

        new_particles.resize_with(pattern.points_len, || (Vec::new(), Vec2::ZERO));

        let chunk = &self.chunks[chunk_index];
        let half_size = (BREAK_PATTERN_SIZE / 2) as i32;
        for part in chunk.parts.iter() {
            for x in 0..CHUNK_PART_SIZE {
                for y in 0..CHUNK_PART_SIZE {
                    let hex = ivec2(x, y);
                    let mut particle = part.particles[hex_to_particle_index(hex)];
                    if particle.mass == 0 {
                        continue;
                    }

                    let hex_pattern = half_size - start_hex + hex + part.pos * CHUNK_PART_SIZE;
                    let index = pattern.grid
                        [(hex_pattern.x * BREAK_PATTERN_SIZE as i32 + hex_pattern.y) as usize];

                    particle.material = index * 850;

                    new_particles[index as usize]
                        .0
                        .push((particle, hex + part.pos * CHUNK_PART_SIZE));
                    new_particles[index as usize].1 += hex_in_chunk_frame(hex, part.pos);
                }
            }
        }

        let vel_transform = self.physics_controller.get_velocity(chunk);

        self.remove_chunk(chunk_index);

        for (particles, pos_offset) in new_particles {
            if particles.is_empty() {
                continue;
            }

            let mut transform = self.chunks[chunk_index].transform;
            transform.pos += pos_offset * 0.0001;

            self.add_chunk(transform, vel_transform, particles, false);
        }
    }

    #[allow(dead_code)]
    pub fn chunk_from_break_pattern(
        &mut self,
        pattern_index: usize,
        transform: Transform,
        velocity_transform: Transform,
    ) -> &Chunk {
        let grid = &self.physics_controller.destruction_solver.patterns[pattern_index].grid;
        let mut particles = Vec::with_capacity(grid.len());
        for x in 0..BREAK_PATTERN_SIZE as i32 {
            for y in 0..BREAK_PATTERN_SIZE as i32 {
                let hex = ivec2(x, y);
                let material = grid[(hex.x * BREAK_PATTERN_SIZE as i32 + hex.y) as usize];

                particles.push((Particle::new(1, material * 850), hex))
            }
        }

        self.add_chunk(transform, velocity_transform, particles, false)
    }
}

fn apply_linear_fall_off(pos: Vec2, a: f32) -> Vec2 {
    a * pos
}

fn apply_quadratic_fall_off(pos: Vec2, a: f32, b: f32) -> Vec2 {
    let l = pos.length();
    a * l * pos + b * pos
}

fn apply_quibic_fall_off(pos: Vec2, a: f32, b: f32, c: f32) -> Vec2 {
    let l = pos.length();
    a * l * l * pos + b * l * pos + c * pos
}
