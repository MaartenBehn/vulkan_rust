use super::{VulkanApp, texture::Texture, swapchain::SwapchainProperties};

use ash::{vk, Device};

impl VulkanApp{

    pub fn create_framebuffers(
        device: &Device,
        image_views: &[vk::ImageView],
        color_texture: Texture,
        depth_texture: Texture,
        render_pass: vk::RenderPass,
        swapchain_properties: SwapchainProperties,
    ) -> Vec<vk::Framebuffer> {
        image_views
            .iter()
            .map(|view| [color_texture.view, depth_texture.view, *view])
            .map(|attachments| {
                let framebuffer_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(swapchain_properties.extent.width)
                    .height(swapchain_properties.extent.height)
                    .layers(1)
                    .build();
                unsafe { device.create_framebuffer(&framebuffer_info, None).unwrap() }
            })
            .collect::<Vec<_>>()
    }
}