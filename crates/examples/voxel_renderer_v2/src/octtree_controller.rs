use std::mem::align_of;
use std::mem::size_of;

use app::anyhow::Result;
use app::log;
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::Buffer;
use app::vulkan::Context;
use octtree_v2::node::Node;

use crate::octtree::Octtree;

pub struct OcttreeController {
    pub loaded_pages: usize,

    pub octtree_lookup: Vec<u32>,
    pub octtree: Octtree,

    pub octtree_buffer: Buffer,
    pub octtree_lookup_buffer: Buffer,
}

impl OcttreeController {
    pub fn new(context: &Context, octtree: Octtree, loaded_pages: usize) -> Result<Self> {
        log::info!("Creating Tree Buffer");

        let octtree_buffer_size = size_of::<Node>() * octtree.metadata.page_size * loaded_pages;
        log::info!(
            "Buffer Size: {} byte {} MB {} GB",
            octtree_buffer_size,
            octtree_buffer_size as f32 / 1000000.0,
            octtree_buffer_size as f32 / 1000000000.0
        );

        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            octtree_buffer_size as _,
        )?;

        let octtree_lookup_buffer_size = size_of::<u32>() * (2 * loaded_pages + 4);
        let octtree_lookup_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            octtree_lookup_buffer_size as _,
        )?;

        let mut octtree_lookup = Vec::new();
        for i in 0..loaded_pages {
            octtree_lookup.push(i as u32);
            octtree_lookup.push(i as u32);
        }

        Ok(OcttreeController {
            loaded_pages,
            octtree_lookup,
            octtree,
            octtree_buffer,
            octtree_lookup_buffer,
        })
    }

    pub fn init_push(&self) -> Result<()> {
        log::info!("Pushing Tree Lookup");
        self.push_lookup()?;

        log::info!("Pushing Tree");
        for (i, page) in self.octtree.pages.iter().enumerate() {
            self.octtree_buffer.copy_data_to_buffer_complex(
                page.nodes.as_slice(),
                i * self.octtree.metadata.page_size,
                align_of::<Node>(),
            )?;
        }

        log::info!("Pushed {} pages.", self.octtree.pages.len());

        Ok(())
    }

    pub fn push_lookup(&self) -> Result<()> {
        self.octtree_lookup_buffer.copy_data_to_buffer_complex(
            self.octtree_lookup.as_slice(),
            0,
            align_of::<u32>(),
        )?;

        Ok(())
    }
}
