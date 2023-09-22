use std::mem::align_of;
use std::mem::size_of;

use app::anyhow::Result;
use app::glam::ivec3;
use app::glam::Vec3;
use app::log;
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::Buffer;
use app::vulkan::Context;
use octtree_v2::aabb::AABB;
use octtree_v2::node::CompressedNode;

pub struct OcttreeController {
    pub loaded_pages: usize,

    pub octtree_lookup: Vec<[u32; 2]>,
    pub octtree: Octtree,

    pub octtree_buffer: Buffer,
    pub octtree_lookup_buffer: Buffer,
}

impl OcttreeController {
    pub fn new(context: &Context, octtree: Octtree, loaded_pages: usize) -> Result<Self> {
        log::info!("Creating Tree Buffer");

        let octtree_buffer_size =
            size_of::<CompressedNode>() * octtree.metadata.page_size * loaded_pages;
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
            octtree_lookup.push([i as u32; 2]);
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
        let mut pushed_pages = 0;
        for (i, page) in self.octtree.pages.iter().enumerate() {
            if i >= self.loaded_pages {
                break;
            }

            self.octtree_buffer.copy_data_to_buffer_complex(
                page.nodes.as_slice(),
                i * self.octtree.metadata.page_size,
                align_of::<CompressedNode>(),
            )?;

            pushed_pages += 1;
        }

        log::info!("Pushed {} pages.", pushed_pages);

        Ok(())
    }

    pub fn sort_lookup(&mut self) {
        self.octtree_lookup.sort_by(|a, b| a[0].cmp(&b[0]));
    }

    pub fn push_lookup(&self) -> Result<()> {
        self.octtree_lookup_buffer.copy_data_to_buffer_complex(
            self.octtree_lookup.as_slice(),
            0,
            align_of::<u32>(),
        )?;

        Ok(())
    }

    fn insert_page(&mut self, lookup_index: usize, nr: usize) -> Result<()> {
        self.octtree_lookup[lookup_index][0] = nr as u32;
        let page_index = self.octtree_lookup[lookup_index][1];

        self.octtree_buffer.copy_data_to_buffer_complex(
            self.octtree.pages[nr].nodes.as_slice(),
            page_index as usize * self.octtree.metadata.page_size,
            align_of::<CompressedNode>(),
        )?;

        Ok(())
    }

    pub fn update(&mut self, pos: Vec3, size: i32) -> Result<()> {
        let player_pos = ivec3(pos.x as i32, pos.y as i32, pos.z as i32);
        let player_size = ivec3(size, size, size);
        let player_aabb = AABB::new(player_pos - player_size, player_pos + player_size);

        let mut collided_pages = Vec::new();
        for (nr, aabb) in self.octtree.metadata.aabbs.iter().enumerate() {
            if player_aabb.collide(aabb) {
                collided_pages.push(nr);
            }
        }

        //log::info!("Loded Pages: {:?}", self.octtree_lookup);
        //log::info!("Colliding Pages: {:?}", collided_pages);

        if collided_pages.len() == 0 {
            return Ok(());
        }

        let mut free_in_lookup_indices = Vec::new();
        let mut free_in_collide = Vec::new();
        let mut i = 0;
        let mut j = 0;

        loop {
            let lookup = self.octtree_lookup[i][0] as usize;
            let collide = collided_pages[j];

            if lookup == collide {
                i += 1;
                j += 1;
            } else if lookup < collide {
                free_in_lookup_indices.push(i);

                i += 1;
            } else if collide < lookup {
                free_in_collide.push(collide);

                j += 1;
            }

            if j >= collided_pages.len() || i >= self.loaded_pages {
                break;
            }
        }

        while i < self.loaded_pages {
            free_in_lookup_indices.push(i);
            i += 1;
        }

        while j < collided_pages.len() {
            let collide = collided_pages[j];
            free_in_collide.push(collide);
            j += 1;
        }

        log::info!("Needed Pages: {:?}", free_in_collide);

        for free_index in free_in_lookup_indices {
            let matched_collide = free_in_collide.pop();
            if matched_collide.is_none() {
                break;
            }

            self.insert_page(free_index, matched_collide.unwrap())?;
        }

        self.sort_lookup();
        self.push_lookup()?;

        Ok(())
    }
}
