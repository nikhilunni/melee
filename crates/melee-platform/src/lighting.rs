//! Fixed-capacity GPU lighting data; all animation comes from presentation.
use melee_lib::presentation::Presentation;
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Directional {
    direction: [f32; 4],
    color: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Lighting {
    ambient: [f32; 4],
    lights: [Directional; 8],
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
        }
        result
    }
}
