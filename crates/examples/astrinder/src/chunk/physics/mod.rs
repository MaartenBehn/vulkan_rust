

use std::time::{Instant, Duration};

use app::glam::{Vec2};
use crate::chunk::CHUNK_PART_SIZE;
use crate::chunk::math::{world_pos_to_hex, };
use crate::chunk::physics::collide::CollisionSearch;

use super::math::vector2_to_vec2;
use super::{ChunkController, transform::Transform, math::{point2_to_vec2, cross2d}};

const ROTATION_DAMPING: f32 = 0.9;

const GRAVITY_ON: bool = false;
const GRAVITY_G: f32 = 0.01;
const GRAVITY_MAX_FORCE: f32 = 1.0;

const BREAKING_ON: bool = true;
pub const BREAK_COOL_DOWN: Duration = Duration::from_secs(100);

mod collide;
pub mod destruction;

impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32) {
        let mut accelerations = vec![Transform::default(); self.chunks.len()];
        let l = self.chunks.len();

        

        // Gravity
        if GRAVITY_ON && l >= 2 {
            for (i, chunk) in self.chunks.iter().enumerate() {
                for j in (i+1)..l {
                    let other_chunk = &self.chunks[j];
    
                    let gravity_force = get_gravity_force(
                        chunk.transform.pos, 
                        other_chunk.transform.pos, 
                        chunk.mass, 
                        other_chunk.mass);
                    
                    accelerations[i].pos -= gravity_force / chunk.mass;
                    accelerations[j].pos += gravity_force / other_chunk.mass;
                }
            }
        }

        for (i, chunk) in self.chunks.iter_mut().enumerate() {
            chunk.velocity_transform.rot *= ROTATION_DAMPING;

            chunk.velocity_transform += accelerations[i];
            chunk.transform += chunk.velocity_transform * time_step;
            chunk.on_transform_change();
        }

        if l < 2 {
            return;
        }

        let mut collision_search = CollisionSearch::new();
        loop {
            let result = collision_search.get_next_collision(&self.chunks);
            if result.is_none() {
                break;
            }

            let chunk0 = &self.chunks[collision_search.chunk0_index];
            let chunk1 = &self.chunks[collision_search.chunk1_index];

            let res = result.unwrap();
            let normal = vector2_to_vec2(res.normal).normalize();
            let point = point2_to_vec2(res.contact_point);

            // Resolve collsion
            let mass0_fraction = chunk0.mass / (chunk0.mass + chunk1.mass);
            let mass1_fraction = 1.0 - mass0_fraction;

            let offset0 = normal * mass1_fraction * -res.penetration_depth;
            let offset1 = normal * mass0_fraction * res.penetration_depth;

            // Collision Response
            let r_a = point - chunk0.transform.pos;
            let r_b = point - chunk1.transform.pos;
            let r_a_cross_n = cross2d(r_a, normal);
            let r_b_cross_n = cross2d(r_a, normal);
            let r_a_cross_n_2 = r_a_cross_n.powf(2.0);
            let r_b_cross_n_2 = r_b_cross_n.powf(2.0);

            let c = 0.0;

            let j = (1.0 - c) 
                    * (chunk0.velocity_transform.pos.dot(normal) - chunk1.velocity_transform.pos.dot(normal)
                    + chunk0.velocity_transform.rot * r_a_cross_n 
                    - chunk1.velocity_transform.rot * r_b_cross_n)
                    / (1.0 / chunk0.mass + 1.0 / chunk1.mass + r_a_cross_n_2)
                    / chunk0.moment_of_inertia 
                    + r_b_cross_n_2 / chunk1.moment_of_inertia;

            let apply = (j >= 0.0) as u8 as f32;

            let j_vec = normal * j;

            let vel0 = j_vec / chunk0.mass;
            let rot_vel0 = cross2d(r_a, j_vec) / chunk0.moment_of_inertia;

            let vel1 = j_vec / chunk1.mass;
            let rot_vel1 = cross2d(r_b, j_vec) / chunk1.moment_of_inertia;

            // Resolve collsion
            self.chunks[collision_search.chunk0_index].transform.pos += offset0;
            self.chunks[collision_search.chunk1_index].transform.pos += offset1;

            // Collision Response
            self.chunks[collision_search.chunk0_index].velocity_transform.pos -= vel0 * apply;
            self.chunks[collision_search.chunk0_index].velocity_transform.rot += rot_vel0 * apply;

            self.chunks[collision_search.chunk1_index].velocity_transform.pos += vel1 * apply;
            self.chunks[collision_search.chunk1_index].velocity_transform.rot -= rot_vel1 * apply; 

            self.chunks[collision_search.chunk0_index].on_transform_change();
            self.chunks[collision_search.chunk1_index].on_transform_change();
        }

        if BREAKING_ON {
            let old_chunks = self.chunks.clone();
            for entry in collision_search.log.iter() {
                let chunk0 = &old_chunks[entry.chunk0_index];
                let chunk1 = &old_chunks[entry.chunk1_index];

                let part0 = &chunk0.parts[entry.parts[0].0];
                let part1 = &chunk1.parts[entry.parts[0].1];
                
                let point = entry.points[0];
                let normal = entry.normals[0];

                let hex0 = world_pos_to_hex(part0.transform, point - normal * 0.5) + part0.pos * CHUNK_PART_SIZE;
                let hex1 = world_pos_to_hex(part1.transform, point + normal * 0.5) + part1.pos * CHUNK_PART_SIZE;

                let force = (chunk0.velocity_transform.pos - chunk1.velocity_transform.pos).length();

                let now = Instant::now();
                let break0 = chunk0.particle_counter > 1 
                    && force > 1.0 
                    && now.duration_since(chunk0.break_cool_down) > BREAK_COOL_DOWN;

                let break1 = chunk1.particle_counter > 1 
                    && force > 1.0
                    && now.duration_since(chunk1.break_cool_down) > BREAK_COOL_DOWN;

                assert!(entry.chunk0_index < entry.chunk1_index);
                if break1 {
                    self.remove_chunk(entry.chunk1_index);
                }
                if break0 {
                    self.remove_chunk(entry.chunk0_index);
                }

                if break0 {
                    let mut new_chunks_0 = self.destruction_solver.patterns[0].apply_to_chunk(chunk0, hex0, &mut self.part_id_counter);
                    self.chunks.append(&mut new_chunks_0);
                }

                if break1 {
                    let mut new_chunks_1 = self.destruction_solver.patterns[0].apply_to_chunk(chunk1, hex1, &mut self.part_id_counter);
                    self.chunks.append(&mut new_chunks_1);
                }
            }
        }
    }
}


fn get_gravity_force(pos0: Vec2, pos1: Vec2, mass0: f32, mass1: f32) -> Vec2 {
    let diff = pos0 - pos1;
    let dist = diff.length();
    let force = f32::min((GRAVITY_G * mass0 * mass1) / (dist * dist), GRAVITY_MAX_FORCE);

    diff * (1.0 / dist) * force
}





