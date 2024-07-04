use crate::node::VOXEL_PER_NODE_SIDE;
use crate::render::mesh::Mesh;
use octa_force::glam::{ivec3, IVec3};

const ASTEROID_CHUNK_SIZE: IVec3 = ivec3(32, 32, 32);
const ASTEROID_CHUNK_VOXEL_SIZE: IVec3 = ivec3(
    ASTEROID_CHUNK_SIZE.x * VOXEL_PER_NODE_SIDE,
    ASTEROID_CHUNK_SIZE.y * VOXEL_PER_NODE_SIDE,
    ASTEROID_CHUNK_SIZE.z * VOXEL_PER_NODE_SIDE,
);

pub struct AsteroidManager {
    pub asteroids: Vec<Asteroid>,
}

pub struct Asteroid {
    pub mesh: Mesh,
}

impl AsteroidManager {
    pub fn new() -> Self {
        AsteroidManager { asteroids: vec![] }
    }
}

impl Asteroid {
    pub fn new(num_frames: usize) -> Self {
        let mesh = Mesh::new(num_frames, ASTEROID_CHUNK_SIZE, ASTEROID_CHUNK_SIZE);

        Asteroid { mesh }
    }
}
