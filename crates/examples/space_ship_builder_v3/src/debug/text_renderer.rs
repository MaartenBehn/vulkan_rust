use crate::debug::DebugController;
use octa_force::camera::Camera;
use octa_force::glam::Vec3;
use octa_force::gui::{GuiId, InWorldGui, InWorldGuiTransform};
use octa_force::imgui::{Condition, Ui};
use octa_force::vulkan::ash::vk::Extent2D;
use octa_force::vulkan::CommandBuffer;

pub struct DebugTextRenderer {
    pub gui_id: GuiId,
    pub texts: Vec<DebugText>,
    pub render_texts: Vec<DebugText>,
}

#[derive(Debug, Clone)]
pub struct DebugText {
    pub lines: Vec<String>,
    pub pos: Vec3,
}

impl DebugTextRenderer {
    pub fn new(gui_id: GuiId) -> Self {
        Self {
            gui_id,
            texts: Vec::new(),
            render_texts: Vec::new(),
        }
    }

    pub(crate) fn push_texts(
        &mut self,
        debug_gui: &mut InWorldGui,
    ) -> octa_force::anyhow::Result<()> {
        if self.texts.is_empty() {
            return Ok(());
        }

        self.render_texts.clear();
        self.render_texts.append(&mut self.texts);
        self.texts.clear();

        let mut transforms = Vec::new();
        for text in self.render_texts.iter() {
            let mut t = InWorldGuiTransform::default();
            t.pos = text.pos;

            transforms.push(t);
        }
        debug_gui.set_transfrom(&transforms);

        Ok(())
    }

    fn render(
        &mut self,
        buffer: &CommandBuffer,
        camera: &Camera,
        extent: Extent2D,
        debug_gui: &mut InWorldGui,
    ) -> octa_force::anyhow::Result<()> {
        if self.render_texts.is_empty() {
            return Ok(());
        }

        debug_gui.draw(buffer, extent, camera, |ui: &Ui| {
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

        Ok(())
    }
}

impl DebugController {
    pub fn add_text(&mut self, lines: Vec<String>, pos: Vec3) {
        self.text_renderer.texts.push(DebugText { lines, pos });
    }
}
