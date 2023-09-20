use std::fs;
use app::anyhow::Result;

#[allow(dead_code)]
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

#[allow(dead_code)]
pub unsafe fn any_as_u8_slice_mut<T: Sized>(p: &mut T) -> &mut [u8] {
    ::core::slice::from_raw_parts_mut((p as *const T) as *mut u8, ::core::mem::size_of::<T>())
}

#[allow(dead_code)]
pub unsafe fn slice_as_u8_slice<T: Sized>(p: &[T]) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p.first().unwrap() as *const T) as *const u8,
        ::core::mem::size_of::<T>() * p.len(),
    )
}

#[allow(dead_code)]
pub unsafe fn slice_as_u8_slice_mut<T: Sized>(p: &mut [T]) -> &mut [u8] {
    ::core::slice::from_raw_parts_mut(
        (p.first().unwrap() as *const T) as *mut u8,
        ::core::mem::size_of::<T>() * p.len(),
    )
}

pub fn create_dir(folder_path: &str) -> Result<()> {
    let _ = fs::remove_dir_all(folder_path);
    fs::create_dir(folder_path)?;

    Ok(())
}
