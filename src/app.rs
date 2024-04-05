use std::sync::{atomic::AtomicBool, Arc};

use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{DeviceExtensions, DeviceFeatures},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
            DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo,
        },
        InstanceCreateInfo, InstanceExtensions,
    },
    memory::allocator::StandardMemoryAllocator,
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::VulkanoWindows,
};

pub struct App {
    pub context: VulkanoContext,
    pub windows: VulkanoWindows,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    _debug_callback: DebugUtilsMessenger,

    pub validation_error_encountered: Arc<AtomicBool>,
}

impl App {
    pub fn new() -> Self {
        let mut config = VulkanoConfig {
            device_extensions: DeviceExtensions {
                khr_swapchain: true,
                ext_mesh_shader: true,
                // khr_acceleration_structure: true,
                // khr_ray_tracing_pipeline: true,
                // khr_deferred_host_operations: true,
                ..DeviceExtensions::empty()
            },
            device_features: DeviceFeatures {
                dynamic_rendering: true,
                fill_mode_non_solid: true,
                mesh_shader: true,
                task_shader: true,
                maintenance4: true,
                fragment_stores_and_atomics: true,
                shader_int16: true,
                shader_float16: true,
                ..DeviceFeatures::empty()
            },
            instance_create_info: InstanceCreateInfo {
                enabled_layers: vec!["VK_LAYER_KHRONOS_validation".to_owned()],
                enabled_extensions: InstanceExtensions {
                    ext_debug_utils: true,
                    ..InstanceExtensions::empty()
                },
                ..Default::default()
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
        let validation_error_encountered = Arc::new(AtomicBool::new(false));

        let debug_callback = unsafe {
            let validation_error_encountered = validation_error_encountered.clone();
            DebugUtilsMessenger::new(
                context.instance().clone(),
                DebugUtilsMessengerCreateInfo {
                    message_severity: DebugUtilsMessageSeverity::ERROR
                        | DebugUtilsMessageSeverity::WARNING
                        | DebugUtilsMessageSeverity::INFO
                        | DebugUtilsMessageSeverity::VERBOSE,
                    message_type: DebugUtilsMessageType::GENERAL
                        | DebugUtilsMessageType::VALIDATION
                        | DebugUtilsMessageType::PERFORMANCE,
                    ..DebugUtilsMessengerCreateInfo::user_callback(
                        DebugUtilsMessengerCallback::new(
                            move |message_severity, message_type, callback_data| {
                                use log::{debug, error, info, warn};

                                // Determine the message type
                                let ty = if message_type.intersects(DebugUtilsMessageType::GENERAL)
                                {
                                    "general"
                                } else if message_type.intersects(DebugUtilsMessageType::VALIDATION)
                                {
                                    "validation"
                                } else if message_type
                                    .intersects(DebugUtilsMessageType::PERFORMANCE)
                                {
                                    "performance"
                                } else {
                                    ""
                                };

                                let message_id_name =
                                    callback_data.message_id_name.unwrap_or("unknown");

                                if message_severity.intersects(DebugUtilsMessageSeverity::ERROR) {
                                    error!(
                                        "{}: {}: {}",
                                        message_id_name, ty, callback_data.message
                                    );
                                    validation_error_encountered
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                } else if message_severity
                                    .intersects(DebugUtilsMessageSeverity::WARNING)
                                {
                                    warn!("{} {}: {}", message_id_name, ty, callback_data.message);
                                } else if message_severity
                                    .intersects(DebugUtilsMessageSeverity::VERBOSE)
                                {
                                    debug!("{} {}: {}", message_id_name, ty, callback_data.message);
                                } else {
                                    info!("{} {}: {}", message_id_name, ty, callback_data.message);
                                }
                            },
                        ),
                    )
                },
            )
            .unwrap()
        };

        Self {
            context,
            windows,
            command_buffer_allocator,
            descriptor_set_allocator,
            _debug_callback: debug_callback,
            validation_error_encountered,
        }
    }

    pub fn memory_allocator(&self) -> Arc<StandardMemoryAllocator> {
        self.context.memory_allocator().clone()
    }
}
