use std::{env, io::Write, time::Instant};

use app::App;
use fsr::FsrContextVulkan;
use log::{debug, info};
use renderer::{
    draw,
    render_faces::{Camera, RenderFacesPipeline},
};
use vulkano::{
    command_buffer::{
        CommandBufferBeginInfo, CommandBufferLevel, CommandBufferUsage, CopyImageInfo,
        RecordingCommandBuffer,
    },
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage, SampleCount},
    memory::allocator::AllocationCreateInfo,
    pipeline::graphics::{subpass::PipelineRenderingCreateInfo, viewport::Viewport},
    sync::GpuFuture,
    VulkanObject,
};
use vulkano_util::{renderer::VulkanoWindowRenderer, window::WindowDescriptor};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

mod app;
mod fsr;
mod model;
mod renderer;
mod resources;
mod texture;
mod types;

fn run(app: &mut App) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window_id = app.windows.create_window(
        &event_loop,
        &app.context,
        &WindowDescriptor {
            width: 1680.0,
            height: 960.0,
            title: "block-world".to_string(),
            resizable: false,
            ..Default::default()
        },
        |create_info| {
            create_info.image_usage = ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST;
            // create_info.image_format = Format::R16G16B16A16_SFLOAT;
            // create_info.image_color_space = ColorSpace::ExtendedSrgbLinear;
        },
    );

    let queue = app.context.graphics_queue().clone();

    let render_faces_pipeline = RenderFacesPipeline::new(
        &app,
        queue.clone(),
        PipelineRenderingCreateInfo {
            color_attachment_formats: vec![
                Some(
                    app.windows
                        .get_renderer(window_id)
                        .unwrap()
                        .swapchain_format(),
                ),
                Some(Format::R16G16_SFLOAT),
            ],
            depth_attachment_format: Some(Format::D16_UNORM),
            ..Default::default()
        },
    );

    // println!(
    //     "{:?}",
    //     app.windows
    //         .get_renderer_mut(window_id)
    //         .unwrap()
    //         .set_present_mode()
    // );

    let render_start = Instant::now();
    let camera_fn = || {
        let elapsed = render_start.elapsed().as_secs_f32();
        let position = cgmath::Point3::new(
            (elapsed * 0.5).sin() * 3.0,
            elapsed.sin() * 3.0,
            (elapsed * 0.5).cos() * 3.0,
        );
        let near = 0.1;
        let far = 100.0;
        let fovy = cgmath::Deg(60.0);

        Camera {
            position,
            view: cgmath::Matrix4::look_at_rh(
                position,
                cgmath::Point3::new(0.0, 0.0, 0.0),
                cgmath::Vector3::unit_y(),
            ),
            proj: cgmath::perspective(fovy, 1280.0 / 720.0, near, far),
            near,
            far,
            fovy,
        }
    };

    let samples = SampleCount::Sample1;

    let display_size_extent = app
        .windows
        .get_renderer_mut(window_id)
        .unwrap()
        .swapchain_image_view()
        .image()
        .extent();
    let display_size = [display_size_extent[0], display_size_extent[1]];
    let render_size = [1680, 960];
    let render_size_extent = [render_size[0], render_size[1], 1];
    // let render_size = display_size;
    // let render_size_extent = [render_size[0], render_size[1], 1];

    println!("Render size: {:?}", render_size);
    println!("Display size: {:?}", display_size);

    let color_image = ImageView::new_default(
        Image::new(
            app.memory_allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                extent: render_size_extent,
                format: app
                    .windows
                    .get_renderer(window_id)
                    .unwrap()
                    .swapchain_format(),
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::SAMPLED,
                samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();
    debug!(
        "Color image view: {:?}, image: {:?}",
        color_image.handle(),
        color_image.image().handle()
    );

    let depth_image = ImageView::new_default(
        Image::new(
            app.memory_allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                extent: render_size_extent,
                format: Format::D16_UNORM,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::SAMPLED,
                samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();
    debug!(
        "Depth image view: {:?}, image: {:?}",
        depth_image.handle(),
        depth_image.image().handle()
    );

    let motion_vector_image = ImageView::new_default(
        Image::new(
            app.memory_allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                extent: render_size_extent,
                format: Format::R16G16_SFLOAT,
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::SAMPLED,
                samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();
    debug!(
        "Motion vector image view: {:?}, image: {:?}",
        motion_vector_image.handle(),
        motion_vector_image.image().handle()
    );

    let output_image = ImageView::new_default(
        Image::new(
            app.memory_allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                extent: display_size_extent,
                format: app
                    .windows
                    .get_renderer(window_id)
                    .unwrap()
                    .swapchain_format(),
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::STORAGE
                    | ImageUsage::TRANSFER_SRC,
                samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();
    debug!(
        "Output image view: {:?}, image: {:?}",
        output_image.handle(),
        output_image.image().handle()
    );

    let ash_device = unsafe {
        ash::Device::load(
            &app.context.instance().fns().v1_0,
            app.context.device().handle(),
        )
    };

    let mut fsr_context =
        unsafe { FsrContextVulkan::new(app.context.device(), render_size, display_size) };
    info!("FsrContextVulkan created");

    let command_buffer_allocator = app.command_buffer_allocator.clone();
    let mut previous_camera = camera_fn();
    let mut frame_time = Instant::now();
    let mut redraw = |renderer: &mut VulkanoWindowRenderer| {
        let before = renderer.acquire(None, |_| {}).unwrap();

        let jitter_matrix = unsafe { fsr_context.step_jitter() };

        let mut camera = camera_fn();
        camera.proj = jitter_matrix * camera.proj;

        let viewport = Viewport {
            extent: [render_size[0] as f32, render_size[1] as f32],
            ..Default::default()
        };

        let mut builder = RecordingCommandBuffer::new(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferLevel::Primary,
            CommandBufferBeginInfo {
                usage: CommandBufferUsage::OneTimeSubmit,
                ..Default::default()
            },
        )
        .unwrap();

        debug!(
            "Swapchain image view: {:?}, image: {:?}",
            renderer.swapchain_image_view().handle(),
            renderer.swapchain_image_view().image().handle()
        );

        draw(
            &mut builder,
            color_image.clone(),
            motion_vector_image.clone(),
            depth_image.clone(),
            viewport,
            |builder| {
                render_faces_pipeline.render_cube_faces(builder, &previous_camera, &camera);
            },
        );
        previous_camera = camera.clone();

        let mut fsr_builder = RecordingCommandBuffer::new(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferLevel::Primary,
            CommandBufferBeginInfo {
                usage: CommandBufferUsage::OneTimeSubmit,
                ..Default::default()
            },
        )
        .unwrap();

        let elapsed = frame_time.elapsed();
        frame_time = Instant::now();
        print!(
            "Frame time: {:.2?}, FPS: {:.2}\r",
            elapsed,
            1.0 / elapsed.as_secs_f32(),
        );
        std::io::stdout().flush().unwrap();

        let fsr_command_buffer = unsafe {
            debug!("fsr_command_buffer: {:?}", fsr_builder.raw().handle());
            fsr_context.dispatch(
                ash_device.clone(),
                &fsr_builder.raw(),
                &color_image,
                &depth_image,
                &motion_vector_image,
                &output_image,
                elapsed.as_millis() as f32,
                camera,
            );
            debug!("Recording command buffer");
            fsr_builder
                .copy_image(CopyImageInfo::images(
                    output_image.image().clone(),
                    renderer.swapchain_image_view().image().clone(),
                ))
                .unwrap();
            fsr_builder.end().unwrap()
        };

        let command_buffer = builder.end().unwrap();

        let after = before
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_execute(queue.clone(), fsr_command_buffer)
            .unwrap()
            .then_signal_semaphore_and_flush()
            .unwrap()
            .boxed();
        renderer.present(after, true);
    };

    event_loop
        .run(move |event, elwt| {
            let renderer = app.windows.get_renderer_mut(window_id).unwrap();
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(..) => {
                        renderer.resize();
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        renderer.resize();
                    }
                    WindowEvent::RedrawRequested => {
                        redraw(renderer);
                        if app
                            .validation_error_encountered
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            // panic!("Validation error encountered");
                        }
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    app.windows.get_window(window_id).unwrap().request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();
    info!("Starting block-world");
    let mut app = App::new();
    run(&mut app);
}
