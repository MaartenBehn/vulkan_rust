use std::sync::Arc;

use anyhow::Result;
use ash::{extensions::khr::Swapchain as AshSwapchain, vk::{self, ImageUsageFlags}};

use crate::{device::Device, Context, Image, ImageView, Queue, Semaphore};

pub struct AcquiredImage {
    pub index: u32,
    pub is_suboptimal: bool,
}

pub struct Depth {
    pub format: vk::Format,
    pub image: Image,
    pub views: ImageView,
}

impl Depth {
    pub fn new(context: &Context, width: u32, height: u32) -> Result<Self> {
        log::debug!("Creating vulkan depth buffer");

        // Depth format
        let format = {
            let formats = unsafe {
                context.surface.inner.get_physical_device_surface_formats(
                    context.physical_device.inner,
                    context.surface.surface_khr,
                )?
            };
            if formats.len() == 1 && formats[0].format == vk::Format::UNDEFINED {
                vk::Format::D32_SFLOAT
            } else {
                formats
                    .iter()
                    .find(|format| {
                        format.format == vk::Format::D32_SFLOAT
                    })
                    .unwrap_or(&formats[0]).format
            }
        };
        log::debug!("Depth format: {format:?}");

        
        
        Ok(Depth{
            format, 
            image, 
            views: image_view,
        })

    }

   
}
