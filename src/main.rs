mod vulkan;

use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Renderer")
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();

    let mut app = vulkan::VulkanApp::new(&window, WIDTH, HEIGHT);
    let mut dirty_swapchain = false;

    // Used to accumutate input events from the start to the end of a frame
    let mut is_left_clicked = None;
    let mut cursor_position = None;
    let mut last_position = app.cursor_position;
    let mut wheel_delta = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::NewEvents(_) => {
                // reset input states on new frame
                {
                    is_left_clicked = None;
                    cursor_position = None;
                    last_position = app.cursor_position;
                    wheel_delta = None;
                }
            }
            Event::MainEventsCleared => {
                // update input state after accumulating event
                {
                    if let Some(is_left_clicked) = is_left_clicked {
                        app.is_left_clicked = is_left_clicked;
                    }
                    if let Some(position) = cursor_position {
                        app.cursor_position = position;
                        app.cursor_delta = Some([
                            position[0] - last_position[0],
                            position[1] - last_position[1],
                        ]);
                    } else {
                        app.cursor_delta = None;
                    }
                    app.wheel_delta = wheel_delta;
                }

                // render
                {
                    if dirty_swapchain {
                        let size = window.inner_size();
                        if size.width > 0 && size.height > 0 {
                            app.recreate_swapchain();
                        } else {
                            return;
                        }
                    }
                    dirty_swapchain = app.draw_frame();
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized { .. } => dirty_swapchain = true,
                // Accumulate input events
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    if state == ElementState::Pressed {
                        is_left_clicked = Some(true);
                    } else {
                        is_left_clicked = Some(false);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let position: (i32, i32) = position.into();
                    cursor_position = Some([position.0, position.1]);
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, v_lines),
                    ..
                } => {
                    wheel_delta = Some(v_lines);
                }
                _ => (),
            },
            Event::LoopDestroyed => app.wait_gpu_idle(),
            _ => (),
        }
    });
}