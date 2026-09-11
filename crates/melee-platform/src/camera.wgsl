// Shared by meshes, billboards and volume depth calculations.
struct Camera {
    projection:mat4x4<f32>, right:vec4<f32>, up:vec4<f32>,
    toward_eye:vec4<f32>, eye:vec4<f32>
}
fn project(world:vec3<f32>)->vec4<f32> { return camera.projection*vec4(world-camera.eye.xyz,1.0); }
