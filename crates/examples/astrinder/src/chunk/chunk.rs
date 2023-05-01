use std::time::{Instant, Duration};

use app::glam::{vec2, ivec2, IVec2, Vec2};

use crate::{chunk::math::*, aabb::AABB};

use super::{CHUNK_PART_SIZE, particle::Particle, transform::Transform, math::{cross2d, point2_to_vec2, part_pos_to_world}, part::{ChunkPart, PartIdCounter}, ChunkController, physics::BREAK_COOL_DOWN};

#[derive(Clone)]
pub struct Chunk { 
    pub parts: Vec<ChunkPart>, 

    pub mass: f32,

    pub aabb: AABB,
    particle_max_dist_to_transform: Vec2,
    
    pub transform: Transform,
    pub center_of_mass: Vec2,

    pub particle_counter: usize,
    particle_pos_sum: Vec2,
   
    pub velocity_transform: Transform,

    pub area: f32,
    pub density: f32,
    pub moment_of_inertia: f32,

    pub break_cool_down: Instant,
}

#[allow(dead_code)]
impl Chunk {
    pub fn new(
        transform: Transform, 
        velocity_transform: Transform, 
        particles: Vec<(Particle, IVec2)>, 
        part_id_counter: &mut PartIdCounter,
        new_spawn: bool,
    ) -> Self {
        let mut chunk = Self { 
            parts: Vec::new(),
            mass: 0.0,

            aabb: AABB::default(),
            particle_max_dist_to_transform: Vec2::ZERO,
           
            transform: transform,
            center_of_mass: Vec2::ZERO,

            particle_counter: 0,
            particle_pos_sum: Vec2::ZERO,

            velocity_transform: velocity_transform,

            area: 0.0,
            density: 0.0,
            moment_of_inertia: 0.0,

            break_cool_down: if new_spawn { Instant::now() - BREAK_COOL_DOWN } else { Instant::now() },
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos, part_id_counter)
        }

        chunk.on_chunk_change();

        if !new_spawn {
            chunk.transform.pos += chunk.center_of_mass - vec2(0.75, 0.5);
        }

        chunk.on_transform_change();

        chunk
    }

    pub fn get_part_by_pos(&self, pos: IVec2) -> Option<&ChunkPart>{
        for p in &self.parts {
            if p.pos == pos  {
                return Some(p);
            }
        }

        return None;
    }

    pub fn get_part_index_by_pos(&self, pos: IVec2) -> Option<usize>{
        for (i, p) in self.parts.iter().enumerate() {
            if p.pos == pos  {
                return Some(i);
            }
        }

        return None;
    }

    pub fn get_part_by_pos_mut(&mut self, pos: IVec2) -> Option<&mut ChunkPart>{
        for p in &mut self.parts {
            if p.pos == pos {
                return Some(p);
            }
        }

        return None;
    }

    pub fn add_particle(
        &mut self, 
        p: Particle, 
        hex_pos: IVec2,
        part_id_counter: &mut PartIdCounter,
    ) {
        let part_pos = hex_to_chunk_part_pos(hex_pos);
        let mut part = self.get_part_by_pos_mut(part_pos);

        if part.is_none() {
            let part_id = part_id_counter.pop_free();
            if part_id.is_none() {
                println!("Part Id Counter maxed out!!!");
                return;
            }

            let new_part= ChunkPart::new(part_pos, part_id.unwrap());

            self.parts.push(new_part);

            part = self.parts.last_mut();
        }
        debug_assert!(part.is_some());

        let in_part_pos = hex_to_particle_index(hex_pos);
        part.unwrap().particles[in_part_pos] = p;
        
        self.mass += p.mass as f32;

        // Needed for center of mass
        let particle_pos = hex_to_coord(hex_pos);
        self.particle_pos_sum += particle_pos;

        // Needed for AABB
        let particle_pos_abs = particle_pos.abs();
        self.particle_max_dist_to_transform.x = if self.particle_max_dist_to_transform.x < particle_pos_abs.x { 
            particle_pos_abs.x } else { self.particle_max_dist_to_transform.x };
        self.particle_max_dist_to_transform.y = if self.particle_max_dist_to_transform.y < particle_pos_abs.y { 
                particle_pos_abs.y } else { self.particle_max_dist_to_transform.y };

        self.particle_counter += 1;
    }

    pub fn on_transform_change(&mut self) {
        self.aabb = AABB::new(
            self.transform.pos - self.particle_max_dist_to_transform, 
            self.transform.pos + self.particle_max_dist_to_transform);

        self.update_part_tranforms();
    }

    pub fn on_chunk_change(&mut self) {
        self.center_of_mass = self.particle_pos_sum / Vec2::new(
            self.particle_counter as f32, 
            self.particle_counter as f32);
            

        for part in self.parts.iter_mut() {
            part.update_colliders();
        }

        self.update_area();
        self.update_moment_of_inertia();
    }

    pub fn update_part_tranforms(&mut self){
        for part in self.parts.iter_mut() {
            part.transform = part_pos_to_world(self.transform, part.pos, self.center_of_mass);
        }
    }

    pub fn update_area(&mut self) {
        // https://fotino.me/moment-of-inertia-algorithm/

        self.area = 0.0;
        for part in self.parts.iter() {
            for collider in part.colliders.iter() {
                for i in 0..collider.vertices.len() - 1 {
                    let (p1, p2, p3) = (
                        point2_to_vec2(collider.vertices[0]), 
                        point2_to_vec2(collider.vertices[i]), 
                        point2_to_vec2(collider.vertices[i + 1]));

                    if p1 == p2 || p1 == p3 || p2 == p3 {
                        continue;
                    }
    
                    let v1 = p3 - p1; 
                    let v2 = p2 - p1;
                    self.area += cross2d(v1, v2) / 2.0;
                }
            }
        }
    }

    pub fn update_moment_of_inertia(&mut self) {
        // https://fotino.me/moment-of-inertia-algorithm/

        let density = (self.mass / 100.0) / self.area;
        let mut moment_of_inertia = 0.0;
        for part in self.parts.iter() {
            for collider in part.colliders.iter() {
                for i in 0..collider.vertices.len() - 1 {
                    let (p1, p2, p3) = (
                        point2_to_vec2(collider.vertices[0]), 
                        point2_to_vec2(collider.vertices[i]), 
                        point2_to_vec2(collider.vertices[i + 1]));

                    if p1 == p2 || p1 == p3 || p2 == p3 {
                        continue;
                    }

                    let w = p1.distance(p2);
    
                    let w1 = ((p1 - p2).dot(p3 - p2) / w).abs();
                    let w2 = (w - w1).abs();
    
                    let signed_tri_area = cross2d(p3 - p1, p2 - p1) / 2.0;
                    let h = 2.0 * signed_tri_area.abs() / w;
    
                    let p4 = p2 + (p1 - p2) * (w1 / w);
    
                    let cm1 = (p2 + p3 + p4) / 3.0;
                    let cm2 = (p1 + p3 + p4) / 3.0;
    
                    let i1 = density * w1 * h * ((h * h / 4.0) + (w1 * w1 / 12.0));
                    let i2 = density * w2 * h * ((h * h / 4.0) + (w2 * w2 / 12.0));
    
                    let m1 = 0.5 * w1 * density;
                    let m2 = 0.5 * w2 * density;
                    
                    let i1cm = i1 - (m1 * cm1.distance(p3).powf(2.0));
                    let i2cm = i2 - (m2 * cm2.distance(p3).powf(2.0));
    
                    let moment_of_inertia_part1 = i1cm + (m1 * cm1.length().powf(2.0));
                    let moment_of_inertia_part2 = i2cm + (m2 * cm2.length().powf(2.0));
    
                    if cross2d(p1 - p3, p4 - p3) > 0.0 {
                        moment_of_inertia += moment_of_inertia_part1;
                    } else {
                        moment_of_inertia -= moment_of_inertia_part1;
                    }
    
                    if cross2d(p4 - p3, p2 - p3) > 0.0 {
                        moment_of_inertia += moment_of_inertia_part2;
                    } else {
                        moment_of_inertia -= moment_of_inertia_part2;
                    }
                }
            }
        }
    
        self.moment_of_inertia = moment_of_inertia.abs();
    }

    pub fn get_neigbor_particles_pos(&self, part_pos: IVec2, part_index: usize, hex: IVec2) -> Vec<Option<(IVec2, usize)>> {

        let mut neigbor_particles = Vec::new();

        for i in 0..6 {
            let res = self.get_neigbor_particle_pos(part_pos, part_index, hex, i);

            neigbor_particles.push(res);
        }

        neigbor_particles
    }

    pub fn get_neigbor_particles_pos_cleaned(&self, part_pos: IVec2, part_index: usize, hex: IVec2) -> Vec<(IVec2, usize)> {

        let mut neigbor_particles = Vec::new();

        for i in 0..6 {
            let res = self.get_neigbor_particle_pos(part_pos, part_index, hex, i);

            if res.is_none() {
                continue;
            }
            neigbor_particles.push(res.unwrap());
        }

        neigbor_particles
    }


    pub fn get_neigbor_particle_pos(&self, part_pos: IVec2, part_index: usize, hex: IVec2, neigbor_index: usize) -> Option<(IVec2, usize)> {

        let mut hex_neigbor = hex + neigbor_hex_offsets()[neigbor_index];
        let neigbor_part_index = match neigbor_index {
            0 => {
                if hex_neigbor.x < CHUNK_PART_SIZE { Some(part_index) }
                else { 
                    hex_neigbor.x -= CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(1, 0)) 
                }
            },
            1 => {
                if hex_neigbor.y < CHUNK_PART_SIZE { Some(part_index) }
                else {
                    hex_neigbor.y -= CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(0, 1)) 
                }
            },
            2 => {

                if hex_neigbor.x >= 0 && hex_neigbor.y < CHUNK_PART_SIZE { 
                    Some(part_index) 
                }
                else { 
                    let mut new_pos = part_pos;
                    if hex_neigbor.x < 0{ 
                        hex_neigbor.x += CHUNK_PART_SIZE;
                        new_pos += ivec2(-1, 0);
                    }
                    if hex_neigbor.y >= CHUNK_PART_SIZE {
                        hex_neigbor.y -= CHUNK_PART_SIZE;
                        new_pos += ivec2(0, 1);
                    }

                    self.get_part_index_by_pos(new_pos) 
                }
            },
            3 => {
                if hex_neigbor.x >= 0 { Some(part_index) }
                else { 
                    hex_neigbor.x += CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(-1, 0)) 
                }
            },
            4 => {
                if hex_neigbor.y >= 0 { Some(part_index) }
                else { 
                    hex_neigbor.y += CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(0, -1)) 
                }
            },
            5 => {
                if hex_neigbor.x < CHUNK_PART_SIZE && hex_neigbor.y >= 0 { 
                    Some(part_index) 
                }
                else { 
                    let mut new_pos = part_pos;
                    if  hex_neigbor.x >= CHUNK_PART_SIZE { 
                        hex_neigbor.x -= CHUNK_PART_SIZE;
                        new_pos += ivec2(1, 0);
                    }
                    if hex_neigbor.y < 0 {
                        hex_neigbor.y += CHUNK_PART_SIZE;
                        new_pos += ivec2(0, -1);
                    }

                    self.get_part_index_by_pos(new_pos) 
                }
            }
            _ => { None }
        };

        if neigbor_part_index.is_none() || self.parts[neigbor_part_index.unwrap()].particles[hex_to_particle_index(hex_neigbor)].mass == 0 {
            return None;
        }

        return Some((hex_neigbor, neigbor_part_index.unwrap()))
    }

}

impl ChunkController {
    pub fn remove_chunk(&mut self, chunk_index: usize) {
        let chunk = &self.chunks[chunk_index];
        for part in chunk.parts.iter() {
            self.part_id_counter.add_free(part.id);
        }

        self.chunks.remove(chunk_index);
    }
}


