mod vulkan;

#[macro_use] extern crate log;
extern crate simplelog;

use simplelog::*;
use std::fs::File;

use vulkan::VulkanApp;
use winit::dpi::PhysicalSize;
use game_loop::game_loop;

use game_loop::winit::event::{Event, WindowEvent};
use game_loop::winit::event_loop::EventLoop;
use game_loop::winit::window::{Window, WindowBuilder};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const MAX_UPS: u32 = 30;
const MIN_FPS: u32 = 30;

fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("vulkan_rust.log").unwrap()),
        ]
    ).unwrap();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Renderer")
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();

    let app = vulkan::VulkanApp::new(&window, [WIDTH, HEIGHT]);

    let game = Game::new(app);
    game_loop(event_loop, window,  game,MAX_UPS, 1.0 / MIN_FPS as f64, |g| {
        g.game.update();
        trace!("FPS: {:?}", 1.0 / g.last_frame_time())
    }, |g| {
        g.game.render(&g.window);
    }, | g, event| {
        if !g.game.window(event) { g.exit(); }
    });
}

struct Game {
    app: VulkanApp,
    dirty_swapchain: bool,
}

impl Game {
    pub fn new(app: VulkanApp) -> Self {
        Game{ 
            app: app,
            dirty_swapchain: false,
        }
    }

    pub fn update(&mut self) {

    }

    pub fn render(&mut self, window: &Window) {
        
        if self.dirty_swapchain {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                self.app.recreate_size_dependent([size.width, size.height]);
            } else {
                return;
            }
        }
        self.dirty_swapchain = self.app.draw_frame();
    }

    pub fn window(&mut self, event: Event<()>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => return false,
                WindowEvent::Resized { .. } => self.dirty_swapchain = true,
                _ => (),
            },
            Event::LoopDestroyed => self.app.wait_gpu_idle(),
            _ => (),
        }

        true
    }
}