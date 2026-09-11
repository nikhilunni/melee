//! Application-owned framing, independent of display cadence and simulation.
//! Perspective uses the retail default field of view and viewing direction;
//! fitting living fighters remains application policy, independent of frame rate.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub projection: [[f32; 4]; 4],
    pub right: [f32; 4],
    pub up: [f32; 4],
    pub toward_eye: [f32; 4],
    pub eye: [f32; 4],
    pub viewport: [f32; 4],
}
#[derive(Debug)]
pub struct Camera {
    size: [u32; 2],
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
        let height = ((high[1] - low[1]) * 0.5 + 32.0).max(80.0);
        let aspect = size[0].max(1) as f32 / size[1].max(1) as f32;
        let width = width.max(height * aspect);
        Self {
            size,
            center,
            half_extent: [width, width / aspect],
        }
    }
    pub fn uniform(&self) -> Uniform {
        // cm_803BCB64: 30-degree vertical FOV, near 0.1, far 16384.
        // Direction is normalized from cm_803BCB3C - cm_803BCB50.
        // These are display constants, not a port of cmCamera's tracking math.
        const SIN: f32 = 0.100216754;
        const COS: f32 = 0.9949656;
        const TAN_HALF_FOV: f32 = 0.2679492;
        const NEAR: f32 = 0.1;
        const FAR: f32 = 16384.0;
        let distance = self.half_extent[1] / TAN_HALF_FOV;
        let sx = distance / self.half_extent[0];
        let sy = distance / self.half_extent[1];
        let depth = FAR / (FAR - NEAR);
        let [x, y] = self.center;
        Uniform {
            viewport: [
                self.size[0].max(1) as f32,
                self.size[1].max(1) as f32,
                0.0,
                0.0,
            ],
            projection: [
                [sx, 0.0, 0.0, 0.0],
                [0.0, sy * COS, -depth * SIN, -SIN],
                [0.0, -sy * SIN, -depth * COS, -COS],
                [0.0, 0.0, -depth * NEAR, 0.0],
            ],
            right: [1.0, 0.0, 0.0, 0.0],
            up: [0.0, COS, -SIN, 0.0],
            toward_eye: [0.0, SIN, COS, 0.0],
            eye: [x, y + SIN * distance, COS * distance, 1.0],
        }
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
    fn perspective_keeps_targets_visible_and_maps_near_and_far_depth() {
        let project = |camera: Uniform, point: [f32; 3]| -> [f32; 3] {
            let p = [
                point[0] - camera.eye[0],
                point[1] - camera.eye[1],
                point[2] - camera.eye[2],
                1.0,
            ];
            let clip: [f32; 4] = std::array::from_fn(|row| {
                (0..4).map(|col| camera.projection[col][row] * p[col]).sum()
            });
            [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]]
        };
        for size in [[1280, 720], [640, 1200], [2560, 720]] {
            let targets = [Some([-160.0, -55.0]), Some([125.0, 140.0])];
            let camera = Camera::frame(&targets, size).uniform();
            for p in targets.into_iter().flatten() {
                let clip = project(camera, [p[0], p[1], 0.0]);
                assert!(clip[0].abs() < 1.0 && clip[1].abs() < 1.0);
                assert!((0.0..1.0).contains(&clip[2]));
            }
            for (distance, depth) in [(0.1, 0.0), (16384.0, 1.0)] {
                let point =
                    std::array::from_fn(|i| camera.eye[i] - camera.toward_eye[i] * distance);
                assert!(
                    (project(camera, point)[2] - depth).abs() < 0.001,
                    "size {size:?}, distance {distance}, depth {}",
                    project(camera, point)[2]
                );
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
                .projection
                .iter()
                .flatten()
                .all(|v| v.is_finite()));
        }
    }
}
