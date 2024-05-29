use crate::rules::Rules;
use crate::ship::builder::ShipBuilder;
use crate::ship::data::ShipData;
use crate::ship::mesh::ShipMesh;
use crate::ship::renderer::{ShipRenderer, RENDER_MODE_BASE};
use crate::ship::save::ShipSave;
use crate::voxel_loader::VoxelLoader;
use crate::INPUT_INTERVALL;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::IVec3;
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::Extent2D;
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::cmp::{max, min};
use std::time::Duration;

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode::WFC};

pub mod builder;
pub mod data;
pub mod mesh;
mod node_order;
mod possible_nodes;
pub mod renderer;
pub mod save;

pub const CHUNK_SIZE: i32 = 32;
const SHIP_SAVE_FILE_PATH: &str = "./assets/ship.bin";

pub const MIN_TICK_LENGTH: Duration = Duration::from_millis(20);
pub const MAX_TICK_LENGTH: Duration = Duration::from_millis(25);

pub struct ShipManager {
    pub ships: Vec<Ship>,
    pub renderer: ShipRenderer,

    actions_per_tick: usize,
    last_full_tick: bool,

    last_input: Duration,
}

pub struct Ship {
    pub data: ShipData,
    pub mesh: ShipMesh,
    pub builder: Option<ShipBuilder>,
}

impl ShipManager {
    pub fn new(
        context: &Context,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        extent: vk::Extent2D,
        num_frames: usize,
        rules: &Rules,
    ) -> Result<ShipManager> {
        // let mut ship = Ship::try_load_save(SHIP_SAVE_FILE_PATH, num_frames, rules);
        let mut ship = Ship::new(num_frames, rules);
        ship.add_builder(rules);

        let renderer = ShipRenderer::new(
            context,
            num_frames as u32,
            color_attachment_format,
            depth_attachment_format,
            extent,
            rules,
        )?;

        Ok(ShipManager {
            ships: vec![ship],
            renderer,

            actions_per_tick: 4,
            last_full_tick: false,

            last_input: Duration::default(),
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
        extent: Extent2D,

        #[cfg(debug_assertions)] debug_controller: &DebugController,
    ) -> Result<()> {
        if delta_time < MIN_TICK_LENGTH && self.last_full_tick {
            self.actions_per_tick = min(self.actions_per_tick * 2, usize::MAX / 2);
        } else if delta_time > MAX_TICK_LENGTH {
            self.actions_per_tick = max(self.actions_per_tick / 2, 4);
        }

        for ship in self.ships.iter_mut() {
            if ship.builder.is_some() {
                let mut builder = ship.builder.take().unwrap();

                #[cfg(debug_assertions)]
                if debug_controller.mode != WFC || controls.lshift {
                    builder.update(&mut ship.data, controls, camera, rules, total_time)?;
                }

                #[cfg(not(debug_assertions))]
                builder.update(&mut ship.data, controls, camera, rules, total_time)?;

                ship.builder = Some(builder);
            }

            let (full, changed_chunks) = ship.data.tick(
                self.actions_per_tick,
                rules,
                #[cfg(debug_assertions)]
                false,
            );
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

        self.renderer.update(camera, extent)?;

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
        let mut data = ShipData::new(CHUNK_SIZE, rules);

        let mesh = ShipMesh::new(num_frames, data.nodes_per_chunk, data.nodes_per_chunk);

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

        let mesh = ShipMesh::new(num_frames, data.nodes_per_chunk, data.nodes_per_chunk);

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
