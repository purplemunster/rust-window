// Window related libraries
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop, EventLoopWindowTarget };
use winit::window::WindowBuilder;
use winit::platform::run_return::EventLoopExtRunReturn;

use dolly::prelude::*;
use glam::{ f32::Vec3 };

#[cfg(target_os = "macos")]
use objc::rc::autoreleasepool;

#[cfg(target_os = "macos")]
pub mod mtl;
#[cfg(target_os = "macos")]
pub use mtl::*;

pub struct WindowWrapper
{
    window: winit::window::Window,
    event_loop: winit::event_loop::EventLoop<()>,
    renderer: RenderContext
}

struct MeshPrimitive {
    pub vertex_offset : u64,
    pub index_offset : u64,
    pub index_count : u64
}

struct AABB {
    min: [f32;3], max: [f32; 3]
}

impl AABB {
    fn union(&mut self, point: &[f32;3]) {

        self.min[0] = self.min[0].min(point[0]);
        self.min[1] = self.min[1].min(point[1]);
        self.min[2] = self.min[2].min(point[2]);

        self.max[0] = self.max[0].max(point[0]);
        self.max[1] = self.max[1].max(point[1]);
        self.max[2] = self.max[2].max(point[2]);
    }

    fn center(&self) -> [f32; 3] {
        [(self.max[0] - self.min[0]) * 0.5,
         (self.max[1] - self.min[1]) * 0.5,
         (self.max[2] - self.min[2]) * 0.5]
    }
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
            .from_shader_lib("src/shaders.metallib")
            .with_vertex_function("triangle_vertex")
            .with_fragment_function("triangle_fragment")
            .with_attachment(metal::MTLPixelFormat::BGRA8Unorm);

        let pso = renderer.create_raster_pipeline(&raster_pipeline);

        let (document, buffers, _images) = gltf::import("../kajiya/assets/meshes/336_lrm/scene.gltf").expect("Failed to load gltf!");
        let _scene = document.default_scene().unwrap();

        let mut triangle_vertices : Vec<[f32; 3]> = Vec::new();
        let mut triangle_normals : Vec<[f32; 3]> = Vec::new();
        let mut triangle_indices : Vec<u32> = Vec::new();

        let vertex_stride = std::mem::size_of::<[f32;3]>();
        let index_stride = std::mem::size_of::<u32>();

        let mut mesh_primitives : Vec<MeshPrimitive> = Vec::new();

        let mut bounds = AABB { 
            min: [std::f32::MAX, std::f32::MAX, std::f32::MAX],
            max: [-std::f32::MAX, -std::f32::MAX, -std::f32::MAX]
        };

        // Walk the mesh list and gather primitive vertex attributes and indices
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {

                // find associated buffer
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // read indices
                let indices = reader.read_indices().unwrap().into_u32().collect::<Vec<_>>();
                
                mesh_primitives.push(MeshPrimitive {
                    vertex_offset: (triangle_vertices.len() * vertex_stride) as u64,
                    index_offset: (triangle_indices.len() * index_stride)  as u64,
                    index_count: indices.len() as u64
                });

                // append to index buffer
                triangle_indices.extend_from_slice(&indices);

                // and vertices
                let vertices = reader.read_positions().unwrap().collect::<Vec<_>>();
                triangle_vertices.extend_from_slice(&vertices);

                for vertex in vertices.iter() {
                    bounds.union(vertex);
                }

                // and normals
                let normals = reader.read_normals().unwrap().collect::<Vec<_>>();
                triangle_normals.extend_from_slice(&normals);
            }
        }

        // Create mesh buffers
        let vertex_buffer = {
            renderer.create_buffer_with_data(
                triangle_vertices.as_ptr() as *const _,
                (triangle_vertices.len() * std::mem::size_of::<[f32;3]>()) as u64
            )
        };
        let normal_buffer = {
            renderer.create_buffer_with_data(
                triangle_normals.as_ptr() as *const _,
                (triangle_normals.len() * std::mem::size_of::<[f32;3]>()) as u64
            )
        };
        let index_buffer = {
            renderer.create_buffer_with_data(
                triangle_indices.as_ptr() as *const _,
                (triangle_indices.len() * std::mem::size_of::<u32>()) as u64
            )
        };

        /*
        let acceleration_structure = renderer.new_triangle_acceleration_structure(&vertex_buffer, std::mem::size_of::<[f32;3]>() as u32, None, 1u32);
        let ray_intersector = renderer.new_ray_intersector(
            std::mem::size_of::<metal::MPSRayOriginMinDistanceDirectionMaxDistance>() as u64,
            metal::MPSRayDataType::OriginMinDistanceDirectionMaxDistance,
            std::mem::size_of::<metal::MPSIntersectionDistancePrimitiveIndexCoordinates>() as u64,
            metal::MPSIntersectionDataType::DistancePrimitiveIndexCoordinates
        );
        */

        let bounds_center = bounds.center();
        let pos = bounds.max; //bounds.center();

        let mut camera = CameraRig::builder()
            .with(Position::new(Vec3::new(pos[0], pos[1], pos[2])))
            .with(YawPitch::new())
            .with(Smooth::new_position_rotation(1.0, 1.0))
            .build();

        let display_size = renderer.display_size();

        // create depth texture
        let depth_buffer = renderer.create_depth_texture([display_size[0] as u64, display_size[1] as u64]);
        let ds_state = renderer.create_depth_stencil_state();

        let aspect_ratio = display_size[0] as f32 / display_size[1] as f32;
        let z_near = 0.1;
        let z_far = 1000.0;
        let perspective_matrix = glam::f32::Mat4::perspective_rh(65.0_f32.to_radians(), aspect_ratio, z_near, z_far);

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
                        let target = Vec3::new(bounds_center[0], bounds_center[1], bounds_center[2]);

                        //camera_xform.position + camera_xform.forward();
                        let view_matrix = glam::f32::Mat4::look_at_rh(camera_xform.position, target, camera_xform.up());

                        let mut mvp_array : [f32; 16] = [0.0; 16];
                        (perspective_matrix * view_matrix).transpose().write_cols_to_slice(&mut mvp_array);

                        let render_pass_descriptor = metal::RenderPassDescriptor::new();
                        let color_attachment = render_pass_descriptor.color_attachments().object_at(0).unwrap();

                        let drawable = match renderer.swapchain().next_drawable() {
                            Some(drawable) => drawable,
                            None => return
                        };

                        color_attachment.set_texture(Some(drawable.texture()));
                        color_attachment.set_load_action(metal::MTLLoadAction::Clear);
                        color_attachment.set_clear_color(metal::MTLClearColor::new(0.2, 0.25, 0.3, 1.0));
                        color_attachment.set_store_action(metal::MTLStoreAction::Store);

                        let depth_attachment = render_pass_descriptor.depth_attachment().unwrap();
                        depth_attachment.set_texture(Some(&depth_buffer));
                        depth_attachment.set_clear_depth(1.0);
                        depth_attachment.set_store_action(metal::MTLStoreAction::DontCare);

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
                        encoder.set_vertex_bytes(0, 64, mvp_array.as_ptr() as *const std::ffi::c_void);
                        encoder.set_cull_mode(metal::MTLCullMode::Back);
                        encoder.set_front_facing_winding(metal::MTLWinding::Clockwise);
                        encoder.set_depth_stencil_state(&ds_state);

                        for prim in mesh_primitives.iter() {

                            encoder.set_vertex_buffer(1, Some(&vertex_buffer), prim.vertex_offset);
                            encoder.set_vertex_buffer(2, Some(&normal_buffer), prim.vertex_offset);
                            encoder.draw_indexed_primitives(
                                metal::MTLPrimitiveType::Triangle,
                                prim.index_count,
                                metal::MTLIndexType::UInt32,
                                &index_buffer,
                                prim.index_offset);
                        }

                        encoder.end_encoding();

                        cmd_buffer.present_drawable(drawable);
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