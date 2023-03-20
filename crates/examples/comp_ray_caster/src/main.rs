
use std::time::{Duration};

use app::anyhow::Result;
use app::glam::{Vec3};
use app::vulkan::ash::vk::{self};
use app::vulkan::{CommandBuffer, WriteDescriptorSet, WriteDescriptorSetKind,};
use app::{App, BaseApp, log};
use gui::imgui::{Condition, Ui};


mod octtree;
use octtree::*;
mod octtree_controller;
use octtree_controller::*;
mod octtree_builder;
use octtree_builder::*;
mod octtree_loader;
use octtree_loader::*;
mod materials;
use materials::*;
mod renderer;
use renderer::*;

mod debug;
use debug::*;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Ray Caster";

const DEBUG_LOADING: bool = false;
const MOVEMENT_DEBUG_READ: bool = false;

fn main() -> Result<()> {
    app::run::<RayCaster>(APP_NAME, WIDTH, HEIGHT, false, true)
}
pub struct RayCaster {
    total_time: Duration,
    frame_counter: usize,

    octtree_controller: OcttreeController,
    renderer: Renderer,
    builder: OcttreeBuilder,
    loader: OcttreeLoader,

    movement_debug: MovementDebug,
}

impl App for RayCaster {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let images = &base.swapchain.images;
        let images_len = images.len() as u32;

        log::info!("Creating Octtree");
        let depth = 6;
        let mut octtree_controller = OcttreeController::new(
            context,
            Octtree::new(depth, 123), 
            50000,
            100,
            5000
        )?;

        log::info!("Creating Renderer");
        let renderer = Renderer::new(
            context, 
            images_len, 
            &base.storage_images, 
            &octtree_controller.octtree_buffer, 
            &octtree_controller.octtree_info_buffer,
        )?;

        log::info!("Creating Builder");
        let builder = OcttreeBuilder::new(
            context, 
            &octtree_controller.octtree_buffer, 
            &octtree_controller.octtree_info_buffer,
            octtree_controller.buffer_size,
        )?;

        log::info!("Creating Loader");
        let loader = OcttreeLoader::new(
            context, 
            &octtree_controller, 
            &octtree_controller.octtree_buffer, 
            &octtree_controller.octtree_info_buffer,
        )?;

        log::info!("Setting inital camera pos");
        base.camera.position = Vec3::new(-50.0, 0.0, 0.0);
        base.camera.direction = Vec3::new(1.0, 0.0,0.0).normalize();
        base.camera.z_far = 100.0;


        log::info!("Init done");
        Ok(Self {
            total_time: Duration::ZERO,
            frame_counter: 0,

            octtree_controller,
            renderer,
            builder,
            loader,

            movement_debug: MovementDebug::new(MOVEMENT_DEBUG_READ)?,
        })
    }

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _: usize,
        delta_time: Duration,
    ) -> Result<()> {

        log::info!("Frame: {:?}", &self.frame_counter);

        self.total_time += delta_time;
        
        self.octtree_controller.octtree_info_buffer.copy_data_to_buffer(&[self.octtree_controller.octtree_info])?;
        self.renderer.ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            screen_size: [base.swapchain.extent.width as f32, base.swapchain.extent.height as f32],
            mode: gui.mode,
            debug_scale: gui.debug_scale,
            pos: base.camera.position,
            fill_1: 0,
            dir: base.camera.direction,
            fill_2: 0,
        }])?;

        self.builder.build_tree = gui.build || self.frame_counter == 0;
        self.loader.load_tree = gui.load && self.frame_counter != 0;

        if  self.loader.load_tree {
            let mut request_data: Vec<u32> = self.loader.request_buffer.get_data_from_buffer(self.octtree_controller.transfer_size + LOAD_DEBUG_DATA_SIZE)?;

            let mut render_counter = 0;
            let mut needs_children_counter = 0;

            if cfg!(debug_assertions) && DEBUG_LOADING
            {
                // Debug data from load shader
                render_counter = request_data[self.octtree_controller.transfer_size] as usize;
                needs_children_counter = request_data[self.octtree_controller.transfer_size + 1] as usize;

                gui.render_counter = render_counter;
                gui.needs_children_counter = needs_children_counter;

                request_data.truncate(self.octtree_controller.transfer_size);
            }
            
            let (requested_nodes, transfer_counter) = self.octtree_controller.get_requested_nodes(&request_data);
            self.loader.transfer_buffer.copy_data_to_buffer(&requested_nodes)?;
            
            if cfg!(debug_assertions) && DEBUG_LOADING
            {
                gui.transfer_counter = transfer_counter;

                log::debug!("Render Counter: {:?}", &render_counter);
                log::debug!("Needs Children Counter: {:?}", &needs_children_counter);
                log::debug!("Transfer Counter: {:?}", &transfer_counter);
                log::debug!("Request Data: {:?}", &request_data);
            }
        }

        // Updateing Gui
        gui.frame = self.frame_counter;
        gui.pos = base.camera.position;
        gui.dir = base.camera.direction;
        gui.octtree_buffer_size = self.octtree_controller.buffer_size;
        gui.transfer_buffer_size = self.octtree_controller.transfer_size;

        self.octtree_controller.step();


        if MOVEMENT_DEBUG_READ {
            self.movement_debug.read(&mut base.camera, self.frame_counter)?;
        }
        else{
            self.movement_debug.write(&base.camera)?;
        }


        self.frame_counter += 1;

        Ok(())
    }

    fn record_compute_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize
    ) -> Result<()> {

        if self.loader.load_tree {
            self.loader.render(base, buffer, image_index)?;
        }

        if self.builder.build_tree {
            self.builder.render(base, buffer, image_index)?;
        }

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

    fn record_raytracing_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        // prevents reports of unused parameters without needing to use #[allow]
        let _ = base;
        let _ = buffer;
        let _ = image_index;

        Ok(())
    }

    fn record_raster_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        // prevents reports of unused parameters without needing to use #[allow]
        let _ = base;
        let _ = buffer;
        let _ = image_index;

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
    transfer_buffer_size: usize
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            frame : 0,
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
                if ui.radio_button_bool("Build Tree", build){
                    build = !build;
                }
                self.build = build;

                let mut load = self.load;
                if ui.radio_button_bool("Load Tree", load){
                    load = !load;
                }
                self.load = load;

                let render_counter = self.render_counter;
                let percent = (self.render_counter as f32 / self.octtree_buffer_size as f32) * 100.0; 
                ui.text(format!("Rendered Nodes: {render_counter} ({:.0}%)", percent));

                let needs_children = self.needs_children_counter;
                ui.text(format!("Needs Children: {needs_children}"));


                let transfer_counter = self.transfer_counter;
                let percent = (self.transfer_counter as f32 / self.transfer_buffer_size as f32) * 100.0; 
                ui.text(format!("Transfered Nodes: {transfer_counter} ({:.0}%)", percent));

            });
    }
}


