use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cgmath::Deg;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{CopyBufferToImageInfo, RecordingCommandBuffer},
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::Format,
    image::{
        view::{ImageView, ImageViewCreateInfo, ImageViewType},
        Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    padded::Padded,
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{CompareOp, DepthState, DepthStencilState},
            multisample::MultisampleState,
            rasterization::{CullMode, RasterizationState},
            subpass::PipelineRenderingCreateInfo,
            viewport::{Scissor, Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
};

use crate::{app::App, types::ChunkPosition};

mod task {
    vulkano_shaders::shader!(
        ty: "task",
        path: "src/renderer/render_faces/render_faces.task.glsl",
        vulkan_version: "1.3"
    );
}

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
    );
}

// Fix-sized array of CHUNK_SIZE^3 blocks, stored sparsely.
pub use task::Block as GpuBlock;
pub use task::Chunk as GpuChunk;

struct GpuChunkStorage {
    chunk_buffer: Subbuffer<task::ChunkBuffer>,
    index_buffer: Subbuffer<task::IndexBuffer>,

    chunk_blocks_map: HashMap<ChunkPosition, (u32, HashSet<u32>)>, // chunk index, block indices
    chunk_holes: Vec<u32>,
}

struct ChunkUpdate {
    block_index: u32,
    block: Option<GpuBlock>,
}

impl GpuChunkStorage {
    pub fn new(allocator: Arc<StandardMemoryAllocator>, chunks: u64) -> Self {
        let chunk_buffer = Buffer::new_unsized(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            chunks,
        )
        .unwrap();

        let index_buffer = Buffer::new_unsized(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            chunks * 16 * 16 * 16, // a chunk is 16x16x16 blocks
        )
        .unwrap();

        Self {
            chunk_buffer,
            index_buffer,
            chunk_blocks_map: HashMap::new(),
            chunk_holes: (0..chunks as u32).rev().collect(),
        }
    }

    pub fn update(
        &mut self,
        chunk_position: ChunkPosition,
        updates: impl IntoIterator<Item = ChunkUpdate>,
    ) {
        let (chunk_index, block_indices) = self
            .chunk_blocks_map
            .entry(chunk_position)
            .or_insert_with(|| {
                let chunk_index = self.chunk_holes.pop().unwrap();
                (chunk_index, HashSet::new())
            });

        let mut chunk = self.chunk_buffer.write().unwrap();
        for update in updates {
            if let Some(block) = update.block {
                chunk.chunks[*chunk_index as usize].blocks[update.block_index as usize] = block;
                block_indices.insert(update.block_index);
            } else {
                block_indices.remove(&update.block_index);
            }
        }
    }

    pub fn upload_indices(&self) -> usize {
        let mut index_write = self.index_buffer.write().unwrap();
        let mut i = 0;
        for (_, (chunk_index, block_indices)) in self.chunk_blocks_map.iter() {
            for block_index in block_indices.iter() {
                index_write.indices[i] = [*chunk_index, *block_index];
                i += 1;
            }
        }
        i
    }

    // pub fn upload_indices_with_culling(&self, frustum: Frustum) {}
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub view: cgmath::Matrix4<f32>,
    pub proj: cgmath::Matrix4<f32>,
    pub position: cgmath::Point3<f32>,
    pub near: f32,
    pub far: f32,
    pub fovy: Deg<f32>,
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

    let view_create_info = ImageViewCreateInfo {
        view_type: ImageViewType::Dim2dArray,
        ..ImageViewCreateInfo::from_image(&image)
    };
    ImageView::new(image, view_create_info).unwrap()
}

pub struct RenderFacesPipeline {
    pipeline: Arc<GraphicsPipeline>,
    descriptor_sets: Vec<Arc<DescriptorSet>>,

    gpu_chunk_storage: GpuChunkStorage,
}

impl RenderFacesPipeline {
    pub fn new(
        app: &App,
        queue: Arc<Queue>,
        rendering_info: PipelineRenderingCreateInfo,
    ) -> RenderFacesPipeline {
        let pipeline = {
            let device = queue.device().clone();
            let task = task::load(device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let mesh = mesh::load(device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let frag = frag::load(device.clone())
                .unwrap()
                .entry_point("main")
                .unwrap();

            let stages = [
                PipelineShaderStageCreateInfo::new(task),
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
                        // cull_mode: CullMode::None,
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

        let mut gpu_chunk_storage = GpuChunkStorage::new(app.context.memory_allocator().clone(), 1);
        let chunk_updates = (0..16 * 16 * 16).map(|i| ChunkUpdate {
            block_index: i,
            block: Some(GpuBlock {
                voxel_offset: 0,
                voxel_len: 2,
                connected_bits: 0,
            }),
        });
        gpu_chunk_storage.update(ChunkPosition { x: 0, z: 0 }, chunk_updates);
        gpu_chunk_storage.upload_indices();

        let descriptor_sets = {
            // let mut command_buffer = RecordingCommandBuffer::new(
            //     app.command_buffer_allocator.clone(),
            //     queue.queue_family_index(),
            //     CommandBufferLevel::Primary,
            //     CommandBufferBeginInfo {
            //         usage: CommandBufferUsage::OneTimeSubmit,
            //         ..Default::default()
            //     },
            // )
            // .unwrap();

            let set_layouts = pipeline.layout().set_layouts();

            let descriptor_set_0 = DescriptorSet::new(
                app.descriptor_set_allocator.clone(),
                set_layouts[0].clone(),
                [
                    WriteDescriptorSet::buffer(0, gpu_chunk_storage.chunk_buffer.clone()),
                    WriteDescriptorSet::buffer(1, gpu_chunk_storage.index_buffer.clone()),
                ],
                None,
            )
            .unwrap();

            let voxel_buffer = Buffer::new_unsized::<task::VoxelBuffer>(
                app.context.memory_allocator().clone(),
                BufferCreateInfo {
                    usage: BufferUsage::STORAGE_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                2,
            )
            .unwrap();

            {
                let mut voxel_write = voxel_buffer.write().unwrap();
                voxel_write.voxels[0] = task::Voxel {
                    faces: [
                        Padded(task::VoxelFace {
                            cullface: 1,
                            texture_index: 0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        }),
                        Padded(task::VoxelFace {
                            cullface: 1,
                            texture_index: 0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        }),
                        Padded(task::VoxelFace {
                            cullface: 1,
                            texture_index: 0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        }),
                        Padded(task::VoxelFace {
                            cullface: 1,
                            texture_index: 0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        }),
                        Padded(task::VoxelFace {
                            cullface: 1,
                            texture_index: 0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        }),
                        Padded(task::VoxelFace {
                            cullface: 6,
                            texture_index: 0,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        }),
                    ],
                    from: Padded([0.0, 0.0, 0.0]),
                    to: Padded([1.0, 1.0, 1.0]),
                };
                voxel_write.voxels[1] = voxel_write.voxels[0];
                voxel_write.voxels[1].from = Padded([0.5, 0.5, 0.5]);
                voxel_write.voxels[1].to = Padded([1.5, 1.5, 1.5]);
            }

            let descriptor_set_1 = DescriptorSet::new(
                app.descriptor_set_allocator.clone(),
                set_layouts[1].clone(),
                [WriteDescriptorSet::buffer(0, voxel_buffer.clone())],
                None,
            )
            .unwrap();

            vec![descriptor_set_0, descriptor_set_1]
        };
        Self {
            pipeline,
            descriptor_sets,
            gpu_chunk_storage,
        }
    }

    pub fn render_cube_faces(
        &self,
        builder: &mut RecordingCommandBuffer,
        previous_camera: &Camera,
        camera: &Camera,
    ) {
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
                    current_view_proj: (camera.proj * camera.view).into(),
                    previous_view_proj: (previous_camera.proj * previous_camera.view).into(),
                    camera_pos: camera.position.into(),
                },
            )
            .unwrap();
        unsafe { builder.draw_mesh_tasks([16u32.pow(3), 1, 1]).unwrap() };
    }
}
