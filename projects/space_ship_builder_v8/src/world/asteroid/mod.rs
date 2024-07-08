mod metaball;

use crate::render::mesh::Mesh;
use crate::render::mesh_renderer::{MeshRenderer, RENDER_MODE_BASE};
use crate::rules::Rules;
use crate::world::asteroid::metaball::Metaball;
use crate::world::block_object::BlockObject;
use crate::world::data::block::BlockNameIndex;
use crate::world::data::node::VOXEL_PER_NODE_SIDE;
use crate::world::ship::{MAX_TICK_LENGTH, MIN_TICK_LENGTH};
use fastnoise_lite::{FastNoiseLite, NoiseType};
use log::{debug, info};
use octa_force::anyhow::{bail, Result};
use octa_force::glam::{ivec3, IVec3, Vec3};
use octa_force::vulkan::{CommandBuffer, Context};
use std::cmp::{max, min};
use std::time::Duration;

const ASTEROID_CHUNK_SIZE: IVec3 = ivec3(32, 32, 32);
const ASTEROID_CHUNK_VOXEL_SIZE: IVec3 = ivec3(
    ASTEROID_CHUNK_SIZE.x * VOXEL_PER_NODE_SIDE,
    ASTEROID_CHUNK_SIZE.y * VOXEL_PER_NODE_SIDE,
    ASTEROID_CHUNK_SIZE.z * VOXEL_PER_NODE_SIDE,
);

pub struct AsteroidManager {
    pub asteroids: Vec<Asteroid>,
    pub asteroid_block_name_index: BlockNameIndex,

    pub actions_per_tick: usize,
    last_full_tick: bool,
}

pub struct Asteroid {
    pub mesh: Mesh,
    pub block_object: BlockObject,
}

#[derive(Debug, Copy, Clone)]
pub struct AsteroidGenerationConfig {
    size: i32,
    num_points: usize,
    gravity_merge_strength: f32,
    cut_off_dist: f32,
}

impl AsteroidManager {
    pub fn new(num_frames: usize, rules: &Rules) -> Self {
        let asteroid_block_name_index = rules.get_block_name_index("Stone");

        let asteroid = Asteroid::new(asteroid_block_name_index, num_frames, rules);

        AsteroidManager {
            asteroids: vec![asteroid],
            asteroid_block_name_index,
            actions_per_tick: 4,
            last_full_tick: false,
        }
    }

    pub fn update(
        &mut self,
        context: &Context,
        image_index: usize,
        delta_time: Duration,
        rules: &Rules,
        renderer: &MeshRenderer,
    ) -> Result<()> {
        if delta_time < MIN_TICK_LENGTH && self.last_full_tick {
            self.actions_per_tick = min(self.actions_per_tick * 2, usize::MAX / 2);
        } else if delta_time > MAX_TICK_LENGTH {
            self.actions_per_tick = max(self.actions_per_tick / 2, 4);
        }

        for asteroid in self.asteroids.iter_mut() {
            let (full, changed_chunks) = asteroid.block_object.tick(self.actions_per_tick, rules);
            if full {
                info!("Asteroid Full Tick: {}", self.actions_per_tick);
            }
            self.last_full_tick = full;

            asteroid.mesh.update(
                &asteroid.block_object,
                changed_chunks,
                image_index,
                context,
                &renderer.chunk_descriptor_layout,
                &renderer.descriptor_pool,
            )?;
        }

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer, image_index: usize, renderer: &MeshRenderer) {
        for asteroid in self.asteroids.iter() {
            renderer.render(buffer, image_index, RENDER_MODE_BASE, &asteroid.mesh);
        }
    }
}

impl Asteroid {
    pub fn new(
        asteroid_block_name_index: BlockNameIndex,
        num_frames: usize,
        rules: &Rules,
    ) -> Self {
        let mesh = Mesh::new(num_frames, ASTEROID_CHUNK_SIZE, ASTEROID_CHUNK_SIZE);
        let block_object = BlockObject::new(ASTEROID_CHUNK_SIZE.x, rules.block_names.len());

        let mut asteroid = Asteroid { mesh, block_object };

        let config = get_config_from_size(11).unwrap();
        info!("Asteroid Config: {:?}", config);

        asteroid.generate_from_config(asteroid_block_name_index, config);

        asteroid
    }

    fn generate_from_config(
        &mut self,
        asteroid_block_name_index: BlockNameIndex,
        config: AsteroidGenerationConfig,
    ) {
        let mut metaball = Metaball::new();
        metaball.add_random_points_in_area(
            Vec3::NEG_ONE * config.size as f32,
            Vec3::ONE * config.size as f32,
            config.num_points,
            config.cut_off_dist,
            1.0,
        );
        metaball.gravity_merge(config.gravity_merge_strength);

        /*
        metaball.add_random_points_in_area_at_field_value(
            Vec3::NEG_ONE * config.size as f32,
            Vec3::ONE * config.size as f32,
            0.3,
            0.5,
            100,
            -4.0,
            3.0,
            300,
        );
         */

        let size_twice = config.size * 2;
        for x in (-size_twice)..size_twice {
            for y in (-size_twice)..size_twice {
                for z in (-size_twice)..size_twice {
                    let world_block_pos = ivec3(x, y, z);

                    if metaball.get_field(world_block_pos.as_vec3()) > 0.5 {
                        self.block_object
                            .place_block(world_block_pos, asteroid_block_name_index)
                    }
                }
            }
        }
    }
}

fn get_config_from_size(size: i32) -> Result<AsteroidGenerationConfig> {
    let configs = [
        AsteroidGenerationConfig {
            size: 10,
            num_points: 10,
            gravity_merge_strength: 0.5,
            cut_off_dist: 10.0,
        },
        AsteroidGenerationConfig {
            size: 30,
            num_points: 30,
            gravity_merge_strength: 0.5,
            cut_off_dist: 20.0,
        },
        AsteroidGenerationConfig {
            size: 60,
            num_points: 50,
            gravity_merge_strength: 0.7,
            cut_off_dist: 20.0,
        },
        AsteroidGenerationConfig {
            size: 100,
            num_points: 100,
            gravity_merge_strength: 0.3,
            cut_off_dist: 40.0,
        },
    ];

    let mut low_config = None;
    let mut high_config = None;
    for config in configs.iter() {
        if config.size < size {
            low_config = Some(config);
        }

        if config.size > size && high_config.is_none() {
            high_config = Some(config);
        }
    }

    if low_config.is_none() {
        bail!("Size is to low");
    }

    if high_config.is_none() {
        bail!("Size is to to high");
    }

    let low_config = low_config.unwrap();
    let high_config = high_config.unwrap();
    let factor = (size - low_config.size) as f32 / (high_config.size - low_config.size) as f32;
    let one_minus_factor = 1.0 - factor;

    Ok(AsteroidGenerationConfig {
        size: (low_config.size as f32 * one_minus_factor + high_config.size as f32 * factor) as i32,
        num_points: (low_config.num_points as f32 * one_minus_factor
            + high_config.num_points as f32 * factor) as usize,
        gravity_merge_strength: low_config.gravity_merge_strength * one_minus_factor
            + high_config.gravity_merge_strength * factor,
        cut_off_dist: low_config.cut_off_dist * one_minus_factor
            + high_config.cut_off_dist * factor,
    })
}
