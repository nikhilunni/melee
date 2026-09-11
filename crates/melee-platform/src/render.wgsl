// Display-only approximation of HSD's common texture expression stages.
// Authored texture expressions are supported; lighting and material animation
// remain separate from these display calculations.
struct Camera { extent: vec4<f32> }
@group(0) @binding(0) var<storage, read> poses: array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read> instances: array<mat4x4<f32>>;
@group(0) @binding(1) var<uniform> camera: Camera;
struct Layer { scale: vec4<f32>, translation: vec4<f32>, rotation: vec4<f32>, operations: vec4<u32>, color_operation:vec4<u32>, alpha_operation:vec4<u32>, color_inputs:vec4<u32>, alpha_inputs:vec4<u32>, constants:array<vec4<f32>,3>, activation:vec4<u32> }
struct Material { diffuse: vec4<f32>, config: vec4<u32>, alpha: vec4<u32>, layers: array<Layer,8> }
@group(1) @binding(0) var<uniform> material: Material;
// TEXTURE_BINDINGS
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>, @location(2) normal: vec3<f32>
}
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) matrix: u32, @location(4) normal: vec3<f32>, @builtin(instance_index) instance: u32) -> Out {
    let transform=instances[instance]*poses[matrix];
    let p = transform * vec4(position, 1.0);
    var out: Out;
    out.position = vec4((p.x-camera.extent.z) / camera.extent.x, (p.y-camera.extent.w) / camera.extent.y, clamp((800.0-p.z)/1600.0, 0.0, 1.0), 1.0);
    out.uv = uv; out.color = color;
    let transformed = (transform * vec4(normal,0.0)).xyz;
    out.normal = transformed / max(length(transformed), 0.000001);
    return out;
}
fn coordinates(in: Out, layer: Layer) -> vec2<f32> {
    var uv = in.uv;
    if layer.operations.z == 1u || layer.operations.z == 2u { uv = vec2(in.normal.x, -in.normal.y) * 0.5 + 0.5; }
    var scale=vec2<f32>(0.0);
    if abs(layer.scale.x) >= 0.00000011920929 { scale.x=layer.scale.z/layer.scale.x; }
    if abs(layer.scale.y) >= 0.00000011920929 { scale.y=layer.scale.w/layer.scale.y; }
    var translation = layer.translation.xy;
    if layer.translation.w != 0.0 && scale.y != 0.0 { translation.y += 1.0 / scale.y; }
    var p=vec3(uv-translation,0.0);
    let c=cos(layer.rotation.xyz); let s=sin(layer.rotation.xyz);
    p=vec3(p.x,c.x*p.y-s.x*p.z,s.x*p.y+c.x*p.z);
    p=vec3(c.y*p.x+s.y*p.z,p.y,-s.y*p.x+c.y*p.z);
    // HSD texture Z rotation has the opposite sign to joint rotation.
    return vec2(c.z*p.x+s.z*p.y,-s.z*p.x+c.z*p.y)*scale;
}
// CUSTOM_COMBINERS
fn combine(previous:vec4<f32>,tex:vec4<f32>,layer:Layer)->vec4<f32> {
    var color=previous;
    switch layer.operations.x {
        case 1u: { color=vec4(mix(previous.rgb,tex.rgb,tex.a),color.a); }
        case 2u: { color=vec4(mix(previous.rgb,tex.rgb,tex.rgb),color.a); }
        case 3u: { color=vec4(mix(previous.rgb,tex.rgb,layer.translation.z),color.a); }
        case 4u: { color=vec4(previous.rgb*tex.rgb,color.a); }
        case 5u: { color=vec4(tex.rgb,color.a); }
        case 7u: { color=vec4(previous.rgb+tex.rgb,color.a); }
        case 8u: { color=vec4(previous.rgb-tex.rgb,color.a); }
        default: {}
    }
    if layer.operations.w != 0u {
        switch layer.operations.y {
            case 1u: { color.a=mix(previous.a,tex.a,tex.a); }
            case 2u: { color.a=mix(previous.a,tex.a,layer.translation.z); }
            case 3u: { color.a=previous.a*tex.a; }
            case 4u: { color.a=tex.a; }
            case 6u: { color.a=previous.a+tex.a; }
            case 7u: { color.a=previous.a-tex.a; }
            default: {}
        }
    }
    return clamp(color,vec4(0.0),vec4(1.0));
}
fn alpha_compare(value:f32, reference:u32, operation:u32)->bool {
    let a=u32(round(clamp(value,0.0,1.0)*255.0));
    switch operation {
        case 0u: { return false; } case 1u: { return a<reference; }
        case 2u: { return a==reference; } case 3u: { return a<=reference; }
        case 4u: { return a>reference; } case 5u: { return a!=reference; }
        case 6u: { return a>=reference; } default: { return true; }
    }
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> {
    var color=material.diffuse;
    let diffuse=material.config.x & 3u;
    if diffuse==2u { color=vec4(in.color.rgb,color.a); }
    if diffuse==3u { color=vec4(color.rgb*in.color.rgb,color.a); }
    var alpha=(material.config.x>>13u)&3u;
    if alpha==0u { alpha=diffuse; }
    if alpha==2u { color.a=in.color.a; }
    if alpha==3u { color.a*=in.color.a; }
    // TEXTURE_SAMPLES
    let a=alpha_compare(color.a,material.alpha.y,material.alpha.x);
    let b=alpha_compare(color.a,material.alpha.w,material.alpha.z);
    var visible=a && b;
    switch material.config.z { case 1u: { visible=a||b; } case 2u: { visible=a!=b; } case 3u: { visible=a==b; } default: {} }
    if !visible { discard; }
    // GX expressions operate on encoded colors; the sRGB target encodes again.
    let linear=select(color.rgb/12.92,pow((color.rgb+0.055)/1.055,vec3(2.4)),color.rgb>vec3(0.04045));
    return vec4(linear,color.a);
}
