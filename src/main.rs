mod vulkan;

use vulkan::VulkanApp;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta},
};
use game_loop::{game_loop};

use game_loop::winit::event::{Event, WindowEvent};
use game_loop::winit::event_loop::EventLoop;
use game_loop::winit::window::{Window, WindowBuilder};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const MAX_UPS: u32 = 30;
const MIN_FPS: u32 = 30;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Renderer")
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();

    let app = vulkan::VulkanApp::new(&window, WIDTH, HEIGHT);

    let game = Game::new(app);
    game_loop(event_loop, window,  game,MAX_UPS, 1.0 / MIN_FPS as f64, |g| {
        g.game.update();
        println!("FPS: {:?}", 1.0 / g.last_frame_time())

    }, |g| {
        g.game.render(&g.window);
    }, | g, event| {
        if !g.game.window(event) { g.exit(); }
    });
}

struct Game {
    is_left_clicked: Option<bool>,
    cursor_position: Option<[i32; 2]>,
    last_position: [i32; 2],
    wheel_delta: Option<f32>,
    app: VulkanApp,
    dirty_swapchain: bool,
}

impl Game {
    pub fn new(app: VulkanApp) -> Self {
        Game{ 
            is_left_clicked: None, 
            last_position: [0,0], 
            cursor_position: Some(app.cursor_position),
            wheel_delta: None, 
            app: app,
            dirty_swapchain: false,
        }
    }

    pub fn update(&mut self) {
        if let Some(is_left_clicked) = self.is_left_clicked {
            self.app.is_left_clicked = is_left_clicked;
        }
        if let Some(position) = self.cursor_position {
            self.app.cursor_position = position;
            self.app.cursor_delta = Some([
                position[0] - self.last_position[0],
                position[1] - self.last_position[1],
            ]);
        } else {
            self.app.cursor_delta = None;
        }
        self.app.wheel_delta = self.wheel_delta;

        
        self.is_left_clicked = None;
        self.cursor_position = None;
        self.last_position = self.app.cursor_position;
        self.wheel_delta = None;
    }

    pub fn render(&mut self, window: &Window) {
        if self.dirty_swapchain {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                self.app.recreate_swapchain();
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
                // Accumulate input events
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    if state == ElementState::Pressed {
                        self.is_left_clicked = Some(true);
                    } else {
                        self.is_left_clicked = Some(false);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let position: (i32, i32) = position.into();
                    self.cursor_position = Some([position.0, position.1]);
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, v_lines),
                    ..
                } => {
                    self.wheel_delta = Some(v_lines/ 100.0);
                }
                _ => (),
            },
            Event::LoopDestroyed => self.app.wait_gpu_idle(),
            _ => (),
        }

        true
    }
}