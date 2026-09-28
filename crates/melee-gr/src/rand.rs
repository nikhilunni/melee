//! Random helpers shared by stage controllers.
use gekko_math::HsdRng;

/// The retail "random between" inline (grpstadium.c `randi_between`,
/// groldpupupu.c): equal endpoints return without a draw, and reversed
/// endpoints draw over the same span. The top is exclusive.
pub fn range(rng: &mut HsdRng, [a, b]: [i32; 2]) -> i32 {
    if a == b {
        a
    } else if a < b {
        a + rng.randi(b - a)
    } else {
        b + rng.randi(a - b)
    }
}
