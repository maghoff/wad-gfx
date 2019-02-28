use ndarray::prelude::*;
use ndarray::ShapeError;
use num_rational::Rational32;

pub struct Flat<'a> {
    pixels: ArrayView2<'a, u8>,
}

impl<'a> Flat<'a> {
    pub fn new(pixels: &[u8]) -> Result<Flat, ShapeError> {
        Ok(Flat {
            pixels: ArrayView2::from_shape((64, 64), pixels)?,
        })
    }
}

impl<'a> crate::Gfx for Flat<'a> {
    fn pixel_aspect_ratio(&self) -> Rational32 {
        Rational32::new(1, 1)
    }

    fn dim(&self) -> (u32, u32) {
        (64, 64)
    }

    fn draw_column(&self, col: u32, mut target: ArrayViewMut1<u8>, scale: Rational32) {
        let scaled_height = Rational32::from(self.pixels.dim().0 as i32) * scale;
        let scaled_height = scaled_height.to_integer();

        for y in 0..scaled_height {
            let scaled_y = Rational32::from(y) / scale;
            let scaled_y = scaled_y.to_integer() as usize;

            target[[y as usize]] = self.pixels[[scaled_y, col as usize]];
        }
    }
}
