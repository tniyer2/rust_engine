
use gfx_hal::window::Extent2D;

use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;

mod renderer;
use renderer::Renderer;

fn main() {
    // Set Constants
    const APP_NAME: &'static str = "Rust Engine";
    const WINDOW_SIZE: [u32; 2] = [512, 512];

    // Create the EventLoop
    let event_loop = EventLoop::new();

    // Calculate the Logical and Physical Window Size
    let (logical_window_size, physical_window_size) = {
        use winit::dpi::{LogicalSize, PhysicalSize};

        let dpi = event_loop.primary_monitor().unwrap().scale_factor();
        let logical: LogicalSize<u32> = WINDOW_SIZE.into();
        let physical: PhysicalSize<u32> = logical.to_physical(dpi);

        (logical, physical)
    };

    // Create a Window
    let window = WindowBuilder::new()
        .with_title(APP_NAME)
        .with_inner_size(logical_window_size)
        .build(&event_loop)
        .expect("Failed to create window");

    // Describe Window Dimensions
    let mut surface_extent = Extent2D {
        width: physical_window_size.width,
        height: physical_window_size.height,
    };

    Renderer::new(APP_NAME, window, surface_extent, event_loop);
}
