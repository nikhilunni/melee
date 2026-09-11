// HSD MakeColorGenTExp: the authored texture expression precedes lightmap
// combination. Arithmetic is GPU display math; fixed-point GX rounding is not
// claimed here. Register selectors refer to TObj constants, never another draw.
fn color_input(selector:u32, tex:vec4<f32>, layer:Layer)->vec3<f32> {
    switch selector {
        case 8u: { return tex.rgb; } case 9u: { return vec3(tex.a); }
        case 12u: { return vec3(1.0); } case 13u: { return vec3(0.5); }
        case 128u: { return layer.constants[0].rgb; }
        case 129u: { return vec3(layer.constants[0].r); }
        case 130u: { return vec3(layer.constants[0].g); }
        case 131u: { return vec3(layer.constants[0].b); }
        case 132u: { return vec3(layer.constants[0].a); }
        case 133u: { return layer.constants[1].rgb; }
        case 134u: { return vec3(layer.constants[1].a); }
        case 135u: { return layer.constants[2].rgb; }
        case 136u: { return vec3(layer.constants[2].a); }
        default: { return vec3(0.0); }
    }
}
fn alpha_input(selector:u32, tex:vec4<f32>, layer:Layer)->f32 {
    switch selector {
        case 4u: { return tex.a; }
        case 64u: { return layer.constants[0].r; }
        case 65u: { return layer.constants[0].g; }
        case 66u: { return layer.constants[0].b; }
        case 67u: { return layer.constants[0].a; }
        case 68u: { return layer.constants[1].a; }
        case 69u: { return layer.constants[2].a; }
        default: { return 0.0; }
    }
}
fn packed_rgb(rgb:vec3<f32>, components:u32)->u32 {
    let bytes=vec3<u32>(round(clamp(rgb,vec3(0.0),vec3(1.0))*255.0));
    if components==0u { return bytes.r; }
    if components==1u { return (bytes.g<<8u)|bytes.r; }
    return (bytes.b<<16u)|(bytes.g<<8u)|bytes.r;
}
fn tev_operation(a:vec3<f32>,b:vec3<f32>,c:vec3<f32>,d:vec3<f32>,op:vec4<u32>)->vec3<f32> {
    var result=d;
    if op.x<8u {
        let interpolated=mix(a,b,c);
        result+=select(interpolated,-interpolated,op.x==1u);
        if op.y==1u { result+=0.5; }
        if op.y==2u { result-=0.5; }
        let scales=array<f32,4>(1.0,2.0,4.0,0.5);
        result*=scales[op.z];
    } else {
        var matches=vec3<bool>(false);
        if op.x<14u {
            let components=(op.x-8u)/2u;
            let av=packed_rgb(a,components); let bv=packed_rgb(b,components);
            matches=vec3(select(av>bv,av==bv,(op.x&1u)!=0u));
        } else {
            let av=round(a*255.0); let bv=round(b*255.0);
            matches=select(av>bv,av==bv,vec3((op.x&1u)!=0u));
        }
        result+=select(vec3(0.0),c,matches);
    }
    if op.w!=0u { return clamp(result,vec3(0.0),vec3(1.0)); }
    return clamp(result,vec3(-1024.0/255.0),vec3(1023.0/255.0));
}
fn custom_texture(tex:vec4<f32>,layer:Layer)->vec4<f32> {
    var result=tex;
    if (layer.activation.x&0x40000000u)!=0u {
        let selectors=layer.color_inputs;
        result=vec4(tev_operation(color_input(selectors.x,tex,layer),color_input(selectors.y,tex,layer),color_input(selectors.z,tex,layer),color_input(selectors.w,tex,layer),layer.color_operation),result.a);
    }
    if (layer.activation.x&0x80000000u)!=0u {
        let selectors=layer.alpha_inputs;
        result.a=tev_operation(vec3(alpha_input(selectors.x,tex,layer)),vec3(alpha_input(selectors.y,tex,layer)),vec3(alpha_input(selectors.z,tex,layer)),vec3(alpha_input(selectors.w,tex,layer)),layer.alpha_operation).x;
    }
    return result;
}
