pub mod line_renderer;
pub mod text_renderer;
pub mod wave_renderer;

use crate::debug::line_renderer::DebugLineRenderer;
use crate::debug::text_renderer::DebugTextRenderer;
use crate::debug::wave_renderer::{DebugWaveRenderer, WAVE_DEBUG_PS};
use crate::ship::Ship;
use crate::ship_renderer::ShipRenderer;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::egui_winit::winit::window::Window;
use octa_force::glam::vec3;
use octa_force::vulkan::ash::vk::{Extent2D, Format};
use octa_force::vulkan::{CommandBuffer, Context};
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
    pub wave_renderer: DebugWaveRenderer,

    last_mode_change: Duration,
}

impl DebugController {
    pub fn new(
        context: &Context,
        images_len: usize,
        format: Format,
        window: &Window,
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

        let wave_renderer = DebugWaveRenderer::new(images_len)?;

        Ok(DebugController {
            mode: DebugMode::OFF,
            line_renderer,
            text_renderer,
            wave_renderer,
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

        if self.mode == DebugMode::WFC {
            self.add_text(vec!["WFC".to_owned()], vec3(-1.0, 0.0, 0.0))
        } else {
            self.add_text(vec!["WFC Skip".to_owned()], vec3(-1.0, 0.0, 0.0))
        }

        if self.mode != DebugMode::OFF {
            ship.debug_show_wave(self);

            self.text_renderer.push_texts()?;
            self.line_renderer.push_lines()?;
            self.wave_renderer.update(
                ship,
                image_index,
                &context,
                &renderer.chunk_descriptor_layout,
                &renderer.descriptor_pool,
            )?;
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
        renderer: &ShipRenderer,
    ) -> Result<()> {
        if self.mode == DebugMode::OFF {
            return Ok(());
        }

        self.text_renderer.render(buffer, camera, extent)?;
        self.line_renderer.render(buffer, image_index);
        self.wave_renderer.render(buffer, renderer, image_index);

        Ok(())
    }
}
