use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "doom-gfx", about = "Extract graphics from Doom WAD files")]
struct Opt {
    /// Input WAD file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Flat to extract
    flat: String,

    /// Which palette to use
    #[structopt(short = "p", long = "palette", default_value = "0")]
    palette: usize,

    /// Which colormap to use
    #[structopt(short = "c", long = "colormap", default_value = "0")]
    colormap: usize,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let wad = wad::load_wad_file(&opt.input)?;

    let palettes = wad.lump_by_name("PLAYPAL").ok_or("Missing PLAYPAL")?;
    let palette_index = opt.palette.checked_mul(768).ok_or("Overflow")?;
    let palette = &palettes[palette_index..palette_index+768];

    let colormaps = wad.lump_by_name("COLORMAP").ok_or("Missing COLORMAP")?;
    let colormap_index = opt.colormap.checked_mul(256).ok_or("Overflow")?;
    let colormap = &colormaps[colormap_index..colormap_index+256];

    let raw_gfx = wad.lump_by_name(&opt.flat).ok_or_else(|| format!("Cannot find {}", opt.flat))?;

    // GFX is stored in column-major order. Transpose:
    let mut gfx = [0u8; 64*64];
    for x in 0..64 {
        for y in 0..64 {
            gfx[x + y*64] = raw_gfx[x*64 + y];
        }
    }

    // For reading and opening files
    use std::fs::File;
    use std::io::BufWriter;
    use png::HasParameters;

    let file = File::create(format!("{}.png", opt.flat.to_ascii_lowercase()))?;
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, 64, 64);
    encoder.set(png::ColorType::Indexed);
    let mut writer = encoder.write_header()?;
    writer.write_chunk(*b"PLTE", palette)?;
    writer.write_image_data(&gfx.iter().map(|x| colormap[*x as usize]).collect::<Vec<_>>())?;

    Ok(())
}
