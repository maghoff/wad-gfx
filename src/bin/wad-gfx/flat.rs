use std::path::Path;

use num_rational::Rational32;
use wad_gfx::Flat;

use super::{do_scale, write_png};

pub fn flat_cmd(
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

    write_png(output, Some(palette), scaled.view())?;

    Ok(())
}
