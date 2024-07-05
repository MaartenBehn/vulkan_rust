pub mod collapse_log;
pub mod hull_basic;
pub mod hull_multi;
pub mod line_renderer;
pub mod nodes;
pub mod rotation_debug;
pub mod text_renderer;

use crate::debug::collapse_log::CollapseLogRenderer;
use crate::debug::hull_basic::DebugHullBasicRenderer;
use crate::debug::hull_multi::DebugHullMultiRenderer;
use crate::debug::line_renderer::DebugLineRenderer;
use crate::debug::nodes::DebugNodesRenderer;
use crate::debug::rotation_debug::RotationRenderer;
use crate::debug::text_renderer::DebugTextRenderer;
use crate::render::mesh_renderer::MeshRenderer;
use crate::rules::Rules;
use crate::world::data::node::NodeID;
use crate::world::ship::ShipManager;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::egui;
use octa_force::egui::Widget;
use octa_force::egui_winit::winit::event::WindowEvent;
use octa_force::egui_winit::winit::window::Window;
use octa_force::glam::UVec2;
use octa_force::gui::Gui;
use octa_force::vulkan::ash::vk::Format;
use octa_force::vulkan::{CommandBuffer, Context};
use std::time::Duration;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum DebugMode {
    Off,
    RotationDebug,
    Nodes,
    HullBasic,
    HullMulti,
    CollapseLog,
}

pub const SELECTABE_DEBUG_MODES: [DebugMode; 5] = [
    DebugMode::RotationDebug,
    DebugMode::Nodes,
    DebugMode::HullBasic,
    DebugMode::HullMulti,
    DebugMode::CollapseLog,
];

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(500);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: DebugTextRenderer,

    pub rotation_renderer: RotationRenderer,
    pub nodes_renderer: DebugNodesRenderer,
    pub hull_basic_renderer: DebugHullBasicRenderer,
    pub hull_multi_renderer: DebugHullMultiRenderer,
    pub collapse_log_renderer: CollapseLogRenderer,

    last_mode_change: Duration,

    pub gui: Gui,
}

impl DebugController {
    pub fn new(
        context: &Context,
        images_len: usize,
        format: Format,
        depth_format: Format,
        window: &Window,
        test_node_id: NodeID,
        ship_manager: &ShipManager,
        renderer: &MeshRenderer,
    ) -> Result<Self> {
        let line_renderer = DebugLineRenderer::new(
            1000000,
            context,
            images_len as u32,
            format,
            Format::D32_SFLOAT,
            &renderer,
        )?;

        let text_renderer =
            DebugTextRenderer::new(context, format, depth_format, window, images_len)?;
        let rotation_renderer = RotationRenderer::new(images_len, test_node_id);
        let nodes_renderer = DebugNodesRenderer::new(images_len);
        let hull_basic_renderer = DebugHullBasicRenderer::new(images_len);
        let hull_multi_renderer = DebugHullMultiRenderer::new(images_len);
        let collapse_log_renderer =
            CollapseLogRenderer::new(images_len, &ship_manager.ships[0].block_objects);

        let gui = Gui::new(context, format, depth_format, window, images_len)?;

        Ok(DebugController {
            mode: DebugMode::Off,
            line_renderer,
            text_renderer,
            rotation_renderer,
            nodes_renderer,
            hull_basic_renderer,
            hull_multi_renderer,
            collapse_log_renderer,
            last_mode_change: Duration::ZERO,
            gui,
        })
    }

    pub fn update(
        &mut self,
        context: &Context,
        controls: &Controls,
        total_time: Duration,
        ship_manager: &mut ShipManager,
        image_index: usize,
        rules: &Rules,
        camera: &Camera,
        res: UVec2,
        renderer: &MeshRenderer,
    ) -> Result<()> {
        if controls.f2 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::RotationDebug {
                DebugMode::RotationDebug
            } else {
                DebugMode::Off
            }
        }
        if controls.f3 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::Nodes {
                DebugMode::Nodes
            } else {
                DebugMode::Off
            }
        }
        if controls.f4 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::HullBasic {
                DebugMode::HullBasic
            } else {
                DebugMode::Off
            }
        }
        if controls.f5 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::HullMulti {
                DebugMode::HullMulti
            } else {
                DebugMode::Off
            }
        }
        if controls.f6 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::CollapseLog {
                DebugMode::CollapseLog
            } else {
                DebugMode::Off
            }
        }

        match self.mode {
            DebugMode::Off => {
                self.line_renderer.vertecies_count = 0;
            }
            DebugMode::RotationDebug => {
                self.update_rotation_debug(
                    controls,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;
            }
            DebugMode::Nodes => {
                self.update_nodes(
                    rules,
                    controls,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;
            }
            DebugMode::HullBasic => {
                self.update_hull_base(
                    rules.solvers[1].to_hull()?,
                    controls,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;
            }
            DebugMode::HullMulti => {
                self.update_hull_multi(
                    rules.solvers[1].to_hull()?,
                    controls,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;
            }
            DebugMode::CollapseLog => {
                self.update_collapse_log_debug(
                    &mut ship_manager.ships[0].block_objects,
                    controls,
                    rules,
                    camera,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;
            }
        }

        Ok(())
    }

    pub fn on_event(&mut self, window: &Window, event: &WindowEvent) {
        self.gui.handle_event(window, event);
    }

    pub fn render(
        &mut self,
        context: &Context,
        window: &Window,
        buffer: &CommandBuffer,
        image_index: usize,
        res: UVec2,
        camera: &Camera,
        renderer: &MeshRenderer,
    ) -> Result<()> {
        if self.mode == DebugMode::Off {
            return Ok(());
        }

        self.text_renderer.render(buffer, camera, res)?;
        self.line_renderer.render(buffer, image_index);

        match self.mode {
            DebugMode::Off => {}
            DebugMode::RotationDebug => {
                self.rotation_renderer.render(buffer, renderer, image_index)
            }
            DebugMode::Nodes => {
                self.nodes_renderer.render(buffer, renderer, image_index);
            }
            DebugMode::HullBasic => {
                self.hull_basic_renderer
                    .render(buffer, renderer, image_index);
            }
            DebugMode::HullMulti => {
                self.hull_multi_renderer
                    .render(buffer, renderer, image_index);
            }
            DebugMode::CollapseLog => {
                self.collapse_log_renderer
                    .render(buffer, renderer, image_index);
            }
        }

        self.gui
            .cmd_draw(buffer, res, image_index, window, context, |ctx| {
                egui::Window::new("Debug Menue")
                    .default_open(true)
                    .show(ctx, |ui| {
                        egui::ComboBox::from_label("Debug Mode")
                            .selected_text(format!("{:?}", self.mode))
                            .show_ui(ui, |ui| {
                                for debug_mode in SELECTABE_DEBUG_MODES {
                                    ui.selectable_value(
                                        &mut self.mode,
                                        debug_mode,
                                        format!("{:?}", debug_mode),
                                    );
                                }
                            });

                        if egui::Button::new("quit").ui(ui).clicked() {
                            self.mode = DebugMode::Off;
                        };
                    });
            })?;

        Ok(())
    }
}
