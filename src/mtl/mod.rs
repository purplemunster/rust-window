use metal::*;
use core_graphics_types::geometry::CGSize;
use std::mem;
use objc::runtime::YES;
use cocoa::{ appkit::NSView };
use winit::window::Window;
use winit::platform::macos::WindowExtMacOS;

pub struct RenderContext
{
    device: metal::Device,
    queue: metal::CommandQueue,
    swapchain: metal::MetalLayer
}

impl RenderContext
{
    pub fn new(window: &Window, display_size: [u32; 2]) -> Self {

        let device = metal::Device::system_default().expect("No default device found!");
        let queue = device.new_command_queue();

        let layer = metal::MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);
        layer.set_drawable_size(CGSize::new(display_size[0] as f64, display_size[1] as f64));

        // bind rendering layer to NSView
        unsafe
        {
            let view = window.ns_view() as cocoa::base::id;
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(layer.as_ref()));
        }

        RenderContext {
            device: device,
            queue: queue,
            swapchain: layer
        }
    }

    pub fn resize(&self, size: [u32; 2]) {
        self.swapchain.set_drawable_size(CGSize::new(size[0] as f64, size[1] as f64));
    }

    pub fn create_raster_pipeline(&self, builder: &PipelineBuilder) -> metal::RenderPipelineState {

        let descriptor = builder.build(&self.device);
        self.device.new_render_pipeline_state(&descriptor).expect("Failed to create pipeline state")
    }
}

pub struct PipelineBuilder
{
    shader_lib: String,
    vertex_shader: String,
    fragment_shader: String
}

impl PipelineBuilder
{
    pub fn new() -> Self {
        PipelineBuilder {
            shader_lib: "".to_string(),
            vertex_shader: "".to_string(),
            fragment_shader: "".to_string(),
        }
    }

    pub fn from_shader_lib(mut self, path: &str) -> Self {
        self.shader_lib = path.to_string();
        self
    }

    pub fn with_vertex_function(mut self, function: &str) -> Self {
        self.vertex_shader = function.to_string();
        self
    }

    pub fn with_fragment_function(mut self, function: &str) -> Self {
        self.fragment_shader = function.to_string();
        self
    }

    pub fn build(&self, device: &Device) -> RenderPipelineDescriptor {

        let options = metal::CompileOptions::new();
        let shader_source = std::fs::read_to_string(self.shader_lib.clone()).expect("Failed to read shader file");

        let shader_lib = device.new_library_with_source(&shader_source, &options).expect("Failed to compile shader source");
        let vertex_shader = shader_lib.get_function(&self.vertex_shader, None).expect("Failed to create vertex shader");
        let pixel_shader = shader_lib.get_function(&self.fragment_shader, None).expect("Failed to create pixel shader");

        let pipeline_state_descriptor = RenderPipelineDescriptor::new();
        pipeline_state_descriptor.set_vertex_function(Some(&vertex_shader));
        pipeline_state_descriptor.set_fragment_function(Some(&pixel_shader));

        pipeline_state_descriptor
    }
}