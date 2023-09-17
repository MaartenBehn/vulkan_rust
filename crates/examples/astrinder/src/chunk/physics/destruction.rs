use app::glam::{ivec2, vec2, IVec2, Vec2};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::chunk::chunk::Chunk;
use crate::chunk::particle::Particle;
use crate::chunk::{ChunkController, CHUNK_PART_SIZE};
use crate::math::transform::Transform;
use crate::math::{coord_to_hex, hex_to_coord, hex_to_particle_index};

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

            let break0 = contact_force_event.total_force_magnitude
                * self.settings.destruction_force_factor
                > self.chunks[chunk0_index].stability;
            let break1 = contact_force_event.total_force_magnitude
                * self.settings.destruction_force_factor
                > self.chunks[chunk1_index].stability;

            let pos0 = self.chunks[chunk0_index].transform.pos;
            let pos1 = self.chunks[chunk1_index].transform.pos;
            let pos_diff = pos1 - pos0;

            if break0 {
                let start_hex0 = coord_to_hex(pos0 + pos_diff);
                self.break_chunk(0, chunk0_index, start_hex0);
            }

            if break1 {
                let start_hex1 = coord_to_hex(pos1 - pos_diff);
                self.break_chunk(0, chunk1_index, start_hex1);
            }

            // self.step = 0;
        }
    }

    pub fn break_chunk(&mut self, pattern_index: usize, chunk_index: usize, start_hex: IVec2) {
        let mut new_particles = Vec::new();
        let pattern = &self.physics_controller.destruction_solver.patterns[pattern_index];

        new_particles.resize_with(pattern.points_len, || BreakParticlesIter::default());

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

                    let abs_hex = hex + part.pos * CHUNK_PART_SIZE;
                    let hex_pattern = half_size - start_hex + abs_hex;

                    let index = pattern.grid
                        [(hex_pattern.x * BREAK_PATTERN_SIZE as i32 + hex_pattern.y) as usize];

                    particle.material = index * 850;

                    new_particles[index as usize]
                        .particles
                        .push((particle, abs_hex));
                    new_particles[index as usize].offset += abs_hex;
                }
            }
        }

        let old_transform = chunk.transform;
        let old_vel_transform = self.physics_controller.get_velocity(chunk);
        self.remove_chunk(chunk_index);

        for mut iter in new_particles {
            if iter.is_empty() {
                continue;
            }

            iter.make_ready();

            let pos_offset = hex_to_coord(iter.offset);

            let mut transform = old_transform;
            transform.pos += pos_offset * 1.0001;

            self.add_chunk(transform, old_vel_transform, iter, false);
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

        self.add_chunk(transform, velocity_transform, particles.into_iter(), false)
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

#[derive(Clone)]
struct BreakParticlesIter {
    current: usize,
    pub particles: Vec<(Particle, IVec2)>,
    pub offset: IVec2,
}

impl BreakParticlesIter {
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn make_ready(&mut self) {
        self.offset /= self.particles.len() as i32;
    }
}

impl Default for BreakParticlesIter {
    fn default() -> Self {
        Self {
            current: 0,
            particles: Vec::new(),
            offset: IVec2::ZERO,
        }
    }
}

impl Iterator for BreakParticlesIter {
    type Item = (Particle, IVec2);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.particles.len() {
            let (p, mut pos) = self.particles[self.current];
            pos -= self.offset;

            self.current += 1;
            Some((p, pos))
        } else {
            None
        }
    }
}
