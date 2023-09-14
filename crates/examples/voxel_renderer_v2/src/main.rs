use std::time::Duration;

use app::anyhow::{ensure, Ok, Result};
use app::camera::Camera;
use app::controls::Controls;
use app::glam::Vec3;
use app::vulkan::ash::vk::{self};
use app::vulkan::{CommandBuffer, WriteDescriptorSet, WriteDescriptorSetKind};
use app::{log, App, BaseApp};
use gui::imgui::{Condition, Ui};
use renderer::Renderer;

use crate::materials::{MaterialController, MaterialList};
use crate::octtree::Octtree;
use crate::octtree_controller::OcttreeController;
use crate::renderer::ComputeUbo;

pub mod materials;
pub mod node;
pub mod octtree;
pub mod octtree_controller;
pub mod renderer;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Ray Caster";

const PRINT_DEBUG_LOADING: bool = false;
const MOVEMENT_DEBUG_READ: bool = false;
const SAVE_FOLDER: &str = "./assets/octtree";

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");

    app::run::<RayCaster>(APP_NAME, WIDTH, HEIGHT, false, true)?;
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
        let tree = Octtree::new();
        let tree_controller = OcttreeController::new(&context, tree)?;
        tree_controller.init_copy();

        log::info!("Creating Materials");
        let material_controller = MaterialController::new(MaterialList::default(), context)?;

        log::info!("Creating Renderer");
        let renderer = Renderer::new(
            context,
            images_len,
            &base.storage_images,
            &tree_controller.octtree_buffer,
            &material_controller.material_buffer,
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.extent);
        camera.position = Vec3::new(-50.0, 0.0, 0.0);
        camera.direction = Vec3::new(1.0, 0.0, 0.0).normalize();
        camera.speed = 50.0;

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

        self.renderer.ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            screen_size: [
                base.swapchain.extent.width as f32,
                base.swapchain.extent.height as f32,
            ],
            mode: gui.mode,
            debug_scale: gui.debug_scale,

            pos: self.camera.position,
            step_to_root: gui.step_to_root as u32,

            dir: self.camera.direction,
            fill_2: 0,
        }])?;

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
    build: bool,
    load: bool,
    debug_scale: u32,

    render_counter: usize,
    needs_children_counter: usize,
    octtree_buffer_size: usize,

    transfer_counter: usize,
    transfer_buffer_size: usize,

    step_to_root: bool,

    loaded_batches: u32,
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            frame: 0,
            pos: Vec3::default(),
            dir: Vec3::default(),
            mode: 1,
            build: false,
            load: true,
            debug_scale: 1,

            render_counter: 0,
            needs_children_counter: 0,
            octtree_buffer_size: 0,
            transfer_counter: 0,
            transfer_buffer_size: 0,

            step_to_root: true,

            loaded_batches: 0,
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

                let mut build = self.build;
                if ui.radio_button_bool("Build Tree", build) {
                    build = !build;
                }
                self.build = build;

                let mut load = self.load;
                if ui.radio_button_bool("Load Tree", load) {
                    load = !load;
                }
                self.load = load;

                let render_counter = self.render_counter;
                let percent =
                    (self.render_counter as f32 / self.octtree_buffer_size as f32) * 100.0;
                ui.text(format!(
                    "Rendered Nodes: {render_counter} ({:.0}%)",
                    percent
                ));

                let needs_children = self.needs_children_counter;
                ui.text(format!("Needs Children: {needs_children}"));

                let transfer_counter = self.transfer_counter;
                let percent =
                    (self.transfer_counter as f32 / self.transfer_buffer_size as f32) * 100.0;
                ui.text(format!(
                    "Transfered Nodes: {transfer_counter} ({:.0}%)",
                    percent
                ));

                let mut step_to_root = self.step_to_root;
                if ui.radio_button_bool("Step to Root", step_to_root) {
                    step_to_root = !step_to_root;
                }
                self.step_to_root = step_to_root;

                let loaded_batches = self.loaded_batches;
                ui.text(format!("Loaded Batches: {loaded_batches}"));
            });
    }
}
