use byteorder::{ByteOrder, LittleEndian};

pub struct TextureDirectory<'a> {
    num_textures: u32,
    offsets: &'a [[u8; 4]],
    data: &'a [u8],
}

impl<'a> TextureDirectory<'a> {
    pub fn new(data: &[u8]) -> TextureDirectory {
        let num_textures = LittleEndian::read_u32(&data[0..4]);
        assert!(num_textures & 0x80000000 == 0);

        let offset_array_start = 8;
        let offset_array_byte_size = num_textures as usize * 4;
        let offset_array_end = offset_array_start + offset_array_byte_size;
        assert!(data.len() >= offset_array_end);

        // The following transmute is safe because:
        //  * [u8; 4] does not have alignment constraints
        //  * The slice has been verified to be large enough
        let offsets: &[[u8; 4]] = unsafe {
            std::slice::from_raw_parts(
                data[offset_array_start..].as_ptr() as *const _,
                num_textures as usize,
            )
        };

        TextureDirectory {
            num_textures,
            offsets,
            data,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn construct_ok() {
        let _ = TextureDirectory::new(include_bytes!("texture1.texture_dir"));
    }
}
