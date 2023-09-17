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

const LOADED_PAGES: usize = 10000;

pub struct OcttreeController {
    pub octtree: Octtree,
    pub octtree_buffer: Buffer,
}

impl OcttreeController {
    pub fn new(context: &Context, octtree: Octtree) -> Result<Self> {
        log::info!("Creating Tree Buffer");

        let buffer_size = (size_of::<Node>() * PAGE_SIZE * LOADED_PAGES);
        log::info!(
            "Buffer Size: {} byte {} MB {} GB",
            buffer_size,
            buffer_size as f32 / 1000000.0,
            buffer_size as f32 / 1000000000.0
        );

        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            buffer_size as _,
        )?;

        Ok(OcttreeController {
            octtree,
            octtree_buffer,
        })
    }

    pub fn init_copy(&self) -> Result<()> {
        log::info!("Pushing Tree");

        for (i, page) in self.octtree.pages.iter().enumerate() {
            self.octtree_buffer.copy_data_to_buffer_complex(
                page,
                i * PAGE_SIZE,
                align_of::<Node>(),
            )?;
        }

        log::info!("Pushed {} pages.", self.octtree.pages.len());

        Ok(())
    }
}
