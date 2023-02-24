use super::{VulkanApp, swapchain::*, debug::*};

use ash::extensions::khr::{Surface, Swapchain};
use ash::vk::{AccelerationStructureKHR, AccelerationStructureBuildTypeKHR};
use ash::{vk, Device, Instance};
use std::ffi::CStr;

#[derive(Clone, Copy)]
pub struct QueueFamiliesIndices {
    pub graphics_index: u32,
    pub present_index: u32,
}

impl VulkanApp{
    /// Pick the first suitable physical device.
    ///
    /// # Requirements
    /// - At least one queue family with one queue supportting graphics.
    /// - At least one queue family with one queue supporting presentation to `surface_khr`.
    /// - Swapchain extension support.
    ///
    /// # Returns
    ///
    /// A tuple containing the physical device and the queue families indices.
    pub fn pick_physical_device(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
    ) -> (vk::PhysicalDevice, QueueFamiliesIndices) {
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        let device = devices
            .into_iter()
            .find(|device| Self::is_device_suitable(instance, surface, surface_khr, *device))
            .expect("No suitable physical device.");

        let props = unsafe { instance.get_physical_device_properties(device) };

        let mut subgroup = vk::PhysicalDeviceSubgroupProperties::builder()
            .build();
        let mut props2 = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut subgroup)
            .build();

        unsafe { instance.get_physical_device_properties2(device, &mut props2) };
        
        unsafe { 
            log::debug!("Selected physical device: {:?}
            \tVersion: {:?}
            \tSubgroup: {:?}
            \tCompute work invocaition: {:?}
            \tCompute work size: {:?}", 
            CStr::from_ptr(props.device_name.as_ptr()),
            props.api_version,
            subgroup.subgroup_size,
            props.limits.max_compute_work_group_invocations,
            props.limits.max_compute_work_group_size,
        )};

        let (graphics, present) = Self::find_queue_families(instance, surface, surface_khr, device);
        let queue_families_indices = QueueFamiliesIndices {
            graphics_index: graphics.unwrap(),
            present_index: present.unwrap(),
        };

        (device, queue_families_indices)
    }

    fn is_device_suitable(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> bool {
        let (graphics, present) = Self::find_queue_families(instance, surface, surface_khr, device);
        let extention_support = Self::check_device_extension_support(instance, device);
        let is_swapchain_adequate = {
            let details = SwapchainSupportDetails::new(device, surface, surface_khr);
            !details.formats.is_empty() && !details.present_modes.is_empty()
        };
        let features = unsafe { instance.get_physical_device_features(device) };
        graphics.is_some()
            && present.is_some()
            && extention_support
            && is_swapchain_adequate
            && features.sampler_anisotropy == vk::TRUE
    }

    fn check_device_extension_support(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        let required_extentions = Self::get_required_device_extensions();

        let extension_props = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
        };

        for required in required_extentions.iter() {
            let found = extension_props.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                required == &name
            });

            if !found {
                log::info!("Required Extension: {:?} not found.", required);
                return false;
            }
        }

        true
    }

    pub fn get_required_device_extensions() -> [&'static CStr; 1] {
        [Swapchain::name()]
    }

    pub fn find_queue_families(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> (Option<u32>, Option<u32>) {
        let mut graphics = None;
        let mut present = None;

        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE) && graphics.is_none() {
                graphics = Some(index);
            }

            let present_support = unsafe {
                surface
                    .get_physical_device_surface_support(device, index, surface_khr)
                    .unwrap()
            };
            if present_support && present.is_none() {
                present = Some(index);
            }

            if graphics.is_some() && present.is_some() {
                break;
            }
        }

        (graphics, present)
    }

    /// Create the logical device to interact with `device`, a graphics queue
    /// and a presentation queue.
    ///
    /// # Returns
    ///
    /// Return a tuple containing the logical device, the graphics queue and the presentation queue.
    pub fn create_logical_device_with_graphics_queue(
        instance: &Instance,
        device: vk::PhysicalDevice,
        queue_families_indices: QueueFamiliesIndices,
    ) -> (Device, vk::Queue, vk::Queue) {
        let graphics_family_index = queue_families_indices.graphics_index;
        let present_family_index = queue_families_indices.present_index;
        let queue_priorities = [1.0f32];

        let queue_create_infos = {
            // Vulkan specs does not allow passing an array containing duplicated family indices.
            // And since the family for graphics and presentation could be the same we need to
            // deduplicate it.
            let mut indices = vec![graphics_family_index, present_family_index];
            indices.dedup();

            // Now we build an array of `DeviceQueueCreateInfo`.
            // One for each different family index.
            indices
                .iter()
                .map(|index| {
                    vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(*index)
                        .queue_priorities(&queue_priorities)
                        .build()
                })
                .collect::<Vec<_>>()
        };

        let device_extensions = Self::get_required_device_extensions();
        let device_extensions_ptrs = device_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let device_features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .build();

        let (_layer_names, layer_names_ptrs) = get_layer_names_and_pointers();

        let mut device_create_info_builder = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions_ptrs)
            .enabled_features(&device_features);
        if ENABLE_VALIDATION_LAYERS {
            device_create_info_builder =
                device_create_info_builder.enabled_layer_names(&layer_names_ptrs)
        }
        let device_create_info = device_create_info_builder.build();

        // Build device and queues
        let device = unsafe {
            instance
                .create_device(device, &device_create_info, None)
                .expect("Failed to create logical device.")
        };
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };

        (device, graphics_queue, present_queue)
    }
}