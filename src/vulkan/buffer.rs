use super::{VulkanApp, context::VkContext, math, swapchain::SwapchainProperties, camera::Camera};

use ash::{vk::{self, DeviceMemory}, Device};
use cgmath::{Deg, Matrix4, Point3, Vector3};
use std::mem::{align_of, size_of};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct UniformBufferObject {
    pos: Vector3<f32>,
    dir: Vector3<f32>,
}

impl UniformBufferObject {
    pub fn get_descriptor_set_layout_binding() -> vk::DescriptorSetLayoutBinding {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            // .immutable_samplers() null since we're not creating a sampler descriptor
            .build()
    }
}

impl VulkanApp{

    pub fn create_uniform_buffers(
        vk_context: &VkContext,
        count: usize,
    ) -> (Vec<vk::Buffer>, Vec<vk::DeviceMemory>) {
        let size = size_of::<UniformBufferObject>() as vk::DeviceSize;
        let mut buffers = Vec::new();
        let mut memories = Vec::new();

        for _ in 0..count {
            let (buffer, memory, _) = Self::create_buffer(
                vk_context,
                size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            buffers.push(buffer);
            memories.push(memory);
        }

        (buffers, memories)
    }

    /// Create a buffer and allocate its memory.
    ///
    /// # Returns
    ///
    /// The buffer, its memory and the actual size in bytes of the
    /// allocated memory since in may differ from the requested size.
    pub fn create_buffer(
        vk_context: &VkContext,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        mem_properties: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory, vk::DeviceSize) {
        let device = vk_context.device();
        let buffer = {
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();
            unsafe { device.create_buffer(&buffer_info, None).unwrap() }
        };

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory = {
            let mem_type = Self::find_memory_type(
                mem_requirements,
                vk_context.get_mem_properties(),
                mem_properties,
            );

            let alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(mem_type)
                .build();
            unsafe { device.allocate_memory(&alloc_info, None).unwrap() }
        };

        unsafe { device.bind_buffer_memory(buffer, memory, 0).unwrap() };

        (buffer, memory, mem_requirements.size)
    }

    pub fn update_uniform_buffers(
        vk_context: &VkContext,
        properties: &SwapchainProperties,
        camera: &mut Camera,
        uniform_buffer_memories: &Vec<DeviceMemory>,
        current_image: u32
    ) {
        /*
        if self.is_left_clicked && self.cursor_delta.is_some() {
            let delta = self.cursor_delta.take().unwrap();
            let x_ratio = delta[0] as f32 / self.size_dependent.properties.extent.width as f32;
            let y_ratio = delta[1] as f32 / self.size_dependent.properties.extent.height as f32;
            let theta = x_ratio * 180.0_f32.to_radians();
            let phi = y_ratio * 90.0_f32.to_radians();
            self.setup.camera.rotate(theta, phi);
        }
        if let Some(wheel_delta) = self.wheel_delta {
            self.setup.camera.forward(wheel_delta * 0.3);
        }

        */

        camera.forward(-0.01);
        let ubo = UniformBufferObject {
            pos: camera.pos,
            dir: camera.dir,
        };
        let ubos = [ubo];

        let buffer_mem = uniform_buffer_memories[current_image as usize];
        let size = size_of::<UniformBufferObject>() as vk::DeviceSize;
        unsafe {
            let device = vk_context.device();
            let data_ptr = device
                .map_memory(buffer_mem, 0, size, vk::MemoryMapFlags::empty())
                .unwrap();
            let mut align = ash::util::Align::new(data_ptr, align_of::<f32>() as _, size);
            align.copy_from_slice(&ubos);
            device.unmap_memory(buffer_mem);
        }
    }
}

