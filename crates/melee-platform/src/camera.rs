//! Application-owned framing, independent of display cadence and simulation.
//! This fits living fighters with space for attacks; it is not retail cmCamera.
#[derive(Debug)]
pub struct Camera {
    center: [f32; 2],
    half_extent: [f32; 2],
}
impl Camera {
    pub fn frame(targets: &[Option<[f32; 2]>; 2], size: [u32; 2]) -> Self {
        let mut low = [f32::INFINITY; 2];
        let mut high = [f32::NEG_INFINITY; 2];
        for target in targets
            .iter()
            .flatten()
            .filter(|p| p.iter().all(|v| v.is_finite()))
        {
            for axis in 0..2 {
                low[axis] = low[axis].min(target[axis]);
                high[axis] = high[axis].max(target[axis]);
            }
        }
        if !low[0].is_finite() {
            low = [-60.0, 0.0];
            high = [60.0, 0.0];
        }
        let center = [(low[0] + high[0]) * 0.5, (low[1] + high[1]) * 0.5 + 12.0];
        let width = ((high[0] - low[0]) * 0.5 + 28.0).max(65.0);
        let height = ((high[1] - low[1]) * 0.5 + 32.0).max(42.0);
        let aspect = size[0].max(1) as f32 / size[1].max(1) as f32;
        let width = width.max(height * aspect);
        Self {
            center,
            half_extent: [width, width / aspect],
        }
    }
    pub fn uniform(&self) -> [f32; 4] {
        [
            self.half_extent[0],
            self.half_extent[1],
            self.center[0],
            self.center[1],
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fighters_remain_inside_landscape_and_portrait_frames() {
        for size in [[1280, 720], [640, 1200], [2560, 720]] {
            let targets = [Some([-160.0, -55.0]), Some([125.0, 140.0])];
            let c = Camera::frame(&targets, size);
            for p in targets.into_iter().flatten() {
                assert!((p[0] - c.center[0]).abs() < c.half_extent[0]);
                assert!((p[1] - c.center[1]).abs() < c.half_extent[1]);
            }
        }
    }
    #[test]
    fn eliminated_and_nonfinite_targets_do_not_poison_camera() {
        for targets in [
            [None, None],
            [Some([f32::NAN, 0.0]), None],
            [Some([10.0, 20.0]), None],
        ] {
            assert!(Camera::frame(&targets, [0, 0])
                .uniform()
                .iter()
                .all(|v| v.is_finite()));
        }
    }
}
