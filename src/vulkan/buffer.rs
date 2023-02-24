use super::{VulkanApp, context::VkContext, math, vertex::Vertex, camera::Camera};

use ash::{vk::{self, Extent2D, DeviceMemory}, Device};
use cgmath::{Deg, Matrix4, Point3, Vector3, vec3};
use std::{mem::{align_of, size_of}};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct UniformBufferObject {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
}


impl UniformBufferObject {
    pub fn get_descriptor_set_layout_binding() -> vk::DescriptorSetLayoutBinding {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            // .immutable_samplers() null since we're not creating a sampler descriptor
            .build()
    }
}

impl VulkanApp{

    pub fn create_vertex_buffer(
        vk_context: &VkContext,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        vertices: &[Vertex],
    ) -> (vk::Buffer, vk::DeviceMemory) {
        Self::create_device_local_buffer_with_data::<u32, _>(
            vk_context,
            command_pool,
            transfer_queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vertices,
        )
    }

    pub fn create_index_buffer(
        vk_context: &VkContext,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        indices: &[u32],
    ) -> (vk::Buffer, vk::DeviceMemory) {
        Self::create_device_local_buffer_with_data::<u16, _>(
            vk_context,
            command_pool,
            transfer_queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            indices,
        )
    }

    /// Create a buffer and it's gpu  memory and fill it.
    ///
    /// This function internally creates an host visible staging buffer and
    /// a device local buffer. The data is first copied from the cpu to the
    /// staging buffer. Then we copy the data from the staging buffer to the
    /// final buffer using a one-time command buffer.
    pub fn create_device_local_buffer_with_data<A, T: Copy>(
        vk_context: &VkContext,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        usage: vk::BufferUsageFlags,
        data: &[T],
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let device = vk_context.device();
        let size = (data.len() * size_of::<T>()) as vk::DeviceSize;
        let (staging_buffer, staging_memory, staging_mem_size) = Self::create_buffer(
            vk_context,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let data_ptr = device
                .map_memory(staging_memory, 0, size, vk::MemoryMapFlags::empty())
                .unwrap();
            let mut align = ash::util::Align::new(data_ptr, align_of::<A>() as _, staging_mem_size);
            align.copy_from_slice(data);
            device.unmap_memory(staging_memory);
        };

        let (buffer, memory, _) = Self::create_buffer(
            vk_context,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        Self::copy_buffer(
            device,
            command_pool,
            transfer_queue,
            staging_buffer,
            buffer,
            size,
        );

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        };

        (buffer, memory)
    }

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

    /// Copy the `size` first bytes of `src` into `dst`.
    ///
    /// It's done using a command buffer allocated from
    /// `command_pool`. The command buffer is cubmitted tp
    /// `transfer_queue`.
    pub fn copy_buffer(
        device: &Device,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        src: vk::Buffer,
        dst: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        Self::execute_one_time_commands(&device, &command_pool, &transfer_queue, |buffer| {
            let region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            };
            let regions = [region];

            unsafe { device.cmd_copy_buffer(buffer, src, dst, &regions) };
        });
    }

    pub fn update_uniform_buffers(
        current_image: u32, 
        extent: &Extent2D, 
        camera: &mut Camera, 
        uniform_buffer_memories: &Vec<vk::DeviceMemory>,
        device: &Device)
    {
        let aspect = extent.width as f32 / extent.height as f32;
        let ubo = UniformBufferObject {
            model: Matrix4::from_angle_x(Deg(0.0)),
            view: camera.matrix(),
            proj: math::perspective(Deg(45.0), aspect, 0.1, 10000.0),
        };

        let ubos = [ubo];

        let buffer_mem = uniform_buffer_memories[current_image as usize];
        let size = size_of::<UniformBufferObject>() as vk::DeviceSize;
        unsafe {
            let data_ptr = device
                .map_memory(buffer_mem, 0, size, vk::MemoryMapFlags::empty())
                .unwrap();
            let mut align = ash::util::Align::new(data_ptr, align_of::<f32>() as _, size);
            align.copy_from_slice(&ubos);
            device.unmap_memory(buffer_mem);
        }
    }
}