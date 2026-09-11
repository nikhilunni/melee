// Display-only approximation of HSD's common texture expression stages.
// Authored texture expressions and directional lighting are evaluated in display
// floats. Exact GX rounding remains separate from this display pipeline.
// CAMERA
@group(0) @binding(0) var<storage, read> poses: array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read> instances: array<mat4x4<f32>>;
@group(0) @binding(1) var<uniform> camera: Camera;
struct Layer { scale: vec4<f32>, translation: vec4<f32>, rotation: vec4<f32>, operations: vec4<u32>, color_operation:vec4<u32>, alpha_operation:vec4<u32>, color_inputs:vec4<u32>, alpha_inputs:vec4<u32>, constants:array<vec4<f32>,3>, activation:vec4<u32>, image:vec4<u32>, addressing:vec4<u32> }
struct Material { overlay:vec4<f32>, diffuse: vec4<f32>, ambient:vec4<f32>, specular:vec4<f32>, config: vec4<u32>, alpha: vec4<u32>, layers: array<Layer,8> }
@group(1) @binding(0) var<uniform> material: Material;
struct DirectionalLight { direction:vec4<f32>, color:vec4<f32> }
struct Lighting { ambient:vec4<f32>, lights:array<DirectionalLight,8> }
@group(0) @binding(3) var<uniform> lighting:Lighting;
fn safe_normalize(v:vec3<f32>)->vec3<f32> {
    return v/max(length(v),0.000001);
}
fn normal_transform(transform:mat4x4<f32>, normal:vec3<f32>)->vec3<f32> {
    let a=transform[0].xyz; let b=transform[1].xyz; let c=transform[2].xyz;
    let cofactor=mat3x3(cross(b,c),cross(c,a),cross(a,b));
    // Inverse transpose, with determinant magnitude removed by normalization.
    // Retain its sign for reflected transforms; singular poses produce zero.
    return safe_normalize(cofactor*normal*sign(dot(a,cross(b,c))));
}
fn specular_weight_for_view(normal:vec3<f32>, toward_light:vec3<f32>, shininess:f32, toward_eye:vec3<f32>)->f32 {
    if dot(normal,toward_light)<=0.0 { return 0.0; }
    let half_vector=safe_normalize(toward_light+toward_eye);
    let cosine=max(dot(normal,half_vector),0.0);
    let squared=cosine*cosine;
    let half_shininess=shininess*0.5;
    // HSD_LObjSetup uses GX's rational specular attenuation, not a pow lobe.
    return squared/max(half_shininess+(1.0-half_shininess)*squared,0.000001);
}
// Address each tap within the selected image, even when array layers have
// different sizes. Padding never leaks into repeat, mirror or linear filtering.
fn address_texel(value:i32, size:i32, mode:u32)->i32 {
    if mode==0u { return clamp(value,0,size-1); }
    let period=select(size,size*2,mode==2u);
    let wrapped=((value%period)+period)%period;
    return select(wrapped,period-1-wrapped,wrapped>=size);
}
fn image_texel(image:texture_2d_array<f32>, p:vec2<i32>, layer:Layer)->vec4<f32> {
    let at=vec2(address_texel(p.x,i32(layer.image.y),layer.addressing.x),
                address_texel(p.y,i32(layer.image.z),layer.addressing.y));
    return textureLoad(image,at,i32(layer.image.x),0);
}
fn sample_image(image:texture_2d_array<f32>, uv:vec2<f32>, layer:Layer)->vec4<f32> {
    let position=uv*vec2<f32>(layer.image.yz);
    if layer.addressing.z!=0u { return image_texel(image,vec2<i32>(floor(position)),layer); }
    let pixel=position-vec2(0.5); let base=vec2<i32>(floor(pixel)); let fraction=fract(pixel);
    return mix(mix(image_texel(image,base,layer),image_texel(image,base+vec2(1,0),layer),fraction.x),
               mix(image_texel(image,base+vec2(0,1),layer),image_texel(image,base+vec2(1,1),layer),fraction.x),fraction.y);
}
fn specular_weight(normal:vec3<f32>, toward_light:vec3<f32>, shininess:f32)->f32 {
    return specular_weight_for_view(normal,toward_light,shininess,vec3(0.0,0.0,1.0));
}
// TEXTURE_BINDINGS
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>, @location(2) normal: vec3<f32>,
    @location(3) diffuse_light:vec3<f32>, @location(4) specular_light:vec3<f32>
}
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) matrix: u32, @location(4) normal: vec3<f32>, @builtin(instance_index) instance: u32) -> Out {
    let transform=instances[instance]*poses[matrix];
    let p = transform * vec4(position, 1.0);
    var out: Out;
    out.position = project(p.xyz);
    out.uv = uv; out.color = color;
    out.normal = normal_transform(transform,normal);
    let surface_normal=out.normal;
    var diffuse_light=material.ambient.rgb*lighting.ambient.rgb;
    var specular_light=vec3(0.0);
    for (var i=0u;i<u32(lighting.ambient.w);i++) {
        let light=lighting.lights[i];
        let direction=safe_normalize(light.direction.xyz);
        diffuse_light+=light.color.rgb*max(dot(surface_normal,direction),0.0)*light.direction.w;
        specular_light+=light.color.rgb*specular_weight_for_view(surface_normal,direction,material.specular.w,safe_normalize(camera.eye.xyz-p.xyz))*light.color.w;
    }
    out.diffuse_light=clamp(diffuse_light,vec3(0.0),vec3(1.0));
    out.specular_light=specular_light;
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
    if (material.config.x&4u)!=0u { color=vec4(color.rgb*clamp(in.diffuse_light,vec3(0.0),vec3(1.0)),color.a); }
    if (material.config.x&8u)!=0u { color=vec4(clamp(color.rgb+material.specular.rgb*in.specular_light,vec3(0.0),vec3(1.0)),color.a); }
    color=vec4(mix(color.rgb,material.overlay.rgb,material.overlay.a),color.a);
    let a=alpha_compare(color.a,material.alpha.y,material.alpha.x);
    let b=alpha_compare(color.a,material.alpha.w,material.alpha.z);
    var visible=a && b;
    switch material.config.z { case 1u: { visible=a||b; } case 2u: { visible=a!=b; } case 3u: { visible=a==b; } default: {} }
    if !visible { discard; }
    // GX expressions operate on encoded colors; the sRGB target encodes again.
    let linear=select(color.rgb/12.92,pow((color.rgb+0.055)/1.055,vec3(2.4)),color.rgb>vec3(0.04045));
    return vec4(linear,color.a);
}
