struct Sprite { previous:vec4<f32>, center:vec4<f32>, extent:vec4<f32>, uv:vec4<f32>, color:vec4<f32>, environment:vec4<f32>, flags:vec4<u32> }
// CAMERA
// PIXEL
@group(0) @binding(0) var<storage,read> sprites:array<Sprite>;
@group(0) @binding(1) var<uniform> camera:Camera;
@group(0) @binding(2) var atlas:texture_2d<f32>;
@group(0) @binding(3) var linear_sampler:sampler;
@group(0) @binding(4) var nearest_sampler:sampler;
struct Out { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32>, @location(1) local:vec2<f32>, @location(2) @interpolate(flat) instance:u32, @location(3) world:vec3<f32>, @location(4) alpha:f32 }
@vertex fn vertex(@builtin(vertex_index) vertex:u32,@builtin(instance_index) instance:u32)->Out {
    let corners=array<vec2<f32>,6>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(-1.0,1.0),vec2(1.0,-1.0),vec2(1.0,1.0));
    let sprite=sprites[instance];
    let local=corners[vertex];
    let scaled=local*sprite.extent.xy;
    let trail=(sprite.flags.x&(1u<<20u))!=0u;
    let point=(sprite.flags.x&(1u<<30u))!=0u;
    let oriented=(sprite.flags.x&(1u<<21u))!=0u;
    var angle=sprite.center.w;
    let center_clip=project(sprite.center.xyz);
    let previous_clip=project(sprite.previous.xyz);
    let direction=center_clip.xy/center_clip.w-previous_clip.xy/previous_clip.w;
    if (trail||oriented) && dot(direction,direction)>0.0000000001 {
        angle=atan2(direction.x*camera.viewport.x,-direction.y*camera.viewport.y);
        if oriented {angle+=sprite.center.w;}
    }
    let c=cos(angle); let s=sin(angle);
    let right=camera.right.xyz*c+camera.up.xyz*s;
    let up=camera.up.xyz*c-camera.right.xyz*s;
    var p=sprite.center.xyz+right*scaled.x+up*scaled.y;
    var out:Out;
    out.alpha=1.0;
    if trail {
        p=select(sprite.previous.xyz,sprite.center.xyz,local.x>0.0);
        let axis=select(up*sprite.extent.y,right*sprite.extent.x,local.x==local.y);
        p+=axis*local.y;
        out.alpha=select(sprite.previous.w,1.0,local.x>0.0);
    }
    out.position=project(p);
    if point {
        // GX's 1/6-pixel point/line width, scaled from the 480-line framebuffer.
        let width=floor(min(sprite.extent.x,42.5)*6.0)/6.0;
        var clip=center_clip;
        var offset=local*width/480.0;
        if trail {
            clip=select(previous_clip,center_clip,local.x>0.0);
            let pixels=direction*camera.viewport.xy;
            let normal=normalize(vec2(-pixels.y,pixels.x)+vec2(0.0000001,0.0));
            offset=normal*local.y*width/480.0;
        }
        clip.x+=offset.x*camera.viewport.y/camera.viewport.x*clip.w;
        clip.y+=offset.y*clip.w;
        out.position=clip;
    }
    out.world=p;
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
        let surface=project(in.world+camera.toward_eye.xyz*sqrt(1.0-r)*min(sprite.extent.x,sprite.extent.y));
        depth=surface.z/surface.w;
    } else {
        var tex=textureSampleLevel(atlas,linear_sampler,in.uv,0.0);
        if (sprite.flags.x&(1u<<9u))!=0u {tex=textureSampleLevel(atlas,nearest_sampler,in.uv,0.0);}
        color=vec4(mix(sprite.environment.rgb,sprite.color.rgb,tex.rgb),tex.a*sprite.color.a);
    }
    color.a*=in.alpha;
    let mode=sprite.flags.y;
    let a=alpha_compare(color.a,sprite.flags.z,(mode>>3u)&7u);
    let b=alpha_compare(color.a,sprite.flags.w,mode&7u);
    var visible=a&&b;
    switch (mode>>6u)&3u {case 1u:{visible=a||b;} case 2u:{visible=a!=b;} case 3u:{visible=a==b;} default:{}}
    if !visible {discard;}
    if color.a<=0.0 {discard;}
    let linear=select(color.rgb/12.92,pow((color.rgb+0.055)/1.055,vec3(2.4)),color.rgb>vec3(0.04045));
    var out:Fragment;
    out.color=vec4(linear,color.a);
    out.depth=depth;
    return out;
}
