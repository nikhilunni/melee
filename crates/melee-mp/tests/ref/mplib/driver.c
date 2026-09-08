#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "gekko_fma.h"

#define ABS(x) ((x) < 0 ? -(x) : (x))
#ifdef MP_REF_UNFUSED
#include "geom.c"
#else
#include "retail/geom.c"
#endif

/* Native-endian records: ten input floats; hit flag and two output bits.
 * Miss coordinates are zero, so even the entire miss record is comparable. */
int main(int argc, char** argv)
{
    if (argc != 4) return 2;
    int op = atoi(argv[1]);
    if (op < 0 || op > 3) return 2;
    FILE* input = fopen(argv[2], "rb");
    FILE* output = fopen(argv[3], "wb");
    if (!input || !output) return 3;
    float a[10];
    size_t n;
    while ((n = fread(a, sizeof(float), 10, input)) == 10) {
        float x = 0, y = 0;
        bool hit = true;
        switch (op) {
        case 0:
            mpRemap2d(&x, &y, a[0], a[1], a[2], a[3], a[4], a[5],
                      a[6], a[7], a[8], a[9]);
            break;
        case 1:
            hit = mpLineIntersection(a[0], a[1], a[2], a[3], a[4], a[5],
                                     a[6], a[7], &x, &y);
            break;
        case 2:
            hit = mpLineIntersectionH(&x, &y, a[0], a[1], a[2], a[4],
                                      a[5], a[6], a[7]);
            break;
        case 3:
            hit = mpLineIntersectionV(&x, &y, a[0], a[1], a[3], a[4],
                                      a[5], a[6], a[7]);
            break;
        }
        uint32_t record[3] = {hit, 0, 0};
        if (hit) {
            memcpy(&record[1], &x, 4);
            memcpy(&record[2], &y, 4);
        }
        if (fwrite(record, sizeof(record), 1, output) != 1) return 4;
    }
    if (n || ferror(input)) return 5;
    int result = fclose(output);
    fclose(input);
    return result != 0;
}
