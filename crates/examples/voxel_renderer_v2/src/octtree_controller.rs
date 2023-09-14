use std::mem::align_of;
use std::mem::size_of;

use app::anyhow::Result;
use app::log;
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::Buffer;
use app::vulkan::Context;

use crate::node::Node;
use crate::octtree::Octtree;
use crate::octtree::PAGE_SIZE;

const LOADED_PAGES: usize = 100;

pub struct OcttreeController {
    pub octtree: Octtree,
    pub octtree_buffer: Buffer,
}

impl OcttreeController {
    pub fn new(context: &Context, octtree: Octtree) -> Result<Self> {
        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Node>() * PAGE_SIZE * LOADED_PAGES) as _,
        )?;

        Ok(OcttreeController {
            octtree,
            octtree_buffer,
        })
    }

    pub fn init_copy(&self) -> Result<()> {
        for (i, page) in self.octtree.pages.iter().enumerate() {
            self.octtree_buffer.copy_data_to_buffer_complex(
                page,
                i * PAGE_SIZE,
                align_of::<Node>(),
            )?;
        }

        Ok(())
    }
}
