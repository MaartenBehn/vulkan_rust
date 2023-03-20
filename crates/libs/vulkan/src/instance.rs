use std::ffi::{c_void, CStr, CString, c_char};

use anyhow::{Result, ensure};
use ash::{extensions::ext::DebugUtils, vk::{self, DebugUtilsMessengerEXT}, Entry, Instance as AshInstance};
use raw_window_handle::HasRawDisplayHandle;

use crate::{physical_device::PhysicalDevice, surface::Surface, Version};

const REQUIRED_DEBUG_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub struct Instance {
    pub(crate) inner: AshInstance,
    debug_report_callback: Option<(DebugUtils, DebugUtilsMessengerEXT)>,
    physical_devices: Vec<PhysicalDevice>,
}

impl Instance {
    pub(crate) fn new(
        entry: &Entry,
        display_handle: &dyn HasRawDisplayHandle,
        api_version: Version,
        app_name: &str,
    ) -> Result<Self> {
        // Vulkan instance
        let app_name = CString::new(app_name)?;

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .api_version(api_version.make_api_version());


        let mut extension_names =
            ash_window::enumerate_required_extensions(display_handle.raw_display_handle())?
                .to_vec();

        // Validation
        #[cfg(debug_assertions)]
        extension_names.push(DebugUtils::name().as_ptr());

        let mut instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);


        // Validation
        #[cfg(debug_assertions)]
        let (_layer_names, layer_names_ptrs) = get_validation_layer_names_and_pointers();
        #[cfg(debug_assertions)]
        {
            check_validation_layer_support(&entry)?;
            instance_create_info = instance_create_info.enabled_layer_names(&layer_names_ptrs);
        }

        // Debug Printing
        #[cfg(debug_assertions)]
        let mut validation_features = vk::ValidationFeaturesEXT::builder()
                .enabled_validation_features(&[vk::ValidationFeatureEnableEXT::DEBUG_PRINTF]);
        #[cfg(debug_assertions)]
        {
            instance_create_info = instance_create_info.push_next(&mut validation_features);
        }
        
        // Creating Instance
        let inner = unsafe { entry.create_instance(&instance_create_info, None)? };
        
        // Validation
        let debug_report_callback = setup_debug_messenger(&entry, &inner);

        Ok(Self {
            inner,
            debug_report_callback,
            physical_devices: vec![],
        })
    }

    pub(crate) fn enumerate_physical_devices(
        &mut self,
        surface: &Surface,
    ) -> Result<&[PhysicalDevice]> {
        if self.physical_devices.is_empty() {
            let physical_devices = unsafe { self.inner.enumerate_physical_devices()? };

            let mut physical_devices = physical_devices
                .into_iter()
                .map(|pd| PhysicalDevice::new(&self.inner, surface, pd))
                .collect::<Result<Vec<_>>>()?;

            physical_devices.sort_by_key(|pd| match pd.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                _ => 2,
            });

            self.physical_devices = physical_devices;
        }

        Ok(&self.physical_devices)
    }
}


/// Get the pointers to the validation layers names.
/// Also return the corresponding `CString` to avoid dangling pointers.
pub fn get_validation_layer_names_and_pointers() -> (Vec<CString>, Vec<*const c_char>) {
    let layer_names = REQUIRED_DEBUG_LAYERS
        .iter()
        .map(|name| CString::new(*name).unwrap())
        .collect::<Vec<_>>();
    let layer_names_ptrs = layer_names
        .iter()
        .map(|name| name.as_ptr())
        .collect::<Vec<_>>();
    (layer_names, layer_names_ptrs)
}

/// Check if the required validation set in `REQUIRED_LAYERS`
/// are supported by the Vulkan instance.
///
/// # Panics
///
/// Panic if at least one on the layer is not supported.
pub fn check_validation_layer_support(entry: &Entry) -> Result<()> {
    for required in REQUIRED_DEBUG_LAYERS.iter() {
        let found = entry
            .enumerate_instance_layer_properties()
            .unwrap()
            .iter()
            .any(|layer| {
                let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                let name = name.to_str().expect("Failed to get layer name pointer");
                required == &name
            });

        ensure!(found, "Validation layer not supported: {}", required);
    }

    Ok(())
}


unsafe extern "system" fn vulkan_debug_callback(
    flag: vk::DebugUtilsMessageSeverityFlagsEXT,
    typ: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    use vk::DebugUtilsMessageSeverityFlagsEXT as Flag;

    let message = CStr::from_ptr((*p_callback_data).p_message);
    match flag {
        Flag::VERBOSE => log::trace!("{:?} - {:?}", typ, message),
        Flag::INFO => log::info!("{:?} - {:?}", typ, message),
        Flag::WARNING => log::warn!("{:?} - {:?}", typ, message),
        _ => log::error!("{:?} - {:?}", typ, message),
    }
    vk::FALSE
}

/// Setup the debug message if validation layers are enabled.
pub fn setup_debug_messenger(
    entry: &Entry,
    instance: &AshInstance,
) -> Option<(DebugUtils, vk::DebugUtilsMessengerEXT)> {

    #[cfg(not(debug_assertions))]
    return None;

    let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE | 
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL | 
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE | 
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION)
        .pfn_user_callback(Some(vulkan_debug_callback));


    let debug_utils = DebugUtils::new(entry, instance);
    let debug_utils_messenger = unsafe {
        debug_utils
            .create_debug_utils_messenger(&create_info, None)
            .unwrap()
    };

    Some((debug_utils, debug_utils_messenger))
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            if let Some((utils, messenger)) = self.debug_report_callback.take() {
                utils.destroy_debug_utils_messenger(messenger, None);
            }
            self.inner.destroy_instance(None);
        }
    }
}




