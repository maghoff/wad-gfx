use std::path::Path;

use ndarray::prelude::*;
use num_rational::Rational32;
use structopt::StructOpt;
use wad_gfx::Sprite;

use crate::format::Format;
use crate::rangetools::{add, intersect};
use crate::{do_scale, write_png, write_png_32};

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
pub struct SpriteOpt {
    /// Canvas size for the output. Defaults to the size of the sprite.
    /// See the output from --info.
    #[structopt(long = "canvas", parse(try_from_str = "parse_pair"))]
    pub canvas_size: Option<(u32, u32)>,

    /// Place the sprite's hotspot at these coordinates. Defaults to the
    /// coordinates of the hotspot. See the output from --info.
    #[structopt(long = "pos", parse(try_from_str = "parse_pair"))]
    pub pos: Option<(i32, i32)>,

    /// Print information about the sprite to stdout instead of
    /// generating an output image
    #[structopt(short = "I", long = "info")]
    pub info: bool,

    /// Output format: full/f, indexed/i or mask/m. Full color uses the
    /// alpha channel for transparency. Indexed color does not include
    /// transparency, but can be combined with the mask for transparent
    /// sprites.
    #[structopt(short = "f", long = "format", default_value = "full")]
    pub format: Format,

    /// Color index to use for the background
    #[structopt(short = "b", long = "background")]
    pub background: Option<u8>,

    /// Output anamorphic (non-square) pixels. Like the original assets,
    /// the pixel aspect ratio will be 5:6.
    #[structopt(short = "a", long = "anamorphic")]
    pub anamorphic: bool,
}

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
    scale: usize,
    output: impl AsRef<Path>,
    SpriteOpt {
        canvas_size,
        pos,
        info,
        format,
        background,
        anamorphic,
    }: SpriteOpt,
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

    let (scale_aspect, store_aspect) = if anamorphic {
        (
            Rational32::new(1, 1),
            Rational32::new(4, 3) / Rational32::new(320, 200),
        )
    } else {
        (
            Rational32::new(4, 3) / Rational32::new(320, 200),
            Rational32::new(1, 1),
        )
    };

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
            let background =
                background.ok_or("--background must be specified for the indexed format")?;

            let mut target: Array2<u8> = Array2::from_elem(canvas_size, background);

            draw_sprite(target.view_mut(), &sprite, pos, |x| colormap[x as usize]);

            let scaled = do_scale(
                target.view(),
                scale as u32,
                Rational32::from(scale as i32) / scale_aspect,
            );

            write_png(output, Some(palette), store_aspect, scaled.view())?;

            Ok(())
        }
        Format::Mask => {
            if background.is_some() {
                eprintln!("warning: --background has no effect for mask format");
            }

            let mut target: Array2<u8> = Array2::zeros(canvas_size);

            draw_sprite(target.view_mut(), &sprite, pos, |_| 1);

            let scaled = do_scale(
                target.view(),
                scale as u32,
                Rational32::from(scale as i32) / scale_aspect,
            );

            const MASK_PALETTE: &[u8] = &[0, 0, 0, 255, 255, 255];
            write_png(output, Some(MASK_PALETTE), store_aspect, scaled.view())?;

            Ok(())
        }
        Format::Full => {
            let colormapper = |x| -> [u8; 4] {
                let i = colormap[x as usize] as usize;
                let c = &palette[i * 3..i * 3 + 3];
                [c[0], c[1], c[2], 255]
            };

            let background = background.map(colormapper).unwrap_or_default();

            let mut target: Array2<[u8; 4]> = Array2::from_elem(canvas_size, background);

            draw_sprite(target.view_mut(), &sprite, pos, colormapper);

            let scaled = do_scale(
                target.view(),
                scale as u32,
                Rational32::from(scale as i32) / scale_aspect,
            );

            write_png_32(output, None, store_aspect, scaled.view())?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_pair_x_separator() {
        assert_eq!(parse_pair("10x10"), Ok((10, 10)));
    }

    #[test]
    fn parse_pair_comma_separator() {
        assert_eq!(parse_pair("10,10"), Ok((10, 10)));
    }

    #[test]
    fn parse_pair_error_on_extra_separators() {
        assert!(parse_pair::<i32>("10x10x10").is_err());
    }

    #[test]
    fn parse_pair_u32() {
        assert_eq!(parse_pair("10,10"), Ok((10u32, 10u32)));
    }

    #[test]
    fn parse_pair_i16() {
        assert_eq!(parse_pair("10,10"), Ok((10i16, 10i16)));
    }

    #[test]
    fn parse_pair_result_as_y_x() {
        assert_eq!(parse_pair("320x200"), Ok((200, 320)));
    }
}
