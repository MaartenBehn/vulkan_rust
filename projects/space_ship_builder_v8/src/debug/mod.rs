pub mod collapse_log;
pub mod hull_basic;
pub mod hull_multi;
pub mod line_renderer;
pub mod rotation_debug;
pub mod text_renderer;

use crate::debug::collapse_log::CollapseLogRenderer;
use crate::debug::hull_basic::DebugHullBasicRenderer;
use crate::debug::hull_multi::DebugHullMultiRenderer;
use crate::debug::line_renderer::DebugLineRenderer;
use crate::debug::rotation_debug::RotationDebugRenderer;
use crate::debug::text_renderer::DebugTextRenderer;
use crate::node::{NodeID, Voxel};
use crate::rules::Rules;
use crate::ship::data::ShipData;
use crate::ship::renderer::ShipRenderer;
use crate::ship::ShipManager;
use crate::voxel_loader::VoxelLoader;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::egui_winit::winit::window::Window;
use octa_force::glam::{IVec2, IVec3};
use octa_force::vulkan::ash::vk::{Extent2D, Format};
use octa_force::vulkan::{CommandBuffer, Context};
use std::time::Duration;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum DebugMode {
    OFF,
    ROTATION_DEBUG,
    HULL_BASIC,
    HULL_MULTI,
    COLLAPSE_LOG,
}

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(500);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: DebugTextRenderer,

    pub rotation_debug: RotationDebugRenderer,
    pub hull_basic_renderer: DebugHullBasicRenderer,
    pub hull_multi_renderer: DebugHullMultiRenderer,
    pub collapse_log_renderer: CollapseLogRenderer,

    last_mode_change: Duration,
}

impl DebugController {
    pub fn new(
        context: &Context,
        images_len: usize,
        format: Format,
        window: &Window,
        test_node_id: NodeID,
        ship_manager: &ShipManager,
    ) -> Result<Self> {
        let line_renderer = DebugLineRenderer::new(
            1000000,
            context,
            images_len as u32,
            format,
            Format::D32_SFLOAT,
            &ship_manager.renderer,
        )?;

        let text_renderer = DebugTextRenderer::new(context, format, window, images_len)?;
        let rotation_debug_renderer = RotationDebugRenderer::new(images_len, test_node_id);
        let hull_basic_renderer = DebugHullBasicRenderer::new(images_len);
        let hull_multi_renderer = DebugHullMultiRenderer::new(images_len);
        let collapse_log_renderer =
            CollapseLogRenderer::new(images_len, &ship_manager.ships[0].data);

        Ok(DebugController {
            mode: DebugMode::OFF,
            line_renderer,
            text_renderer,
            rotation_debug: rotation_debug_renderer,
            hull_basic_renderer,
            hull_multi_renderer,
            collapse_log_renderer,
            last_mode_change: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
        context: &Context,
        controls: &Controls,
        voxel_loader: &mut VoxelLoader,
        total_time: Duration,
        ship_manager: &mut ShipManager,
        image_index: usize,
        rules: &Rules,
        camera: &Camera,
        extent: Extent2D,
    ) -> Result<()> {
        let last_mode = self.mode;

        if controls.f2 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::ROTATION_DEBUG {
                DebugMode::ROTATION_DEBUG
            } else {
                DebugMode::OFF
            }
        }
        if controls.f3 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::HULL_BASIC {
                DebugMode::HULL_BASIC
            } else {
                DebugMode::OFF
            }
        }
        if controls.f4 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::HULL_MULTI {
                DebugMode::HULL_MULTI
            } else {
                DebugMode::OFF
            }
        }
        if controls.f5 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::COLLAPSE_LOG {
                DebugMode::COLLAPSE_LOG
            } else {
                DebugMode::OFF
            }
        }

        ship_manager.renderer.update(camera, extent)?;

        match self.mode {
            DebugMode::OFF => {
                self.line_renderer.vertecies_count = 0;
            }
            DebugMode::ROTATION_DEBUG => {
                self.update_rotation_debug(
                    controls,
                    image_index,
                    &context,
                    &ship_manager.renderer.chunk_descriptor_layout,
                    &ship_manager.renderer.descriptor_pool,
                )?;
            }
            DebugMode::HULL_BASIC => {
                self.update_hull_base(
                    rules.solvers[1].to_hull()?,
                    controls,
                    image_index,
                    &context,
                    &ship_manager.renderer.chunk_descriptor_layout,
                    &ship_manager.renderer.descriptor_pool,
                )?;
            }
            DebugMode::HULL_MULTI => {
                self.update_hull_multi(
                    rules.solvers[1].to_hull()?,
                    controls,
                    image_index,
                    &context,
                    &ship_manager.renderer.chunk_descriptor_layout,
                    &ship_manager.renderer.descriptor_pool,
                )?;
            }
            DebugMode::COLLAPSE_LOG => {
                self.update_collapse_log_debug(
                    &mut ship_manager.ships[0].data,
                    controls,
                    rules,
                    camera,
                    image_index,
                    &context,
                    &ship_manager.renderer.chunk_descriptor_layout,
                    &ship_manager.renderer.descriptor_pool,
                )?;
            }
        }

        Ok(())
    }

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        image_index: usize,
        camera: &Camera,
        extent: Extent2D,
        renderer: &ShipRenderer,
    ) -> Result<()> {
        if self.mode == DebugMode::OFF {
            return Ok(());
        }

        self.text_renderer.render(buffer, camera, extent)?;
        self.line_renderer.render(buffer, image_index);

        match self.mode {
            DebugMode::OFF => {}
            DebugMode::ROTATION_DEBUG => self.rotation_debug.render(buffer, renderer, image_index),
            DebugMode::HULL_BASIC => {
                self.hull_basic_renderer
                    .render(buffer, renderer, image_index);
            }
            DebugMode::HULL_MULTI => {
                self.hull_multi_renderer
                    .render(buffer, renderer, image_index);
            }
            DebugMode::COLLAPSE_LOG => {
                self.collapse_log_renderer
                    .render(buffer, renderer, image_index);
            }
        }

        Ok(())
    }
}
