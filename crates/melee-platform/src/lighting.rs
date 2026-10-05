//! Fixed-capacity GPU lighting data; all animation comes from presentation.
use melee_lib::presentation::Presentation;
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Directional {
    direction: [f32; 4],
    color: [f32; 4],
    /// Point lights: k0, k1, k2 of GX distance attenuation; w = 1.
    attenuation: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Lighting {
    ambient: [f32; 4],
    lights: [Directional; 8],
    /// Encoded fog color; w = 1 while fog is on.
    fog_color: [f32; 4],
    /// Start and end eye depth.
    fog_range: [f32; 4],
}
impl Lighting {
    pub fn capture(scene: &Presentation) -> Self {
        let a = scene.ambient_light();
        let mut result = Self {
            ambient: [a[0], a[1], a[2], scene.directional_lights().len() as f32],
            ..Default::default()
        };
        for (out, light) in result.lights.iter_mut().zip(scene.directional_lights()) {
            out.direction = [
                light.direction[0],
                light.direction[1],
                light.direction[2],
                if light.diffuse { 1.0 } else { 0.0 },
            ];
            out.color = [
                light.color[0],
                light.color[1],
                light.color[2],
                if light.specular { 1.0 } else { 0.0 },
            ];
            out.attenuation = match light.distance_attenuation {
                Some([k0, k1, k2]) => [k0, k1, k2, 1.0],
                None => [1.0, 0.0, 0.0, 0.0],
            };
        }
        if let Some(fog) = scene.fog() {
            let c = fog.color.map(|v| f32::from(v) / 255.0);
            result.fog_color = [c[0], c[1], c[2], 1.0];
            result.fog_range = [fog.start, fog.end, 0.0, 0.0];
        }
        result
    }
}
