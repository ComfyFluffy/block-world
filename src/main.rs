use std::time::Instant;

use renderer::{
    draw,
    render_faces::{Camera, RenderFacesPipeline},
};
use vulkano::{
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage, SampleCount},
    memory::allocator::AllocationCreateInfo,
    pipeline::graphics::subpass::PipelineRenderingCreateInfo,
};
use vulkano_util::{renderer::VulkanoWindowRenderer, window::WindowDescriptor};
use window::App;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

mod renderer;
mod resources;
mod texture;
mod types;
mod window;

fn run(app: &mut App) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window_id = app.windows.create_window(
        &event_loop,
        &app.context,
        &WindowDescriptor {
            width: 1280.0,
            height: 720.0,
            title: "block-world".to_string(),
            resizable: false,
            ..Default::default()
        },
        |create_info| {
            // create_info.image_format = Format::R16G16B16A16_SFLOAT;
            // create_info.image_color_space = ColorSpace::ExtendedSrgbLinear;
        },
    );

    let queue = app.context.graphics_queue().clone();

    let render_faces_pipeline = RenderFacesPipeline::new(
        &app,
        queue.clone(),
        PipelineRenderingCreateInfo {
            color_attachment_formats: vec![Some(
                app.windows
                    .get_renderer(window_id)
                    .unwrap()
                    .swapchain_format(),
            )],
            depth_attachment_format: Some(Format::D32_SFLOAT),
            ..Default::default()
        },
    );

    let render_start = Instant::now();
    let camera_fn = || {
        let elapsed = render_start.elapsed().as_secs_f32();
        let position = cgmath::Point3::new(
            (elapsed * 0.5).sin() * 3.0,
            1.0,
            (elapsed * 0.5).cos() * 3.0,
        );
        Camera {
            position,
            view: cgmath::Matrix4::look_at_rh(
                position,
                cgmath::Point3::new(0.0, 0.0, 0.0),
                cgmath::Vector3::unit_y(),
            ),
            proj: cgmath::perspective(cgmath::Deg(60.0), 1280.0 / 720.0, 0.1, 100.0),
        }
    };

    let extent = app
        .windows
        .get_renderer_mut(window_id)
        .unwrap()
        .swapchain_image_view()
        .image()
        .extent();

    let samples = SampleCount::Sample4;

    let depth_image = ImageView::new_default(
        Image::new(
            app.memory_allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                extent: [extent[0], extent[1], 1],
                format: Format::D32_SFLOAT,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();

    let msaa_color_image = ImageView::new_default(
        Image::new(
            app.memory_allocator(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                extent: [extent[0], extent[1], 1],
                format: app
                    .windows
                    .get_renderer(window_id)
                    .unwrap()
                    .swapchain_format(),
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();

    let command_buffer_allocator = app.command_buffer_allocator.clone();
    let redraw = |renderer: &mut VulkanoWindowRenderer| {
        let before = renderer.acquire(None, |_| {}).unwrap();

        let after = draw(
            before,
            command_buffer_allocator.clone(),
            queue.clone(),
            msaa_color_image.clone(),
            renderer.swapchain_image_view(),
            depth_image.clone(),
            |builder| {
                render_faces_pipeline.render_cube_faces(builder, &camera_fn());
            },
        );
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
    let mut app = App::new();
    run(&mut app);
}
