pub mod line_renderer;
pub mod text_renderer;
pub mod wave_renderer;

use crate::debug::line_renderer::DebugLineRenderer;
use crate::debug::text_renderer::DebugTextRenderer;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::{vec3, Vec3, Vec4};
use octa_force::gui::InWorldGui;
use octa_force::vulkan::ash::vk::Extent2D;
use octa_force::vulkan::CommandBuffer;
use std::time::Duration;

#[derive(PartialEq)]
pub enum DebugMode {
    OFF,
    WFC,
    WFCSkip,
}

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(100);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: DebugTextRenderer,

    last_mode_change: Duration,
}

impl DebugController {
    pub fn new(line_renderer: DebugLineRenderer, text_renderer: DebugTextRenderer) -> Result<Self> {
        Ok(DebugController {
            mode: DebugMode::OFF,
            line_renderer,
            text_renderer,
            last_mode_change: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
        controls: &Controls,
        total_time: Duration,
        debug_gui: &mut InWorldGui,
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

            self.mode = if self.mode != DebugMode::WFCSkip {
                DebugMode::WFCSkip
            } else {
                DebugMode::OFF
            }
        }

        if self.mode != DebugMode::OFF {
            self.text_renderer.push_texts(debug_gui)?;
            self.line_renderer.push_lines()?;
        } else {
            self.line_renderer.vertecies_count = 0;
        }

        Ok(())
    }

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        image_index: usize,
        camera: &Camera,
        extent: Extent2D,
        debug_gui: &mut InWorldGui,
    ) -> Result<()> {
        if self.mode == DebugMode::OFF {
            return Ok(());
        }

        self.text_renderer
            .render(buffer, camera, extent, debug_gui)?;
        self.line_renderer.render(buffer, image_index);

        Ok(())
    }
}
