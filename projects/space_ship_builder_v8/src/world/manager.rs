use crate::render::Renderer;
use crate::rules::Rules;
use crate::world::asteroid::AsteroidGenerator;
use crate::world::block_object::BlockObject;
use crate::world::builder::BlockBuilder;
use crate::world::profile::{TickProfile, ENABLE_SHIP_PROFILING};
use crate::world::region::Region;
use crate::INPUT_INTERVALL;
use log::info;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::IVec3;
use octa_force::vulkan::{CommandBuffer, Context};
use std::cmp::{max, min};
use std::time::{Duration, Instant};

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);
pub const CHUNK_SIZE: i32 = 32;

pub struct WorldManager {
    pub asteroid_generator: AsteroidGenerator,

    pub loaded_regions: Vec<Region>,
    pub region_size: i32,

    pub ticks: usize,
    pub last_ticks_left: usize,
    pub tick_profile: TickProfile,
    pub builder: BlockBuilder,

    pub last_input: Instant,
}

impl WorldManager {
    pub fn new(region_size: i32, rules: &mut Rules) -> WorldManager {
        WorldManager {
            asteroid_generator: AsteroidGenerator::new(rules),

            region_size,
            loaded_regions: vec![],

            ticks: 4,
            last_ticks_left: 0,
            tick_profile: TickProfile::new(),
            builder: BlockBuilder::new(rules),

            last_input: Instant::now(),
        }
    }

    pub fn add_start_data(&mut self) {
        let mut region = Region::new(IVec3::ZERO);

        //let ship = BlockObject::new(CHUNK_SIZE, rules.block_names.len());
        //region.loaded_objects.push(ship);

        let asteroid = self.asteroid_generator.generate(11);
        region.loaded_objects.push(asteroid);

        self.loaded_regions.push(region);
    }

    pub fn update(
        &mut self,
        rules: &Rules,
        total_time: Duration,
        delta_time: Duration,

        num_frames: usize,
        frame_index: usize,
        context: &Context,

        controls: &Controls,
        camera: &Camera,
        renderer: &mut Renderer,
    ) -> octa_force::anyhow::Result<()> {
        if delta_time < MIN_TICK_LENGTH && self.last_ticks_left == 0 {
            self.ticks = min(self.ticks * 2, usize::MAX / 2);
        } else if delta_time > MAX_TICK_LENGTH {
            self.ticks = max(self.ticks / 2, 4);
        }

        let mut ticks = self.ticks;
        for region in self.loaded_regions.iter_mut() {
            for object in region.loaded_objects.iter_mut() {
                if object.builder_active {
                    self.builder.update(
                        object,
                        controls,
                        camera,
                        rules,
                        total_time,
                        &mut self.tick_profile,
                    )?;
                }

                if ENABLE_SHIP_PROFILING {
                    self.tick_profile.ship_computing_start(self.ticks);
                }

                let (ticks_left, changed_chunks) = object.tick(ticks, rules);
                if ticks != ticks_left {
                    info!("Ticked: {}", ticks - ticks_left)
                }
                ticks = ticks_left;

                if ENABLE_SHIP_PROFILING {
                    self.tick_profile.ship_computing_done();
                }

                renderer.update_object(object, changed_chunks, context, frame_index, num_frames)?;
            }
        }
        if ENABLE_SHIP_PROFILING && self.last_ticks_left == 0 && ticks != 0 {
            self.tick_profile.print_state();
        }

        self.last_ticks_left = ticks;

        if controls.f12 && self.last_input.elapsed() > INPUT_INTERVALL {
            self.last_input = Instant::now();

            // TODO
            info!("Saving");
        }

        Ok(())
    }

    pub fn render(&mut self, renderer: &Renderer, buffer: &CommandBuffer, frame_index: usize) {
        let chunks_to_render = self
            .loaded_regions
            .iter()
            .map(|region| {
                region
                    .loaded_objects
                    .iter()
                    .map(|object| object.chunks.iter())
                    .flatten()
            })
            .flatten();

        renderer.render(buffer, frame_index, chunks_to_render);
    }
}
