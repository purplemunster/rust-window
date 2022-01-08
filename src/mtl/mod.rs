use metal::*;
use core_graphics_types::geometry::CGSize;
use std::mem;
use objc::runtime::YES;
use cocoa::{ appkit::NSView };
use winit::window::Window;
use winit::platform::macos::WindowExtMacOS;

pub struct SwapChain
{
    raw: metal::MetalLayer,
}

impl SwapChain
{
    pub fn resize(&self, size: [u32; 2]) {
        self.raw.set_drawable_size(CGSize::new(size[0] as f64, size[1] as f64));
    }

    pub fn next_drawable(&self) -> Option<&MetalDrawableRef> {
        self.raw.next_drawable()
    }
}

pub struct RenderContext
{
    device: metal::Device,
    queue: metal::CommandQueue,
    swapchain: SwapChain,
    display_size: [u32; 2]
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
            swapchain: SwapChain { raw: layer },
            display_size: display_size
        }
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.display_size = size;
        self.swapchain.resize(size);
    }

    pub fn create_raster_pipeline(&self, builder: &PipelineBuilder) -> metal::RenderPipelineState {

        let descriptor = builder.build(&self.device);
        self.device.new_render_pipeline_state(&descriptor).expect("Failed to create pipeline state")
    }

    pub fn create_buffer_with_data(&self, data: *const std::ffi::c_void, size: u64) -> metal::Buffer {
        let buffer = self.device.new_buffer_with_data(
            data,
            size,
            MTLResourceOptions::CPUCacheModeDefaultCache | MTLResourceOptions::StorageModeManaged,
        );

        buffer
    }

    pub fn display_size(&self) -> [u32; 2] {
        self.display_size
    }

    pub fn swapchain(&self) -> &SwapChain {
        &self.swapchain
    }

    pub fn new_command_buffer(&self) -> &metal::CommandBufferRef {
        self.queue.new_command_buffer()
    }

    pub fn new_ray_intersector(&self,
        ray_stride: u64, ray_data_type: MPSRayDataType,
        intersection_stride: u64, intersection_data_type: MPSIntersectionDataType) -> RayIntersector
    {
        let ray_intersector = RayIntersector::from_device(&self.device).expect("Failed to create ray intersector");

        ray_intersector.set_ray_stride(ray_stride);
        ray_intersector.set_ray_data_type(ray_data_type);
        ray_intersector.set_intersection_stride(intersection_stride);
        ray_intersector.set_intersection_data_type(intersection_data_type);

        ray_intersector
    }

    pub fn new_triangle_acceleration_structure(&self,
        vertex_buffer: &metal::BufferRef,
        vertex_stride: u32,
        index_buffer: Option<&metal::BufferRef>,
        triangle_count: u32) -> TriangleAccelerationStructure
    {
        let acceleration_structure = TriangleAccelerationStructure::from_device(&self.device).expect("Failed to create triangle acceleration structure");

        acceleration_structure.set_vertex_buffer(Some(&vertex_buffer));
        acceleration_structure.set_vertex_stride(vertex_stride as u64);
        acceleration_structure.set_index_buffer(index_buffer);
        acceleration_structure.set_index_type(MPSDataType::UInt32);
        acceleration_structure.set_triangle_count(triangle_count as u64);
        acceleration_structure.set_usage(MPSAccelerationStructureUsage::None);
        acceleration_structure.rebuild();

        acceleration_structure
    }

    pub fn create_depth_texture(&self, dims: [u64; 2]) -> metal::Texture {
        let desc = metal::TextureDescriptor::new();
        desc.set_texture_type(metal::MTLTextureType::D2);
        desc.set_pixel_format(metal::MTLPixelFormat::Depth32Float);
        desc.set_width(dims[0]);
        desc.set_height(dims[1]);
        desc.set_usage(metal::MTLTextureUsage::RenderTarget);
        desc.set_storage_mode(metal::MTLStorageMode::Private);

        self.device.new_texture(&desc)
    }

    pub fn create_depth_stencil_state(&self) -> metal::DepthStencilState {
        let desc = metal::DepthStencilDescriptor::new();
        desc.set_depth_compare_function(metal::MTLCompareFunction::LessEqual);
        desc.set_depth_write_enabled(true);
        desc.set_label("DepthStencil State");

        self.device.new_depth_stencil_state(&desc)
    }
}

pub struct PipelineBuilder
{   
    shader_lib: String,
    vertex_shader: String,
    fragment_shader: String,
    attachments: Vec<MTLPixelFormat>
}

impl PipelineBuilder
{
    pub fn new() -> Self {
        PipelineBuilder {
            shader_lib: "".to_string(),
            vertex_shader: "".to_string(),
            fragment_shader: "".to_string(),
            attachments: Vec::new()
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

    pub fn with_attachment(mut self, fmt: metal::MTLPixelFormat) -> Self {
        self.attachments.push(fmt);
        self
    }

    pub fn build(&self, device: &Device) -> RenderPipelineDescriptor {

        let library_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(self.shader_lib.clone());

        println!("{}", library_path.to_str().unwrap());
        let shader_lib = device.new_library_with_file(library_path).expect("Failed to compile shader source");
        let vertex_shader = shader_lib.get_function(&self.vertex_shader, None).expect("Failed to create vertex shader");
        let pixel_shader = shader_lib.get_function(&self.fragment_shader, None).expect("Failed to create pixel shader");

        let pipeline_state_descriptor = RenderPipelineDescriptor::new();
        pipeline_state_descriptor.set_vertex_function(Some(&vertex_shader));
        pipeline_state_descriptor.set_fragment_function(Some(&pixel_shader));

        for i in 0..self.attachments.len() {
            let attachment = pipeline_state_descriptor
            .color_attachments()
            .object_at(i as u64)
            .unwrap();

            attachment.set_pixel_format(self.attachments[i]);
        }

        pipeline_state_descriptor
    }
}