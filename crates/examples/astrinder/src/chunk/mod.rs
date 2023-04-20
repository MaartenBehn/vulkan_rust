use app::glam::{UVec2, Vec2, IVec2, uvec2, vec2, ivec2};
use collision::primitive::ConvexPolygon;
use crate::{aabb::AABB, chunk::math::*};

use self::{particle::{Particle}, transform::Transform};

pub mod math;
pub mod particle;
pub mod shapes;
pub mod render;
pub mod physics;
pub mod transform;
pub mod debug;

const CHUNK_PART_SIZE: i32 = 10;

pub struct ChunkController {
    pub chunks: Vec<Chunk>
}

impl ChunkController {
    pub fn new() -> Self {
        let mut chunks = Vec::new();

        chunks.push(Chunk::new_noise_hexagon(
            Transform::new(vec2(0.0, 0.0), 0.0), 
            Transform::new(vec2(0., 0.), 0.0),
            10));
        chunks.push(Chunk::new_noise_hexagon(
            Transform::new(vec2(30.0, 0.0), 0.0), 
            Transform::new(vec2(0.0, -20.0), 0.0),
            2));

        Self { 
            chunks: chunks 
        }
    }
}

pub struct Chunk { 
    pub parts: Vec<ChunkPart>, 

    pub mass: f32,

    pub aabb: AABB,
    particle_max_dist_to_transform: Vec2,
    
    pub transform: Transform,
    pub render_to_transform: Vec2,

    particle_counter: usize,
    particle_pos_sum: Vec2,
   
    pub velocity_transform: Transform,
}

#[allow(dead_code)]
impl Chunk {
    pub fn new(transform: Transform, velocity_transform: Transform, particles: Vec<(Particle, IVec2)>) -> Self {
        let mut chunk = Self { 
            parts: Vec::new(),
            mass: 0.0,

            aabb: AABB::default(),
            particle_max_dist_to_transform: Vec2::ZERO,
           
            transform: transform,
            render_to_transform: Vec2::ZERO,

            particle_counter: 0,
            particle_pos_sum: Vec2::ZERO,

            velocity_transform: velocity_transform,
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos)
        }

        for part in chunk.parts.iter_mut() {
            //part.update_collider();
        }

        chunk.render_to_transform = chunk.particle_pos_sum / Vec2::new(
            chunk.particle_counter as f32, 
            chunk.particle_counter as f32);


        chunk.on_transform_update();

        chunk
    }

    pub fn add_particle(
        &mut self, 
        p: Particle, 
        hex_pos: IVec2,
    ){
        let part_pos = hex_to_chunk_part_pos(hex_pos);
        
        let mut part = None;
        for p in &mut self.parts {
            if p.pos == part_pos  {
                part = Some(p);
            }
        }

        if part.is_none() {
            let new_part= ChunkPart::new(part_pos);

            self.parts.push(new_part);

            part = self.parts.last_mut();
        }
        debug_assert!(part.is_some());

        let in_part_pos = hex_to_in_chunk_part_pos(hex_pos);
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

    fn on_transform_update (&mut self) {
        self.aabb = AABB::new(
            self.transform.pos - self.particle_max_dist_to_transform, 
            self.transform.pos + self.particle_max_dist_to_transform);

        self.update_part_tranforms();
    }

    pub fn update_part_tranforms(&mut self){
        for part in self.parts.iter_mut() {
            part.transform = part_pos_to_world(self.transform, part.pos, self.render_to_transform);
        }
    }
}



#[derive(Clone)]
pub struct ChunkPart{
    pub pos: IVec2,
    pub particles: [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    pub transform: Transform,

    pub colliders: Vec<ConvexPolygon<f32>>
}

impl ChunkPart {
    pub fn new(pos: IVec2) -> Self {
        Self { 
            pos: pos,
            particles: [Particle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
            transform: Transform::default(),

            colliders: Vec::new(),
        }
    }   

    pub fn get_colliders(&self) -> Vec<(u8, Vec<IVec2>)> {

        let mut active_colliders: Vec<(u8, Vec<IVec2>)> = Vec::new();

        let mut last_new_collider_instert = None;
        let mut last_exsiting_collider_instert = None;
        let add_new_collider = |x, y, last_insert_in_line,  active_colliders: &mut Vec<(u8, Vec<IVec2>)>| {
            if last_insert_in_line == Some(ivec2(x - 1, y)) {
                active_colliders.last_mut().unwrap().1.push(ivec2(x, y));
            }
            else {
                active_colliders.push((get_valid_dir_bits(true, true, true), vec![ivec2(x, y)]));
            }
        };

        for y in 0..CHUNK_PART_SIZE  {
            for x in 0..CHUNK_PART_SIZE {

                /*
                println!("");
                for collider in active_colliders.iter() {
                    println!("{:?}", collider);
                }
                */
               
                let index = get_index(x, y);
                let material = self.particles[index].material;
                let is_empty = material == 0;
                
                if !is_empty {
                    if y == 0 {
                        add_new_collider(x, y, last_new_collider_instert, &mut active_colliders);
                        last_new_collider_instert = Some(ivec2(x, y));
                        continue;
                    }

                    let case_0 = ivec2(x, y - 1);
                    let case_1 = if x != CHUNK_PART_SIZE - 1 {Some(ivec2(x + 1, y - 1))} else { None };
                    let case_2 = if x != CHUNK_PART_SIZE - 1 {Some((ivec2(x, y - 1), ivec2(x + 1, y + - 1)))} else { None }; 

                    let mut found = false;
                    for (valid_dirs, points) in active_colliders.iter_mut() {

                        let mut test_valid_dirs = 0;
                        let l = points.len();

                        for (i, point) in points.iter().rev().enumerate() {
                            if point.y < y - 1 {
                                break;
                            }

                            test_valid_dirs |= get_valid_dir_bits(
                                point.x == case_0.x,
                                case_1.is_some() 
                                    && point.x == case_1.unwrap().x,
                                case_2.is_some() 
                                    && point.x == case_2.unwrap().1.x 
                                    && l >= 2 && l >= (i + 2)
                                    && points[l - (i + 2)].x == case_2.unwrap().0.x,
                            );
                        }

                        let res_valid_dirs = *valid_dirs & test_valid_dirs;                   
                        if res_valid_dirs != 0 {
                            if (res_valid_dirs & 1) != 0 && 
                                last_exsiting_collider_instert.is_some() && 
                                last_exsiting_collider_instert.unwrap() != ivec2(x - 1, y) {
                                // Remove case 1.
                                *valid_dirs &= !2;
                            }

                            if (res_valid_dirs & 2) != 0 && 
                                self.particles[get_index(x + 1, y)].material == 0 {
                                // Remove case 0.
                                *valid_dirs &= !1;
                            }

                            points.push(ivec2(x, y));
                            last_exsiting_collider_instert = Some(ivec2(x, y));

                            found = true;
                            break;
                        }
                    }

                    if !found {
                        add_new_collider(x, y, last_new_collider_instert, &mut active_colliders);
                        last_new_collider_instert = Some(ivec2(x, y));
                    }
                }
            }     
        }

        active_colliders
    }
}


fn get_index(x: i32, y: i32) -> usize {
    (x * CHUNK_PART_SIZE + y) as usize
}

fn get_valid_dir_bits(c1: bool, c2: bool, c3: bool) -> u8 {
    (c1 as u8) + ((c2 as u8) << 1) +  ((c3 as u8) << 2)
}


