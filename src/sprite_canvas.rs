use crate::rangetools::*;
use crate::Sprite;
use byteorder::{LittleEndian, WriteBytesExt};
use ndarray::prelude::*;
use ndarray::s;
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
                assert!(span_len <= 128, "Span dimensions exceed what's encodeable");
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

    pub fn into_planes_col_major(self) -> (Array2<u8>, Array2<bool>) {
        (self.pixels, self.mask)
    }

    pub fn into_planes_row_major(&self) -> (Array2<u8>, Array2<bool>) {
        let pixels = self.pixels.t().iter().cloned().collect::<Vec<_>>();
        let mask = self.mask.t().iter().cloned().collect::<Vec<_>>();
        let shape = (self.height() as usize, self.width() as usize);
        (
            Array2::from_shape_vec(shape, pixels).unwrap(),
            Array2::from_shape_vec(shape, mask).unwrap(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip() {
        let first_sprite = Sprite::new(include_bytes!("trooa1.sprite"));
        let mut canvas =
            SpriteCanvas::new(first_sprite.width() as u16, first_sprite.height() as u16);
        canvas.draw_patch(first_sprite.left(), first_sprite.top(), &first_sprite);

        let rendered = canvas.make_sprite();
        let (first_pixels, first_mask) = canvas.into_planes_col_major();

        let second_sprite = Sprite::new(&rendered);
        let mut canvas =
            SpriteCanvas::new(second_sprite.width() as u16, second_sprite.height() as u16);
        canvas.draw_patch(second_sprite.left(), second_sprite.top(), &second_sprite);

        let (second_pixels, second_mask) = canvas.into_planes_col_major();

        assert_eq!(first_sprite.dim(), second_sprite.dim());
        assert_eq!(&first_pixels, &second_pixels);
        assert_eq!(&first_mask, &second_mask);
    }

    #[test]
    fn transpose() {
        let sprite = Sprite::new(include_bytes!("trooa1.sprite"));
        let mut canvas = SpriteCanvas::new(sprite.width() as u16, sprite.height() as u16);
        canvas.draw_patch(sprite.left(), sprite.top(), &sprite);
        let (pixels, mask) = canvas.into_planes_row_major();

        let expected_pixels = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 71, 68, 68, 71, 72, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 72, 70, 69, 69, 69, 69, 70, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 69, 69, 70, 70, 70, 70, 69, 69, 72,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 72, 65, 68, 71, 73, 73, 71, 68, 65, 69, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 71, 66, 64, 65, 67, 70, 70, 67, 65, 64,
            66, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 70, 74, 76, 70, 68, 66, 66, 68, 70, 76, 74, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 73, 187, 175, 78, 77, 76,
            78, 175, 187, 73, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 70, 73, 190, 76, 73, 73, 76, 190, 73, 70, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 69, 74, 73,
            69, 69, 73, 74, 69, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 69, 72, 67, 79, 96, 168, 89, 96, 79, 67, 72, 69, 71, 0, 0,
            0, 0, 0, 0, 0, 95, 89, 99, 0, 0, 0, 0, 0, 0, 96, 90, 0, 0, 0, 0, 0, 0, 0, 0, 68, 69,
            70, 72, 68, 77, 185, 79, 79, 185, 77, 68, 72, 70, 69, 69, 72, 0, 0, 0, 99, 90, 92, 99,
            0, 0, 0, 0, 0, 0, 0, 0, 96, 92, 96, 99, 0, 0, 70, 68, 67, 68, 70, 73, 75, 69, 76, 185,
            188, 188, 185, 76, 69, 75, 73, 70, 68, 67, 68, 69, 90, 81, 90, 99, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 93, 90, 81, 70, 68, 67, 69, 70, 74, 76, 77, 70, 75, 185, 184, 184, 185, 75,
            70, 77, 76, 74, 70, 69, 67, 68, 69, 70, 99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 98,
            90, 72, 70, 68, 67, 69, 72, 74, 76, 71, 74, 96, 168, 89, 96, 74, 71, 76, 74, 72, 69,
            67, 68, 69, 70, 71, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 73, 73, 72, 69, 68,
            69, 70, 73, 75, 73, 75, 63, 61, 61, 63, 75, 73, 74, 71, 70, 68, 68, 69, 70, 71, 72, 73,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 74, 75, 74, 71, 70, 71, 71, 71, 77, 75, 72,
            74, 75, 75, 74, 72, 75, 73, 70, 69, 69, 69, 70, 72, 73, 73, 74, 76, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 77, 76, 76, 77, 73, 73, 73, 96, 88, 74, 77, 75, 74, 73, 73, 74, 75,
            76, 72, 70, 90, 81, 70, 73, 74, 76, 74, 75, 76, 81, 90, 95, 0, 0, 0, 0, 0, 96, 96, 90,
            81, 76, 77, 78, 78, 76, 74, 74, 74, 77, 75, 76, 77, 76, 78, 79, 78, 76, 74, 72, 71, 72,
            77, 73, 75, 76, 77, 76, 76, 76, 76, 76, 0, 0, 0, 0, 0, 0, 0, 75, 75, 74, 75, 76, 78,
            79, 78, 76, 75, 75, 77, 75, 75, 76, 77, 77, 79, 77, 76, 75, 74, 73, 74, 77, 75, 76, 77,
            78, 78, 77, 77, 77, 78, 0, 0, 0, 0, 0, 0, 0, 75, 74, 74, 76, 77, 79, 79, 79, 78, 77,
            76, 77, 75, 74, 75, 77, 78, 78, 78, 76, 75, 75, 74, 76, 77, 77, 78, 78, 79, 77, 76, 76,
            76, 77, 78, 0, 0, 0, 0, 0, 0, 75, 75, 76, 77, 79, 79, 79, 76, 77, 78, 77, 77, 77, 77,
            78, 78, 78, 78, 78, 78, 77, 77, 77, 77, 77, 77, 76, 78, 79, 76, 75, 74, 75, 76, 78, 0,
            0, 0, 0, 0, 75, 76, 77, 78, 79, 79, 78, 77, 78, 76, 77, 77, 76, 75, 74, 75, 77, 78, 76,
            78, 77, 75, 74, 75, 76, 77, 78, 77, 79, 79, 76, 74, 73, 74, 76, 78, 0, 0, 0, 0, 0, 75,
            77, 78, 79, 79, 77, 76, 0, 0, 78, 77, 76, 75, 76, 77, 76, 74, 75, 79, 75, 74, 76, 77,
            76, 75, 76, 78, 78, 0, 0, 78, 75, 74, 75, 77, 79, 0, 0, 90, 81, 75, 74, 73, 76, 78, 79,
            76, 0, 0, 0, 0, 77, 76, 76, 78, 77, 75, 75, 77, 79, 77, 75, 75, 77, 78, 76, 76, 77, 0,
            0, 0, 0, 105, 77, 77, 79, 79, 77, 0, 0, 0, 75, 73, 73, 75, 77, 78, 76, 0, 0, 0, 0, 0,
            76, 78, 79, 79, 79, 78, 77, 77, 77, 78, 78, 78, 78, 78, 76, 0, 0, 0, 0, 0, 0, 78, 78,
            78, 76, 76, 0, 0, 0, 74, 72, 73, 75, 77, 78, 0, 0, 0, 0, 0, 0, 77, 79, 78, 77, 75, 73,
            75, 78, 74, 73, 75, 77, 78, 78, 77, 0, 0, 0, 0, 0, 0, 79, 79, 78, 75, 74, 75, 0, 0, 73,
            71, 74, 75, 77, 78, 0, 0, 0, 0, 0, 0, 76, 77, 78, 78, 76, 76, 77, 78, 77, 76, 76, 78,
            78, 77, 76, 0, 0, 0, 0, 0, 0, 79, 78, 77, 74, 73, 73, 0, 0, 72, 70, 75, 76, 77, 0, 0,
            0, 0, 0, 0, 0, 75, 76, 77, 78, 78, 77, 77, 76, 77, 77, 78, 77, 77, 76, 75, 0, 0, 0, 0,
            0, 0, 78, 77, 76, 74, 72, 73, 0, 0, 72, 71, 75, 77, 78, 0, 0, 0, 0, 0, 0, 77, 75, 76,
            76, 75, 74, 73, 74, 75, 73, 72, 73, 75, 76, 76, 75, 76, 0, 0, 0, 0, 0, 78, 76, 76, 73,
            72, 100, 0, 0, 72, 72, 75, 77, 0, 0, 0, 0, 0, 0, 0, 77, 75, 76, 77, 76, 74, 73, 75, 77,
            75, 73, 75, 76, 77, 77, 75, 76, 0, 0, 0, 0, 0, 77, 76, 75, 73, 72, 0, 0, 0, 72, 72, 75,
            78, 0, 0, 0, 0, 0, 0, 0, 76, 74, 75, 76, 77, 75, 75, 76, 78, 76, 75, 76, 77, 77, 76,
            75, 76, 0, 0, 0, 0, 0, 77, 75, 74, 73, 74, 0, 0, 0, 73, 73, 75, 78, 0, 0, 0, 0, 0, 0,
            0, 76, 73, 74, 75, 76, 77, 76, 75, 75, 75, 76, 77, 77, 76, 75, 74, 75, 76, 0, 0, 0, 73,
            71, 71, 73, 72, 0, 0, 0, 0, 73, 74, 73, 76, 0, 0, 0, 0, 0, 0, 76, 75, 73, 73, 74, 75,
            76, 77, 76, 76, 76, 77, 77, 76, 75, 74, 74, 75, 76, 0, 0, 75, 76, 73, 73, 73, 72, 0, 0,
            0, 0, 75, 74, 72, 72, 77, 0, 0, 0, 0, 0, 77, 75, 73, 72, 73, 74, 76, 78, 77, 77, 77,
            77, 77, 76, 75, 73, 73, 74, 76, 77, 0, 77, 77, 76, 74, 73, 0, 0, 0, 0, 0, 75, 73, 73,
            76, 77, 0, 0, 0, 0, 0, 78, 75, 73, 71, 72, 73, 75, 78, 79, 79, 79, 77, 76, 75, 74, 72,
            73, 74, 76, 77, 0, 78, 78, 77, 74, 75, 0, 0, 0, 0, 0, 74, 75, 77, 78, 79, 0, 0, 0, 0,
            0, 78, 76, 73, 70, 71, 72, 75, 78, 79, 79, 79, 77, 76, 74, 73, 72, 73, 75, 76, 77, 0,
            0, 79, 79, 79, 0, 0, 0, 0, 0, 0, 0, 79, 79, 79, 0, 0, 0, 0, 0, 0, 78, 76, 73, 71, 70,
            72, 76, 78, 77, 0, 0, 78, 76, 74, 73, 72, 74, 75, 76, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 78, 75, 74, 72, 70, 73, 76, 78, 76, 0, 0, 78, 77, 75,
            73, 73, 74, 76, 77, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            77, 73, 75, 74, 73, 74, 77, 77, 75, 0, 0, 78, 78, 76, 73, 74, 75, 76, 78, 77, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 77, 73, 74, 75, 74, 76, 76, 76,
            75, 0, 0, 77, 78, 77, 75, 75, 76, 77, 78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 77, 75, 73, 74, 77, 77, 77, 75, 75, 0, 0, 76, 77, 78, 77, 76, 77,
            78, 78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 76, 75,
            76, 78, 78, 77, 75, 75, 0, 0, 76, 77, 78, 78, 77, 77, 78, 77, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 78, 76, 77, 79, 79, 78, 77, 76, 0, 0, 0,
            77, 78, 76, 76, 77, 78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 78, 79, 76, 75, 76, 78, 77, 0, 0, 0, 78, 78, 75, 83, 76, 78, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 77, 79, 74, 74, 74, 78, 78,
            0, 0, 0, 78, 78, 77, 90, 77, 78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 77, 74, 83, 74, 78, 78, 0, 0, 0, 78, 78, 79, 96, 78, 78, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 77, 77, 82, 75,
            77, 77, 75, 0, 0, 77, 78, 79, 79, 78, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 77, 77, 78, 78, 77, 76, 74, 0, 0, 78, 78, 79, 79, 78,
            77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 76,
            78, 78, 78, 77, 75, 74, 0, 0, 78, 78, 77, 77, 78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 78, 78, 78, 77, 75, 75, 0, 0, 78, 77,
            76, 76, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 78, 77, 76, 76, 76, 0, 0, 0, 77, 76, 76, 76, 76, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 76, 74, 73, 75, 0, 0, 0, 77,
            76, 75, 76, 75, 76, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 75, 73, 72, 74, 0, 0, 0, 76, 77, 76, 0, 76, 76, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 75, 73, 72, 72, 74, 76, 0,
            0, 76, 90, 77, 0, 90, 76, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 73, 73, 72, 73, 74, 74, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 73, 72, 72, 76, 0, 74, 72, 72,
            75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 73, 89, 77, 77, 0, 76, 89, 76, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];
        let expected_mask = [
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, true, true, true, true,
            true, true, true, true, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, true,
            true, true, true, true, true, true, true, true, true, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, true, true, true, true, true, true, true, true,
            true, true, true, true, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, true, true, true, true, true,
            true, true, true, true, true, true, true, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, true,
            true, true, true, true, true, true, true, true, true, true, true, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, true, true, true, true, true, true, true,
            true, true, true, false, false, false, false, false, false, false, false, false, false,
            false, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, true, true, true, true, true,
            true, true, true, true, true, true, true, true, false, false, false, false, false,
            false, false, true, true, true, false, false, false, false, false, false, true, true,
            false, false, false, false, false, false, false, false, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, false, false,
            false, true, true, true, true, false, false, false, false, false, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, false, false, false, false, false, false, false, false, false,
            false, false, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, false, false, false, false, false, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, false, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, false, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, false, false, false, false, false, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, true, false, false, false, false, false, true,
            true, true, true, true, true, true, false, false, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, false,
            false, true, true, true, true, true, true, false, false, true, true, true, true, true,
            true, true, true, true, false, false, false, false, true, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, false, false, false,
            false, true, true, true, true, true, true, false, false, false, true, true, true, true,
            true, true, true, false, false, false, false, false, true, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, false, false, false, false,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, true, true, false, false, false, false, false, false, true, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, false, false, false,
            false, false, false, true, true, true, true, true, true, false, false, true, true,
            true, true, true, true, false, false, false, false, false, false, true, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, false, false,
            false, false, false, false, true, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, false, false, false, false, false, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, true, true, true, true, true, true, false, false,
            true, true, true, true, true, false, false, false, false, false, false, true, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, false, false, false, false, false, true, true, true, true, true, true, false,
            false, true, true, true, true, false, false, false, false, false, false, false, true,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, false, false, false, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, false, false, false, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false, false, false, false, false,
            false, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, false, false, false, true, true, true, true, true, false,
            false, false, false, true, true, true, true, false, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, false, false, true, true, true, true, true, true, false,
            false, false, false, true, true, true, true, true, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, false, true, true, true, true, true, false, false,
            false, false, false, true, true, true, true, true, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, false, true, true, true, true, true, false, false,
            false, false, false, true, true, true, true, true, false, false, false, false, false,
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true, true, true, true, true, false, false, true, true, true, false, false,
            false, false, false, false, false, true, true, true, false, false, false, false, false,
            false, true, true, true, true, true, true, true, true, true, false, false, true, true,
            true, true, true, true, true, true, true, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, true, true, true, false,
            false, true, true, true, true, true, true, true, true, true, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, true, true, true, true, true, true, true,
            true, true, false, false, true, true, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, true, true, true, true, true,
            true, true, true, true, false, false, true, true, true, true, true, true, true, true,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, true, true, true,
            true, true, true, true, true, true, false, false, true, true, true, true, true, true,
            true, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, true, true, true, true, true, true, true, true, false, false, true, true, true,
            true, true, true, true, true, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, true, true, false, false,
            false, true, true, true, true, true, true, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, true, true, true, true, true, true,
            true, false, false, false, true, true, true, true, true, true, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, true, true, true,
            true, true, true, true, false, false, false, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, true, true, true, true, true, true, false, false, false, true, true, true, true,
            true, true, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, true, true, true, true, true, true, true, false, false,
            true, true, true, true, true, true, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, true, true, true, true, true, true,
            true, false, false, true, true, true, true, true, true, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, true, true, true,
            true, true, true, true, false, false, true, true, true, true, true, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, true, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, true, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, true, true, true, true,
            false, false, false, true, true, true, true, true, true, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            true, true, true, true, false, false, false, true, true, true, false, true, true,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true, true, true, true, true, true, false, false, true, true,
            true, false, true, true, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, true, true, true, true, true, true, true, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, true, true, true, true, false,
            true, true, true, true, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, true,
            true, true, true, false, true, true, true, true, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false,
        ];

        assert_eq!(pixels.len(), expected_pixels.len());
        assert!(pixels
            .iter()
            .zip(expected_pixels.iter())
            .all(|(a, b)| a == b));

        assert_eq!(mask.len(), expected_mask.len());
        assert!(mask.iter().zip(expected_mask.iter()).all(|(a, b)| a == b));
    }
}
