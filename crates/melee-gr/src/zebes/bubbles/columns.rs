//! grZebes_801D881C's collision line over the chain: six vertices of
//! collision joint 0, spaced a fifth of the gap between the two bounds,
//! each as high as the held bubbles over it reach, then smoothed to slope
//! at most one column width per column. The ends are the two parts'
//! points (grZe_803B7FF0's upper and lower vertex).
use super::{BubbleState, Bubbles};
use gekko_math::{
    fma::{fmadd, fmadds},
    msl::fctiwz_double,
};

pub const VERTICES: usize = 6;
/// A column no bubble reaches stays this low (`@397`).
const UNREACHED: f32 = -9999.0;
/// A bubble covers the columns 0.9 to either side of its centre.
const HALF_COVER: f64 = 0.9;
/// Its top is its centre plus 1.8 times its size.
const TOP_SCALE: f64 = 1.8;

/// The vertices, or the first column whose x the C leaves to whatever
/// its stack held (no bubble reached it).
pub type Vertices = Result<[(f32, f32); VERTICES], usize>;

/// One column index: `(s32)(0.5 + (f64)(offset / width))`, kept in 0..=5.
fn column(offset: f32, width: f32) -> usize {
    // retail 0x801D8D68 fdivs, fadd (double 0.5), fctiwz on the double.
    let index = fctiwz_double(0.5 + f64::from(offset / width));
    index.clamp(0, VERTICES as i32 - 1) as usize
}

impl Bubbles {
    /// The vertices of joint 0 (mpVtxSetPos 0..=5), `upper` and `lower`
    /// the line's ends on maps 6 and 7.
    pub fn line(&self, upper: (f32, f32), lower: (f32, f32)) -> Vertices {
        let left = self.points.bounds[0].x;
        // retail 0x801D8D00: fdivs by 5.0f.
        let width = (self.points.bounds[1].x - left) / 5.0;
        let mut xs = [None; VERTICES];
        let mut heights = [UNREACHED; VERTICES];
        for bubble in &self.entries {
            if bubble.state != BubbleState::Held {
                continue;
            }
            let offset = bubble.x - left;
            // retail 0x801D8D50: fmadd (double), frsp.
            let top = fmadd(TOP_SCALE, f64::from(bubble.size), f64::from(bubble.y)) as f32;
            // retail 0x801D8D54 / 0x801D8D58: fsub / fadd in double, frsp.
            let near = (f64::from(offset) - HALF_COVER) as f32;
            let far = (HALF_COVER + f64::from(offset)) as f32;
            for side in [near, far] {
                let index = column(side, width);
                // retail 0x801D8DC0: fmadds.
                xs[index] = Some(fmadds(index as f32, width, left));
                if top > heights[index] {
                    heights[index] = top;
                }
            }
        }
        xs[0] = Some(upper.0);
        heights[0] = upper.1;
        xs[VERTICES - 1] = Some(lower.0);
        heights[VERTICES - 1] = lower.1;
        for index in 0..VERTICES - 1 {
            if index != 0 && heights[index] < heights[index + 1] - width {
                heights[index] = heights[index + 1] - width;
            }
            if heights[index + 1] < heights[index] - width {
                heights[index + 1] = heights[index] - width;
            }
        }
        let mut vertices = [(0.0, 0.0); VERTICES];
        for (index, vertex) in vertices.iter_mut().enumerate() {
            *vertex = (xs[index].ok_or(index)?, heights[index]);
        }
        Ok(vertices)
    }
}
