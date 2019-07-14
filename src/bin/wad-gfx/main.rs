extern crate wad_gfx;
extern crate rangetools;

mod flat;
mod sprite;

use std::path::{Path, PathBuf};

use ndarray::prelude::*;
use num_rational::Rational32;
use structopt::StructOpt;
use wad::EntryId;

#[derive(Debug, StructOpt)]
enum Graphics {
    /// Extract a flat
    #[structopt(name = "flat")]
    Flat,

    /// Extract a sprite
    #[structopt(name = "sprite")]
    Sprite(sprite::SpriteOpt),
}

#[derive(Debug, StructOpt)]
#[structopt(name = "wad-gfx", about = "Extract graphics from Doom WAD files")]
struct Opt {
    /// Input WAD file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// The lump name of the graphic to extract
    name: String,

    /// Output filename. If absent, will default to <name>.png
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(subcommand)]
    gfx: Graphics,

    /// Which palette to use (0-13)
    #[structopt(short = "p", long = "palette", default_value = "0")]
    palette: usize,

    /// Which colormap to use (0-33)
    #[structopt(short = "c", long = "colormap", default_value = "0")]
    colormap: usize,

    /// Scale with beautiful nearest neighbor filtering
    #[structopt(short = "s", long = "scale", default_value = "2")]
    scale: usize,
}

struct PhysChunk {
    x: i32,
    y: i32,
    unit: u8,
}

use std::io::Write;

impl PhysChunk {
    fn serialize(&self, w: &mut impl Write) -> std::io::Result<()> {
        use byteorder::{BigEndian, WriteBytesExt};
        w.write_i32::<BigEndian>(self.x)?;
        w.write_i32::<BigEndian>(self.y)?;
        w.write_u8(self.unit)?;
        Ok(())
    }
}

fn write_png(
    filename: impl AsRef<Path>,
    palette: Option<&[u8]>,
    pixel_aspect: Rational32,
    gfx: ArrayView2<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    use png::HasParameters;
    use std::fs::File;
    use std::io::{BufWriter, Cursor};

    assert!(gfx.dim().0 <= i32::max_value() as usize);
    assert!(gfx.dim().1 <= i32::max_value() as usize);
    assert_eq!(gfx.stride_of(Axis(1)), 1);
    assert_eq!(gfx.stride_of(Axis(0)), gfx.dim().1 as isize);

    let file = File::create(filename)?;
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, gfx.dim().1 as u32, gfx.dim().0 as u32);
    encoder.set(png::ColorType::Indexed);
    encoder.set(png::Compression::Best);
    let mut writer = encoder.write_header()?;

    if let Some(palette) = palette {
        writer.write_chunk(png::chunk::PLTE, palette)?;
    }

    if pixel_aspect != Rational32::from(1) {
        let mut buf = [0u8; 9];
        let phys_chunk = PhysChunk {
            x: *pixel_aspect.numer(),
            y: *pixel_aspect.denom(),
            unit: 0,
        };
        phys_chunk.serialize(&mut Cursor::new(&mut buf as &mut [u8]))?;
        writer.write_chunk(png::chunk::pHYs, &buf)?;
    }

    writer.write_image_data(gfx.into_slice().unwrap())?;

    Ok(())
}

fn write_png_32(
    filename: impl AsRef<Path>,
    palette: Option<&[u8]>,
    pixel_aspect: Rational32,
    gfx: ArrayView2<[u8; 4]>,
) -> Result<(), Box<dyn std::error::Error>> {
    use png::HasParameters;
    use std::fs::File;
    use std::io::{BufWriter, Cursor};

    assert!(gfx.dim().0 <= i32::max_value() as usize);
    assert!(gfx.dim().1 <= i32::max_value() as usize);
    assert_eq!(gfx.stride_of(Axis(1)), 1);
    assert_eq!(gfx.stride_of(Axis(0)), gfx.dim().1 as isize);

    let file = File::create(filename)?;
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, gfx.dim().1 as u32, gfx.dim().0 as u32);
    encoder.set(png::ColorType::RGBA);
    encoder.set(png::Compression::Best);
    let mut writer = encoder.write_header()?;

    if let Some(palette) = palette {
        writer.write_chunk(png::chunk::PLTE, palette)?;
    }

    if pixel_aspect != Rational32::from(1) {
        let mut buf = [0u8; 9];
        let phys_chunk = PhysChunk {
            x: *pixel_aspect.numer(),
            y: *pixel_aspect.denom(),
            unit: 0,
        };
        phys_chunk.serialize(&mut Cursor::new(&mut buf as &mut [u8]))?;
        writer.write_chunk(png::chunk::pHYs, &buf)?;
    }

    let raw_data = gfx.into_slice().unwrap();
    writer.write_image_data(unsafe {
        std::slice::from_raw_parts(raw_data.as_ptr() as *const u8, raw_data.len() * 4)
    })?;

    Ok(())
}

fn do_scale<Px: Default + Copy>(input: ArrayView2<Px>, sx: u32, sy: Rational32) -> Array2<Px> {
    let mut target: Array2<Px> = Array2::default((
        (Rational32::from(input.dim().0 as i32) * sy).to_integer() as usize,
        (input.dim().1 as u32 * sx) as usize,
    ));

    for y in 0..target.dim().0 {
        let src_y = (Rational32::from(y as i32) / sy).to_integer();
        for x in 0..target.dim().1 {
            let src_x = x as u32 / sx;
            target[(y, x)] = input[(src_y as usize, src_x as usize)];
        }
    }

    target
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let wad = wad::load_wad_file(&opt.input)?;

    let palettes = wad.by_id(b"PLAYPAL").ok_or("Missing PLAYPAL")?;
    let palette_index = opt.palette.checked_mul(768).ok_or("Overflow")?;
    let palette = &palettes[palette_index..palette_index + 768];

    let colormaps = wad.by_id(b"COLORMAP").ok_or("Missing COLORMAP")?;
    let colormap_index = opt.colormap.checked_mul(256).ok_or("Overflow")?;
    let colormap = &colormaps[colormap_index..colormap_index + 256];

    let gfx_id =
        EntryId::from_str(&opt.name).ok_or_else(|| format!("Invalid ID: {:?}", opt.name))?;
    let gfx = wad
        .by_id(gfx_id)
        .ok_or_else(|| format!("Cannot find {}", opt.name))?;

    let output = opt
        .output
        .clone()
        .unwrap_or_else(|| format!("{}.png", opt.name.to_ascii_lowercase()).into());

    match opt.gfx {
        Graphics::Flat => flat::flat_cmd(palette, colormap, gfx, opt.scale, output),
        Graphics::Sprite(opts) => {
            sprite::sprite_cmd(palette, colormap, gfx, opt.scale, output, opts)
        }
    }
}
