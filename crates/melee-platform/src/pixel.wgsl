fn alpha_compare(value:f32, reference:u32, operation:u32)->bool {
    let a=u32(round(clamp(value,0.0,1.0)*255.0));
    switch operation {
        case 0u: { return false; } case 1u: { return a<reference; }
        case 2u: { return a==reference; } case 3u: { return a<=reference; }
        case 4u: { return a>reference; } case 5u: { return a!=reference; }
        case 6u: { return a>=reference; } default: { return true; }
    }
}
