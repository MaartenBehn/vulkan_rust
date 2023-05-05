use app::glam::{ivec2, vec2, IVec2, Vec2};

use crate::chunk::CHUNK_PART_SIZE;

use self::transform::Transform;

pub mod transform;

pub fn coord_to_hex(coord: Vec2) -> IVec2 {
    ivec2((coord.x - coord.y * 0.5) as i32, coord.y as i32)
}

pub fn hex_to_coord(hex: IVec2) -> Vec2 {
    Vec2::new(
        (hex.x as f32 + 0.5) + (hex.y as f32 + 0.5) * 0.5,
        hex.y as f32 + 0.5,
    )
}

pub fn hex_to_chunk_part_pos(hex: IVec2) -> IVec2 {
    IVec2::new(
        hex.x / CHUNK_PART_SIZE - (hex.x % CHUNK_PART_SIZE < 0) as i32,
        hex.y / CHUNK_PART_SIZE - (hex.y % CHUNK_PART_SIZE < 0) as i32,
    )
}

pub fn hex_to_particle_index(hex: IVec2) -> usize {
    let chunk_part = hex_to_chunk_part_pos(hex);
    let p = ivec2(
        hex.x - chunk_part.x * CHUNK_PART_SIZE,
        hex.y - chunk_part.y * CHUNK_PART_SIZE,
    );

    let res = ((p.x) * CHUNK_PART_SIZE + (p.y)) as usize;
    debug_assert!(res < (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize);
    res
}

pub fn hex_in_chunk_frame(hex: IVec2, part_hex: IVec2) -> Vec2 {
    hex_to_coord(hex + part_hex * CHUNK_PART_SIZE)
}

const CHUNK_PART_SIZE_VEC2: Vec2 = Vec2::new(CHUNK_PART_SIZE as f32, CHUNK_PART_SIZE as f32);
pub fn part_pos_to_chunk(part_pos: IVec2) -> Vec2 {
    Vec2::new(
        part_pos.x as f32 + part_pos.y as f32 * 0.5,
        part_pos.y as f32,
    ) * CHUNK_PART_SIZE_VEC2
}

pub fn part_pos_to_world(chunk_transform: Transform, part_pos: IVec2) -> Transform {
    let part_pos = part_pos_to_chunk(part_pos);

    let angle_vec = Vec2::from_angle(chunk_transform.rot);
    let rotated_pos = Vec2::rotate(angle_vec, part_pos);

    Transform::new(chunk_transform.pos + rotated_pos, chunk_transform.rot)
}

pub fn world_pos_to_hex(part_transform: Transform, world_pos: Vec2) -> IVec2 {
    let angle_vec = Vec2::from_angle(part_transform.rot);
    let rotated_pos = Vec2::rotate(angle_vec, world_pos - part_transform.pos);

    coord_to_hex(rotated_pos)
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

pub fn neigbor_hex_offsets() -> [IVec2; 6] {
    [
        ivec2(1, 0),
        ivec2(0, 1),
        ivec2(-1, 1),
        ivec2(-1, 0),
        ivec2(0, -1),
        ivec2(1, -1),
    ]
}

pub fn neigbor_pos_offsets() -> [Vec2; 6] {
    [
        vec2(-0.3, -0.5),
        vec2(-0.5, 0.0),
        vec2(-0.3, 0.5),
        vec2(0.3, 0.5),
        vec2(0.5, 0.0),
        vec2(0.3, -0.5),
    ]
}
