use crate::render::mesh::Mesh;
use crate::render::mesh_renderer::{MeshRenderer, RENDER_MODE_BASE};
use crate::rules::Rules;
use crate::ship::builder::ShipBuilder;
use crate::ship::data::ShipData;
use crate::ship::profile::ShipProfile;
use crate::INPUT_INTERVALL;
use log::info;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::UVec2;
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{CommandBuffer, Context};
use std::cmp::{max, min};
use std::time::Duration;

pub mod builder;
pub mod collapse;
pub mod data;
pub mod order;
pub mod possible_blocks;
mod profile;
pub mod save;

pub const CHUNK_SIZE: i32 = 32;
const SHIP_SAVE_FILE_PATH: &str = "./assets/ship.bin";

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

pub const ENABLE_SHIP_PROFILING: bool = true;

pub struct ShipManager {
    pub ships: Vec<Ship>,
    pub renderer: MeshRenderer,

    pub actions_per_tick: usize,
    last_full_tick: bool,

    last_input: Duration,

    ship_profile: ShipProfile,
}

pub struct Ship {
    pub data: ShipData,
    pub mesh: Mesh,
    pub builder: Option<ShipBuilder>,
}

impl ShipManager {
    pub fn new(
        context: &Context,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        res: UVec2,
        num_frames: usize,
        rules: &Rules,
    ) -> Result<ShipManager> {
        let mut ship = Ship::try_load_save(SHIP_SAVE_FILE_PATH, num_frames, rules);
        // let mut ship = Ship::new(num_frames, rules);
        ship.add_builder(rules);

        let renderer = MeshRenderer::new(
            context,
            num_frames as u32,
            color_attachment_format,
            depth_attachment_format,
            res,
            rules,
        )?;

        Ok(ShipManager {
            ships: vec![ship],
            renderer,

            actions_per_tick: 4,
            last_full_tick: false,

            last_input: Duration::default(),

            ship_profile: ShipProfile::new(),
        })
    }

    pub fn update(
        &mut self,
        rules: &Rules,
        total_time: Duration,
        delta_time: Duration,

        image_index: usize,
        context: &Context,

        controls: &Controls,
        camera: &Camera,
        res: UVec2,
    ) -> Result<()> {
        if delta_time < MIN_TICK_LENGTH && self.last_full_tick {
            self.actions_per_tick = min(self.actions_per_tick * 2, usize::MAX / 2);
        } else if delta_time > MAX_TICK_LENGTH {
            self.actions_per_tick = max(self.actions_per_tick / 2, 4);
        }

        for ship in self.ships.iter_mut() {
            if ship.builder.is_some() {
                let mut builder = ship.builder.take().unwrap();

                builder.update(
                    &mut ship.data,
                    controls,
                    camera,
                    rules,
                    total_time,
                    &mut self.ship_profile,
                )?;

                ship.builder = Some(builder);
            }

            if ENABLE_SHIP_PROFILING {
                self.ship_profile
                    .ship_computing_start(self.actions_per_tick);
            }

            let (full, changed_chunks) = ship.data.tick(self.actions_per_tick, rules);
            if full {
                info!("Full Tick: {}", self.actions_per_tick);
            }

            if ENABLE_SHIP_PROFILING {
                self.ship_profile.ship_computing_done();

                if self.last_full_tick && !full {
                    self.ship_profile.print_state();
                }
            }

            self.last_full_tick = full;

            ship.mesh.update(
                &ship.data,
                changed_chunks,
                image_index,
                context,
                &self.renderer.chunk_descriptor_layout,
                &self.renderer.descriptor_pool,
            )?;
        }

        self.renderer.update(camera, res)?;

        if controls.f12 && self.last_input + INPUT_INTERVALL < total_time {
            self.last_input = total_time;

            log::info!("Saving Ship");
            self.ships[0].save(SHIP_SAVE_FILE_PATH)?;
            log::info!("Saved Ship");
        }

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer, image_index: usize) {
        for ship in self.ships.iter() {
            self.renderer
                .render(buffer, image_index, RENDER_MODE_BASE, &ship.mesh);
        }
    }

    pub fn on_voxel_change(
        &mut self,
        context: &Context,
        num_frames: usize,
        rules: &mut Rules,
    ) -> Result<()> {
        for ship in self.ships.iter_mut() {
            let save = ship.data.get_save();
            ship.data = ShipData::new_from_save(save, rules);

            if ship.has_builder() {
                let mut builder = ship.builder.take().unwrap();

                builder.on_rules_changed();

                ship.builder = Some(builder);
            }
        }

        self.renderer.on_rules_changed(rules, context, num_frames)?;

        Ok(())
    }
}

impl Ship {
    pub fn new(num_frames: usize, rules: &Rules) -> Ship {
        let data = ShipData::new(CHUNK_SIZE, rules);

        let mesh = Mesh::new(num_frames, data.nodes_per_chunk, data.nodes_per_chunk);

        Ship {
            data,
            mesh,
            builder: None,
        }
    }

    pub fn try_load_save(path: &str, num_frames: usize, rules: &Rules) -> Ship {
        let r = ShipData::load(path, rules);
        let data = if r.is_ok() {
            r.unwrap()
        } else {
            ShipData::new(CHUNK_SIZE, rules)
        };

        let mesh = Mesh::new(num_frames, data.nodes_per_chunk, data.nodes_per_chunk);

        Ship {
            data,
            mesh,
            builder: None,
        }
    }

    pub fn save(&self, path: &str) -> Result<()> {
        self.data.save(path)?;
        Ok(())
    }

    pub fn has_builder(&self) -> bool {
        self.builder.is_some()
    }

    pub fn add_builder(&mut self, rules: &Rules) {
        if self.has_builder() {
            return;
        }

        let builder = ShipBuilder::new(rules);
        self.builder = Some(builder);
    }

    pub fn remove_builder(&mut self) {
        if !self.has_builder() {
            return;
        }

        self.builder = None;
    }
}
