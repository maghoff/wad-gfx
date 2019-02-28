extern crate wad_gfx;

use std::path::{Path, PathBuf};

use ndarray::prelude::*;
use ndarray::s;
use num_rational::Rational32;
use structopt::StructOpt;
use wad::EntryId;
use wad_gfx::*;

#[derive(Debug, StructOpt)]
enum Graphics {
    #[structopt(name = "flat")]
    Flat { name: String },

    #[structopt(name = "sprite")]
    Sprite { name: String },
}

#[derive(Debug, StructOpt)]
#[structopt(name = "wad-gfx", about = "Extract graphics from Doom WAD files")]
struct Opt {
    /// Input WAD file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Graphics to extract
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

fn write_png(
    filename: impl AsRef<Path>,
    palette: &[u8],
    gfx: ArrayView2<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    use png::HasParameters;
    use std::fs::File;
    use std::io::BufWriter;

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
    writer.write_chunk(*b"PLTE", palette)?;
    writer.write_image_data(gfx.into_slice().unwrap())?;

    Ok(())
}

fn paint_gfx(gfx: &dyn Gfx, scale: usize) -> Array2<u8> {
    let pixel_aspect_ratio = gfx.pixel_aspect_ratio();

    let mut target: Array2<u8> = Array2::zeros((
        (Rational32::from((gfx.dim().0 as usize * scale) as i32) * pixel_aspect_ratio).to_integer()
            as usize,
        gfx.dim().1 as usize * scale,
    ));

    for x in 0..target.dim().1 {
        gfx.draw_column(
            (x / scale) as u32,
            target.slice_mut(s![.., x]),
            pixel_aspect_ratio * Rational32::from(scale as i32),
        );
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

    let (name, gfx): (String, Box<Gfx>) = match opt.gfx {
        Graphics::Flat { name } => {
            let flat_id =
                EntryId::from_str(&name).ok_or_else(|| format!("Invalid ID: {:?}", name))?;
            let gfx = wad
                .by_id(flat_id)
                .ok_or_else(|| format!("Cannot find {}", name))?;

            (name, Box::new(Flat::new(gfx)?) as _)
        }
        Graphics::Sprite { name } => {
            let sprite_id =
                EntryId::from_str(&name).ok_or_else(|| format!("Invalid ID: {:?}", name))?;
            let gfx = wad
                .by_id(sprite_id)
                .ok_or_else(|| format!("Cannot find {}", name))?;

            (name, Box::new(Sprite::new(gfx)) as _)
        }
    };

    let r: &dyn Gfx = &*gfx;
    let mut target = paint_gfx(r, opt.scale);

    // When painting sprites with transparency, the way to do it might be
    // to paint in 32 bit RGBA color space.  In that case, colormapping
    // must come earlier. Maybe paint_gfx could take some painter parameter
    // which could transparently apply a colormap?
    target.iter_mut().for_each(|x| *x = colormap[*x as usize]);

    // PNG can store the pixel aspect ratio in the pHYs chunk. So, I can
    // envision two modes: correcting the pixel aspect ratio by scaling
    // during rendering or storing anamorphic pixels, but specifying the
    // correct pixel aspect ratio in the PNG. I don't know of any software
    // that supports this, but Adobe Photoshop might.
    write_png(
        format!("{}.png", name.to_ascii_lowercase()),
        palette,
        target.view(),
    )?;

    Ok(())
}
