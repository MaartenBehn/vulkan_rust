use std::cmp::{max, min};
use std::time::{Duration, Instant};
use log::info;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::vulkan::{CommandBuffer, Context};
use crate::INPUT_INTERVALL;
use crate::render::Renderer;
use crate::rules::Rules;
use crate::world::builder::BlockBuilder;
use crate::world::profile::{ENABLE_SHIP_PROFILING, TickProfile};
use crate::world::region::Region;

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

pub struct WorldManager {
    pub loaded_regions: Vec<Region>,
    pub region_size: i32,

    pub ticks: usize,
    pub last_ticks: usize,
    pub tick_profile: TickProfile,
    pub builder: BlockBuilder,
    
    pub last_input: Instant,
}

impl WorldManager {
    pub fn new(region_size: i32, rules: &mut Rules) -> WorldManager {
        WorldManager {
            region_size,
            loaded_regions: vec![],
            
            ticks: 4,
            last_ticks: 0,
            tick_profile: TickProfile::new(),
            builder: BlockBuilder::new(rules),
            
            last_input: Instant::now(),
        }
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
        renderer: &Renderer,
    ) -> octa_force::anyhow::Result<()> {
        if delta_time < MIN_TICK_LENGTH && self.last_ticks != 0 {
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
                    self.tick_profile
                        .ship_computing_start(self.ticks);
                }

                let (ticks_left, changed_chunks) = object.tick(ticks, rules);
                ticks = ticks_left;

                if ENABLE_SHIP_PROFILING {
                    self.tick_profile.ship_computing_done();

                    if ticks == 0 {
                        self.tick_profile.print_state();
                    }
                }

                renderer.update_object(object, changed_chunks, context, frame_index, num_frames);
            }
        }
        self.last_ticks = ticks;
        
        if controls.f12 && self.last_input.elapsed() > INPUT_INTERVALL {
            self.last_input = Instant::now();
            
            // TODO
            info!("Saving");
        }

        Ok(())
    }
    
    pub fn render(
        &mut self,
        renderer: &Renderer,
        buffer: &CommandBuffer,
        frame_index: usize,
    ) {
        let chunks_to_render = self.loaded_regions.iter().map(|region| {
                region.loaded_objects.iter().map(|object| {
                    object.chunks.iter()
                })
            }).flatten();
        
        renderer.render(buffer, frame_index, chunks_to_render);
    }
}

