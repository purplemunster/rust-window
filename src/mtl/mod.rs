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
}