use std::sync::Arc;

use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::Queue,
    image::Image,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
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
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
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

fn image_to_view(path: &str) -> Arc<Image> {
    let image = image::open(path).unwrap().to_rgba8();
    let image_data = image.into_raw();
    let image_dimensions = image.dimensions();

    let (image, future) = Image::new().unwrap();

    future.flush().unwrap();

    image
}

struct RenderFacesPipeline {
    pipeline: Arc<GraphicsPipeline>,
}

impl RenderFacesPipeline {
    pub fn new(
        app: &App,
        queue: Arc<Queue>,
        rendering_info: PipelineRenderingCreateInfo,
    ) -> RenderFacesPipeline {
        let (pipeline, layout) = {
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

            (
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
                        ..GraphicsPipelineCreateInfo::layout(layout.clone())
                    },
                )
                .unwrap(),
                layout,
            )
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
            let set_layouts = layout.set_layouts();

            let mesh_descriptor_set = DescriptorSet::new(
                app.descriptor_set_allocator.clone(),
                set_layouts[0].clone(),
                [WriteDescriptorSet::buffer(0, cube_faces_buffer.clone())],
                None,
            );

            let frag_descriptor_set = DescriptorSet::new(
                app.descriptor_set_allocator.clone(),
                set_layouts[1].clone(),
                [WriteDescriptorSet::image_view_sampler_array(0, 0, elements)],
                None,
            );
        };
        Self { pipeline }
    }
}
