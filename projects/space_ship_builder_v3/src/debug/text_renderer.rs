use crate::debug::DebugController;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::egui::Ui;
use octa_force::egui_winit::winit::window::Window;
use octa_force::glam::{UVec2, Vec3};
use octa_force::gui::Gui;
use octa_force::vulkan::ash::vk::{Extent2D, Format};
use octa_force::vulkan::{CommandBuffer, Context};

pub struct DebugTextRenderer {
    pub gui: Gui,
    pub texts: Vec<DebugText>,
    pub render_texts: Vec<DebugText>,
}

#[derive(Debug, Clone)]
pub struct DebugText {
    pub lines: Vec<String>,
    pub pos: Vec3,
}

impl DebugTextRenderer {
    pub fn new(
        context: &Context,
        format: Format,
        depth_format: Format,
        window: &Window,
        in_flight_frames: usize,
    ) -> Result<Self> {
        Ok(Self {
            gui: Gui::new(context, format, depth_format, window, in_flight_frames)?,
            texts: Vec::new(),
            render_texts: Vec::new(),
        })
    }

    pub(crate) fn push_texts(&mut self) -> octa_force::anyhow::Result<()> {
        if self.texts.is_empty() {
            return Ok(());
        }

        self.render_texts.clear();
        self.render_texts.append(&mut self.texts);
        self.texts.clear();

        /*
        TODO 3D Gui

        let mut transforms = Vec::new();
        for text in self.render_texts.iter() {
            let mut t = InWorldGuiTransform::default();
            t.pos = text.pos;

            transforms.push(t);
        }
        self.gui.set_transfrom(&transforms);

         */

        Ok(())
    }

    pub(crate) fn render(
        &mut self,
        buffer: &CommandBuffer,
        camera: &Camera,
        extent: UVec2,
    ) -> octa_force::anyhow::Result<()> {
        if self.render_texts.is_empty() {
            return Ok(());
        }

        /*
        self.gui.draw(buffer, extent, camera, |ui: &Ui| {
            for (i, text) in self.render_texts.iter().enumerate() {
                let y = 10.0 + text.lines.len() as f32 * 20.0;

                ui.window(format!("{i}"))
                    .position([i as f32, 0.0], Condition::Always)
                    .size([120.0, y], Condition::Always)
                    .resizable(false)
                    .movable(false)
                    .no_decoration()
                    .no_nav()
                    .no_inputs()
                    .build(|| {
                        for s in text.lines.iter() {
                            ui.text(s);
                        }
                    });
            }
            Ok(())
        })?;
         */

        Ok(())
    }
}

impl DebugController {
    pub fn add_text(&mut self, lines: Vec<String>, pos: Vec3) {
        self.text_renderer.texts.push(DebugText { lines, pos });
    }
}
