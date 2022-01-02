// Window related libraries
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop, EventLoopWindowTarget };
use winit::window::WindowBuilder;
use winit::platform::run_return::EventLoopExtRunReturn;

use dolly::prelude::*;
use directx_math::*;
use glam::*;

#[cfg(target_os = "macos")]
use objc::rc::autoreleasepool;

#[cfg(target_os = "macos")]
pub mod mtl;
pub use mtl::*;

pub struct WindowWrapper
{
    window: winit::window::Window,
    event_loop: winit::event_loop::EventLoop<()>,
    renderer: RenderContext
}

struct Vertex
{
    position : [f32; 3],
    color : [f32; 4]
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

        let renderer =  RenderContext::new(&window, dims);

        WindowWrapper {
            window: window,
            event_loop: event_loop,
            renderer: renderer
        }

    }

    pub fn run(self) {

        #[allow(unused_variables, unused_mut)]
        let WindowWrapper {
            mut window,
            mut event_loop,
            mut renderer,
        } = self;

        let mut last_time = std::time::SystemTime::now();

        // default library
        let raster_pipeline = PipelineBuilder::new()
            .from_shader_lib("src/shaders.metal")
            .with_vertex_function("triangle_vertex")
            .with_fragment_function("triangle_fragment")
            .with_attachment(metal::MTLPixelFormat::BGRA8Unorm);

        let pso = renderer.create_raster_pipeline(&raster_pipeline);

        let triangle_vertices = [
            // 2D positions,    RGBA colors
            Vertex { position: [ 0.0, 0.5, -2.0], color: [1.0, 0.0, 0.0, 1.0] },
            Vertex { position: [ 0.5,-0.5, -2.0], color: [0.0, 1.0, 0.0, 1.0] },
            Vertex { position: [-0.5,-0.5, -2.0], color: [0.0, 0.0, 1.0, 1.0] }
        ];

        let stride = std::mem::size_of::<Vertex>();

        let vertex_buffer = {
            renderer.create_buffer_with_data(
                triangle_vertices.as_ptr() as *const _,
                (triangle_vertices.len() * stride) as u64
            )
        };

        let mut camera = CameraRig::builder()
            .with(Position::new(Vec3::new(0.0, 0.0, 0.0)))
            .with(YawPitch::new())
            .with(Smooth::new_position_rotation(1.0, 1.0))
            .build();

        let display_size = renderer.display_size();

        let fov_y = XMConvertToRadians(65.0);
        let aspect_ratio = display_size[0] as f32 / display_size[0] as f32;
        let z_near = 0.1;
        let z_far = 1000.0;
        let perspective_matrix = XMMatrixPerspectiveFovRH(fov_y, aspect_ratio, z_near, z_far);

        let mut move_direction = Vec3::new(0.0, 0.0, 0.0);
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

                        WindowEvent::Resized(size) => {
                            renderer.resize([size.width, size.height]);
                        },

                        // Handle keyboard events
                        WindowEvent::KeyboardInput { input, ..} => match input.virtual_keycode {
                            Some(keycode) => {
                                match keycode {
                                    winit::event::VirtualKeyCode::Z => {
                                        camera.driver_mut::<YawPitch>().rotate_yaw_pitch(-90.0, 0.0);
                                    },
                                    winit::event::VirtualKeyCode::X => {
                                        camera.driver_mut::<YawPitch>().rotate_yaw_pitch(90.0, 0.0);
                                    },
                                    winit::event::VirtualKeyCode::W => {
                                        move_direction.z = -1.0;
                                    },
                                    winit::event::VirtualKeyCode::S => {
                                        move_direction.z = 1.0;
                                    },
                                    winit::event::VirtualKeyCode::A => {
                                        move_direction.x = -1.0;
                                    },
                                    winit::event::VirtualKeyCode::D => {
                                        move_direction.x = 1.0;
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

                        let move_vec = camera.final_transform.rotation * move_direction.clamp_length_max(1.0) * 10.0f32.powf(0.0);

                        camera.driver_mut::<Position>().translate(move_vec * time_delta_seconds * 10.0);
                        camera.update(time_delta_seconds);

                        let camera_xform = camera.final_transform;
                        let target = camera_xform.position + camera_xform.forward();
                        let eye = XMVectorSet(camera_xform.position.x, camera_xform.position.y, camera_xform.position.z, 1.0);
                        let focus = XMVectorSet(target.x, target.y, target.z, 1.0);
                        let up = XMVectorSet(camera_xform.up().x, camera_xform.up().y, camera_xform.up().z, 1.0);

                        let view_matrix = XMMatrixLookAtRH(eye, focus, up);

                        let mvp_array : [f32; 16] = XMMatrix(XMMatrixTranspose(XMMatrixMultiply(view_matrix, &perspective_matrix))).into();

                        let render_pass_descriptor = metal::RenderPassDescriptor::new();
                        let color_attachment = render_pass_descriptor.color_attachments().object_at(0).unwrap();

                        let drawable = renderer.swapchain().next_drawable();
                        color_attachment.set_texture(Some(drawable));
                        color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                        color_attachment.set_clear_color(metal::MTLClearColor::new(0.2, 0.25, 0.3, 1.0));
                        color_attachment.set_store_action(metal::MTLStoreAction::Store);

                        let cmd_buffer = renderer.new_command_buffer();
                        let encoder = cmd_buffer.new_render_command_encoder(&render_pass_descriptor);

                        encoder.set_viewport(metal::MTLViewport {
                            originX: 0.0, originY: 0.0,
                            width: display_size[0] as f64, height: display_size[1] as f64, znear: 0.0, zfar: 1.0
                        });
                        encoder.set_scissor_rect(metal::MTLScissorRect {
                            x: 0, y: 0, width: display_size[0] as u64, height: display_size[1] as u64
                        });

                        encoder.set_render_pipeline_state(&pso);
                        encoder.set_vertex_buffer(1, Some(&vertex_buffer), 0);
                        encoder.set_vertex_bytes(0, 64, mvp_array.as_ptr() as *const std::ffi::c_void);
                        encoder.draw_primitives(metal::MTLPrimitiveType::Triangle, 0, 3);
                        encoder.end_encoding();

                        renderer.swapchain().present(cmd_buffer);
                        cmd_buffer.commit();

                        // reset state
                        move_direction.x = 0.0;
                        move_direction.z = 0.0;
                        move_direction.y = 0.0;
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