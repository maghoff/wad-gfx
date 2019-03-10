use std::path::Path;

use ndarray::prelude::*;
use num_rational::Rational32;
use wad_gfx::Sprite;

use crate::rangetools::{add, intersect};
use crate::{do_scale, write_png};

fn draw_sprite(mut target: ArrayViewMut2<u8>, sprite: &Sprite, pos: (i32, i32)) {
    let (o_y, o_x) = sprite.origin();
    let (o_y, o_x) = (o_y as i32, o_x as i32);

    // Sprite dimensions
    let x_range = 0..sprite.dim().1 as i32;

    // Position sprite origin at user specified position
    let x_offset = pos.1 - o_x;
    let x_range = add(&x_range, x_offset);

    // Clip to target dimensions
    let x_range = intersect(&x_range, &(0..target.dim().1 as i32));

    for x in x_range {
        for span in sprite.col((x - x_offset) as _) {
            let span_range = 0..span.pixels.len() as i32;
            let y_offset = span.top as i32 + pos.0 - o_y;
            let span_range = add(&span_range, y_offset);
            let span_range = intersect(&span_range, &(0..target.dim().0 as i32));
            for y in span_range {
                target[[y as usize, x as usize]] = span.pixels[(y - y_offset) as usize];
            }
        }
    }
}

pub fn sprite_cmd(
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

    let mut target: Array2<u8> = Array2::zeros(canvas_size);

    let pos = pos.unwrap_or_else(|| {
        let (y, x) = sprite.origin();
        (y as _, x as _)
    });

    draw_sprite(target.view_mut(), &sprite, pos);

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
