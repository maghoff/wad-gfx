use super::Sprite;
use byteorder::{LittleEndian, WriteBytesExt};
use ndarray::prelude::*;
use ndarray::s;
use rangetools::*;
use std::ops::Range;

pub struct SpriteCanvas {
    pixels: Array2<u8>,
    mask: Array2<bool>,
}

fn find_spans(buf: &[bool]) -> Vec<Range<i32>> {
    let mut spans = vec![];

    let mut i = 0;
    while i < buf.len() {
        while i < buf.len() && !buf[i] {
            i += 1;
        }
        if i == buf.len() {
            break;
        }
        let span_start = i as _;
        while i < buf.len() && buf[i] {
            i += 1;
        }
        let span_end = i as _;
        spans.push(span_start..span_end);
    }

    spans
}

impl SpriteCanvas {
    pub fn new(width: u16, height: u16) -> SpriteCanvas {
        let dim = (width as usize, height as usize);

        SpriteCanvas {
            pixels: ndarray::Array2::zeros(dim),
            mask: ndarray::Array2::default(dim),
        }
    }

    pub fn width(&self) -> u16 {
        self.pixels.dim().0 as _
    }

    pub fn height(&self) -> u16 {
        self.pixels.dim().1 as _
    }

    pub fn draw_patch(&mut self, pos_x: i16, pos_y: i16, sprite: &Sprite) {
        let (top, left) = sprite.origin();
        let origin = (left as i32, top as i32); // Flip xy

        // Position sprite origin at given coordinates
        let offset = (pos_x as i32 - origin.0, pos_y as i32 - origin.1);

        let x_range = 0..sprite.width() as i32; // Sprite dimension
        let x_range = add(x_range, offset.0); // Position on canvas
        let x_range = intersect(x_range, 0..self.width() as i32); // Clip to canvas

        for x in x_range {
            for span in sprite.col((x - offset.0) as _) {
                let y_offset = offset.1 + span.top as i32;

                let span_range = 0..span.pixels.len() as i32;
                let span_range = add(span_range, y_offset);
                let span_range = intersect(span_range, 0..self.height() as i32);

                for y in span_range {
                    self.pixels[[x as usize, y as usize]] = span.pixels[(y - y_offset) as usize];
                    self.mask[[x as usize, y as usize]] = true;
                }
            }
        }
    }

    pub fn make_sprite(&self) -> Vec<u8> {
        let mut column_array: Vec<u32> = vec![];
        let mut data: Vec<u8> = vec![];

        for x in 0..self.width() {
            column_array.push(data.len() as u32);

            for span in find_spans(self.mask.slice(s![x as usize, ..]).as_slice().unwrap()) {
                let span_len = span.end - span.start;
                assert!(span_len < 128, "Span dimensions exceed what's encodeable");
                data.push(span.start as u8);
                data.push(span_len as u8);
                data.push(span_len as u8);
                data.extend(self.pixels.slice(s![x as usize, span]));
                data.push(0);
            }
            data.push(255);
        }

        let mut out = vec![];

        out.write_u16::<LittleEndian>(self.width()).unwrap();
        out.write_u16::<LittleEndian>(self.height()).unwrap();
        out.write_u16::<LittleEndian>(0).unwrap(); // left
        out.write_u16::<LittleEndian>(0).unwrap(); // top

        let data_start = (8 /* header size */ + 4 * column_array.len()) as u32;
        for col in column_array {
            out.write_u32::<LittleEndian>(col + data_start).unwrap();
        }

        out.extend(data);

        out
    }

    pub fn into_planes(self) -> (Array2<u8>, Array2<bool>) {
        (self.pixels, self.mask)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip() {
        let original_data = include_bytes!("trooa1.sprite");

        let first_sprite = Sprite::new(original_data);
        let mut canvas =
            SpriteCanvas::new(first_sprite.width() as u16, first_sprite.height() as u16);
        canvas.draw_patch(first_sprite.left(), first_sprite.top(), &first_sprite);

        let rendered = canvas.make_sprite();
        let (first_pixels, first_mask) = canvas.into_planes();

        let second_sprite = Sprite::new(&rendered);
        let mut canvas =
            SpriteCanvas::new(second_sprite.width() as u16, second_sprite.height() as u16);
        canvas.draw_patch(second_sprite.left(), second_sprite.top(), &second_sprite);

        let (second_pixels, second_mask) = canvas.into_planes();

        assert_eq!(first_sprite.dim(), second_sprite.dim());
        assert_eq!(&first_pixels, &second_pixels);
        assert_eq!(&first_mask, &second_mask);
    }
}
