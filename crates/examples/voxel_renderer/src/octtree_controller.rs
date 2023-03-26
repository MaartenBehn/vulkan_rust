use std::mem::size_of;

use app::vulkan::Buffer;
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::{vulkan::Context};
use app::anyhow::Result;
use octtree::Tree;
use octtree::octtree_node::OcttreeNode;

use crate::octtree_loader::REQUEST_STEP;


pub struct OcttreeController{
    pub octtree: Box<dyn Tree>,
    pub octtree_info: OcttreeInfo,

    pub buffer_size: usize, 
    pub transfer_size: usize,

    pub octtree_buffer: Buffer,
    pub octtree_info_buffer: Buffer,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct OcttreeInfo {
    tree_size_0: u32,
    tree_size_1: u32,
    buffer_size: u32,
    transfer_buffer_size: u32,

    depth: u32,
    loader_size: u32,
    fill_0: u32,
    fill_1: u32,
}

impl OcttreeController{
    pub fn new(
        context: &Context, 
        octtree: Box<dyn Tree>, 
        buffer_size: usize, 
        transfer_size: usize,
        loader_size: usize,
    ) -> Result<Self> {

        let depth = octtree.get_depth();
        let size = octtree.get_max_size();

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

        Ok(OcttreeController { 
            octtree, 
            octtree_info: OcttreeInfo { 
                tree_size_0:            size as u32, 
                tree_size_1:            (size >> 32) as u32,
                buffer_size:            buffer_size as u32, 
                transfer_buffer_size:   transfer_size as u32, 

                depth:                  depth as u32, 
                loader_size:            loader_size as u32, 
                fill_0:                 0,
                fill_1:                 0,
            },

            buffer_size:        buffer_size, 
            transfer_size:      transfer_size,

            octtree_buffer,
            octtree_info_buffer
        })
    }

    pub fn step(& mut self){
        
        
    }

    pub fn get_requested_nodes(&mut self, requested_data: &Vec<u32>) -> Result<(Vec<OcttreeNode>, usize)> {

        let mut nodes = vec![OcttreeNode::default(); self.transfer_size];

        let mut counter = 0;
        for i in 0..self.transfer_size {
            let id = (requested_data[i * REQUEST_STEP] as u64) + ((requested_data[i * REQUEST_STEP + 1] as u64) << 32);
            let child_nr = requested_data[i * REQUEST_STEP + 2] as usize;
            let depth = requested_data[i * REQUEST_STEP + 3] as u16;

            if id >= self.octtree.get_max_size() {
                break;
            }
            counter += 1;

            let seek = self.octtree.get_child_id(id, child_nr, depth);
            nodes[i] = self.octtree.get_node(seek)?;
        }

        Ok((nodes, counter)) 
    }
}
