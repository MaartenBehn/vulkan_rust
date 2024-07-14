use octa_force::glam::{ivec3, uvec3, BVec3, IVec3, UVec3};
use std::iter;

pub mod aabb;
pub mod random;
pub mod rotation;

pub fn to_1d(pos: UVec3, max: UVec3) -> usize {
    ((pos.z * max.x * max.y) + (pos.y * max.x) + pos.x) as usize
}

pub fn to_1d_i(pos: IVec3, max: IVec3) -> usize {
    ((pos.z * max.x * max.y) + (pos.y * max.x) + pos.x) as usize
}

pub fn to_3d(mut i: u32, max: UVec3) -> UVec3 {
    let z = i / (max.x * max.y);
    i -= z * max.x * max.y;
    let y = i / max.x;
    let x = i % max.x;
    uvec3(x, y, z)
}

pub fn to_3d_i(mut i: i32, max: IVec3) -> IVec3 {
    let z = i / (max.x * max.y);
    i -= z * max.x * max.y;
    let y = i / max.x;
    let x = i % max.x;
    ivec3(x, y, z)
}

pub fn get_neighbors() -> [IVec3; 27] {
    [
        ivec3(-1, -1, -1),
        ivec3(0, -1, -1),
        ivec3(1, -1, -1),
        ivec3(-1, 0, -1),
        ivec3(0, 0, -1),
        ivec3(1, 0, -1),
        ivec3(-1, 1, -1),
        ivec3(0, 1, -1),
        ivec3(1, 1, -1),
        ivec3(-1, -1, 0),
        ivec3(0, -1, 0),
        ivec3(1, -1, 0),
        ivec3(-1, 0, 0),
        ivec3(0, 0, 0),
        ivec3(1, 0, 0),
        ivec3(-1, 1, 0),
        ivec3(0, 1, 0),
        ivec3(1, 1, 0),
        ivec3(-1, -1, 1),
        ivec3(0, -1, 1),
        ivec3(1, -1, 1),
        ivec3(-1, 0, 1),
        ivec3(0, 0, 1),
        ivec3(1, 0, 1),
        ivec3(-1, 1, 1),
        ivec3(0, 1, 1),
        ivec3(1, 1, 1),
    ]
}

pub fn get_neighbors_without_zero() -> [IVec3; 26] {
    [
        ivec3(-1, -1, -1),
        ivec3(0, -1, -1),
        ivec3(1, -1, -1),
        ivec3(-1, 0, -1),
        ivec3(0, 0, -1),
        ivec3(1, 0, -1),
        ivec3(-1, 1, -1),
        ivec3(0, 1, -1),
        ivec3(1, 1, -1),
        ivec3(-1, -1, 0),
        ivec3(0, -1, 0),
        ivec3(1, -1, 0),
        ivec3(-1, 0, 0),
        ivec3(1, 0, 0),
        ivec3(-1, 1, 0),
        ivec3(0, 1, 0),
        ivec3(1, 1, 0),
        ivec3(-1, -1, 1),
        ivec3(0, -1, 1),
        ivec3(1, -1, 1),
        ivec3(-1, 0, 1),
        ivec3(0, 0, 1),
        ivec3(1, 0, 1),
        ivec3(-1, 1, 1),
        ivec3(0, 1, 1),
        ivec3(1, 1, 1),
    ]
}

pub fn oct_positions() -> [IVec3; 8] {
    [
        ivec3(0, 0, 0),
        ivec3(1, 0, 0),
        ivec3(0, 1, 0),
        ivec3(1, 1, 0),
        ivec3(0, 0, 1),
        ivec3(1, 0, 1),
        ivec3(0, 1, 1),
        ivec3(1, 1, 1),
    ]
}

pub fn oct_positions_with_minus() -> [IVec3; 8] {
    [
        ivec3(-1, -1, -1),
        ivec3(1, -1, -1),
        ivec3(-1, 1, -1),
        ivec3(1, 1, -1),
        ivec3(-1, -1, 1),
        ivec3(1, -1, 1),
        ivec3(-1, 1, 1),
        ivec3(1, 1, 1),
    ]
}

pub fn all_bvec3s() -> [BVec3; 8] {
    [
        BVec3::new(false, false, false),
        BVec3::new(true, false, false),
        BVec3::new(false, true, false),
        BVec3::new(true, true, false),
        BVec3::new(false, false, true),
        BVec3::new(true, false, true),
        BVec3::new(false, true, true),
        BVec3::new(true, true, true),
    ]
}

pub fn all_sides_dirs() -> [IVec3; 6] {
    [
        ivec3(1, 0, 0),
        ivec3(-1, 0, 0),
        ivec3(0, 1, 0),
        ivec3(0, -1, 0),
        ivec3(0, 0, 1),
        ivec3(0, 0, -1),
    ]
}

pub fn get_all_poses(size: UVec3) -> impl Iterator<Item = UVec3> {
    (0..size.x.to_owned())
        .zip(iter::repeat(0..size.y))
        .map(|(xv, yi)| iter::repeat(xv).zip(yi))
        .flatten()
        .zip(iter::repeat(0..size.z))
        .map(|((x, y), zi)| iter::repeat((x, y)).zip(zi))
        .flatten()
        .map(|((x, y), z)| uvec3(x, y, z))
}

pub const PACKED_WORD_SIZE: usize = 8;
pub fn get_packed_index(index: usize) -> (usize, u8) {
    (index / PACKED_WORD_SIZE, 1 << (index % PACKED_WORD_SIZE))
}
