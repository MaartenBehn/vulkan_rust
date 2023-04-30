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

            // Collision Response
            let r_a = point - chunk0.transform.pos;
            let r_b = point - chunk1.transform.pos;
            let r_a_cross_n = cross2d(r_a, normal);
            let r_b_cross_n = cross2d(r_a, normal);
            let r_a_cross_n_2 = r_a_cross_n.powf(2.0);
            let r_b_cross_n_2 = r_b_cross_n.powf(2.0);

            let c = 1.0;

            let j = -(-1.0 - c) 
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
            self.chunks[collision_search.chunk0_index].velocity_transform.pos += vel0 * apply;
            self.chunks[collision_search.chunk0_index].velocity_transform.rot += rot_vel0 * apply;

            self.chunks[collision_search.chunk1_index].velocity_transform.pos -= vel1 * apply;
            self.chunks[collision_search.chunk1_index].velocity_transform.rot -= rot_vel1 * apply; 

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

                let force0 = chunk0.velocity_transform.pos * chunk0.mass;
                let force1 = chunk1.velocity_transform.pos * chunk1.mass;

                
                destruction(
                    &mut self.chunks,
                    entry.chunk0_index, 
                    hex_in_part0, 
                    part0_pos, 
                    *part0_index, 
                    force1.length());

                destruction(
                    &mut self.chunks,
                    entry.chunk1_index, 
                    hex_in_part1, 
                    part1_pos, 
                    *part1_index, 
                    force0.length());
                
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
    start_strain: f32
) { 
    let strain_factor = 0.1;
    let min_strain = 0.5;
    

    let mut particles = Vec::new();
    particles.push((start_hex, start_part_pos, start_part_index, 0, start_strain));
    let mut current_particle_index = 0;

    loop {
        if current_particle_index >= particles.len() {
            break;
        }

        let (hex, part_pos, part_index, depth, strain) = particles[current_particle_index];
        
        if strain < min_strain {
            current_particle_index += 1;
            continue;
        }

        let mut strain_applyed = 0;
        'outer: for (n_hex, n_part_index) in chunks[chunk_index].get_neigbor_particles_pos(part_pos, part_index, hex) {
            
            for (
                test_hex, 
                _, 
                test_part_index, 
                test_depth, 
                test_strain
            ) in particles.iter_mut() {
                if n_hex == *test_hex && n_part_index == *test_part_index {

                    if *test_depth > depth{
                        *test_strain += strain * strain_factor;
                        strain_applyed += 1;
                    }
                    continue 'outer; 
                }
            }

            particles.push((n_hex, chunks[chunk_index].parts[n_part_index].pos, n_part_index, depth + 1, strain * strain_factor));
            strain_applyed += 1;
        }

        particles[current_particle_index].4 -= strain_factor * strain_applyed as f32;
        current_particle_index += 1;
    }

}
