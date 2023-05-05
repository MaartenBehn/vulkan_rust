use ash::vk::{self, DeviceSize};
use std::iter::Iterator;
use std::marker::PhantomData;
use std::mem::size_of;
use std::os::raw::c_void;
use std::slice::from_raw_parts_mut;
use std::{io, slice};

/// [`Align`] handles dynamic alignment. The is useful for dynamic uniform buffers where
/// the alignment might be different. For example a 4x4 f32 matrix has a size of 64 bytes
/// but the min alignment for a dynamic uniform buffer might be 256 bytes. A slice of `&[Mat4x4<f32>]`
/// has a memory layout of `[[64 bytes], [64 bytes], [64 bytes]]`, but it might need to have a memory
/// layout of `[[256 bytes], [256 bytes], [256 bytes]]`.
/// [`Align::copy_from_slice`] will copy a slice of `&[T]` directly into the host memory without
/// an additional allocation and with the correct alignment.
#[derive(Debug, Clone)]
pub struct Align<T> {
    pub ptr: *mut c_void,
    pub elem_size: vk::DeviceSize,
    pub size: vk::DeviceSize,
    pub start: vk::DeviceSize,
    _m: PhantomData<T>,
}

#[derive(Debug)]
pub struct AlignIter<'a, T: 'a> {
    align: &'a mut Align<T>,
    current: vk::DeviceSize,
}

fn calc_padding(adr: vk::DeviceSize, align: vk::DeviceSize) -> vk::DeviceSize {
    (align - adr % align) % align
}

impl<T: Copy> Align<T> {
    pub fn copy_from_slice(&mut self, data: &[T]) {
        if self.elem_size == size_of::<T>() as u64 {
            unsafe {
                let mapped_slice = from_raw_parts_mut(
                    (self.ptr.cast::<u8>()).offset(self.start as isize).cast(),
                    data.len(),
                );
                mapped_slice.copy_from_slice(data);
            }
        } else {
            for (i, val) in self.iter_mut().enumerate().take(data.len()) {
                *val = data[i];
            }
        }
    }

    pub fn copy_to_slice(&mut self, count: usize) -> Vec<T> {
        let mut data = Vec::with_capacity(count);
        for (_, val) in self.iter_mut().enumerate().take(count) {
            data.push(*val);
        }

        data
    }
}

impl<T> Align<T> {
    pub unsafe fn new(
        ptr: *mut c_void,
        alignment: vk::DeviceSize,
        count: usize,
        offset: usize,
    ) -> Self {
        let padding = calc_padding(size_of::<T>() as vk::DeviceSize, alignment);
        let elem_size = size_of::<T>() as vk::DeviceSize + padding;
        //assert!(calc_padding(size, alignment) == 0, "size must be aligned");
        Self {
            ptr,
            elem_size,
            size: (offset + count) as vk::DeviceSize * elem_size,
            start: offset as vk::DeviceSize * elem_size,
            _m: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> AlignIter<T> {
        AlignIter {
            current: self.start,
            align: self,
        }
    }
}

impl<'a, T: Copy + 'a> Iterator for AlignIter<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.align.size {
            return None;
        }
        unsafe {
            // Need to cast to *mut u8 because () has size 0
            let ptr = (self.align.ptr.cast::<u8>())
                .offset(self.current as isize)
                .cast();
            self.current += self.align.elem_size;
            Some(&mut *ptr)
        }
    }
}

/// Decode SPIR-V from bytes.
///
/// This function handles SPIR-V of arbitrary endianness gracefully, and returns correctly aligned
/// storage.
///
/// # Examples
/// ```no_run
/// // Decode SPIR-V from a file
/// let mut file = std::fs::File::open("/path/to/shader.spv").unwrap();
/// let words = ash::util::read_spv(&mut file).unwrap();
/// ```
/// ```
/// // Decode SPIR-V from memory
/// const SPIRV: &[u8] = &[
///     // ...
/// #   0x03, 0x02, 0x23, 0x07,
/// ];
/// let words = ash::util::read_spv(&mut std::io::Cursor::new(&SPIRV[..])).unwrap();
/// ```
pub fn read_spv<R: io::Read + io::Seek>(x: &mut R) -> io::Result<Vec<u32>> {
    let size = x.seek(io::SeekFrom::End(0))?;
    if size % 4 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input length not divisible by 4",
        ));
    }
    if size > usize::max_value() as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "input too long"));
    }
    let words = (size / 4) as usize;
    // https://github.com/MaikKlein/ash/issues/354:
    // Zero-initialize the result to prevent read_exact from possibly
    // reading uninitialized memory.
    let mut result = vec![0u32; words];
    x.seek(io::SeekFrom::Start(0))?;
    x.read_exact(unsafe {
        slice::from_raw_parts_mut(result.as_mut_ptr().cast::<u8>(), words * 4)
    })?;
    const MAGIC_NUMBER: u32 = 0x0723_0203;
    if !result.is_empty() && result[0] == MAGIC_NUMBER.swap_bytes() {
        for word in &mut result {
            *word = word.swap_bytes();
        }
    }
    if result.is_empty() || result[0] != MAGIC_NUMBER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input missing SPIR-V magic number",
        ));
    }
    Ok(result)
}
