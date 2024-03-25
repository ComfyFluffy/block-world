use std::sync::Arc;

use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{DeviceExtensions, Features},
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::VulkanoWindows,
};

pub(crate) struct App {
    pub context: VulkanoContext,
    pub windows: VulkanoWindows,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

impl App {
    pub fn new() -> Self {
        let mut config = VulkanoConfig {
            device_extensions: DeviceExtensions {
                khr_swapchain: true,
                khr_dynamic_rendering: true,
                ext_mesh_shader: true,
                // khr_acceleration_structure: true,
                // khr_ray_tracing_pipeline: true,
                // khr_deferred_host_operations: true,
                ..DeviceExtensions::empty()
            },
            device_features: Features {
                dynamic_rendering: true,
                fill_mode_non_solid: true,
                ..Features::empty()
            },
            ..Default::default()
        };
        config
            .instance_create_info
            .enabled_extensions
            .ext_swapchain_colorspace = true;

        let context = VulkanoContext::new(config);
        let windows = VulkanoWindows::default();

        let device = context.device();

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Self {
            context,
            windows,
            command_buffer_allocator,
            descriptor_set_allocator,
        }
    }
}
