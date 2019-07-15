// Perhaps there is room for a rangetools crate in the world?
// Reddit research: https://www.reddit.com/r/rust/comments/aynxgl/is_there_a_rangetools_crate/

use std::cmp::{max, min};
use std::ops::Range;

pub fn add(r: Range<i32>, d: i32) -> Range<i32> {
    (r.start + d)..(r.end + d)
}

pub fn intersect(a: Range<i32>, b: Range<i32>) -> Range<i32> {
    max(a.start, b.start)..min(a.end, b.end)
}
