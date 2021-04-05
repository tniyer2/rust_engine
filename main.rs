
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder
};

mod renderer;
use renderer::Renderer;

fn main() {
    const APP_NAME: &'static str = "Rust Engine";
    const WINDOW_SIZE: [u32; 2] = [512, 512];

    let event_loop = EventLoop::new();

    let (logical_window_size, physical_window_size) = {
        use winit::dpi::{LogicalSize, PhysicalSize};

        let dpi = event_loop.primary_monitor().unwrap().scale_factor();
        let logical: LogicalSize<u32> = WINDOW_SIZE.into();
        let physical: PhysicalSize<u32> = logical.to_physical(dpi);

        (logical, physical)
    };

    let window = WindowBuilder::new()
        .with_title(APP_NAME)
        .with_inner_size(logical_window_size)
        .build(&event_loop)
        .expect("Failed to create window");

    let mut renderer = Renderer::<backend::Backend>::new(
        APP_NAME,
        physical_window_size.width,
        physical_window_size.height,
        &window);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        // Handle Events
        match event {
            // OS is Requesting to Close the Window.
            Event::WindowEvent {event: WindowEvent::CloseRequested, ..} => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            },

            // The Window has Resized
            Event::WindowEvent {event: WindowEvent::Resized(dims), ..} =>
               renderer.update_dimensions(dims.width, dims.height),

            // The Logical Scale has Changed
            Event::WindowEvent {event: WindowEvent::ScaleFactorChanged {new_inner_size, ..}, ..} =>
               renderer.update_dimensions(new_inner_size.width, new_inner_size.height),

            // Execute Non-draw Logic
            Event::MainEventsCleared => window.request_redraw(),

            // Execute Draw Logic
            Event::RedrawRequested(..) => renderer.render(),

            _ => ()
        }
    });
}
