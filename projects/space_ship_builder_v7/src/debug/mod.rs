pub mod hull_block_reqs;
pub mod hull_node_reqs;
pub mod line_renderer;
pub mod possible_node_renderer;
pub mod text_renderer;

use crate::debug::hull_block_reqs::DebugHullBlockReqRenderer;
use crate::debug::hull_node_reqs::DebugHullNodeReqRenderer;
use crate::debug::line_renderer::DebugLineRenderer;
use crate::debug::possible_node_renderer::DebugPossibleNodeRenderer;
use crate::debug::text_renderer::DebugTextRenderer;
use crate::rules::solver::ToAny;
use crate::rules::Rules;
use crate::ship::data::ShipData;
use crate::ship::renderer::ShipRenderer;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::egui_winit::winit::window::Window;
use octa_force::vulkan::ash::vk::{Extent2D, Format};
use octa_force::vulkan::{CommandBuffer, Context};
use std::time::Duration;

#[derive(PartialEq)]
pub enum DebugMode {
    OFF,
    WFC,
    HULL_BOCK,
    HULL_NODE,
}

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(500);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: DebugTextRenderer,
    pub possible_node_renderer: DebugPossibleNodeRenderer,
    pub hull_block_req_renderer: DebugHullBlockReqRenderer,
    pub hull_node_req_renderer: DebugHullNodeReqRenderer,

    last_mode_change: Duration,
}

impl DebugController {
    pub fn new(
        context: &Context,
        images_len: usize,
        format: Format,
        window: &Window,
        ship: &ShipData,
        renderer: &ShipRenderer,
    ) -> Result<Self> {
        let line_renderer = DebugLineRenderer::new(
            1000000,
            context,
            images_len as u32,
            format,
            Format::D32_SFLOAT,
            &renderer,
        )?;

        let text_renderer = DebugTextRenderer::new(context, format, window, images_len)?;

        let possible_node_renderer = DebugPossibleNodeRenderer::new(images_len, ship);

        let hull_block_req_renderer = DebugHullBlockReqRenderer::new(images_len);
        let hull_node_req_renderer = DebugHullNodeReqRenderer::new(images_len);

        Ok(DebugController {
            mode: DebugMode::OFF,
            line_renderer,
            text_renderer,
            possible_node_renderer,
            hull_block_req_renderer,
            hull_node_req_renderer,
            last_mode_change: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
        context: &Context,
        controls: &Controls,
        renderer: &ShipRenderer,
        total_time: Duration,
        ship: &ShipData,
        image_index: usize,
        rules: &Rules,
    ) -> Result<()> {
        if controls.f2 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::WFC {
                DebugMode::WFC
            } else {
                DebugMode::OFF
            }
        }
        if controls.f3 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::HULL_BOCK {
                DebugMode::HULL_BOCK
            } else {
                DebugMode::OFF
            }
        }
        if controls.f4 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::HULL_NODE {
                DebugMode::HULL_NODE
            } else {
                DebugMode::OFF
            }
        }

        match self.mode {
            DebugMode::OFF => {
                self.line_renderer.vertecies_count = 0;
            }
            DebugMode::WFC => {
                self.update_possible_nodes(
                    ship,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;
            }
            DebugMode::HULL_BOCK => {
                self.update_block_req_hull(
                    rules.solvers[1].to_hull(),
                    controls,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                    total_time,
                )?;
            }
            DebugMode::HULL_NODE => {
                self.update_hull_node_req(
                    rules.solvers[1].to_hull(),
                    controls,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                    total_time,
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
            DebugMode::WFC => {
                self.possible_node_renderer
                    .render(buffer, renderer, image_index);
            }
            DebugMode::HULL_BOCK => {
                self.hull_block_req_renderer
                    .render(buffer, renderer, image_index);
            }
            DebugMode::HULL_NODE => {
                self.hull_node_req_renderer
                    .render(buffer, renderer, image_index);
            }
        }

        Ok(())
    }
}
