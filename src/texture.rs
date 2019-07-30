use byteorder::{ByteOrder, LittleEndian};
use std::convert::TryInto;

use super::{Sprite, SpriteCanvas};

pub struct TextureDirectory<'a> {
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

        // The following unsafe block is safe because:
        //  * [u8; n] does not have alignment constraints
        //  * The slice has been verified to be large enough
        let offsets: &[[u8; 4]] = unsafe {
            std::slice::from_raw_parts(
                data[offset_array_start..].as_ptr() as *const _,
                num_textures as usize,
            )
        };

        TextureDirectory { offsets, data }
    }

    pub fn len(&self) -> u32 {
        self.offsets.len() as u32
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

        // The following unsafe block is safe because:
        //  * [u8; n] does not have alignment constraints
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

    pub fn name(&self) -> [u8; 8] {
        self.name
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn len(&self) -> u16 {
        self.patch_data.len() as u16
    }

    pub fn patch(&self, index: u16) -> Patch {
        Patch::new(self.patch_data[index as usize])
    }
}

pub struct Patch {
    pub origin_x: i16,
    pub origin_y: i16,
    pub patch_id: u16,
    // step_dir: u16,
    // colormap: u16,
}

impl Patch {
    pub fn new(data: [u8; 10]) -> Patch {
        let step_dir = LittleEndian::read_u16(&data[6..8]);
        let colormap = LittleEndian::read_u16(&data[8..10]);
        debug_assert_eq!(step_dir, 1);
        debug_assert_eq!(colormap, 0);

        Patch {
            origin_x: LittleEndian::read_i16(&data[0..2]),
            origin_y: LittleEndian::read_i16(&data[2..4]),
            patch_id: LittleEndian::read_u16(&data[4..6]),
        }
    }
}

pub fn parse_pnames(data: &[u8]) -> &[[u8; 8]] {
    let num_patches = LittleEndian::read_u32(&data[0..4]);
    assert!(num_patches & 0x80000000 == 0);

    let name_array_start = 4;
    let name_array_byte_size = num_patches as usize * 8;
    let name_array_end = name_array_start + name_array_byte_size;
    assert!(data.len() >= name_array_end);

    // The following unsafe block is safe because:
    //  * [u8; n] does not have alignment constraints
    //  * The slice has been verified to be large enough
    let names: &[[u8; 8]] = unsafe {
        std::slice::from_raw_parts(
            data[name_array_start..].as_ptr() as *const _,
            num_patches as usize,
        )
    };

    names
}

pub trait PatchProvider<'a> {
    fn patch(&self, id: u16) -> Option<Sprite<'a>>;
}

pub struct LazyPatchProvider<'a> {
    wad: wad::WadSlice<'a>,
    pnames: &'a [[u8; 8]],
}

impl<'a> LazyPatchProvider<'a> {
    pub fn new(wad: wad::WadSlice<'a>, pnames: &'a [[u8; 8]]) -> LazyPatchProvider<'a> {
        LazyPatchProvider { wad, pnames }
    }
}

impl<'a> PatchProvider<'a> for LazyPatchProvider<'a> {
    fn patch(&self, id: u16) -> Option<Sprite<'a>> {
        let name = self.pnames.get(id as usize)?;
        let sprite = self.wad.by_id(name)?;
        Some(Sprite::new(sprite))
    }
}

pub struct EagerPatchProvider<'a> {
    patches: Vec<&'a [u8]>,
}

impl<'a> EagerPatchProvider<'a> {
    pub fn new(wad: wad::WadSlice<'a>, pnames: &'a [[u8; 8]]) -> EagerPatchProvider<'a> {
        EagerPatchProvider {
            patches: pnames.iter().map(|id| wad.by_id(id).unwrap()).collect(),
        }
    }
}

impl<'a> PatchProvider<'a> for EagerPatchProvider<'a> {
    fn patch(&self, id: u16) -> Option<Sprite<'a>> {
        Some(Sprite::new(self.patches.get(id as usize)?))
    }
}

pub fn render_texture<'a>(texture: Texture, patch_provider: &impl PatchProvider<'a>) -> Vec<u8> {
    let mut canvas = SpriteCanvas::new(texture.width, texture.height);
    for p in 0..texture.len() {
        let patch = texture.patch(p as u16);
        let sprite = patch_provider
            .patch(patch.patch_id)
            .expect("Missing patches not handled");
        canvas.draw_patch(
            patch.origin_x + sprite.left(),
            patch.origin_y + sprite.top(),
            &sprite,
        );
    }

    canvas.make_sprite()
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

    #[test]
    fn parse_pnames_successful() {
        let pnames = parse_pnames(include_bytes!("pnames.pnames"));

        assert_eq!(&pnames[0], b"WALL00_3");
        assert_eq!(pnames.last(), Some(b"SW2_4\0\0\0"));
    }

    #[test]
    fn basic_render_texture() {
        struct TestPatchProvider;

        impl<'a> PatchProvider<'a> for TestPatchProvider {
            fn patch(&self, _id: u16) -> Option<Sprite<'a>> {
                Some(Sprite::new(include_bytes!("trooa1.sprite")))
            }
        }

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let texture = Texture::new(&[
            b'N', b'A', b'M', b'E', 0, 0, 0, 0,
            0, 0, 0, 0,
            16, 0, // width
            16, 0, // height
            0, 0, 0, 0,
            1, 0, // patch count

            // Patch 0:
            0, 0, // origin x
            0, 0, // origin y
            0, 0, // patch ID
            1, 0, // step dir
            0, 0, // colormap
        ]);

        let sprite_data = render_texture(texture, &TestPatchProvider);

        // Could change with valid implementation changes, but it is unlikely
        let expected = [
            16, 0, 16, 0, 0, 0, 0, 0, 72, 0, 0, 0, 73, 0, 0, 0, 74, 0, 0, 0, 75, 0, 0, 0, 81, 0, 0,
            0, 88, 0, 0, 0, 94, 0, 0, 0, 101, 0, 0, 0, 109, 0, 0, 0, 118, 0, 0, 0, 127, 0, 0, 0,
            137, 0, 0, 0, 147, 0, 0, 0, 157, 0, 0, 0, 168, 0, 0, 0, 179, 0, 0, 0, 255, 255, 255,
            10, 1, 1, 96, 0, 255, 10, 2, 2, 90, 96, 0, 255, 11, 1, 1, 92, 0, 255, 11, 2, 2, 96, 93,
            0, 255, 11, 3, 3, 99, 90, 98, 0, 255, 12, 4, 4, 81, 90, 73, 74, 0, 255, 12, 4, 4, 70,
            72, 73, 75, 0, 255, 11, 5, 5, 70, 68, 70, 72, 74, 0, 255, 11, 5, 5, 68, 67, 68, 69, 71,
            0, 255, 11, 5, 5, 67, 69, 67, 68, 70, 0, 255, 10, 6, 6, 68, 68, 70, 69, 69, 71, 0, 255,
            10, 6, 6, 69, 70, 74, 72, 70, 71, 0, 255, 4, 3, 3, 71, 70, 70, 0, 9, 7, 7, 69, 70, 73,
            76, 74, 73, 71, 0, 255,
        ];

        assert_eq!(sprite_data.len(), expected.len());
        assert!(sprite_data.iter().zip(expected.iter()).all(|(a, b)| a == b));
    }
}
