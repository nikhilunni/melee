// CAMERA
@group(0) @binding(0) var<storage,read> poses:array<mat4x4<f32>>;
@group(0) @binding(1) var<uniform> camera:Camera;
@group(0) @binding(4) var<uniform> floors:array<vec4<f32>,2>;
struct Out { @builtin(position) position:vec4<f32>, @location(0) world:vec3<f32>, @location(1) fade:f32, @location(2) @interpolate(flat) owner:u32 }
@vertex fn vertex(@location(0) position:vec3<f32>, @location(3) matrix:u32, @builtin(instance_index) owner:u32)->Out {
    let original=(poses[matrix]*vec4(position,1.0)).xyz;
    let floor=floors[owner];
    let y=floor.y+(floor.w-floor.y)*(original.x-floor.x)/max(floor.z-floor.x,0.000001);
    var out:Out;
    out.world=vec3(original.x,y+0.03,original.z);
    out.position=project(out.world);
    out.fade=clamp(1.0-max(original.y-y,0.0)/120.0,0.0,1.0);
    out.owner=owner;
    return out;
}
@fragment fn fragment(in:Out)->@location(0) vec4<f32> {
    let floor=floors[in.owner];
    if in.world.x<floor.x || in.world.x>floor.z {discard;}
    return vec4(0.0,0.0,0.0,0.28*in.fade);
}
