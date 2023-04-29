use std::collections::VecDeque;

use app::glam::{Vec2, vec2, IVec2};
use app::anyhow::*;

use crate::chunk::math::{world_pos_to_hex, hex_to_particle_index};
use crate::chunk::physics::collide::CollisionSearch;

use super::chunk::ChunkPart;
use super::math::vector2_to_vec2;
use super::{ChunkController, Chunk, transform::Transform, math::{part_corners, point2_to_vec2, cross2d}};

const GRAVITY_G: f32 = 0.01;
const GRAVITY_MAX_FORCE: f32 = 1.0;
const GRAVITY_ON: bool = false;

mod collide;


impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32) {
        let mut accelerations = vec![Transform::default(); self.chunks.len()];
        let l = self.chunks.len();

        // Gravity
        if GRAVITY_ON {
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
            chunk.velocity_transform += accelerations[i];
            chunk.transform += chunk.velocity_transform * time_step;
            chunk.on_transform_change();
        }

        // Collision
        assert!(self.chunks.len() >= 2);
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

            self.chunks[collision_search.chunk0_index].transform.pos += offset0;
            self.chunks[collision_search.chunk1_index].transform.pos += offset1;

            self.chunks[collision_search.chunk0_index].on_transform_change();
            self.chunks[collision_search.chunk1_index].on_transform_change();
        }

        for entry in collision_search.log.iter() {
            for (i, (part0_index, part1_index)) in entry.parts.iter().enumerate() {
                let chunk0 = &self.chunks[entry.chunk0_index];
                let chunk1 = &self.chunks[entry.chunk1_index];

                let part0 = &chunk0.parts[*part0_index];
                let part1 = &chunk1.parts[*part1_index];
                
                let point = entry.points[i];
                let normal = entry.normals[i];

                let hex_in_part0 = world_pos_to_hex(part0.transform, point - normal * 0.5);
                let hex_in_part1 = world_pos_to_hex(part1.transform, point + normal * 0.5);

                let part0_pos = part0.pos;
                let part1_pos = part1.pos;

                let velocity0 = chunk0.velocity_transform.pos;
                let velocity1 = chunk1.velocity_transform.pos;

                destruction(
                    &mut self.chunks,
                    entry.chunk0_index, 
                    hex_in_part0, 
                    part0_pos, 
                    *part0_index, 
                    velocity1);

                destruction(
                    &mut self.chunks,
                    entry.chunk1_index, 
                    hex_in_part1, 
                    part1_pos, 
                    *part1_index, 
                    velocity0);

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

fn destruction(
    chunks: &mut Vec<Chunk>,
    chunk_index: usize, 
    start_hex: IVec2, 
    start_part_pos: IVec2, 
    start_part_index: usize, 
    velocity: Vec2
) { 
    chunks[chunk_index].parts[start_part_index].particles[hex_to_particle_index(start_hex)].velocity = velocity;

    let mut all_particles = Vec::new();
    let mut neigbor_deque = VecDeque::new();

    all_particles.push((start_hex, start_part_index, 0));
    neigbor_deque.push_back((start_hex, start_part_pos, start_part_index, 0));

    loop {
        let (hex, part_pos, part_index, depth) = {
            let res = neigbor_deque.pop_front();
            if res.is_none() {
                break;
            }
            res.unwrap()
        };
        let particle = chunks[chunk_index].parts[part_index].particles[hex_to_particle_index(hex)];

        for (n_hex, n_part_index) in chunks[chunk_index].get_neigbor_particles_pos(part_pos, part_index, hex) {
            
            let mut add_to_deque = true;
            let mut apply_velocity = true;
            for (test_hex, test_part_index, test_depth) in all_particles.iter() {
                if n_hex == *test_hex && n_part_index == *test_part_index {
                    add_to_deque = false;

                    apply_velocity = *test_depth <= depth;
                    break;
                }
            }

            if apply_velocity {
                if add_to_deque {
                    chunks[chunk_index].parts[n_part_index].particles[hex_to_particle_index(n_hex)].velocity = Vec2::ZERO
                }

                chunks[chunk_index].parts[n_part_index].particles[hex_to_particle_index(n_hex)].velocity += particle.velocity * 0.5;
            }
            
            if add_to_deque {
                all_particles.push((n_hex, n_part_index, depth + 1));
                neigbor_deque.push_back((n_hex, chunks[chunk_index].parts[n_part_index].pos, n_part_index, depth + 1));
            }
        }
        
    }

}
