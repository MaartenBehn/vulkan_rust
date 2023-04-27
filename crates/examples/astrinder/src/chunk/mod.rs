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
const MAX_AMMOUNT_OF_PARTS: usize = 1000;

pub struct ChunkController {
    pub chunks: Vec<Chunk>,
    part_id_counter: usize,
}

impl ChunkController {
    pub fn new() -> Self {
        let mut chunks = Vec::new();

        chunks.push(Chunk::new_hexagon(
            Transform::new(vec2(0.0, 0.0), 0.0), 
            Transform::new(vec2(0., 0.), 0.0),
            2)); 

         
        chunks.push(Chunk::new_hexagon(
            Transform::new(vec2(0.0, 20.0), 0.0), 
            Transform::new(vec2(0.0, -4.0), 0.0),
            2)); 

        Self { 
            chunks: chunks,
            part_id_counter: 0,
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


    pub area: f32,
    pub density: f32,
    pub moment_of_inertia: f32,
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

            area: 0.0,
            density: 0.0,
            moment_of_inertia: 0.0,
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos)
        }

        chunk.on_chunk_change();
        chunk.on_transform_change();

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
            let new_part= ChunkPart::new(part_pos, self.particle_counter);

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

    fn on_transform_change(&mut self) {
        self.aabb = AABB::new(
            self.transform.pos - self.particle_max_dist_to_transform, 
            self.transform.pos + self.particle_max_dist_to_transform);

        self.update_part_tranforms();
    }

    fn on_chunk_change(&mut self) {
        self.render_to_transform = self.particle_pos_sum / Vec2::new(
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
            part.transform = part_pos_to_world(self.transform, part.pos, self.render_to_transform);
        }
    }

    fn update_area(&mut self) {
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

    fn update_moment_of_inertia(&mut self) {
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

}



#[derive(Clone)]
pub struct ChunkPart{
    pub id: usize,
    pub pos: IVec2,
    pub particles: [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    pub transform: Transform,

    pub colliders: Vec<ConvexPolygon<f32>>
}

impl ChunkPart {
    pub fn new(pos: IVec2, id: usize) -> Self {
        Self { 
            id: id,
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

        let offsets = [
            vec2(-0.3, -0.5),
            vec2(-0.5, 0.0),
            vec2(-0.3, 0.5),
            vec2(0.3, 0.5),
            vec2(0.5, 0.0),
            vec2(0.3, -0.5),
            ]; 

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
                        for point in points {
                            set_point_used(point.x, point.y, &mut used_points);
                        }
                        
                        cb.corners[data.corner0] = start;
                        cb.corners[data.corner1] = middle;
                        cb.corners[data.corner2] = end;

                        expaned = true;
                    }
                }

                if !expaned {

                    let mut vertex = Vec::new();
                    for (i, corner) in cb.corners.iter().enumerate() {
                        let pos = hex_to_coord(*corner) + offsets[i];

                        vertex.push(vec2_to_point2(pos));
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


