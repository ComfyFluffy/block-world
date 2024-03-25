use std::sync::Arc;

use vulkano::{
    device::Queue,
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{CompareOp, DepthState, DepthStencilState},
            multisample::MultisampleState,
            rasterization::{CullMode, PolygonMode, RasterizationState},
            subpass::PipelineRenderingCreateInfo,
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
};

use crate::window::App;

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

struct RenderFacesPipeline {
    pipeline: Arc<GraphicsPipeline>,
}

impl RenderFacesPipeline {
    pub fn new(
        app: &App,
        queue: Arc<Queue>,
        rendering_info: PipelineRenderingCreateInfo,
    ) -> RenderFacesPipeline {
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
                        polygon_mode: PolygonMode::Line,
                        line_width: 1.0,
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

        Self { pipeline }
    }
}
