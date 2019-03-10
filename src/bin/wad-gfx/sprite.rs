use std::path::Path;

use ndarray::prelude::*;
use num_rational::Rational32;
use wad_gfx::Sprite;

use crate::rangetools::{add, intersect};
use crate::{do_scale, write_png, write_png_32, Format};

fn draw_sprite<Px>(
    mut target: ArrayViewMut2<Px>,
    sprite: &Sprite,
    pos: (i32, i32),
    pixel_mapper: impl Fn(u8) -> Px,
) {
    let (o_y, o_x) = sprite.origin();
    let origin = (o_y as i32, o_x as i32);

    // Position sprite origin at given coordinates
    let offset = (pos.0 - origin.0, pos.1 - origin.1);

    let x_range = 0..sprite.dim().1 as i32; // Sprite dimension
    let x_range = add(x_range, offset.1); // Position on canvas
    let x_range = intersect(x_range, 0..target.dim().1 as i32); // Clip to canvas

    for x in x_range {
        for span in sprite.col((x - offset.1) as _) {
            let y_offset = offset.0 + span.top as i32;

            let span_range = 0..span.pixels.len() as i32;
            let span_range = add(span_range, y_offset);
            let span_range = intersect(span_range, 0..target.dim().0 as i32);

            for y in span_range {
                target[[y as usize, x as usize]] =
                    pixel_mapper(span.pixels[(y - y_offset) as usize]);
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
    format: Format,
    scale: usize,
    output: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(palette.len(), 768);

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

    let pos = pos.unwrap_or_else(|| {
        let (y, x) = sprite.origin();
        (y as _, x as _)
    });

    // PNG can store the pixel aspect ratio in the pHYs chunk. So, I can
    // envision two modes: correcting the pixel aspect ratio by scaling
    // during rendering or storing anamorphic pixels, but specifying the
    // correct pixel aspect ratio in the PNG. I don't know of any software
    // that supports this, but Adobe Photoshop might.

    match format {
        Format::Indexed => {
            let mut target: Array2<u8> = Array2::zeros(canvas_size);

            draw_sprite(target.view_mut(), &sprite, pos, |x| colormap[x as usize]);

            let scaled = do_scale(
                target.view(),
                scale as u32,
                Rational32::from(scale as i32) * pixel_aspect_ratio,
            );

            write_png(output, palette, scaled.view())?;

            Ok(())
        }
        Format::Rgba => {
            let mut target: Array2<[u8; 4]> = Array2::default(canvas_size);

            draw_sprite(target.view_mut(), &sprite, pos, |x| {
                let i = colormap[x as usize] as usize;
                let c = &palette[i * 3..i * 3 + 3];
                [c[0], c[1], c[2], 255]
            });

            let scaled = do_scale(
                target.view(),
                scale as u32,
                Rational32::from(scale as i32) * pixel_aspect_ratio,
            );

            write_png_32(output, scaled.view())?;

            Ok(())
        }
    }
}
