extern crate wad_gfx;

use std::path::{Path, PathBuf};

use ndarray::prelude::*;
use num_rational::Rational32;
use structopt::StructOpt;
use wad::EntryId;
use wad_gfx::*;

fn parse_pair<T: std::str::FromStr>(src: &str) -> Result<(T, T), &'static str> {
    const FORMAT_ERROR: &str =
        "format must be two integers separated by `x` or `,`, eg 320x200 or 100,200";

    let mut split = src
        .splitn(2, |x| x == 'x' || x == ',')
        .map(|x| x.parse().map_err(|_| FORMAT_ERROR));

    let x = split
        .next()
        .expect("splitn() yields at least one element")?;
    let y = split.next().unwrap_or(Err(FORMAT_ERROR))?;

    Ok((y, x))
}

#[derive(Debug, StructOpt)]
enum Graphics {
    /// Extract a flat
    #[structopt(name = "flat")]
    Flat,

    /// Extract a sprite
    #[structopt(name = "sprite")]
    Sprite {
        /// Canvas size for the output
        #[structopt(long = "canvas", parse(try_from_str = "parse_pair"))]
        canvas_size: Option<(u32, u32)>,

        /// Place the sprite's hotspot at these coordinates
        #[structopt(long = "pos", parse(try_from_str = "parse_pair"))]
        pos: Option<(i32, i32)>,

        /// Print information about the sprite to stdout instead of
        /// generating an output image
        #[structopt(short = "I", long = "info")]
        info: bool,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(name = "wad-gfx", about = "Extract graphics from Doom WAD files")]
struct Opt {
    /// Input WAD file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// The lump name of the graphic to extract
    name: String,

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

fn do_scale(input: ArrayView2<u8>, sx: u32, sy: Rational32) -> Array2<u8> {
    let mut target: Array2<u8> = Array2::zeros((
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

fn flat_cmd(
    palette: &[u8],
    colormap: &[u8],
    gfx: &[u8],
    scale: usize,
    output: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let gfx = Flat::new(&gfx)?;
    let mut mapped = [0u8; 64 * 64];

    mapped
        .iter_mut()
        .zip(gfx.view().iter())
        .for_each(|(m, g)| *m = colormap[*g as usize]);

    let flat = Flat::new(&mapped)?;

    let scaled = do_scale(flat.view(), scale as u32, Rational32::from(scale as i32));

    write_png(output, palette, scaled.view())?;

    Ok(())
}

fn sprite_cmd(
    palette: &[u8],
    colormap: &[u8],
    gfx: &[u8],
    info: bool,
    canvas_size: Option<(u32, u32)>,
    pos: Option<(i32, i32)>,
    scale: usize,
    output: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sprite = Sprite::new(gfx);

    if info {
        print!(
            "Dimensions: {}x{}\nOrigin: {},{}\nSize (b): {}\n",
            sprite.dim().1,
            sprite.dim().0,
            sprite.origin().1,
            sprite.origin().0,
            gfx.len(),
        );
        return Ok(());
    }

    let pixel_aspect_ratio = Rational32::new(320, 200) / Rational32::new(4, 3);

    let canvas_size = canvas_size
        .map(|(y, x)| (y as usize, x as usize))
        .unwrap_or(sprite.dim());

    let (o_y, o_x) = sprite.origin();
    let pos = pos.unwrap_or((o_y as _, o_x as _));

    let mut target: Array2<u8> = Array2::zeros(canvas_size);

    for x in 0..sprite.dim().1 {
        for span in sprite.col(x as _) {
            for y in 0..span.pixels.len() {
                target[[
                    y as usize + span.top as usize + pos.0 as usize - o_y as usize,
                    x as usize + pos.1 as usize - o_x as usize,
                ]] = span.pixels[y as usize];
            }
        }
    }

    // When painting sprites with transparency, the way to do it might be
    // to paint in 32 bit RGBA color space.  In that case, colormapping
    // must come earlier. Maybe paint_gfx could take some painter parameter
    // which could transparently apply a colormap?
    target.iter_mut().for_each(|x| *x = colormap[*x as usize]);

    let scaled = do_scale(
        target.view(),
        scale as u32,
        Rational32::from(scale as i32) * pixel_aspect_ratio,
    );

    // PNG can store the pixel aspect ratio in the pHYs chunk. So, I can
    // envision two modes: correcting the pixel aspect ratio by scaling
    // during rendering or storing anamorphic pixels, but specifying the
    // correct pixel aspect ratio in the PNG. I don't know of any software
    // that supports this, but Adobe Photoshop might.
    write_png(output, palette, scaled.view())?;

    Ok(())
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

    let output = format!("{}.png", opt.name.to_ascii_lowercase());

    match opt.gfx {
        Graphics::Flat => flat_cmd(palette, colormap, gfx, opt.scale, output),
        Graphics::Sprite {
            canvas_size,
            pos,
            info,
        } => sprite_cmd(
            palette,
            colormap,
            gfx,
            info,
            canvas_size,
            pos,
            opt.scale,
            output,
        ),
    }
}
