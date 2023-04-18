use app::glam::{Vec2, IVec2, ivec2};

use super::CHUNK_PART_SIZE;

pub fn hex_to_coord(hex: IVec2) -> Vec2 {
    Vec2::new(
        hex.x as f32 * 2.0 + hex.y as f32, 
        hex.y as f32 * 1.5)
}

pub fn coord_to_hex(coord: Vec2) -> IVec2 {
    IVec2::new(
        (coord.x * 0.5 - coord.y * (1.0 / 3.0) + 0.5) as i32,
        (coord.y * (2.0 / 3.0) + 0.5) as i32
    )
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
pub fn part_pos_to_world(chunk_pos: Vec2, part_pos: IVec2) -> Vec2 {
    chunk_pos + Vec2::new(part_pos.x as f32 + part_pos.y as f32 * 0.5, part_pos.y as f32) * CHUNK_PART_SIZE_VEC2
}