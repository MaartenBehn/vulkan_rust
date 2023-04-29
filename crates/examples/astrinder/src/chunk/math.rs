use app::glam::{Vec2, IVec2, ivec2, vec2};

use super::{CHUNK_PART_SIZE, transform::Transform};

pub fn coord_to_hex(coord: Vec2) -> IVec2 {
    ivec2((coord.x - coord.y * 0.5) as i32, coord.y as i32)
}

pub fn hex_to_coord(hex: IVec2) -> Vec2 {
    Vec2::new(
        (hex.x as f32 + 0.5) + (hex.y as f32 + 0.5) * 0.5, 
        hex.y as f32 + 0.5)
}

pub fn hex_to_chunk_part_pos(hex: IVec2) -> IVec2 {
    IVec2::new(
        hex.x / CHUNK_PART_SIZE - (hex.x % CHUNK_PART_SIZE < 0) as i32, 
        hex.y / CHUNK_PART_SIZE - (hex.y % CHUNK_PART_SIZE < 0) as i32)
}

pub fn hex_to_in_chunk_part_pos(hex: IVec2) -> usize {

    let chunk_part = hex_to_chunk_part_pos(hex);
    let p = ivec2(hex.x - chunk_part.x * CHUNK_PART_SIZE, hex.y - chunk_part.y * CHUNK_PART_SIZE);
   
    let res = ((p.x) * CHUNK_PART_SIZE + (p.y)) as usize;
    debug_assert!(res < (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize);
    res
}

const CHUNK_PART_SIZE_VEC2: Vec2 = Vec2::new(CHUNK_PART_SIZE as f32, CHUNK_PART_SIZE as f32);


pub fn part_pos_to_world(chunk_transform: Transform, part_pos: IVec2, render_to_transform: Vec2) -> Transform {
    let part_pos = Vec2::new(part_pos.x as f32 + part_pos.y as f32 * 0.5, part_pos.y as f32) * CHUNK_PART_SIZE_VEC2;

    let angle_vec = Vec2::from_angle(-chunk_transform.rot);
    let rotated_pos = Vec2::rotate(angle_vec, part_pos - render_to_transform);

    Transform::new(
        chunk_transform.pos + rotated_pos, 
        chunk_transform.rot)
}


pub fn world_pos_to_hex(part_transform: Transform, world_pos: Vec2) -> IVec2 {

    let angle_vec = Vec2::from_angle(-part_transform.rot);
    let rotated_pos = Vec2::rotate(angle_vec, world_pos);

    coord_to_hex(rotated_pos - part_transform.pos)
}


pub fn part_corners() -> [Vec2; 4] {
    let one = CHUNK_PART_SIZE as f32;
    [
        vec2(0.0, 0.0),
        vec2(one * 0.5, one),
        vec2(one, 0.0),
        vec2(one * 1.5, one),
    ]
}


pub fn point2_to_vec2(point: cgmath::Point2<f32>) -> Vec2 {
    vec2(point.x, point.y)
}

pub fn vec2_to_point2(vec: Vec2) -> cgmath::Point2<f32> {
    cgmath::Point2::new(vec.x, vec.y)
}

pub fn vector2_to_vec2(point: cgmath::Vector2<f32>) -> Vec2 {
    vec2(point.x, point.y)
}


pub fn cross2d(p1: Vec2, p2: Vec2) -> f32 {
    p1.x * p2.y - p2.x * p1.y
}