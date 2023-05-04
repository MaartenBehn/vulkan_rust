use std::sync::mpsc::Sender;
use std::time::{Instant};

use app::anyhow::*;
use app::glam::{Vec2};
use collision::algorithm::minkowski::GJK2;
use crate::chunk::CHUNK_PART_SIZE;
use crate::chunk::math::{world_pos_to_hex, };
use crate::settings::Settings;


use super::chunk::Chunk;
use super::math::vector2_to_vec2;
use super::{ChunkController, transform::Transform, math::{point2_to_vec2, cross2d}};

pub mod destruction;

impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32) -> Result<()> {
        // https://research.ncl.ac.uk/game/mastersdegree/gametechnologies/physicstutorials/5collisionresponse/Physics%20-%20Collision%20Response.pdf

        let chunks = &mut self.chunks;
        let l = chunks.len();

        let gjk = GJK2::new();

        for i in 0..l {

            init_chunk0(chunks, i, time_step, self.settings);

            for j in (i + 1)..l {

                init_chunk1(chunks, i, j, time_step);

                if chunks[i].aabb.collides(chunks[j].aabb) {

                    let chunk0 = &chunks[i];
                    let chunk1 = &chunks[j];
                    let vel_diff = chunk0.last_velocity_transform.pos - chunk1.last_velocity_transform.pos;

                    let t0 = chunk0.next_transform.into();
                    let t1 = chunk1.next_transform.into();


                    let result = if vel_diff.length() > 100.0 {

                        let t0_last = chunk0.transform.into();
                        let t1_last = chunk1.transform.into();
                        
                        gjk.intersection_complex_time_of_impact(
                            &collision::CollisionStrategy::CollisionOnly, 
                            &chunk1.colliders,
                            &t1_last..&t1,
                            &chunk0.colliders,
                            &t0_last..&t0,
                        )
                    }else {
                        gjk.intersection_complex(
                            &collision::CollisionStrategy::FullResolution, 
                            &chunk0.colliders,
                            &t0,
                            &chunk1.colliders,
                            &t1)
                    };

                    if result.is_some() {
                        let res = result.unwrap();

                        on_contact(chunks, res, i, j, time_step);
                    }
                }

                end_chunk1(chunks, i, j, time_step, self.settings);
            }

            end_chunk0(chunks, i, time_step, self.settings, self.to_render_transform.clone());
        }

        Ok(())
    }

}



fn init_chunk0 (
    chunks: &mut Vec<Chunk>,
    chunk0_index: usize,  
    time_step: f32,
    settings: Settings,
) {

}

fn init_chunk1 (
    chunks: &mut Vec<Chunk>, 
    chunk0_index: usize, 
    chunk1_index: usize,
    time_step: f32,
) {

}

fn on_contact(
    chunks: &mut Vec<Chunk>, 
    contact: collision::Contact<cgmath::Point2<f32>>, 
    chunk0_index: usize, 
    chunk1_index: usize,
    time_step: f32,
) {
    let mut c = 0.66;

    let chunk0 = &chunks[chunk0_index];
    let chunk1 = &chunks[chunk1_index];

    let last_vel0 = chunk0.last_velocity_transform;
    let last_vel1 = chunk1.last_velocity_transform;

    let point = point2_to_vec2(contact.contact_point);
    let mut normal = vector2_to_vec2(contact.normal);


    let total_mass = chunk0.inverse_mass + chunk1.inverse_mass;

    let penetration = if contact.time_of_impact != 0.0 {
        let vel_diff = last_vel0.pos - last_vel1.pos; 
        (1.0 - contact.time_of_impact) * vel_diff.length() * time_step * 1.5
    }
    else {
        contact.penetration_depth
    };

    if normal.is_nan() {
        normal = chunk1.transform.pos - chunk0.transform.pos;
    }
    normal.normalize();

    let resolve0 = -normal * penetration * (chunk0.inverse_mass / total_mass);
    let resolve1 = normal * penetration * (chunk1.inverse_mass / total_mass);


    // Collision Response
    let r_a = point - chunk0.transform.pos;
    let r_b = point - chunk1.transform.pos;
    let r_a_cross_n = cross2d(r_a, normal);
    let r_b_cross_n = cross2d(r_b, normal);
    let r_a_cross_n_2 = r_a_cross_n.powf(2.0);
    let r_b_cross_n_2 = r_b_cross_n.powf(2.0);

    let j = -(1.0 - c) 
            * (last_vel0.pos.dot(normal) - last_vel1.pos.dot(normal) + last_vel0.rot * r_a_cross_n - last_vel1.rot * r_b_cross_n)
            / ((total_mass + r_a_cross_n_2) / chunk0.moment_of_inertia + r_b_cross_n_2 / chunk1.moment_of_inertia);

    let apply = (j >= 0.0) as u8 as f32;

    let j_vec = normal * j * 0.001;

    let acc0 = j_vec * chunk0.inverse_mass;
    let rot_acc0 = cross2d(r_a, j_vec) / chunk0.moment_of_inertia;

    let acc1 = j_vec * chunk1.inverse_mass;
    let rot_acc1 = cross2d(r_b, j_vec) / chunk1.moment_of_inertia;


    chunks[chunk0_index].velocity_transform.pos += acc0 * apply;
    chunks[chunk0_index].velocity_transform.rot += rot_acc0 * apply;

    chunks[chunk1_index].velocity_transform.pos += acc1 * -apply;
    chunks[chunk1_index].velocity_transform.rot += rot_acc1 * -apply;
    

    chunks[chunk0_index].next_transform.pos += resolve0;
    chunks[chunk1_index].next_transform.pos += resolve1;
}

fn end_chunk1 (
    chunks: &mut Vec<Chunk>, 
    chunk0_index: usize, 
    chunk1_index: usize,
    time_step: f32,
    settings: Settings,
) {
    let chunk0 = &chunks[chunk0_index];
    let chunk1 = &chunks[chunk1_index];

    // Gavity 
    let diff = chunk0.transform.pos - chunk1.transform.pos;
    let dist = diff.length();
    let force = f32::min((settings.gravity_factor * chunk0.mass * chunk1.mass) 
        / (dist * dist), settings.gravity_max_force);

    let force = diff * (1.0 / dist) * force;

    let acc0 = force * -chunk0.inverse_mass;
    let acc1 = force * chunk1.inverse_mass;

    chunks[chunk0_index].velocity_transform.pos += acc0;
    chunks[chunk1_index].velocity_transform.pos += acc1;
}

fn end_chunk0 (
    chunks: &mut Vec<Chunk>, 
    chunk0_index: usize,
    time_step: f32,
    settings: Settings,
    to_render_transform: Sender<(usize, Transform)>,
) {
    let chunk = &mut chunks[chunk0_index];

    // Applying tarnsform changes
    chunk.transform = chunk.next_transform;
    let _ = chunk.send_transform(&to_render_transform);


    // Applying Velocity 
    chunk.velocity_transform.rot *= settings.rotation_damping;
    chunk.next_transform += chunk.velocity_transform * time_step;
    chunk.on_transform_change();

    chunk.last_velocity_transform = chunk.velocity_transform;
}






