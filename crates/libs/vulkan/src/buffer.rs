use std::{
    mem::{align_of, size_of_val, size_of},
    sync::{Arc, Mutex}, slice::from_raw_parts_mut,
};

use anyhow::Result;
use ash::vk;
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, Allocator},
    MemoryLocation,
};

use crate::{device::Device, Context, align::Align};

pub struct Buffer {
    device: Arc<Device>,
    allocator: Arc<Mutex<Allocator>>,
    pub(crate) inner: vk::Buffer,
    allocation: Option<Allocation>,
    pub size: vk::DeviceSize,
}

impl Buffer {
    pub(crate) fn new(
        device: Arc<Device>,
        allocator: Arc<Mutex<Allocator>>,
        usage: vk::BufferUsageFlags,
        memory_location: MemoryLocation,
        size: vk::DeviceSize,
    ) -> Result<Self> {
        let create_info = vk::BufferCreateInfo::builder().size(size).usage(usage);
        let inner = unsafe { device.inner.create_buffer(&create_info, None)? };
        let requirements = unsafe { device.inner.get_buffer_memory_requirements(inner) };
        let allocation = allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: "buffer",
            requirements,
            location: memory_location,
            linear: true,
        })?;

        unsafe {
            device
                .inner
                .bind_buffer_memory(inner, allocation.memory(), allocation.offset())?
        };

        Ok(Self {
            device,
            allocator,
            inner,
            allocation: Some(allocation),
            size,
        })
    }

    pub fn copy_data_to_buffer<T: Copy>(&self, data: &[T], offset: usize, alignment: usize) -> Result<()> {
        unsafe {
            let data_ptr = self
                .allocation
                .as_ref()
                .unwrap()
                .mapped_ptr()
                .unwrap()
                .as_ptr();

            let mut align: Align<T> = Align::new(data_ptr, alignment as _, data.len(), offset);
            align.copy_from_slice(data);
        };

        Ok(())
    }

    pub fn get_data_from_buffer<T: Copy>(&self, count: usize, offset: usize, alignment: usize) -> Result<Vec<T>> {
        
        let data;
        unsafe {
            let data_ptr = self
                .allocation
                .as_ref()
                .unwrap()
                .mapped_ptr()
                .unwrap()
                .as_ptr();

            let mut align: Align<T> = Align::new(data_ptr, alignment as _, count, offset);
            data = align.copy_to_slice(count)
        };

        Ok(data)
    }

    pub fn get_device_address(&self) -> u64 {
        let addr_info = vk::BufferDeviceAddressInfo::builder().buffer(self.inner);
        unsafe { self.device.inner.get_buffer_device_address(&addr_info) }
    }
}

impl Context {
    pub fn create_buffer(
        &self,
        usage: vk::BufferUsageFlags,
        memory_location: MemoryLocation,
        size: vk::DeviceSize,
    ) -> Result<Buffer> {
        Buffer::new(
            self.device.clone(),
            self.allocator.clone(),
            usage,
            memory_location,
            size,
        )
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { self.device.inner.destroy_buffer(self.inner, None) };
        self.allocator
            .lock()
            .unwrap()
            .free(self.allocation.take().unwrap())
            .unwrap();
    }
}
