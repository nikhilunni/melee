//! The view the renderer draws: the retail main camera the simulation already
//! computes (`Presentation::view_camera`), independent of display cadence.
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

/// The retail main camera with its vertical field of view kept and the
/// target's aspect, so a wider window shows more to the sides. The
/// projection is applied to `world - eye`: the camera basis (`C_MTXLookAt`
/// rows), then `MTXPerspective` mapped to wgpu's 0..1 depth range.
pub fn retail(view: &melee_lib::presentation::ViewCamera, size: [u32; 2]) -> Uniform {
    let aspect = size[0].max(1) as f32 / size[1].max(1) as f32;
    let sy = 1.0 / (view.fov.to_radians() * 0.5).tan();
    let sx = sy / aspect;
    let depth = view.far / (view.far - view.near);
    let column = |j: usize| {
        [
            sx * view.right[j],
            sy * view.up[j],
            -depth * view.toward_eye[j],
            -view.toward_eye[j],
        ]
    };
    let w = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
    Uniform {
        viewport: [size[0].max(1) as f32, size[1].max(1) as f32, 0.0, 0.0],
        projection: [
            column(0),
            column(1),
            column(2),
            [0.0, 0.0, -depth * view.near, 0.0],
        ],
        right: w(view.right),
        up: w(view.up),
        toward_eye: w(view.toward_eye),
        eye: [view.eye[0], view.eye[1], view.eye[2], 1.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_lib::presentation::ViewCamera;

    /// cm_803BCB64's default view: eye above and in front of the interest.
    fn default_view() -> ViewCamera {
        let toward_eye = {
            let d = [0.0f32, 30.241_425, 300.241];
            let n = (d[1] * d[1] + d[2] * d[2]).sqrt();
            [0.0, d[1] / n, d[2] / n]
        };
        ViewCamera {
            eye: [0.0, 40.241_425, 300.241],
            right: [1.0, 0.0, 0.0],
            up: [0.0, toward_eye[2], -toward_eye[1]],
            toward_eye,
            fov: 30.0,
            near: 0.1,
            far: 16384.0,
            aspect: 1.2,
        }
    }
    fn project(camera: &Uniform, point: [f32; 3]) -> [f32; 3] {
        let p = [
            point[0] - camera.eye[0],
            point[1] - camera.eye[1],
            point[2] - camera.eye[2],
            1.0,
        ];
        let clip: [f32; 4] =
            std::array::from_fn(|row| (0..4).map(|col| camera.projection[col][row] * p[col]).sum());
        [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]]
    }
    #[test]
    fn retail_view_centers_the_interest_and_maps_near_and_far_depth() {
        let view = default_view();
        for size in [[1280, 720], [640, 1200], [0, 0]] {
            let camera = retail(&view, size);
            let interest = project(&camera, [0.0, 10.0, 0.0]);
            assert!(interest[0].abs() < 1e-4 && interest[1].abs() < 1e-4);
            for (distance, depth) in [(0.1, 0.0), (16384.0, 1.0)] {
                let point = std::array::from_fn(|i| view.eye[i] - view.toward_eye[i] * distance);
                assert!((project(&camera, point)[2] - depth).abs() < 0.001);
            }
        }
    }
    #[test]
    fn wider_targets_keep_the_vertical_field_of_view() {
        let view = default_view();
        let wide = retail(&view, [1920, 1080]);
        let tall = retail(&view, [1080, 1920]);
        assert_eq!(wide.projection[1][1], tall.projection[1][1]);
        assert!(wide.projection[0][0] < tall.projection[0][0]);
    }
}
