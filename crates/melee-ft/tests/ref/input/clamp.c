static void HSD_PadClampCheck3(s8* x, s8* y, u8 shift, s8 min, s8 max)
{
    f32 r;

    r = sqrtf(((f32) *x * (f32) *x) + ((f32) *y * (f32) *y));

    if (r < min) {
        *y = 0;
        *x = 0;
        return;
    }
    if (r > max) {
        *x = ((f32) *x * (f32) max) / r;
        *y = ((f32) *y * (f32) max) / r;
        r = sqrtf(((f32) *x * (f32) *x) + ((f32) *y * (f32) *y));
    }

    if (shift == 1 && r > 1.000000013351432e-10f) {
        *x = (f32) *x - (((f32) *x * (f32) min) / r);
        *y = (f32) *y - (((f32) *y * (f32) min) / r);
    }
}
