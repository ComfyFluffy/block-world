mod culling;
pub mod render_faces;

use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBuffer, CommandBufferBeginInfo,
        CommandBufferLevel, CommandBufferUsage, RecordingCommandBuffer, RenderingAttachmentInfo,
        RenderingInfo,
    },
    device::Queue,
    format::ClearValue,
    image::view::ImageView,
    pipeline::graphics::viewport::Viewport,
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
};

pub fn draw(
    mut builder: &mut RecordingCommandBuffer,
    queue: Arc<Queue>,
    dst_image: Arc<ImageView>,
    motion_vector_image: Arc<ImageView>,
    depth_image: Arc<ImageView>,
    viewport: Viewport,
    record_fn: impl FnOnce(&mut RecordingCommandBuffer),
) {
    builder
        .begin_rendering(RenderingInfo {
            color_attachments: vec![
                Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    store_op: AttachmentStoreOp::Store,
                    clear_value: Some([0.5, 0.0, 0.0, 1.0].into()),
                    // resolve_info: Some(RenderingAttachmentResolveInfo::image_view(dst_image)),
                    ..RenderingAttachmentInfo::image_view(dst_image)
                }),
                Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    store_op: AttachmentStoreOp::Store,
                    clear_value: Some([0.0, 0.0, 0.0, 0.0].into()),
                    ..RenderingAttachmentInfo::image_view(motion_vector_image)
                }),
            ],
            depth_attachment: Some(RenderingAttachmentInfo {
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::DontCare,
                clear_value: Some(ClearValue::Depth(1.0)),
                ..RenderingAttachmentInfo::image_view(depth_image)
            }),

            ..Default::default()
        })
        .unwrap()
        .set_viewport(0, [viewport].into_iter().collect())
        .unwrap();

    record_fn(&mut builder);

    builder.end_rendering().unwrap();
}
