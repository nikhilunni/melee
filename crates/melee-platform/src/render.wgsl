// Display-only approximation of HSD's common texture expression stages.
// Authored texture expressions and directional lighting are evaluated in display
// floats. Exact GX rounding remains separate from this display pipeline.
// CAMERA
@group(0) @binding(0) var<storage, read> poses: array<mat4x4<f32>>;
@group(0) @binding(2) var<storage, read> instances: array<mat4x4<f32>>;
@group(0) @binding(1) var<uniform> camera: Camera;
struct Layer { scale: vec4<f32>, translation: vec4<f32>, rotation: vec4<f32>, operations: vec4<u32>, color_operation:vec4<u32>, alpha_operation:vec4<u32>, color_inputs:vec4<u32>, alpha_inputs:vec4<u32>, constants:array<vec4<f32>,3>, activation:vec4<u32>, image:vec4<u32>, addressing:vec4<u32>, lod:vec4<f32> }
struct Material { overlay:vec4<f32>, diffuse: vec4<f32>, ambient:vec4<f32>, specular:vec4<f32>, config: vec4<u32>, alpha: vec4<u32>, layers: array<Layer,8> }
@group(1) @binding(0) var<uniform> material: Material;
struct DirectionalLight { direction:vec4<f32>, color:vec4<f32>, attenuation:vec4<f32> }
struct Lighting { ambient:vec4<f32>, lights:array<DirectionalLight,8>, fog_color:vec4<f32>, fog_range:vec4<f32> }
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
// HSD_JObjMakePositionMtx (0x803740E8) / mkBillBoardMtx: preserve scale and the
// projected up axis, while replacing the facing axis only for authored billboards.
fn billboard_transform(transform:mat4x4<f32>, mode:u32, eye:vec3<f32>, toward_eye:vec3<f32>)->mat4x4<f32> {
    if mode==0u {return transform;}
    let original_up=transform[1].xyz;
    var right=cross(original_up,toward_eye);
    var up=cross(toward_eye,right);
    var forward=toward_eye;
    if mode==2u {
        let position=transform[3].xyz-eye;
        right=cross(position,original_up);
        up=cross(right,position);
        forward=-safe_normalize(position);
    }
    return mat4x4(vec4(safe_normalize(right)*length(transform[0].xyz),0.0),
        vec4(safe_normalize(up)*length(original_up),0.0),
        vec4(forward*length(transform[2].xyz),0.0),transform[3]);
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
// different sizes. Padding never leaks into repeat, mirror or linear filter_modeing.
fn address_texel(value:i32, size:i32, mode:u32)->i32 {
    if mode==0u { return clamp(value,0,size-1); }
    let period=select(size,size*2,mode==2u);
    let wrapped=((value%period)+period)%period;
    return select(wrapped,period-1-wrapped,wrapped>=size);
}
fn image_texel(image:texture_2d_array<f32>, p:vec2<i32>, layer:Layer, level:u32)->vec4<f32> {
    let size=max(layer.image.yz>>vec2(level),vec2(1u));
    let at=vec2(address_texel(p.x,i32(size.x),layer.addressing.x),
                address_texel(p.y,i32(size.y),layer.addressing.y));
    return textureLoad(image,at,i32(layer.image.x),i32(level));
}
fn sample_level(image:texture_2d_array<f32>, uv:vec2<f32>, layer:Layer, level:u32, nearest:bool)->vec4<f32> {
    let position=uv*vec2<f32>(max(layer.image.yz>>vec2(level),vec2(1u)));
    if nearest { return image_texel(image,vec2<i32>(floor(position)),layer,level); }
    let pixel=position-vec2(0.5); let base=vec2<i32>(floor(pixel)); let fraction=fract(pixel);
    return mix(mix(image_texel(image,base,layer,level),image_texel(image,base+vec2(1,0),layer,level),fraction.x),
               mix(image_texel(image,base+vec2(0,1),layer,level),image_texel(image,base+vec2(1,1),layer,level),fraction.x),fraction.y);
}
fn sample_lod(image:texture_2d_array<f32>, uv:vec2<f32>, layer:Layer, lod:f32)->vec4<f32> {
    if lod<=0.0 {return sample_level(image,uv,layer,0u,layer.addressing.z!=0u);}
    let filter_mode=layer.addressing.w;
    let nearest=(filter_mode==0u || filter_mode==2u || filter_mode==4u);
    if filter_mode<2u {return sample_level(image,uv,layer,0u,nearest);}
    let level=clamp(lod,layer.lod.y,min(layer.lod.z,f32(layer.image.w)));
    if filter_mode<4u {return sample_level(image,uv,layer,u32(floor(level+0.5)),nearest);}
    let lower=u32(floor(level));
    return mix(sample_level(image,uv,layer,lower,nearest),
        sample_level(image,uv,layer,min(lower+1u,layer.image.w),nearest),fract(level));
}
fn sample_image(image:texture_2d_array<f32>, uv:vec2<f32>, layer:Layer)->vec4<f32> {
    let dx=dpdx(uv)*vec2<f32>(layer.image.yz);
    let dy=dpdy(uv)*vec2<f32>(layer.image.yz);
    let xx=dot(dx,dx); let yy=dot(dy,dy);
    let major=select(dy,dx,xx>=yy);
    let taps=max(1u,u32(layer.lod.w));
    let footprint=max(min(xx,yy),max(xx,yy)/f32(taps*taps));
    let lod=0.5*log2(max(footprint,0.000001))+layer.lod.x;
    var color=vec4(0.0);
    for(var i=0u;i<taps;i+=1u) {
        let offset=(f32(i)+0.5)/f32(taps)-0.5;
        color+=sample_lod(image,uv+offset*major/vec2<f32>(layer.image.yz),layer,lod);
    }
    return color/f32(taps);
}
fn specular_weight(normal:vec3<f32>, toward_light:vec3<f32>, shininess:f32)->f32 {
    return specular_weight_for_view(normal,toward_light,shininess,vec3(0.0,0.0,1.0));
}
// TEXTURE_BINDINGS
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) color: vec4<f32>, @location(2) normal: vec3<f32>,
    @location(3) diffuse_light:vec3<f32>, @location(4) specular_light:vec3<f32>,
    @location(5) eye_depth:f32
}
@vertex fn vertex(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) matrix: u32, @location(4) normal: vec3<f32>, @location(5) billboard: u32, @builtin(instance_index) instance: u32) -> Out {
    let transform=billboard_transform(instances[instance]*poses[matrix],billboard,camera.eye.xyz,camera.toward_eye.xyz);
    let p = transform * vec4(position, 1.0);
    var out: Out;
    out.position = project(p.xyz);
    out.uv = uv; out.color = color;
    out.eye_depth = dot(camera.eye.xyz-p.xyz,camera.toward_eye.xyz);
    out.normal = normal_transform(transform,normal);
    let surface_normal=out.normal;
    var diffuse_light=material.ambient.rgb*lighting.ambient.rgb;
    var specular_light=vec3(0.0);
    for (var i=0u;i<u32(lighting.ambient.w);i++) {
        let light=lighting.lights[i];
        var direction=safe_normalize(light.direction.xyz);
        var attenuation=1.0;
        if light.attenuation.w!=0.0 {
            // GX point light, spot off (setup_point_lightobj): toward the
            // light from the vertex, 1/(k0+k1 d+k2 d^2) distance attenuation.
            let toward=light.direction.xyz-p.xyz;
            let d=length(toward);
            direction=safe_normalize(toward);
            attenuation=1.0/max(light.attenuation.x+light.attenuation.y*d+light.attenuation.z*d*d,0.000001);
        }
        diffuse_light+=light.color.rgb*max(dot(surface_normal,direction),0.0)*light.direction.w*attenuation;
        specular_light+=light.color.rgb*specular_weight_for_view(surface_normal,direction,material.specular.w,safe_normalize(camera.eye.xyz-p.xyz))*light.color.w*attenuation;
    }
    out.diffuse_light=clamp(diffuse_light,vec3(0.0),vec3(1.0));
    out.specular_light=specular_light;
    return out;
}
fn coordinates(in: Out, layer: Layer) -> vec2<f32> {
    var uv = in.uv;
    if layer.operations.z == 1u {
        // HSD reflection uses the view-space normal, including camera tilt.
        uv=vec2(dot(in.normal,camera.right.xyz),-dot(in.normal,camera.up.xyz))*0.5+0.5;
    }
    if layer.operations.z == 2u {
        if lighting.ambient.w==0.0 {return vec2(1.0);}
        // TObjSetupMtx's HILIGHT matrix uses its first column only.
        let half_vector=safe_normalize(safe_normalize(lighting.lights[0].direction.xyz)+camera.toward_eye.xyz);
        uv=vec2(dot(in.normal,half_vector)*0.5+0.5,0.0);
    }
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
    return combine_values(previous,tex,layer.operations,layer.translation.z);
}
fn combine_values(previous:vec4<f32>,tex:vec4<f32>,operations:vec4<u32>,blending:f32)->vec4<f32> {
    var color=previous;
    switch operations.x {
        case 1u: { color=vec4(mix(previous.rgb,tex.rgb,tex.a),color.a); }
        case 2u: { color=vec4(mix(previous.rgb,tex.rgb,tex.rgb),color.a); }
        case 3u: { color=vec4(mix(previous.rgb,tex.rgb,blending),color.a); }
        case 4u: { color=vec4(previous.rgb*tex.rgb,color.a); }
        case 5u: { color=vec4(tex.rgb,color.a); }
        case 7u: { color=vec4(previous.rgb+tex.rgb,color.a); }
        case 8u: { color=vec4(previous.rgb-tex.rgb,color.a); }
        default: {}
    }
    if operations.w != 0u {
        switch operations.y {
            case 1u: { color.a=mix(previous.a,tex.a,tex.a); }
            case 2u: { color.a=mix(previous.a,tex.a,blending); }
            case 3u: { color.a=previous.a*tex.a; }
            case 4u: { color.a=tex.a; }
            case 6u: { color.a=previous.a+tex.a; }
            case 7u: { color.a=previous.a-tex.a; }
            default: {}
        }
    }
    return clamp(color,vec4(0.0),vec4(1.0));
}
// Keep the per-fragment composition state compact: image banks, texture
// matrices and custom TEV constants belong to sampling, not these stages.
struct Composition {
    specular:vec3<f32>, mode:u32, count:u32,
    operations:array<vec4<u32>,8>, parameters:array<vec2<f32>,8>
}
// MObjMakeTExp (0x80363284): diffuse/ambient textures -> raster lighting -> specular
// textures and raster -> EXT textures. Alpha runs once per texture across
// categories, but every texture within the first category contributes.
fn texture_stage(color:vec4<f32>, mat:Composition, texels:array<vec4<f32>,8>, category:u32, done:u32)->vec4<f32> {
    var result=color;
    for(var i=0u;i<mat.count;i++) {
        let mask=u32(mat.parameters[i].y);
        if (mask&category)==0u {continue;}
        var operations=mat.operations[i];
        operations.w=select(1u,0u,(mask&done)!=0u);
        result=combine_values(result,texels[i],operations,mat.parameters[i].x);
    }
    return result;
}
fn compose_material(base:vec4<f32>, mat:Composition, texels:array<vec4<f32>,8>, diffuse_light:vec3<f32>, specular_light:vec3<f32>)->vec4<f32> {
    var color=texture_stage(base,mat,texels,0x50u,0u);
    if (mat.mode&4u)!=0u {color=vec4(color.rgb*clamp(diffuse_light,vec3(0.0),vec3(1.0)),color.a);}
    var done=0x50u;
    if (mat.mode&8u)!=0u {
        let spec=texture_stage(vec4(mat.specular,color.a),mat,texels,0x20u,done);
        color=vec4(clamp(color.rgb+clamp(spec.rgb*specular_light,vec3(0.0),vec3(1.0)),vec3(0.0),vec3(1.0)),spec.a);
        done|=0x20u;
    }
    return texture_stage(color,mat,texels,0x80u,done);
}
// PIXEL
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> {
    var color=material.diffuse;
    let diffuse=material.config.x & 3u;
    if diffuse==2u { color=vec4(in.color.rgb,color.a); }
    if diffuse==3u { color=vec4(color.rgb*in.color.rgb,color.a); }
    var alpha=(material.config.x>>13u)&3u;
    if alpha==0u { alpha=diffuse; }
    if alpha==2u { color.a=in.color.a; }
    if alpha==3u { color.a*=in.color.a; }
    var texels:array<vec4<f32>,8>;
    var composition:Composition;
    composition.specular=material.specular.rgb;
    composition.mode=material.config.x;
    composition.count=material.config.y;
    // TEXTURE_SAMPLES
    color=compose_material(color, composition, texels, in.diffuse_light, in.specular_light);
    color=vec4(mix(color.rgb,material.overlay.rgb,material.overlay.a),color.a);
    let a=alpha_compare(color.a,material.alpha.y,material.alpha.x);
    let b=alpha_compare(color.a,material.alpha.w,material.alpha.z);
    var visible=a && b;
    switch material.config.z { case 1u: { visible=a||b; } case 2u: { visible=a!=b; } case 3u: { visible=a==b; } default: {} }
    if !visible { discard; }
    // GX_FOG_PERSP_LIN after the TEV and alpha test: eye depth from start to end.
    if lighting.fog_color.w!=0.0 {
        let amount=clamp((in.eye_depth-lighting.fog_range.x)/max(lighting.fog_range.y-lighting.fog_range.x,0.000001),0.0,1.0);
        color=vec4(mix(color.rgb,lighting.fog_color.rgb,amount),color.a);
    }
    // GX expressions operate on encoded colors; the sRGB target encodes again.
    let linear=select(color.rgb/12.92,pow((color.rgb+0.055)/1.055,vec3(2.4)),color.rgb>vec3(0.04045));
    return vec4(linear,color.a);
}
