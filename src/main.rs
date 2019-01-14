use std::path::{Path, PathBuf};

use ndarray::prelude::*;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "doom-gfx", about = "Extract graphics from Doom WAD files")]
struct Opt {
    /// Input WAD file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Flat to extract
    flat: String,

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

trait WadExt {
    fn lump_by_name<'a>(&'a self, name: &str) -> Option<&'a [u8]>;
}

impl WadExt for wad::Wad {
    fn lump_by_name<'a>(&'a self, name: &str) -> Option<&'a [u8]> {
        self.iter()
            .find(|(lump_name, _)| *lump_name == name)
            .map(|x| x.1)
    }
}

fn write_png(
    filename: impl AsRef<Path>,
    palette: &[u8],
    width: u32,
    height: u32,
    gfx: &[u8]
) ->
    Result<(), Box<dyn std::error::Error>>
{
    use std::fs::File;
    use std::io::BufWriter;
    use png::HasParameters;

    let file = File::create(filename)?;
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set(png::ColorType::Indexed);
    let mut writer = encoder.write_header()?;
    writer.write_chunk(*b"PLTE", palette)?;
    writer.write_image_data(gfx)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let wad = wad::load_wad_file(&opt.input)?;

    let palettes = wad.lump_by_name("PLAYPAL").ok_or("Missing PLAYPAL")?;
    let palette_index = opt.palette.checked_mul(768).ok_or("Overflow")?;
    let palette = &palettes[palette_index..palette_index+768];

    let colormaps = wad.lump_by_name("COLORMAP").ok_or("Missing COLORMAP")?;
    let colormap_index = opt.colormap.checked_mul(256).ok_or("Overflow")?;
    let colormap = &colormaps[colormap_index..colormap_index+256];

    let gfx = wad.lump_by_name(&opt.flat).ok_or_else(|| format!("Cannot find {}", opt.flat))?;

    let gfx = ArrayView2::from_shape((64, 64), gfx)?;

    let mut scaled: Array2<u8> = Array2::zeros((gfx.dim().0 * opt.scale, gfx.dim().1 * opt.scale));

    for y in 0..scaled.dim().0 {
        for x in 0..scaled.dim().1 {
            scaled[[x, y]] = gfx[[x/opt.scale, y/opt.scale]];
        }
    }

    let gfx = scaled.iter()
        .map(|x| colormap[*x as usize])
        .collect::<Vec<_>>();

    write_png(
        format!("{}.png", opt.flat.to_ascii_lowercase()),
        palette,
        scaled.dim().0 as u32,
        scaled.dim().1 as u32,
        &gfx
    )?;

    Ok(())
}
