
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder
};

mod graphics;
use graphics::{RenderState, Vertex};

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }
];

const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
    /* padding */ 0
];

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    let mut state = {
        use futures::executor::block_on;
        block_on(RenderState::new(
            &window,
            VERTICES,
            INDICES,
            include_str!("../shaders/default.wgsl")))
    };

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id
        } if window_id == window.id() => if !state.input(event) {
            match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                },

                // Exit when escape is pressed
                WindowEvent::KeyboardInput {input, ..} => match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit;
                    },
                    _ => {}
                },

                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                },

                WindowEvent::ScaleFactorChanged {new_inner_size, ..} => {
                    state.resize(**new_inner_size);
                },
                _ => {}
            };
        },

        Event::RedrawRequested(_) => {
            state.update();

            match state.render() {
                Ok(_) => {},

                // Recreate the swap_chain if lost
                Err(wgpu::SwapChainError::Lost) => {
                    state.resize(state.size);
                },

                // Exit if the system is out of memory
                Err(wgpu::SwapChainError::OutOfMemory) => {
                    *control_flow = ControlFlow::Exit;
                },

                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            };
        },

        Event::MainEventsCleared => {
            // Redraw ASAP, Non-constant framerate
            window.request_redraw();
        },
        _ => {}
    });
}
