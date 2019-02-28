mod flat;
mod sprite;

pub use flat::*;
pub use sprite::*;

use ndarray::prelude::*;
use num_rational::Rational32;

pub trait Gfx {
    fn pixel_aspect_ratio(&self) -> Rational32;

    fn dim(&self) -> (u32, u32);

    fn draw_column(&self, col: u32, target: ArrayViewMut1<u8>, scale: Rational32);
}
