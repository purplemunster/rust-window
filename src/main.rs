// Window related libraries
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop, EventLoopWindowTarget };
use winit::window::WindowBuilder;
use winit::platform::run_return::EventLoopExtRunReturn;

#[cfg(target_os = "macos")]
use objc::rc::autoreleasepool;

pub struct WindowWrapper
{
    window : winit::window::Window,
    event_loop : winit::event_loop::EventLoop<()>
}

impl WindowWrapper
{
    pub fn create(title: &str, dims: [u32; 2]) -> Self {
        
        let event_loop = EventLoop::new();

        let display_size = winit::dpi::PhysicalSize::new(dims[0], dims[1]);

        let window = WindowBuilder::new()
        .with_inner_size(display_size)
        .with_title(title)
        .build(&event_loop).expect("Failed to create window");

        WindowWrapper {
            window: window,
            event_loop: event_loop
        }

    }

    pub fn run(self) {

        #[allow(unused_variables, unused_mut)]
        let WindowWrapper {
            mut window,
            mut event_loop
        } = self;

        let mut last_time = std::time::SystemTime::now();

        // Moving closure, takes ownership of all variables that it uses.
        let event_handler = move | event: Event<'_,()>, _ : &EventLoopWindowTarget<()>, control_flow : &mut ControlFlow |
        {
            let code_block = ||
            {
                *control_flow = ControlFlow::Poll;

                match event
                {
                    Event::WindowEvent { event, .. } => match event
                    {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit
                        },

                        WindowEvent::Resized(_size) => {
                        },

                        // Handle keyboard events
                        WindowEvent::KeyboardInput { input, ..} => match input.virtual_keycode {
                            Some(keycode) => {
                                match keycode {
                                    winit::event::VirtualKeyCode::Z => {
                                    },
                                    winit::event::VirtualKeyCode::X => {
                                    },
                                    winit::event::VirtualKeyCode::W => {
                                    },
                                    winit::event::VirtualKeyCode::S => {
                                    },
                                    winit::event::VirtualKeyCode::A => {
                                    },
                                    winit::event::VirtualKeyCode::D => {
                                    }
                                    _ => (),
                                }
                            },
                            None => ()
                        },
                        _ => (),
                    },

                    Event::MainEventsCleared => {
                        window.request_redraw();
                    },

                    Event::RedrawRequested(_) => {

                        let curr_time = std::time::SystemTime::now();
                        let elapsed_time = match curr_time.duration_since(last_time) {
                            Ok(duration) => duration,
                            Err(_) => std::time::Duration::new(1, 0)
                        };

                        let time_delta_seconds = elapsed_time.as_secs_f32();
                        last_time = curr_time;
                
                        window.set_title(&format!("Window Test: {}seconds", time_delta_seconds));
                    },
                    _ => (),
                }
            };

            #[cfg(target_os = "macos")]
            autoreleasepool(code_block);

            #[cfg(not(target_os = "macos"))]
            code_block();
        };

        event_loop.run_return(event_handler);
    }
}

// Main entry point
fn main()
{
    let wrapper = WindowWrapper::create("Window App", [1280, 720]);
    wrapper.run();
}