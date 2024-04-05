use std::{marker::PhantomData, mem};

use fsr_sys::{
    contextCreate, contextDestroy, contextDispatch,
    vk::{self, getTextureResource},
    Context, ContextDescription, Dimensions2D, DispatchDescription, FloatCoords2D, Resource, OK,
    RESOURCE_STATE_COMPUTE_READ, RESOURCE_STATE_UNORDERED_ACCESS,
};
use vulkano::{
    command_buffer::CommandBuffer, device::physical::PhysicalDevice, format::Format,
    image::view::ImageView, Handle, VulkanObject,
};
use wchar::{wchar_t, wchz};

pub struct FsrContextVulkan {
    _scrach_buffer: Vec<u8>,
    context: Box<Context>,
    non_send_sync: PhantomData<*const ()>,
}

impl FsrContextVulkan {
    pub unsafe fn new(physical_device: PhysicalDevice) -> Self {
        let get_device_proc_addr = physical_device.instance().fns().v1_0.get_device_proc_addr;
        let physical_device = physical_device.handle().as_raw();
        let scrach_buffer_size = vk::getScratchMemorySize(physical_device);
        let mut scrach_buffer = vec![0u8; scrach_buffer_size];

        let mut context_description = ContextDescription {
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

        Self {
            _scrach_buffer: scrach_buffer,
            context,
            non_send_sync: PhantomData,
        }
    }

    unsafe fn get_texture_resource(
        &mut self,
        image_view: &ImageView,
        name: &'static [wchar_t],
    ) -> Resource {
        self.get_texture_resource_with_state(image_view, name, RESOURCE_STATE_COMPUTE_READ)
    }

    unsafe fn get_texture_resource_with_state(
        &mut self,
        image_view: &ImageView,
        name: &'static [wchar_t],
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

    unsafe fn get_texture_resource_empty(&mut self, name: &'static [wchar_t]) -> Resource {
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
        command_buffer: &CommandBuffer,
        color: &ImageView,
        depth: &ImageView,
        motion_vector: &ImageView,
        output: &ImageView,
    ) {
        // assert that all input images have the same extent
        assert_eq!(color.image().extent(), depth.image().extent());
        assert_eq!(color.image().extent(), motion_vector.image().extent());

        let input_extent = color.image().extent();
        let dispatch_description = DispatchDescription {
            commandList: vk::getCommandList(command_buffer.handle().as_raw()),
            color: self.get_texture_resource(color, wchz!("FSR2_InputColor")),
            depth: self.get_texture_resource(depth, wchz!("FSR2_InputDepth")),
            motionVectors: self
                .get_texture_resource(motion_vector, wchz!("FSR2_InputMotionVector")),
            exposure: self.get_texture_resource_empty(wchz!("FSR2_InputExposure")),
            reactive: self.get_texture_resource_empty(wchz!("FSR2_EmptyInputReactiveMap")),
            transparencyAndComposition: self
                .get_texture_resource_empty(wchz!("FSR2_EmptyTransparencyAndCompositionMap")),
            output: self.get_texture_resource_with_state(
                output,
                wchz!("FSR2_OutputColor"),
                RESOURCE_STATE_UNORDERED_ACCESS,
            ),
            jitterOffset: FloatCoords2D { x: 0.0, y: 0.0 }, // TODO
            motionVectorScale: FloatCoords2D {
                x: input_extent[0] as _,
                y: input_extent[1] as _,
            },
            reset: false,
            enableSharpening: true,
            sharpness: 0.5,
            frameTimeDelta: 0.0, // TODO
            preExposure: 1.0,
            renderSize: Dimensions2D {
                width: input_extent[0],
                height: input_extent[1],
            },
            cameraFar: 1000.0,            // TODO
            cameraNear: 0.1,              // TODO
            cameraFovAngleVertical: 45.0, // TODO
            ..Default::default()
        };
        let err = contextDispatch(self.context.as_mut(), &dispatch_description);
        assert_eq!(err, OK, "Failed to dispatch FSR context");
    }
}

impl Drop for FsrContextVulkan {
    fn drop(&mut self) {
        unsafe {
            contextDestroy(self.context.as_mut());
        }
    }
}
