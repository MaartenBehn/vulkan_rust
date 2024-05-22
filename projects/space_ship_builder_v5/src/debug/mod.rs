pub mod line_renderer;
pub mod possible_node_renderer;
pub mod rules;
pub mod text_renderer;

use crate::debug::line_renderer::DebugLineRenderer;
use crate::debug::possible_node_renderer::DebugPossibleNodeRenderer;
use crate::debug::rules::{DebugRulesRenderer, RULES_SIZE};
use crate::debug::text_renderer::DebugTextRenderer;
use crate::rules::Rules;
use crate::ship::Ship;
use crate::ship_renderer::ShipRenderer;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::egui_winit::winit::window::Window;
use octa_force::glam::{vec3, vec4, Vec3};
use octa_force::vulkan::ash::vk::{Extent2D, Format};
use octa_force::vulkan::{CommandBuffer, Context};
use std::time::Duration;

#[derive(PartialEq)]
pub enum DebugMode {
    OFF,
    WFC,
    RULES,
}

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(100);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: DebugTextRenderer,
    pub possible_node_renderer: DebugPossibleNodeRenderer,
    pub rules_renderer: DebugRulesRenderer,

    last_mode_change: Duration,
}

impl DebugController {
    pub fn new(
        context: &Context,
        images_len: usize,
        format: Format,
        window: &Window,
        ship: &Ship,
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

        let possible_node_renderer = DebugPossibleNodeRenderer::new(images_len, ship)?;
        let rules_renderer = DebugRulesRenderer::new(images_len)?;

        Ok(DebugController {
            mode: DebugMode::RULES,
            line_renderer,
            text_renderer,
            possible_node_renderer,
            rules_renderer,
            last_mode_change: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
        context: &Context,
        controls: &Controls,
        renderer: &ShipRenderer,
        total_time: Duration,
        ship: &Ship,
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

            self.mode = if self.mode != DebugMode::RULES {
                DebugMode::RULES
            } else {
                DebugMode::OFF
            }
        }

        match self.mode {
            DebugMode::OFF => {
                self.line_renderer.vertecies_count = 0;
            }
            DebugMode::WFC => {
                self.add_text(vec!["WFC".to_owned()], vec3(-1.0, 0.0, 0.0));

                ship.show_debug(self);
                self.possible_node_renderer.update(
                    ship,
                    image_index,
                    &context,
                    &renderer.chunk_descriptor_layout,
                    &renderer.descriptor_pool,
                )?;

                self.text_renderer.push_texts()?;
                self.line_renderer.push_lines()?;
            }
            DebugMode::RULES => {
                self.update_rules(
                    rules,
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

        if self.mode == DebugMode::WFC {
            self.possible_node_renderer
                .render(buffer, renderer, image_index);
        }

        if self.mode == DebugMode::RULES {
            self.rules_renderer.render(buffer, renderer, image_index);
        }

        Ok(())
    }
}
