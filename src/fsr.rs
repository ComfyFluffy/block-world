use std::{marker::PhantomData, mem};

use cgmath::{Matrix4, Rad};
use fsr_sys::{
    contextCreate, contextDestroy, contextDispatch, getJitterOffset, getJitterPhaseCount,
    vk::{self, getDevice, getTextureResource},
    Context, ContextDescription, Dimensions2D, DispatchDescription, FloatCoords2D, MsgType,
    Resource, ENABLE_AUTO_EXPOSURE, ENABLE_DEBUG_CHECKING, MESSAGE_TYPE_ERROR,
    MESSAGE_TYPE_WARNING, OK, RESOURCE_STATE_COMPUTE_READ, RESOURCE_STATE_UNORDERED_ACCESS,
};
use log::{error, warn};
use vulkano::{
    command_buffer::sys::RawRecordingCommandBuffer, device::Device, format::Format,
    image::view::ImageView, Handle, VulkanObject,
};
use widestring::{widecstr, WideCStr};

use crate::renderer::render_faces::Camera;

pub struct FsrContextVulkan {
    _scrach_buffer: Vec<u8>,
    context: Box<Context>,
    render_size: [u32; 2],
    display_size: [u32; 2],

    jitter_phase_count: i32,
    frame_index: i32,
    jitter_offset: [f32; 2],

    non_send_sync: PhantomData<*const ()>,
}

unsafe extern "C" fn on_fsr_message(msg_type: MsgType, message: *const u16) {
    let message = WideCStr::from_ptr_str(message).display();
    match msg_type {
        MESSAGE_TYPE_ERROR => error!("FSR: {}", message),
        MESSAGE_TYPE_WARNING => warn!("FSR: {}", message),
        _ => {}
    }
}

impl FsrContextVulkan {
    pub unsafe fn new(
        vulkan_device: &Device,
        render_size: [u32; 2],
        display_size: [u32; 2],
    ) -> Self {
        let physical_device = vulkan_device.physical_device();
        let get_device_proc_addr = physical_device.instance().fns().v1_0.get_device_proc_addr;
        let physical_device = physical_device.handle().as_raw();
        let scrach_buffer_size = vk::getScratchMemorySize(physical_device);
        let mut scrach_buffer = vec![0u8; scrach_buffer_size];

        let mut context_description = ContextDescription {
            device: getDevice(vulkan_device.handle().as_raw()),
            maxRenderSize: Dimensions2D {
                width: render_size[0],
                height: render_size[1],
            },
            displaySize: Dimensions2D {
                width: display_size[0],
                height: display_size[1],
            },
            fpMessage: Some(on_fsr_message),
            flags: ENABLE_DEBUG_CHECKING | ENABLE_AUTO_EXPOSURE,
            ..Default::default()
        };

        let err = vk::getInterface(
            &mut context_description.callbacks,
            scrach_buffer.as_mut_ptr() as _,
            scrach_buffer_size,
            physical_device,
            mem::transmute(get_device_proc_addr),
        );
        assert_eq!(err, OK, "Failed to get Vulkan FSR interface");

        let mut context = Box::new(Context::default());
        let err = contextCreate(context.as_mut(), &context_description);
        assert_eq!(err, OK, "Failed to create FSR context with Vulkan");

        let jitter_phase_count = getJitterPhaseCount(render_size[0] as _, display_size[0] as _);

        Self {
            _scrach_buffer: scrach_buffer,
            context,
            render_size,
            display_size,
            jitter_phase_count,
            frame_index: 0,
            non_send_sync: PhantomData,
            jitter_offset: [0.0, 0.0],
        }
    }

    unsafe fn get_texture_resource(
        &mut self,
        image_view: &ImageView,
        name: &'static WideCStr,
    ) -> Resource {
        self.get_texture_resource_with_state(image_view, name, RESOURCE_STATE_COMPUTE_READ)
    }

    unsafe fn get_texture_resource_with_state(
        &mut self,
        image_view: &ImageView,
        name: &'static WideCStr,
        state: u32,
    ) -> Resource {
        let image_extent = image_view.image().extent();
        getTextureResource(
            self.context.as_mut(),
            image_view.image().handle().as_raw(),
            image_view.handle().as_raw(),
            image_extent[0],
            image_extent[1],
            image_view.image().format() as _,
            name.as_ptr(),
            state,
        )
    }

    unsafe fn get_texture_resource_empty(&mut self, name: &'static WideCStr) -> Resource {
        getTextureResource(
            self.context.as_mut(),
            0,
            0,
            1,
            1,
            Format::UNDEFINED as _,
            name.as_ptr(),
            RESOURCE_STATE_COMPUTE_READ,
        )
    }

    pub unsafe fn dispatch(
        &mut self,
        command_buffer: &RawRecordingCommandBuffer,
        color: &ImageView,
        depth: &ImageView,
        motion_vector: &ImageView,
        output: &ImageView,
        frame_time_delta: f32,
        camera: Camera,
    ) {
        // assert that all input images have the same extent
        assert_eq!(
            color.image().extent(),
            [self.render_size[0], self.render_size[1], 1]
        );
        assert_eq!(color.image().extent(), depth.image().extent());
        assert_eq!(color.image().extent(), motion_vector.image().extent());

        assert_eq!(
            output.image().extent(),
            [self.display_size[0], self.display_size[1], 1]
        );

        let input_extent = color.image().extent();
        let dispatch_description = DispatchDescription {
            commandList: vk::getCommandList(command_buffer.handle().as_raw()),
            color: self.get_texture_resource(color, widecstr!("FSR2_InputColor")),
            depth: self.get_texture_resource(depth, widecstr!("FSR2_InputDepth")),
            motionVectors: self
                .get_texture_resource(motion_vector, widecstr!("FSR2_InputMotionVector")),
            exposure: self.get_texture_resource_empty(widecstr!("FSR2_InputExposure")),
            reactive: self.get_texture_resource_empty(widecstr!("FSR2_EmptyInputReactiveMap")),
            transparencyAndComposition: self
                .get_texture_resource_empty(widecstr!("FSR2_EmptyTransparencyAndCompositionMap")),
            output: self.get_texture_resource_with_state(
                output,
                widecstr!("FSR2_OutputColor"),
                RESOURCE_STATE_UNORDERED_ACCESS,
            ),
            jitterOffset: FloatCoords2D {
                x: self.jitter_offset[0],
                y: self.jitter_offset[1],
            },
            motionVectorScale: FloatCoords2D {
                x: input_extent[0] as _,
                y: input_extent[1] as _,
            },
            reset: false,
            enableSharpening: true,
            sharpness: 0.5,
            frameTimeDelta: frame_time_delta,
            preExposure: 1.0,
            renderSize: Dimensions2D {
                width: input_extent[0],
                height: input_extent[1],
            },

            cameraFar: camera.far,
            cameraNear: camera.near,
            cameraFovAngleVertical: Rad::from(camera.fovy).0,
            ..Default::default()
        };
        let err = contextDispatch(self.context.as_mut(), &dispatch_description);
        assert_eq!(err, OK, "Failed to dispatch FSR context");
    }

    pub unsafe fn step_jitter(&mut self) -> Matrix4<f32> {
        let mut jitter_x = 0.0;
        let mut jitter_y = 0.0;
        getJitterOffset(
            &mut jitter_x,
            &mut jitter_y,
            self.frame_index,
            self.jitter_phase_count,
        );
        self.frame_index = (self.frame_index + 1) % self.jitter_phase_count;

        let jitter_x = 2.0 * jitter_x / self.render_size[0] as f32;
        let jitter_y = -2.0 * jitter_y / self.render_size[1] as f32;
        Matrix4::from_translation(cgmath::Vector3::new(jitter_x, jitter_y, 0.0))
    }
}

impl Drop for FsrContextVulkan {
    fn drop(&mut self) {
        unsafe {
            contextDestroy(self.context.as_mut());
        }
    }
}
