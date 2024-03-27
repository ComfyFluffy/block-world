use std::sync::Arc;

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        CommandBufferBeginInfo, CommandBufferLevel, CommandBufferUsage, CopyBufferToImageInfo,
        RecordingCommandBuffer,
    },
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{CompareOp, DepthState, DepthStencilState},
            multisample::MultisampleState,
            rasterization::{CullMode, RasterizationState},
            subpass::PipelineRenderingCreateInfo,
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::GpuFuture,
};

use crate::{types::Direction, window::App};

mod mesh {
    vulkano_shaders::shader!(
        ty: "mesh",
        path: "src/renderer/render_faces/render_faces.mesh.glsl",
        vulkan_version: "1.3"
    );
}

mod frag {
    vulkano_shaders::shader!(
        ty: "fragment",
        path: "src/renderer/render_faces/render_faces.frag.glsl",
        vulkan_version: "1.3"
    );
}

pub struct Camera {
    pub view: cgmath::Matrix4<f32>,
    pub proj: cgmath::Matrix4<f32>,
    pub position: cgmath::Point3<f32>,
}

fn upload_png(
    bytes: &[u8],
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer: &mut RecordingCommandBuffer,
) -> Arc<ImageView> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    let extent = [info.width, info.height, 1];

    let upload_buffer = Buffer::new_slice(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        (info.width * info.height * 4) as u64,
    )
    .unwrap();

    reader
        .next_frame(&mut upload_buffer.write().unwrap())
        .unwrap();

    let image = Image::new(
        memory_allocator,
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::R8G8B8A8_SRGB,
            extent,
            usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )
    .unwrap();

    command_buffer
        .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            upload_buffer,
            image.clone(),
        ))
        .unwrap();

    ImageView::new_default(image).unwrap()
}

pub struct RenderFacesPipeline {
    pipeline: Arc<GraphicsPipeline>,
    descriptor_sets: [Arc<DescriptorSet>; 2],
}

impl RenderFacesPipeline {
    pub fn new(
        app: &App,
        queue: Arc<Queue>,
        rendering_info: PipelineRenderingCreateInfo,
    ) -> RenderFacesPipeline {
        assert!(mesh::PushConstants::LAYOUT == frag::PushConstants::LAYOUT);

        let pipeline = {
            let device = queue.device().clone();
            let mesh = mesh::load(device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let frag = frag::load(device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();

            let stages = [
                PipelineShaderStageCreateInfo::new(mesh),
                PipelineShaderStageCreateInfo::new(frag),
            ];

            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())
                    .unwrap(),
            )
            .unwrap();

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState {
                        cull_mode: CullMode::Back,
                        ..Default::default()
                    }),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        rendering_info.color_attachment_formats.len() as u32,
                        ColorBlendAttachmentState::default(),
                    )),
                    depth_stencil_state: Some(DepthStencilState {
                        depth: Some(DepthState {
                            compare_op: CompareOp::Less,
                            write_enable: true,
                        }),
                        ..Default::default()
                    }),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(rendering_info.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .unwrap()
        };

        let cube_faces: Vec<_> = Direction::ALL
            .iter()
            .map(|&direction| mesh::CubeFace {
                position: [0, 0, 0],
                direction: direction as u32,
            })
            .collect();

        let cube_faces_buffer: Subbuffer<mesh::CubeFaces> = Buffer::new_unsized(
            app.context.memory_allocator().clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            cube_faces.len() as u64,
        )
        .unwrap();
        {
            let mut guard = cube_faces_buffer.write().unwrap();
            for (i, face) in cube_faces.iter().enumerate() {
                guard.cube_faces[i] = *face;
            }
        }

        let descriptor_sets = {
            let mut command_buffer = RecordingCommandBuffer::new(
                app.command_buffer_allocator.clone(),
                queue.queue_family_index(),
                CommandBufferLevel::Primary,
                CommandBufferBeginInfo {
                    usage: CommandBufferUsage::OneTimeSubmit,
                    ..Default::default()
                },
            )
            .unwrap();

            let set_layouts = pipeline.layout().set_layouts();

            let mesh_descriptor_set = DescriptorSet::new(
                app.descriptor_set_allocator.clone(),
                set_layouts[0].clone(),
                [WriteDescriptorSet::buffer(0, cube_faces_buffer.clone())],
                None,
            )
            .unwrap();

            let stone_png_bytes = include_bytes!("stone.png");
            let stone_png_image_view = upload_png(
                stone_png_bytes,
                app.context.memory_allocator().clone(),
                &mut command_buffer,
            );

            let sampler = Sampler::new(
                queue.device().clone(),
                SamplerCreateInfo {
                    mag_filter: Filter::Nearest,
                    min_filter: Filter::Nearest,
                    address_mode: [SamplerAddressMode::Repeat; 3],
                    ..Default::default()
                },
            )
            .unwrap();

            let frag_descriptor_set = DescriptorSet::new(
                app.descriptor_set_allocator.clone(),
                set_layouts[1].clone(),
                [WriteDescriptorSet::image_view_sampler_array(
                    0,
                    0,
                    [(stone_png_image_view, sampler)],
                )],
                None,
            )
            .unwrap();

            command_buffer
                .end()
                .unwrap()
                .execute(queue.clone())
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();
            [mesh_descriptor_set, frag_descriptor_set]
        };
        Self {
            pipeline,
            descriptor_sets,
        }
    }

    pub fn render_cube_faces(&self, builder: &mut RecordingCommandBuffer, camera: &Camera) {
        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                self.pipeline.bind_point(),
                self.pipeline.layout().clone(),
                0,
                self.descriptor_sets.to_vec(),
            )
            .unwrap()
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                mesh::PushConstants {
                    view: camera.view.into(),
                    proj: camera.proj.into(),
                    camera_pos: camera.position.into(),
                },
            )
            .unwrap();
        unsafe { builder.draw_mesh_tasks([6, 1, 1]).unwrap() };
    }
}
