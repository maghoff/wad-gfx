
use byteorder::{ByteOrder, LittleEndian};
use std::convert::TryInto;
pub struct TextureDirectory<'a> {
    num_textures: u32,
    offsets: &'a [[u8; 4]],
    data: &'a [u8],
}

impl<'a> TextureDirectory<'a> {
    pub fn new(data: &[u8]) -> TextureDirectory {
        let num_textures = LittleEndian::read_u32(&data[0..4]);
        assert!(num_textures & 0x80000000 == 0);

        let offset_array_start = 4;
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

    pub fn len(&self) -> u32 {
        self.num_textures
    }

    pub fn texture(&self, index: u32) -> Texture<'a> {
        let start = LittleEndian::read_u32(&self.offsets[index as usize]) as usize;
        let end = self
            .offsets
            .get(index as usize + 1)
            .map(|x| LittleEndian::read_u32(x) as usize)
            .unwrap_or(self.data.len());

        Texture::new(&self.data[start..end])
    }
}

pub struct Texture<'a> {
    name: [u8; 8],
    // masked: bool,
    width: u16,
    height: u16,
    // columndirectory: u32,
    // patch_count: u16,
    patch_data: &'a [[u8; 10]],
}

impl<'a> Texture<'a> {
    pub fn new(data: &[u8]) -> Texture {
        let name = data[0..8].try_into().unwrap();
        let width = LittleEndian::read_u16(&data[12..14]);
        let height = LittleEndian::read_u16(&data[14..16]);
        let patch_count = LittleEndian::read_u16(&data[20..22]);

        let patch_data_start = 22;
        let patch_data_byte_size = patch_count as usize * 10;
        let patch_data_end = patch_data_start + patch_data_byte_size;
        assert!(data.len() >= patch_data_end);

        // The following transmute is safe because:
        //  * [u8; 4] does not have alignment constraints
        //  * The slice has been verified to be large enough
        let patch_data: &[[u8; 10]] = unsafe {
            std::slice::from_raw_parts(
                data[patch_data_start..].as_ptr() as *const _,
                patch_count as usize,
            )
        };

        Texture {
            name,
            width,
            height,
            patch_data,
        }
    }

    pub fn len(&self) -> u16 {
        self.patch_data.len() as u16
    }

    pub fn patch(&self, index: u16) -> Patch {
        Patch::new(self.patch_data[index as usize])
    }
}

pub struct Patch {}

impl Patch {
    pub fn new(_data: [u8; 10]) -> Patch {
        Patch {}
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn construct_ok() {
        let texture_dir = TextureDirectory::new(include_bytes!("texture1.texture_dir"));
        assert_eq!(texture_dir.len(), 125);
    }

    #[test]
    fn get_all_textures() {
        let texture_dir = TextureDirectory::new(include_bytes!("texture1.texture_dir"));

        for i in 0..texture_dir.len() {
            let _ = texture_dir.texture(i);
        }
    }

    #[test]
    fn get_all_patches() {
        let texture_dir = TextureDirectory::new(include_bytes!("texture1.texture_dir"));

        for i in 0..texture_dir.len() {
            let texture = texture_dir.texture(i);

            for p in 0..texture.len() {
                let _ = texture.patch(p as u16);
            }
        }
    }
}
