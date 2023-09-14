use winit::event::{Event, WindowEvent, KeyboardInput, ElementState, MouseButton, DeviceEvent};


const W_SCANCODE: u32 = 17;
const S_SCANCODE: u32 = 31;
const D_SCANCODE: u32 = 32;
const A_SCANCODE: u32 = 30;
const UP_SCANCODE: u32 = 57;
const DOWN_SCANCODE: u32 = 29;

#[derive(Debug, Clone, Copy)]
pub struct Controls {
    pub w: bool,
    pub s: bool,
    pub d: bool,
    pub a: bool,
    pub up: bool,
    pub down: bool,
    pub rigth: bool,
    pub left: bool,
    pub cursor_delta: [f32; 2],
    pub scroll_delta: f32,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            w: false,
            s: false,
            d: false,
            a: false,
            up: false,
            down: false,
            rigth: false,
            left: false,
            cursor_delta: [0.0; 2],
            scroll_delta: 0.0
        }
    }
}

impl Controls {
    pub fn reset(self) -> Self {
        Self {
            cursor_delta: [0.0; 2],
            ..self
        }
    }

    pub fn handle_event(self, event: &Event<()>) -> Self {
        let mut new_state = self;
        //new_state.scroll_delta = 0.0;

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                scancode, state, ..
                            },
                        ..
                    } => {
                        if *scancode == W_SCANCODE {
                            new_state.w = *state == ElementState::Pressed;
                        }
                        if *scancode == S_SCANCODE {
                            new_state.s = *state == ElementState::Pressed;
                        }
                        if *scancode == D_SCANCODE {
                            new_state.d = *state == ElementState::Pressed;
                        }
                        if *scancode == A_SCANCODE {
                            new_state.a = *state == ElementState::Pressed;
                        }
                        if *scancode == UP_SCANCODE {
                            new_state.up = *state == ElementState::Pressed;
                        }
                        if *scancode == DOWN_SCANCODE {
                            new_state.down = *state == ElementState::Pressed;
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if *button == MouseButton::Right {
                            new_state.rigth = *state == ElementState::Pressed;
                        }

                        if *button == MouseButton::Left {
                            new_state.left = *state == ElementState::Pressed;
                        }
                    }
                    _ => {}
                };
            }
            Event::DeviceEvent { event, .. } => {
                match event {
                    DeviceEvent::MouseMotion { delta: (x, y) } => {
                        let x = *x as f32;
                        let y = *y as f32;
                        new_state.cursor_delta = [self.cursor_delta[0] + x, self.cursor_delta[1] + y];
                    },
                    DeviceEvent::MouseWheel { delta } => {
                        match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => {
                                new_state.scroll_delta = *y;
                            },
                            winit::event::MouseScrollDelta::PixelDelta(d ) => {
                                new_state.scroll_delta = d.y as f32;
                            },
                        }
                    },
                    
                    _ => ()
                };
            }
            _ => (),
        }

        new_state
    }
}
