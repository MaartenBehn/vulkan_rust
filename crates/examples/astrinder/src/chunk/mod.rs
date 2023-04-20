use app::glam::{UVec2, Vec2, IVec2, uvec2, vec2, ivec2};
use cgmath::Point2;
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
const MAX_COLLIDER_LEN_DIFF: usize = 100;

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
            part.update_colliders();
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

    pub fn update_colliders(&mut self) {

        self.colliders.clear();

        struct ColliderBuilder {
            corners: [IVec2; 6],
            lens: [usize; 3],
            longest_len: usize
        }

        let mut search_x = 0;
        let mut search_y = 0;

        let mut current_x = 0;
        let mut current_y = 0;

        struct ExpandData {
            corner0: usize,
            corner1: usize,
            corner2: usize,

            offset: IVec2,
            dir0: IVec2,
            dir1: IVec2,
        }

        let expand_data = [
            ExpandData {
                corner0: 1,
                corner1: 2,
                corner2: 3,

                offset: ivec2(-1, 1),
                dir0: ivec2(0, 1),
                dir1: ivec2(1, 0),
            },
            ExpandData {
                corner0: 2,
                corner1: 3,
                corner2: 4,

                offset: ivec2(0, 1),
                dir0: ivec2(1, 0),
                dir1: ivec2(1, -1),
            },
            ExpandData {
                corner0: 3,
                corner1: 4,
                corner2: 5,

                offset: ivec2(1, 0),
                dir0: ivec2(1, -1),
                dir1: ivec2(0, -1),
            },
        ];

        let mut used_points = vec![false; self.particles.len()];
        fn set_point_used(x: i32, y: i32, used_points: &mut Vec<bool>) {
            used_points[get_index(x, y)] = true;
        }

        fn check_point(x: i32, y: i32, part: &ChunkPart, used_points: &Vec<bool>) -> bool {
            if x < 0 || x >= CHUNK_PART_SIZE || y < 0 || y >= CHUNK_PART_SIZE  {
                return false;
            }

            let index = get_index(x, y); 
            part.particles[index].material != 0 && !used_points[index]
        }

        'outer: loop {
            'search: loop {
                if search_y >= CHUNK_PART_SIZE {
                    break 'outer;
                }

                loop {
                    if search_x >= CHUNK_PART_SIZE {
                        search_y += 1;
                        search_x = 0;
                        break;
                    }

                    if check_point(search_x, search_y, self, &used_points) {
                        current_x = search_x;
                        current_y = search_y;
                        search_x += 1;

                        break 'search;
                    }

                    search_x += 1;
                }
            }

            let mut cb = ColliderBuilder { 
                corners: [ivec2(current_x, current_y); 6],
                lens: [1; 3],
                longest_len: 0,
            };
            set_point_used(current_x, current_y, &mut used_points);

            loop {
                let mut expaned = false;
                for (i, data) in expand_data.iter().enumerate() {

                    let corner0 = cb.corners[data.corner0];
                    let corner1 = cb.corners[data.corner1];
                    let corner2 = cb.corners[data.corner2];
    
                    let start = corner0 + data.offset;
                    let middle = corner1 + data.offset;
                    let end = corner2 + data.offset;

                    let mut dir = data.dir0;
                    let mut pos = start;
    
                    let mut points = Vec::new();
                    let expand = loop {
                        if !check_point(pos.x, pos.y, self, &used_points) {
                            break false;
                        }
                        points.push(pos);

                        if pos == middle {
                            dir = data.dir1;
                        }
    
                        if pos == end {
                            break true;
                        }

                        pos += dir;
                    };
    
                    if expand {
                        let new_len = cb.lens[i] + 1;
                        if cb.longest_len < new_len {
                            if (new_len * new_len) > (cb.lens[(i+1) % 3] + cb.lens[(i+2) % 3]) * MAX_COLLIDER_LEN_DIFF {
                                continue;
                            }

                            cb.longest_len = new_len;
                        }
                        cb.lens[i] = new_len;

                        for point in points {
                            set_point_used(point.x, point.y, &mut used_points);
                        }
                        
                        cb.corners[data.corner0] = start;
                        cb.corners[data.corner1] = middle;
                        cb.corners[data.corner2] = end;
                        cb.lens[i] = new_len;

                        expaned = true;
                    }
                }

                if !expaned {

                    let mut vertex = Vec::new();
                    for corner in cb.corners {
                        let pos = hex_to_coord(corner);

                        vertex.push(Point2::new(pos.x, pos.y));
                    }

                    let collider = ConvexPolygon::new(vertex);

                    self.colliders.push(collider);
                    break;
                }
            }
        }
    }
}


fn get_index(x: i32, y: i32) -> usize {
    (x * CHUNK_PART_SIZE + y) as usize
}

fn get_valid_dir_bits(c1: bool, c2: bool, c3: bool) -> u8 {
    (c1 as u8) + ((c2 as u8) << 1) +  ((c3 as u8) << 2)
}


