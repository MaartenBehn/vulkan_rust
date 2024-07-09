use anyhow::Result;
use imgui::{Condition, FontConfig, FontSource, SuspendedContext, Ui};
use imgui_rs_vulkan_renderer::{DynamicRendering, Options, Renderer};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use vulkan::{ash::vk::{Format}, CommandBuffer, CommandPool};
use winit::{event::Event, window::Window};
use std::time::Duration;
use vulkan::ash::vk::Extent2D;
use crate::{FrameStats};

pub trait Gui: Sized {
    fn new() -> anyhow::Result<Self>;

    fn build(&mut self, ui: &Ui);
}

impl Gui for () {
    fn new() -> anyhow::Result<Self> {
        Ok(())
    }

    fn build(&mut self, _ui: &Ui) {}
}

pub struct MainGui {
    imgui_context: Option<SuspendedContext>,
    imgui_platform: WinitPlatform,
    imgui_renderer: Renderer,
    stats_display_mode: StatsDisplayMode,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsDisplayMode {
    None,
    Basic,
    Full,
}

impl StatsDisplayMode {
    fn next(self) -> Self {
        match self {
            Self::None => Self::Basic,
            Self::Basic => Self::Full,
            Self::Full => Self::None,
        }
    }
}

impl MainGui {
    pub fn new(context: &vulkan::Context, command_pool: &CommandPool, window: &Window, format: Format, in_flight_frames: usize) -> Result<Self> {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let mut imgui_platform = WinitPlatform::init(&mut imgui);

        let hidpi_factor = imgui_platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("../../assets/fonts/mplus-1p-regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
                    ..FontConfig::default()
                }),
            },
        ]);
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        imgui_platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);

        let imgui_renderer = Renderer::with_gpu_allocator(
            context.allocator.clone(),
            context.device.inner.clone(),
            context.graphics_queue.inner,
            command_pool.inner,
            DynamicRendering {
                color_attachment_format: format,
                depth_attachment_format: Some(Format::D32_SFLOAT),
            },
            &mut imgui,
            Some(Options {
                in_flight_frames,
                ..Default::default()
            }),
        )?;
        
        Ok(MainGui{
            imgui_context: Some(imgui.suspend()),
            imgui_renderer,
            imgui_platform,
            stats_display_mode: StatsDisplayMode::Basic,
        })
    }
    
    pub fn handle_event<T>(&mut self, window: &Window, event: &Event<T>) {
        let mut imgui = self.imgui_context.take().unwrap().activate().unwrap();
        self.imgui_platform.handle_event(imgui.io_mut(), &window, &event);
        self.imgui_context = Some(imgui.suspend());
    }
    
    pub fn update_delta_time(&mut self, frame_time: Duration) {
        let mut imgui = self.imgui_context.take().unwrap().activate().unwrap();
        imgui.io_mut().update_delta_time(frame_time);
        self.imgui_context = Some(imgui.suspend());
    }
    
    pub fn render<G: Gui>(&mut self, buffer: &CommandBuffer, gui: &mut G, frame_stats: &mut FrameStats, window: &Window, extent: Extent2D) -> Result<()> {
        let mut imgui = self.imgui_context.take().unwrap().activate().unwrap();
        self.imgui_platform.prepare_frame(imgui.io_mut(), window)?;
        let ui = imgui.new_frame();

        gui.build(&ui);
        self.build_perf_ui(&ui, frame_stats, extent);

        self.imgui_platform.prepare_render(&ui, window);
        let draw_data = imgui.render();
        self.imgui_renderer.cmd_draw(buffer.inner, draw_data)?;

        self.imgui_context = Some(imgui.suspend());
        Ok(())
    }

    pub(crate) fn toggle_stats(&mut self) {
        self.stats_display_mode = self.stats_display_mode.next();
    }
    pub(crate) fn build_perf_ui(&self, ui: &Ui, frame_stats: &mut FrameStats, extent: Extent2D) {
        let width = extent.width as f32;
        let height = extent.height as f32;

        if matches!(
                self.stats_display_mode,
                StatsDisplayMode::Basic | StatsDisplayMode::Full
            ) {
            ui.window("Frame stats")
                .focus_on_appearing(false)
                .no_decoration()
                .bg_alpha(0.5)
                .position([5.0, 5.0], Condition::Always)
                .size([160.0, 140.0], Condition::FirstUseEver)
                .build(|| {
                    ui.text("Framerate");
                    ui.label_text("fps", frame_stats.fps_counter.to_string());
                    ui.text("Frametimes");
                    ui.label_text("Frame", format!("{:?}", frame_stats.frame_time));
                    ui.label_text("CPU", format!("{:?}", frame_stats.compute_time));
                    ui.label_text("GPU", format!("{:?}", frame_stats.gpu_time));
                });
        }

        if matches!(self.stats_display_mode, StatsDisplayMode::Full) {
            let graph_size = [width - 80.0, 40.0];
            const SCALE_MIN: f32 = 0.0;
            const SCALE_MAX: f32 = 17.0;

            ui.window("Frametime graphs")
                .focus_on_appearing(false)
                .no_decoration()
                .bg_alpha(0.5)
                .position([5.0, height - 145.0], Condition::Always)
                .size([width - 10.0, 140.0], Condition::Always)
                .build(|| {
                    ui.plot_lines("Frame", &frame_stats.frame_time_ms_log.0)
                        .scale_min(SCALE_MIN)
                        .scale_max(SCALE_MAX)
                        .graph_size(graph_size)
                        .build();
                    ui.plot_lines("CPU", &frame_stats.compute_time_ms_log.0)
                        .scale_min(SCALE_MIN)
                        .scale_max(SCALE_MAX)
                        .graph_size(graph_size)
                        .build();
                    ui.plot_lines("GPU", &frame_stats.gpu_time_ms_log.0)
                        .scale_min(SCALE_MIN)
                        .scale_max(SCALE_MAX)
                        .graph_size(graph_size)
                        .build();
                });
        }
    }
}

