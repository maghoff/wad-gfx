use ndarray::prelude::*;
use ndarray::ShapeError;

pub struct Flat<'a> {
    pixels: ArrayView2<'a, u8>,
}

impl<'a> Flat<'a> {
    pub fn new(pixels: &[u8]) -> Result<Flat, ShapeError> {
        Ok(Flat {
            pixels: ArrayView2::from_shape((64, 64), pixels)?,
        })
    }

    pub fn view(&self) -> ArrayView2<'a, u8> {
        self.pixels
    }
}
