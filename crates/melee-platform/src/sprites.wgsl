struct Sprite { center:vec4<f32>, extent:vec4<f32>, uv:vec4<f32>, color:vec4<f32>, environment:vec4<f32>, flags:vec4<u32> }
struct Camera { extent:vec4<f32> }
@group(0) @binding(0) var<storage,read> sprites:array<Sprite>;
@group(0) @binding(1) var<uniform> camera:Camera;
@group(0) @binding(2) var atlas:texture_2d<f32>;
@group(0) @binding(3) var linear_sampler:sampler;
@group(0) @binding(4) var nearest_sampler:sampler;
struct Out { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32>, @location(1) local:vec2<f32>, @location(2) @interpolate(flat) instance:u32 }
@vertex fn vertex(@builtin(vertex_index) vertex:u32,@builtin(instance_index) instance:u32)->Out {
    let corners=array<vec2<f32>,6>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(-1.0,1.0),vec2(1.0,-1.0),vec2(1.0,1.0));
    let sprite=sprites[instance];
    let local=corners[vertex];
    let scaled=local*sprite.extent.xy;
    let c=cos(sprite.center.w);
    let s=sin(sprite.center.w);
    let p=sprite.center.xy+vec2(c*scaled.x-s*scaled.y,s*scaled.x+c*scaled.y);
    var out:Out;
    out.position=vec4((p-camera.extent.zw)/camera.extent.xy,clamp((800.0-sprite.center.z)/1600.0,0.0,1.0),1.0);
    var uv=vec2(local.x,-local.y)*0.5+0.5;
    if (sprite.flags.x&(1u<<18u))!=0u {uv.x=1.0-uv.x;}
    if (sprite.flags.x&(1u<<19u))!=0u {uv.y=1.0-uv.y;}
    out.uv=mix(sprite.uv.xy,sprite.uv.zw,uv);
    out.local=local;
    out.instance=instance;
    return out;
}
struct Fragment { @location(0) color:vec4<f32>, @builtin(frag_depth) depth:f32 }
@fragment fn fragment(in:Out)->Fragment {
    let sprite=sprites[in.instance];
    var color:vec4<f32>;
    var depth=in.position.z;
    if sprite.extent.z==1.0 {
        let r=dot(in.local,in.local); if r>1.0 {discard;}
        // Procedural shield surface over the actual gameplay collision volume.
        let rim=smoothstep(0.35,1.0,r);
        color=vec4(mix(sprite.color.rgb,vec3(1.0),rim*0.45),sprite.color.a*(0.25+0.65*rim));
        // Shade the front hemisphere, rather than a plane through the fighter.
        depth=clamp(depth-sqrt(1.0-r)*min(sprite.extent.x,sprite.extent.y)/1600.0,0.0,1.0);
    } else {
        var tex=textureSampleLevel(atlas,linear_sampler,in.uv,0.0);
        if (sprite.flags.x&(1u<<9u))!=0u {tex=textureSampleLevel(atlas,nearest_sampler,in.uv,0.0);}
        color=vec4(mix(sprite.environment.rgb,sprite.color.rgb,tex.rgb),tex.a*sprite.color.a);
    }
    if color.a<=0.0 {discard;}
    let linear=select(color.rgb/12.92,pow((color.rgb+0.055)/1.055,vec3(2.4)),color.rgb>vec3(0.04045));
    var out:Fragment;
    out.color=vec4(linear,color.a);
    out.depth=depth;
    return out;
}
