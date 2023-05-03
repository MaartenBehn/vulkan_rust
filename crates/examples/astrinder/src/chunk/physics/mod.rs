use std::time::{Instant};

use app::anyhow::*;
use app::glam::{Vec2};
use collision::algorithm::minkowski::GJK2;
use crate::chunk::CHUNK_PART_SIZE;
use crate::chunk::math::{world_pos_to_hex, };
use crate::chunk::physics::collide::CollisionSearch;
use crate::settings::Settings;

use self::collide::CollisionSearchResult;

use super::math::vector2_to_vec2;
use super::{ChunkController, transform::Transform, math::{point2_to_vec2, cross2d}};

mod collide;
pub mod destruction;

impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32, settings: Settings) -> Result<()> {
        let mut accelerations = vec![Transform::default(); self.chunks.len()];
        let l = self.chunks.len();

        
        // Gravity
        if settings.gravity_on && l >= 2 {
            for (i, chunk) in self.chunks.iter().enumerate() {
                for j in (i+1)..l {
                    let other_chunk = &self.chunks[j];
    
                    let gravity_force = get_gravity_force(
                        chunk.transform.pos, 
                        other_chunk.transform.pos, 
                        chunk.mass, 
                        other_chunk.mass, 
                        settings);
                    
                    accelerations[i].pos -= gravity_force / chunk.mass;
                    accelerations[j].pos += gravity_force / other_chunk.mass;
                }
            }
        }

        let to_render_transform = self.to_render_transform.clone();
        let to_render_particles = self.to_render_particles.clone();
        let mut collision_search = CollisionSearch::new(settings, &mut self.chunks, time_step);
       
        loop {
            match collision_search.get_next(&self.chunks, time_step) {
                CollisionSearchResult::Done => break,

                CollisionSearchResult::Contact(contact) => {
                    continue;

                    let chunk0 = &self.chunks[collision_search.chunk0_index];
                    let chunk1 = &self.chunks[collision_search.chunk1_index];
        
                    let normal = vector2_to_vec2(contact.normal).normalize();
                    let point = point2_to_vec2(contact.contact_point);
        
                    // Collision Response
                    let r_a = point - chunk0.transform.pos;
                    let r_b = point - chunk1.transform.pos;
                    let r_a_cross_n = cross2d(r_a, normal);
                    let r_b_cross_n = cross2d(r_a, normal);
                    let r_a_cross_n_2 = r_a_cross_n.powf(2.0);
                    let r_b_cross_n_2 = r_b_cross_n.powf(2.0);
        
                    let c = 0.0;
        
                    let j = (5.0 - c) 
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
        
                    // Collision Response
                    self.chunks[collision_search.chunk0_index].velocity_transform.pos -= vel0 * apply;
                    self.chunks[collision_search.chunk0_index].velocity_transform.rot += rot_vel0 * apply;
        
                    self.chunks[collision_search.chunk1_index].velocity_transform.pos += vel1 * apply;
                    self.chunks[collision_search.chunk1_index].velocity_transform.rot -= rot_vel1 * apply;
                },

                CollisionSearchResult::ChunkDone(i) => {
                    let chunk = &mut self.chunks[i];

                    chunk.velocity_transform.rot *= settings.rotation_damping;
                    chunk.velocity_transform += accelerations[i];

                    chunk.transform += chunk.velocity_transform * time_step * collision_search.time_of_first_collide[i];
                    chunk.on_transform_change();
                    chunk.send_transform(&to_render_transform)?;
                },
            }           
        }

        /* 
        if !settings.destruction_on {
            return Ok(());
        }

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

            let now = Instant::now();

            let vel_diff = (chunk0.velocity_transform.pos- chunk1.velocity_transform.pos).length();
            let break_val_0 = (chunk0.mass / chunk1.mass) / vel_diff; 
            let break_val_1 = (chunk1.mass / chunk0.mass) / vel_diff; 

            let break0 = chunk0.particle_counter > 1 
                && break_val_0 < 20.
                && now.duration_since(chunk0.break_cool_down) > settings.destruction_cool_down;

            let break1 = chunk1.particle_counter > 1 
                && break_val_1 < 20.
                && now.duration_since(chunk1.break_cool_down) > settings.destruction_cool_down;

            assert!(entry.chunk0_index < entry.chunk1_index);
            if break1 {
                self.remove_chunk(entry.chunk1_index);
            }
            if break0 {
                self.remove_chunk(entry.chunk0_index);
            }

            if break0 {
                let mut new_chunks_0 = self.destruction_solver.patterns[0]
                    .apply_to_chunk(chunk0, hex0, &mut self.part_id_counter, settings);

                for c in new_chunks_0.iter() {
                    c.send(&to_render_transform, &to_render_particles)?;
                }

                self.chunks.append(&mut new_chunks_0);
            }

            if break1 {
                let mut new_chunks_1 = self.destruction_solver.patterns[0]
                    .apply_to_chunk(chunk1, hex1, &mut self.part_id_counter, settings);

                for c in new_chunks_1.iter() {
                    c.send(&to_render_transform, &to_render_particles)?;
                }

                self.chunks.append(&mut new_chunks_1);
            }
        }
        */

        Ok(())
    }
}


fn get_gravity_force(pos0: Vec2, pos1: Vec2, mass0: f32, mass1: f32, settings: Settings) -> Vec2 {
    let diff = pos0 - pos1;
    let dist = diff.length();
    let force = f32::min((settings.gravity_factor * mass0 * mass1) / (dist * dist), settings.gravity_max_force);

    diff * (1.0 / dist) * force
}





