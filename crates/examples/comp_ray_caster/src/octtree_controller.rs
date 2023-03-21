use std::mem::size_of;

use app::vulkan::Buffer;
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::{log, vulkan::Context};
use app::anyhow::Result;
use crate::octtree::{Octtree, OcttreeNode};


pub struct OcttreeController{
    pub octtree: Octtree,
    pub octtree_info: OcttreeInfo,

    pub buffer_size: usize, 
    pub transfer_size: usize,

    pub octtree_buffer: Buffer,
    pub octtree_info_buffer: Buffer,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct OcttreeInfo {
    tree_size: u32,
    buffer_size: u32,
    transfer_buffer_size: u32,
    depth: u32,

    build_offset: u32,
    re_build: u32,
    loader_size: u32,
    debug: u32,
}

impl OcttreeController{
    pub fn new(
        context: &Context, 
        octtree: Octtree, 
        buffer_size: usize, 
        transfer_size: usize,
        loader_size: usize,
    ) -> Result<Self> {

        let depth = octtree.depth;
        let size = octtree.max_size;

        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuOnly, 
            (size_of::<u32>() * 4 * 4 * buffer_size) as _,
        )?;

        let octtree_info_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<OcttreeInfo>() as _,
        )?;


        let debug = cfg!(debug_assertions) as u32;

        Ok(OcttreeController { 
            octtree, 
            octtree_info: OcttreeInfo { 
                tree_size:              size as u32, 
                buffer_size:            buffer_size as u32, 
                transfer_buffer_size:   transfer_size as u32, 
                depth:                  depth as u32, 
                build_offset:           0, 
                re_build:               1, 
                loader_size:            loader_size as u32, 
                debug:                  debug,
            },

            buffer_size:        buffer_size, 
            transfer_size:      transfer_size,

            octtree_buffer,
            octtree_info_buffer
        })
    }

    pub fn step(& mut self){
        self.octtree_info.re_build = 0;
        //self.octtree_info.build_offset = (self.octtree_info.build_offset + self.build_size as u32) % self.buffer_size as u32;
    }

    pub fn get_requested_nodes(&mut self, requested_ids: &Vec<u32>) -> (Vec<OcttreeNode>, usize) {

        let mut nodes = vec![OcttreeNode::default(); self.transfer_size];

        let mut counter = 0;
        for (i, id) in requested_ids.iter().enumerate() {

            if *id >= self.octtree.max_size as u32 {
                log::error!("Requested Child ID: {:?}", id);
            }

            if *id <= 0 || *id >= self.octtree.max_size as u32 {
                continue;
            }
            counter += 1;

            let seek = *id as u64;
            let r = self.octtree.nodes.binary_search_by(|node| node.get_node_id().cmp(&seek));
            match r {
                Ok(_) => nodes[i] = self.octtree.nodes[r.unwrap()],
                Err(_) => {
                    log::error!("Requested Node {:?} not found!", id);
                    continue
                },
            };
        }

        (nodes, counter) 
    }
}
