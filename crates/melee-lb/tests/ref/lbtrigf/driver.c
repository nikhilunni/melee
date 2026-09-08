/*
 * Reference-oracle driver. Built by tests/ref_oracle.rs with
 *   cc -std=c99 -O0 -ffp-contract=off -fno-builtin -fno-strict-aliasing
 *      -fwrapv -I shim driver.c -lm
 *
 * It #includes the two decomp sources directly (rather than linking them as
 * separate translation units) so the static `lb_sqrtf` and the static
 * `atanf_lookup` table are reachable for comparison.
 *
 * Usage: driver <op> <in-file> <out-file>
 *
 *   op            input record            output record
 *   atanf         u32 (f32 bits)          u32 (f32 bits)
 *   acosf         u32                     u32
 *   asinf         u32                     u32
 *   lb_sqrtf      u32                     u32
 *   expf          u32                     u32
 *   atan2f        u32 y, u32 x            u32
 *   powf          u32 base, u32 exp       u32
 *   atanf_lookup  (none)                  46 x u32
 *
 * All records are host-endian raw bytes; the Rust side reads and writes them
 * with to_ne_bytes/from_ne_bytes.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The decomp sources, verbatim. See NOTICE. */
#include "lbtrigf.c"
#include "lb_00CE.c"

/*
 * src/MSL/float.c defines these as `int MSL_TrigF_80400770[] = { 0x7FFFFFFF }`
 * and `{ 0x7F800000 }`; lbtrigf.c declares them as float arrays. In a single
 * translation unit they must be floats, so main() stores the bit patterns.
 */
float MSL_TrigF_80400770[1];
float MSL_TrigF_80400774[1];

static void* slurp(const char* path, size_t* len)
{
    FILE* f = fopen(path, "rb");
    if (!f) {
        perror(path);
        exit(2);
    }
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    void* buf = malloc(n ? (size_t) n : 1);
    if (fread(buf, 1, (size_t) n, f) != (size_t) n) {
        perror("fread");
        exit(2);
    }
    fclose(f);
    *len = (size_t) n;
    return buf;
}

static float bits_to_f32(uint32_t u)
{
    float f;
    memcpy(&f, &u, 4);
    return f;
}
static uint32_t f32_to_bits(float f)
{
    uint32_t u;
    memcpy(&u, &f, 4);
    return u;
}

int main(int argc, char** argv)
{
    if (argc != 4) {
        fprintf(stderr, "usage: %s <op> <in> <out>\n", argv[0]);
        return 2;
    }
    const char* op = argv[1];
    size_t len;
    unsigned char* in = slurp(argv[2], &len);
    FILE* out = fopen(argv[3], "wb");
    if (!out) {
        perror(argv[3]);
        return 2;
    }

    {
        uint32_t nan_bits = 0x7FFFFFFFu;
        uint32_t inf_bits = 0x7F800000u;
        memcpy(&MSL_TrigF_80400770[0], &nan_bits, 4);
        memcpy(&MSL_TrigF_80400774[0], &inf_bits, 4);
    }

    if (strcmp(op, "atanf_lookup") == 0) {
        size_t n = sizeof(atanf_lookup) / sizeof(atanf_lookup[0]);
        for (size_t i = 0; i < n; i++) {
            uint32_t rb = f32_to_bits(atanf_lookup[i]);
            fwrite(&rb, 4, 1, out);
        }
    } else if (strcmp(op, "atan2f") == 0 || strcmp(op, "powf") == 0) {
        float (*fn)(float, float) = strcmp(op, "atan2f") == 0 ? atan2f : powf;
        size_t n = len / 8;
        for (size_t i = 0; i < n; i++) {
            uint32_t ua, ub;
            memcpy(&ua, in + i * 8, 4);
            memcpy(&ub, in + i * 8 + 4, 4);
            uint32_t rb = f32_to_bits(fn(bits_to_f32(ua), bits_to_f32(ub)));
            fwrite(&rb, 4, 1, out);
        }
    } else {
        float (*fn)(float) = NULL;
        if (strcmp(op, "atanf") == 0) {
            fn = atanf;
        } else if (strcmp(op, "acosf") == 0) {
            fn = acosf;
        } else if (strcmp(op, "asinf") == 0) {
            fn = asinf;
        } else if (strcmp(op, "lb_sqrtf") == 0) {
            fn = lb_sqrtf;
        } else if (strcmp(op, "expf") == 0) {
            fn = expf;
        } else {
            fprintf(stderr, "unknown op %s\n", op);
            return 2;
        }
        size_t n = len / 4;
        for (size_t i = 0; i < n; i++) {
            uint32_t ub;
            memcpy(&ub, in + i * 4, 4);
            uint32_t rb = f32_to_bits(fn(bits_to_f32(ub)));
            fwrite(&rb, 4, 1, out);
        }
    }
    fclose(out);
    free(in);
    return 0;
}
