/* Native oracle. clamp.c / scale.c / circle.c are exact function excerpts;
 * only circle_retail.c changes the three audited single FMA sites.
 * Build -ffp-contract=off: clamp's sums are deliberately NOT contracted. */
#include <stdint.h>
#include <stdio.h>
#include <math.h>
#include "gekko_estimate.h"
#include "gekko_fma.h"
typedef int8_t s8;
typedef uint8_t u8;
typedef int32_t s32;
typedef float f32;
typedef struct {
    s8 stickX, stickY, subStickX, subStickY;
    u8 analogL, analogR, analogA, analogB;
    float nml_stickX, nml_stickY, nml_subStickX, nml_subStickY;
    float nml_analogL, nml_analogR, nml_analogA, nml_analogB;
} HSD_PadStatus;
typedef struct { s8 scale_stick; u8 scale_analogLR, scale_analogAB; } PadLibData;
static PadLibData HSD_PadLibData = {80, 140, 255};
static float retail_sqrt(float x) {
    if (x > 0.0f) {
        double guess = gekko_frsqrte(x);
        /* HSD_PadClampCheck3: 80376F20/30/40 and 803770BC/CC/DC.
         * lb_8000D148: 8000D1A0/B0/C0. Same MSL sqrt Newton sequence. */
        guess = (0.5 * guess) * gekko_fnmsub(x, guess * guess, 3.0);
        guess = (0.5 * guess) * gekko_fnmsub(x, guess * guess, 3.0);
        guess = (0.5 * guess) * gekko_fnmsub(x, guess * guess, 3.0);
        return (float)(x * guess);
    }
    return x;
}
#define sqrtf retail_sqrt
#include "clamp.c"
#include "scale.c"
#include "circle_retail.c"
int main(int argc, char **argv) {
    if (argc != 3) return 2;
    FILE *in = fopen(argv[1], "rb"), *out = fopen(argv[2], "wb");
    if (!in || !out) return 3;
    float threshold;
    if (fread(&threshold, sizeof threshold, 1, in) != 1) return 4;
    for (int x = -128; x <= 127; ++x) {
        for (int y = -128; y <= 127; ++y) {
            HSD_PadStatus p = {0};
            p.stickX = x; p.stickY = y;
            HSD_PadClampCheck3(&p.stickX, &p.stickY, 1, 0, 80);
            HSD_PadScale(&p);
            float values[4] = {p.nml_stickX, p.nml_stickY,
                fabsf(p.nml_stickX) <= threshold ? 0.0f : p.nml_stickX,
                fabsf(p.nml_stickY) <= threshold ? 0.0f : p.nml_stickY};
            if (fwrite(values, sizeof values, 1, out) != 1) return 5;
        }
    }
    float points[5];
    while (fread(points, sizeof points, 1, in) == 1) {
        s32 result = lb_8000D148(points[0], points[1], points[2], points[3], 0, 0, points[4]);
        if (fwrite(&result, sizeof result, 1, out) != 1) return 6;
    }
    fclose(in); return fclose(out) != 0;
}
