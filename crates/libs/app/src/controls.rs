use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};

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

    pub q: bool,
    pub e: bool,
    pub r: bool, 
    pub t: bool,

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

            q: false,
            e: false,
            r: false,
            t: false,
            
            cursor_delta: [0.0; 2],
            scroll_delta: 0.0
        }
    }
}

impl Controls {
    pub fn reset(self) -> Self {
        Self {
            cursor_delta: [0.0; 2],
            scroll_delta: 0.0,
            ..self
        }
    }

    pub fn handle_event(self, event: &Event<()>) -> Self {
        let mut new_state = self;
        
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state, virtual_keycode, ..
                            },
                        ..
                    } => {
                        if virtual_keycode.is_some() {
                            match virtual_keycode.unwrap() {
                                VirtualKeyCode::W => { new_state.w = *state == ElementState::Pressed; },
                                VirtualKeyCode::S => { new_state.s = *state == ElementState::Pressed; },
                                VirtualKeyCode::A => { new_state.a = *state == ElementState::Pressed; },
                                VirtualKeyCode::D => { new_state.d = *state == ElementState::Pressed; },

                                VirtualKeyCode::Up => { new_state.up = *state == ElementState::Pressed; },
                                VirtualKeyCode::Down => { new_state.down = *state == ElementState::Pressed; },
                                VirtualKeyCode::Left => { new_state.left = *state == ElementState::Pressed; },
                                VirtualKeyCode::Right => { new_state.rigth = *state == ElementState::Pressed; },

                                VirtualKeyCode::Q => { new_state.q = *state == ElementState::Pressed; },
                                VirtualKeyCode::E => { new_state.e = *state == ElementState::Pressed; },
                                VirtualKeyCode::R => { new_state.r = *state == ElementState::Pressed; },
                                VirtualKeyCode::T => { new_state.t = *state == ElementState::Pressed; },
                                _ => {}
                            }
                            
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
