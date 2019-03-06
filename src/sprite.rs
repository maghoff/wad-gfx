use byteorder::{ByteOrder, LittleEndian};
use ndarray::prelude::*;
use num_rational::Rational32;

#[derive(Debug)]
pub struct Span<'a> {
    pub top: u16,
    pub pixels: &'a [u8],
}

pub struct Column<'a> {
    data: &'a [u8],
}

impl<'a> Column<'a> {
    fn new(data: &[u8]) -> Column {
        Column { data }
    }
}

impl<'a> Iterator for Column<'a> {
    type Item = Span<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let top = self.data[0] as u16;
        if top == 255 {
            return None;
        }

        let count = self.data[1];
        let _dummy = self.data[2]; // Unknown. Use the source?
        let pixels = &self.data[3..3 + count as usize];
        let _dummy2 = self.data[3 + count as usize]; // Unknown. Use the source?

        assert_eq!(pixels.len(), count as usize);

        self.data = &self.data[4 + count as usize..];

        Some(Span { top, pixels })
    }
}

pub struct Sprite<'a> {
    width: u16,
    height: u16,
    left: i16,
    top: i16,
    column_array: &'a [[u8; 4]],
    data_offset: usize,
    data: &'a [u8],
}

impl<'a> Sprite<'a> {
    pub fn new(data: &[u8]) -> Sprite {
        assert!(data.len() >= 8);
        let width = LittleEndian::read_u16(&data[0..2]);
        let height = LittleEndian::read_u16(&data[2..4]);
        let left = LittleEndian::read_i16(&data[4..6]);
        let top = LittleEndian::read_i16(&data[6..8]);

        let column_array_start = 8;
        let column_array_byte_size = width as usize * 4;
        let column_array_end = column_array_start + column_array_byte_size;
        assert!(data.len() >= column_array_end);

        // The following transmute is safe because:
        //  * [u8; 4] does not have alignment constraints
        //  * The slice has been verified to be large enough
        let column_array: &[[u8; 4]] = unsafe {
            std::slice::from_raw_parts(
                data[column_array_start..].as_ptr() as *const _,
                width as usize,
            )
        };

        Sprite {
            width,
            height,
            left,
            top,
            column_array,
            data_offset: column_array_end,
            data: &data[column_array_end..],
        }
    }

    pub fn col(&'a self, i: u32) -> Column<'a> {
        let start =
            LittleEndian::read_u32(&self.column_array[i as usize]) as usize - self.data_offset;
        let end = self
            .column_array
            .get(i as usize + 1)
            .map(|x| LittleEndian::read_u32(x) as usize - self.data_offset)
            .unwrap_or(self.data.len());

        Column::new(&self.data[start..end])
    }

    pub fn origin(&self) -> (i16, i16) {
        (self.top, self.left)
    }

    pub fn pixel_aspect_ratio(&self) -> Rational32 {
        Rational32::new(320, 200) / Rational32::new(4, 3)
    }

    pub fn dim(&self) -> (u32, u32) {
        (self.height as _, self.width as _)
    }

    pub fn draw_column(&self, col: u32, mut target: ArrayViewMut1<u8>, scale: Rational32) {
        for span in self.col(col) {
            let top = (Rational32::from(span.top as i32) * scale)
                .ceil()
                .to_integer();
            let bottom =
                (Rational32::from(span.top as i32 + span.pixels.len() as i32) * scale).to_integer();

            for y in top..bottom {
                let scaled_y = Rational32::from(y) / scale;
                let scaled_y = (scaled_y.to_integer()) as usize - span.top as usize;

                target[y as usize] = span.pixels[scaled_y];
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dimensions() {
        let sprite = Sprite::new(include_bytes!("trooa1.sprite"));
        assert_eq!(sprite.dim(), (57, 41));
    }

    #[test]
    fn column() {
        let sprite = Sprite::new(include_bytes!("trooa1.sprite"));
        assert_eq!(sprite.col(6).count(), 3);
    }

    #[test]
    fn all_columns_can_be_iterated() {
        let sprite = Sprite::new(include_bytes!("trooa1.sprite"));
        for i in 0..sprite.dim().1 {
            sprite.col(i).for_each(|_| ());
        }
    }

    #[test]
    fn can_draw() {
        let sprite = Sprite::new(include_bytes!("trooa1.sprite"));
        let scale = sprite.pixel_aspect_ratio() * Rational32::from(4);
        let mut target: Array1<u8> =
            Array1::zeros((Rational32::from(sprite.dim().0 as i32) * scale).to_integer() as usize);
        for i in 0..sprite.dim().1 {
            sprite.draw_column(i, target.view_mut(), scale);
        }
    }
}
