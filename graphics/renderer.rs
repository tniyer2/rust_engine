
use std::iter;

use raw_window_handle::HasRawWindowHandle;

use shaderc::ShaderKind;

use gfx_hal::{
    Instance,
    window::{Surface, PresentationSurface, Extent2D},
    adapter::Adapter,
    device::Device,
    queue::family::QueueGroup
};

use super::compile_shader::compile_shader;

pub struct Renderer<B: gfx_hal::Backend> {
    resources: Option<Resources<B>>,
    surface_extent: Extent2D,
    should_configure_swapchain: bool
}

struct Resources<B: gfx_hal::Backend> {
    pub instance: B::Instance,
    pub surface: B::Surface,
    pub device: B::Device,
    pub adapter: Adapter<B>,

    pub color_format: gfx_hal::format::Format,

    pub render_passes: Vec<B::RenderPass>,
    pub pipeline_layouts: Vec<B::PipelineLayout>,
    pub pipelines: Vec<B::GraphicsPipeline>,

    pub command_pool: B::CommandPool,
    pub command_buffer: B::CommandBuffer,
    pub queue_group: QueueGroup<B>,

    pub submission_complete_fence: B::Fence,
    pub rendering_complete_semaphore: B::Semaphore
}

impl<B: gfx_hal::Backend> Renderer<B> {
    pub fn new(
        app_name: &str,
        physical_size: [u32; 2],
        window: &impl HasRawWindowHandle,
        vertex_shader: &str,
        fragment_shader: &str
    ) -> Self {

        // Set Up Access to the Graphics Backend
        let (instance, surface, adapter) = {
            // Create an Instance
            // An Instance Exposes the Surface and Adapter
            let instance = B::Instance::create(app_name, 1)
                .expect("Backend not supported");

            // Create a Surface
            // A Surface Describes a Display's Capabilities
            let surface = unsafe {
                instance
                    .create_surface(window)
                    .expect("Failed to create surface for window")
            };

            // Use the First Available Adapter
            // An Adapter Describes a Physical Device
            let adapter = instance.enumerate_adapters().remove(0);

            (instance, surface, adapter)
        };

        // Set Up a Logical Device
        let (device, queue_group) = {
            use gfx_hal::queue::family::QueueFamily;

            // Find a Compatible QueueFamily
            let queue_family = adapter
                .queue_families
                .iter()
                .find(|family| {
                    surface.supports_queue_family(family)
                    && family.queue_type().supports_graphics()
                })
                .expect("No compatible queue family found");

            // Create a Logical Device
            let mut gpu = unsafe {
                use gfx_hal::adapter::PhysicalDevice;

                adapter.physical_device
                    .open(&[(queue_family, &[1.0])], gfx_hal::Features::empty())
                    .expect("Failed to open device")
            };

            // GPU holds a Logical Device and its Queue Groups.
            (gpu.device, gpu.queue_groups.pop().unwrap())
        };

        // Set Up a Command Buffer
        let (command_pool, command_buffer) = unsafe {
            use gfx_hal::pool::{CommandPool, CommandPoolCreateFlags};
            use gfx_hal::command::Level;

            let mut command_pool = device
                .create_command_pool(queue_group.family, CommandPoolCreateFlags::empty())
                .expect("Out of memory");

            let command_buffer = command_pool.allocate_one(Level::Primary);

            (command_pool, command_buffer)
        };

        // Find an SRGB Color Format Compatible with the Surface
        let color_format = {
            use gfx_hal::format::{ChannelType, Format};

            // Get All Compatible Color Formats
            let supported_formats = surface
                .supported_formats(&adapter.physical_device)
                .unwrap_or(vec![]);

            // Set the Default to Preferred Format
            let preferred = supported_formats.get(0);

            // If No Preferred Format, Choose Whatever
            let default = *(preferred.unwrap_or(&Format::Rgba8Srgb));

            // Find an SRGB Color Format or Use the Default
            supported_formats
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap_or(default)
        };

        // Create a Render Pass
        let render_pass = {
            use gfx_hal::pass::{
                Attachment, AttachmentOps,
                AttachmentLoadOp, AttachmentStoreOp,
                SubpassDesc
            };
            use gfx_hal::image::Layout;

            // Describe an Attachment
            let color_attachment = Attachment {
                format: Some(color_format),
                samples: 1,
                ops: AttachmentOps::new(
                    AttachmentLoadOp::Clear,
                    AttachmentStoreOp::Store
                ),
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::Present
            };

            // Describe a Subpass
            let subpass = SubpassDesc {
                colors: &[(0, Layout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                resolves: &[],
                preserves: &[]
            };

            // Create a RenderPass with the Descriptions
            unsafe {
                device
                    .create_render_pass(
                        iter::once(color_attachment),
                        iter::once(subpass),
                        iter::empty())
                    .expect("Out of memory")
            }
        };

        // Create a Pipeline Layout
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(iter::empty(), iter::empty())
                .expect("Out of memory")
        };

        // Create a Pipeline
        let pipeline = unsafe {
            Self::make_pipeline(
                &device,
                &render_pass,
                &pipeline_layout,
                vertex_shader,
                fragment_shader
            )
        };

        // Syncs CPU to GPU
        let submission_complete_fence = device.create_fence(true).expect("Out of memory");
        // Syncs Internal GPU Processes
        let rendering_complete_semaphore = device.create_semaphore().expect("Out of memory");

        Self {
            resources: Some(Resources {
                instance,
                surface,
                device,
                adapter,

                color_format,

                command_pool,
                command_buffer,
                queue_group,

                render_passes: vec![render_pass],
                pipeline_layouts: vec![pipeline_layout],
                pipelines: vec![pipeline],

                submission_complete_fence,
                rendering_complete_semaphore
            }),
            surface_extent: Extent2D {
                width: physical_size[0],
                height: physical_size[1]
            },
            should_configure_swapchain: true
        }
    }

    /// Create and Return a Pipeline.
    unsafe fn make_pipeline(
        device: &B::Device,
        render_pass: &B::RenderPass,
        pipeline_layout: &B::PipelineLayout,
        vertex_shader: &str,
        fragment_shader: &str,
    ) -> B::GraphicsPipeline {
        use gfx_hal::pass::Subpass;
        
        // Create Shader Object Modules
        let vertex_shader_module = device
            .create_shader_module(&compile_shader(vertex_shader, ShaderKind::Vertex))
            .expect("Failed to create vertex shader module");

        let fragment_shader_module = device
            .create_shader_module(&compile_shader(fragment_shader, ShaderKind::Fragment))
            .expect("Failed to create fragment shader module");

        use gfx_hal::pso::{
            EntryPoint, Specialization
        };
        // Describe Shader Entry Points
        let (vertex_shader_entry, fragment_shader_entry) = (
            EntryPoint {
                entry: "main",
                module: &vertex_shader_module,
                specialization: Specialization::default()
            },
            EntryPoint {
                entry: "main",
                module: &fragment_shader_module,
                specialization: Specialization::default()
            },
        );

        use gfx_hal::pso::{
            PrimitiveAssemblerDesc, InputAssemblerDesc, Primitive
        };
        // Describe the Primitive Assembler
        // A Primitive Assembler Transforms Input into Primitives
        let primitive_assembler = PrimitiveAssemblerDesc::Vertex {
            buffers: &[],
            attributes: &[],
            input_assembler: InputAssemblerDesc::new(Primitive::TriangleList),
            vertex: vertex_shader_entry,
            tessellation: None,
            geometry: None
        };

        use gfx_hal::pso::{
            GraphicsPipelineDesc, Rasterizer, Face
        };
        // Describe the Pipeline
        let mut pipeline_desc = GraphicsPipelineDesc::new(
            primitive_assembler,
            Rasterizer {
                cull_face: Face::BACK,
                ..Rasterizer::FILL
            },
            Some(fragment_shader_entry),
            pipeline_layout,
            Subpass {
                index: 0,
                main_pass: render_pass
            }
        );

        use gfx_hal::pso::{
            ColorBlendDesc, ColorMask, BlendState
        };
        // Set Blend Mode to Alpha Blend
        pipeline_desc.blender.targets.push(ColorBlendDesc {
            mask: ColorMask::ALL,
            blend: Some(BlendState::ALPHA)
        });

        // Create the Pipeline
        let pipeline = device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Failed to create graphics pipeline");

        // Clean Up Shader Object Modules
        device.destroy_shader_module(vertex_shader_module);
        device.destroy_shader_module(fragment_shader_module);

        pipeline
    }

    pub fn update_dimensions(&mut self, physical_size: [u32; 2]) {
        self.surface_extent = Extent2D {
            width: physical_size[0],
            height: physical_size[1]
        };
        self.should_configure_swapchain = true;
    }

    pub fn render(&mut self) {
        let res: &mut Resources<_> = self.resources.as_mut().unwrap();
        let render_pass = &res.render_passes[0];
        let pipeline = &res.pipelines[0];

        // Wait Until Previous Draw Commands Are Submitted
        unsafe {
            use gfx_hal::pool::CommandPool;

            // We refuse to wait more than a second, to avoid hanging.
            const RENDER_TIMEOUT_NS: u64 = 1_000_000_000;

            res.device
                .wait_for_fence(&res.submission_complete_fence, RENDER_TIMEOUT_NS)
                .expect("Out of memory or device lost");

            res.device
                .reset_fence(&mut res.submission_complete_fence)
                .expect("Out of memory");

            res.command_pool.reset(false);
        }

        // Update Swapchain if Needed
        // Get Framebuffer Attachment from Swapchain
        let framebuffer_attachment = {
            use gfx_hal::window::SwapchainConfig;

            // Get Supported Swapchain Capabilities
            let caps = res.surface.capabilities(&res.adapter.physical_device);

            // Create a Swapchain Configuration
            let mut swapchain_config =
                SwapchainConfig::from_caps(&caps, res.color_format, self.surface_extent);

            // Fix Fullscreen Slowdown on MacOS.
            if caps.image_count.contains(&3) {
                swapchain_config.image_count = 3;
            }

            // Update new Window Size
            self.surface_extent = swapchain_config.extent;

            let fat = swapchain_config.framebuffer_attachment();

            // Configure the Swapchain with the new Configuration
            if self.should_configure_swapchain {
                unsafe {
                    res.surface
                        .configure_swapchain(&res.device, swapchain_config)
                        .expect("Failed to configure swapchain");
                };

                self.should_configure_swapchain = false;
            }

            fat
        };

        // Get Image From Swapchain
        let surface_image = unsafe {
            let acquire_timeout_ns = 1_000_000_000;

            match res.surface.acquire_image(acquire_timeout_ns) {
                Ok((image, _)) => image,
                Err(_) => {
                    self.should_configure_swapchain = true;
                    return;
                }
            }
        };

        // Create a FrameBuffer
        // A FrameBuffer Stores an Image to Fill an Attachment
        let framebuffer = unsafe {
            use gfx_hal::image::Extent;

            res.device
                .create_framebuffer(
                    render_pass,
                    iter::once(framebuffer_attachment),
                    Extent {
                        width: self.surface_extent.width,
                        height: self.surface_extent.height,
                        depth: 1
                    },
                )
                .unwrap()
        };

        // Describe the Viewport
        let viewport = {
            use gfx_hal::pso::{Viewport, Rect};

            Viewport {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: self.surface_extent.width as i16,
                    h: self.surface_extent.height as i16
                },
                depth: 0.0..1.0
            }
        };

        // Line Up Draw Commands
        unsafe {
            use std::borrow::Borrow;

            use gfx_hal::command::{
                CommandBuffer, CommandBufferFlags,
                RenderAttachmentInfo, ClearValue, ClearColor, SubpassContents
            };

            res.command_buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);

            res.command_buffer.set_viewports(0, iter::once(viewport.clone()));
            res.command_buffer.set_scissors(0, iter::once(viewport.rect));

            // Clear to Black
            res.command_buffer.begin_render_pass(
                render_pass,
                &framebuffer,
                viewport.rect,
                iter::once(RenderAttachmentInfo {
                    image_view: surface_image.borrow(),
                    clear_value: ClearValue {
                        color: ClearColor {
                            float32: [0.0, 0.0, 0.0, 1.0]
                        }
                    }
                }),
                SubpassContents::Inline
            );

            res.command_buffer.bind_graphics_pipeline(pipeline);

            // Draw a Triangle
            res.command_buffer.draw(0..3, 0..1);

            res.command_buffer.end_render_pass();
            res.command_buffer.finish();
        }

        // Execute Draw Commands and Present
        unsafe {
            use gfx_hal::queue::CommandQueue;

            // Submit Commands to be Executed
            res.queue_group.queues[0].submit(
                iter::once(&res.command_buffer),
                iter::empty(),
                iter::once(&res.rendering_complete_semaphore),
                Some(&mut res.submission_complete_fence)
            );

            // Present Swapchain Image after all Commands Execute
            let result = res.queue_group.queues[0].present(
                &mut res.surface,
                surface_image,
                Some(&mut res.rendering_complete_semaphore),
            );

            self.should_configure_swapchain |= result.is_err();

            res.device.destroy_framebuffer(framebuffer);
        }
    }
}

impl<B: gfx_hal::Backend> Drop for Renderer<B> {
    fn drop(&mut self) {
        let mut r = Option::take(&mut self.resources).unwrap();

        unsafe {
            r.device.destroy_semaphore(r.rendering_complete_semaphore);
            r.device.destroy_fence(r.submission_complete_fence);

            for pipeline in r.pipelines {
                r.device.destroy_graphics_pipeline(pipeline);
            }
            for pipeline_layout in r.pipeline_layouts {
                r.device.destroy_pipeline_layout(pipeline_layout);
            }
            for render_pass in r.render_passes {
                r.device.destroy_render_pass(render_pass);
            }

            r.device.destroy_command_pool(r.command_pool);
            r.surface.unconfigure_swapchain(&r.device);
            r.instance.destroy_surface(r.surface);
        };
    }
}
