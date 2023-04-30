use std::collections::VecDeque;
use std::f32::consts::PI;
use std::ops::Index;

use app::glam::{Vec2, vec2, IVec2, ivec2};
use app::anyhow::*;

use crate::chunk::math::{world_pos_to_hex, hex_to_particle_index};
use crate::chunk::physics::collide::CollisionSearch;

use super::chunk::ChunkPart;
use super::math::{vector2_to_vec2, hex_to_coord, neigbor_hex_offsets};
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
                
                let point = entry.points[i];
                let normal = entry.normals[i];

                let force = (chunk0.velocity_transform.pos * chunk0.mass - chunk1.velocity_transform.pos * chunk1.mass).length();

                destruction(
                    &mut self.chunks,
                    entry.chunk0_index, 
                    point, 
                    *part0_index, 
                    force,
                    normal);

                destruction(
                    &mut self.chunks,
                    entry.chunk1_index, 
                    point, 
                    *part1_index, 
                    force,
                    -normal);


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
    point: Vec2, 
    part0_index: usize, 
    mut strain: f32,
    normal: Vec2,
) { 
    
    let mut chunk = &chunks[chunk_index];
    let part0 = &chunk.parts[part0_index];
    let hex = world_pos_to_hex(part0.transform, point + normal * 0.5);

    let possible_start_hex = chunk.get_neigbor_particles_pos(
        part0.pos, 
        part0_index, 
        hex);

    let mut hex0 = ivec2(-1, -1);
    let mut hex1: IVec2 = ivec2(-1, -1);
    let mut part0_index = 0;
    let mut part1_index = 0;
    let mut neigbor_case = 0;

    if part0.particles[hex_to_particle_index(hex)].mass != 0 {
        hex0 = hex;
        part0_index = part0_index;
    }

    let l = possible_start_hex.len();
    for (i, neigbor) in possible_start_hex.iter().enumerate() {

        if neigbor.is_none() 
            || (possible_start_hex[(i + l - 2) % l].is_some() 
            && possible_start_hex[(i + 1) % l].is_some()) {
                continue;
        }

        let (neigbor_hex, neigbor_part_index) = neigbor.unwrap();

        if hex0.x == -1 {
            hex0 = neigbor_hex;
            part0_index = neigbor_part_index;
            continue;
        }

        if hex1.x == -1 {
            hex1 = neigbor_hex;
            part1_index = neigbor_part_index;
            break;
        }
    }

    if hex1.x == -1 {
        // Other start not found.
        return;
    }
    
    let mut last_hex = ivec2(-1, -1);
    loop{
        let connection_particle_nr = neigbor_case / 3;
        let connection_nr = neigbor_case % 3;
        let connection_part_index = if connection_particle_nr == 0 { part0_index } else { part1_index };
        let connection_particle_index = hex_to_particle_index(if connection_particle_nr == 0 { hex0 } else { hex1 });


        let connection = chunk.parts[connection_part_index].particles[connection_particle_index].connections[connection_nr];
        let mut new_connection = connection - strain;
        strain -= connection;
        
        if new_connection < 0.0 {
            new_connection = 0.0;
        }

        chunks[chunk_index].parts[connection_part_index].particles[connection_particle_index].connections[connection_nr] = new_connection;

        chunks[chunk_index].parts[part0_index].particles[hex_to_particle_index(hex0)].material = 200;
        chunks[chunk_index].parts[part1_index].particles[hex_to_particle_index(hex1)].material = 200;

        if new_connection > 0.0 {
            break;
        }

        chunk = &chunks[chunk_index];

        let possible_neigbor0 = (neigbor_case + 5) % 6;
        let possible_neigbor1 = (neigbor_case + 1) % 6;

        let possible_next_0 = chunk.get_neigbor_particle_pos(
            chunk.parts[part0_index].pos, 
            part0_index, 
            hex0, 
            possible_neigbor0
        ); 

        let possible_next_1 = chunk.get_neigbor_particle_pos(
            chunk.parts[part0_index].pos, 
            part0_index, 
            hex0, 
            possible_neigbor1
        ); 

        let last_neigbor_case = neigbor_case;
        let mut hex2 = ivec2(-1, -1);
        let mut part2_index = 0;
        if possible_next_0.is_some() && possible_next_0.unwrap().0 != last_hex {
            hex2 = possible_next_0.unwrap().0;
            part2_index = possible_next_0.unwrap().1;
            neigbor_case = possible_neigbor0;
        }

        if possible_next_1.is_some() && possible_next_1.unwrap().0 != last_hex {
            //assert!(hex2.x == -1);

            hex2 = possible_next_1.unwrap().0;
            part2_index = possible_next_1.unwrap().1;
            neigbor_case = possible_neigbor1;
        }

        if hex2.x == -1 {
            break;
        }

        let dot0 = normal.dot(hex_to_coord(hex0 - hex2)).abs();
        let dot1 = normal.dot(hex_to_coord(hex1 - hex2)).abs();

        if dot0 < dot1 {
            last_hex = hex1;
            hex1 = hex2;
            part1_index = part2_index;
        }
        else {
            last_hex = hex0;
            hex0 = hex2;
            part0_index = part2_index;

            neigbor_case = if (last_neigbor_case + 1) % 6 == neigbor_case {
                (last_neigbor_case + 5) % 6
            } else { 
                (last_neigbor_case + 1) % 6 
            };
        }
    }
}



