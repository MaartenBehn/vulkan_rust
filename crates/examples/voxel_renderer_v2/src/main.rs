use std::time::Duration;

use octa_force::anyhow::{ensure, Ok, Result};
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::Vec3;
use octa_force::vulkan::ash::vk::{self};
use octa_force::vulkan::{CommandBuffer, WriteDescriptorSet, WriteDescriptorSetKind};
use octa_force::{log, App, BaseApp};
use octa_force::imgui::{Condition, Ui};
use octtree_v2::reader::Reader;
use renderer::Renderer;

use crate::materials::{MaterialController, MaterialList};
use crate::octtree_controller::OcttreeController;
use crate::renderer::RenderBuffer;

pub mod materials;
pub mod octtree_controller;
pub mod renderer;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Ray Caster";
const SAVE_FOLDER: &str = "./assets/tree";

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");

    octa_force::run::<RayCaster>(APP_NAME, WIDTH, HEIGHT, false, true)?;
    Ok(())
}
fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}

#[allow(dead_code)]
pub struct RayCaster {
    total_time: Duration,
    frame_counter: usize,

    tree_controller: OcttreeController,
    material_controller: MaterialController,
    renderer: Renderer,

    camera: Camera,
}

impl App for RayCaster {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let images = &base.swapchain.images;
        let images_len = images.len() as u32;

        log::info!("Creating Tree");
        let reader = Reader::new(SAVE_FOLDER.to_owned(), 100)?;
        let mut tree_controller = OcttreeController::new(&context, reader, 100)?;
        tree_controller.init_push()?;

        log::info!("Creating Materials");
        let material_controller = MaterialController::new(MaterialList::default(), context)?;

        log::info!("Creating Renderer");
        let renderer = Renderer::new(
            context,
            images_len,
            &base.storage_images,
            &tree_controller.octtree_buffer,
            &tree_controller.octtree_lookup_buffer,
            &material_controller.material_buffer,
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.extent);

        camera.position = Vec3::new(860.0, 280.0, 520.0);
        camera.direction = Vec3::new(1.0, 0.0, 0.0).normalize();
        camera.speed = 300.0;

        log::info!("Init done");
        Ok(Self {
            total_time: Duration::ZERO,
            frame_counter: 0,

            tree_controller,
            material_controller,
            renderer,
            camera,
        })
    }

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _: usize,
        delta_time: Duration,
        controls: &Controls,
    ) -> Result<()> {
        log::info!("Frame: {:?}", &self.frame_counter);

        self.total_time += delta_time;
        self.camera.update(controls, delta_time);

        self.tree_controller.update(self.camera.position, 300)?;

        self.renderer
            .ubo_buffer
            .copy_data_to_buffer(&[RenderBuffer {
                screen_size: [
                    base.swapchain.extent.width as f32,
                    base.swapchain.extent.height as f32,
                ],
                mode: gui.mode,
                debug_scale: gui.debug_scale,

                pos: self.camera.position,
                fill_1: 0,

                dir: self.camera.direction,
                fill_2: 0,
            }])?;

        // Updateing Gui
        gui.frame = self.frame_counter;
        gui.pos = self.camera.position;
        gui.dir = self.camera.direction;

        self.frame_counter += 1;
        Ok(())
    }

    fn record_compute_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        self.renderer.render(base, buffer, image_index)?;

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        base.storage_images
            .iter()
            .enumerate()
            .for_each(|(index, img)| {
                let set = &self.renderer.descriptor_sets[index];

                set.update(&[WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &img.view,
                    },
                }]);
            });

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Gui {
    frame: usize,
    pos: Vec3,
    dir: Vec3,
    mode: u32,
    debug_scale: u32,
}

impl octa_force::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            frame: 0,
            pos: Vec3::default(),
            dir: Vec3::default(),
            mode: 1,
            debug_scale: 1,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Ray caster")
            .position([5.0, 150.0], Condition::FirstUseEver)
            .size([300.0, 300.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                let frame = self.frame;
                ui.text(format!("Frame: {frame}"));

                let pos = self.pos;
                ui.text(format!("Pos: {pos}"));

                let dir = self.dir;
                ui.text(format!("Dir: {dir}"));

                let mut mode = self.mode as i32;
                ui.input_int("Mode", &mut mode).build();
                mode = mode.clamp(0, 4);
                self.mode = mode as u32;

                let mut debug_scale = self.debug_scale as i32;
                ui.input_int("Scale", &mut debug_scale).build();
                debug_scale = debug_scale.clamp(1, 100);
                self.debug_scale = debug_scale as u32;
            });
    }
}
