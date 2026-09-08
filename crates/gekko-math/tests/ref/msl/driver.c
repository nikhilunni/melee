/*
 * Reference-oracle driver. Built by tests/ref_oracle.rs together with the
 * copied MSL sources in this directory.
 *
 * Usage: driver <op> <in-file> <out-file>
 *
 *   op      input record          output record
 *   sinf    u32 (f32 bits)        u32 (f32 bits)
 *   cosf    u32                   u32
 *   tanf    u32                   u32
 *   logf    u32                   u32
 *   fmodf   u32 a, u32 b          u32
 *   frexp   u64 (f64 bits)        u64 (f64 bits), i64 exponent
 *
 * All records are host-endian raw bytes; the Rust side reads and writes them
 * with to_ne_bytes/from_ne_bytes.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "math.h"

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
static double bits_to_f64(uint64_t u)
{
    double d;
    memcpy(&d, &u, 8);
    return d;
}
static uint64_t f64_to_bits(double d)
{
    uint64_t u;
    memcpy(&u, &d, 8);
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

    /* The retail binary runs this static constructor before main. */
    __sinit_trigf_c();

    if (strcmp(op, "frexp") == 0) {
        size_t n = len / 8;
        for (size_t i = 0; i < n; i++) {
            uint64_t ub;
            memcpy(&ub, in + i * 8, 8);
            int e = 0x55555555;
            double r = frexp(bits_to_f64(ub), &e);
            uint64_t rb = f64_to_bits(r);
            int64_t eb = e;
            fwrite(&rb, 8, 1, out);
            fwrite(&eb, 8, 1, out);
        }
    } else if (strcmp(op, "fmodf") == 0) {
        size_t n = len / 8;
        for (size_t i = 0; i < n; i++) {
            uint32_t ua, ub;
            memcpy(&ua, in + i * 8, 4);
            memcpy(&ub, in + i * 8 + 4, 4);
            uint32_t rb = f32_to_bits(fmodf(bits_to_f32(ua), bits_to_f32(ub)));
            fwrite(&rb, 4, 1, out);
        }
    } else {
        float (*fn)(float) = NULL;
        if (strcmp(op, "sinf") == 0) {
            fn = sinf;
        } else if (strcmp(op, "cosf") == 0) {
            fn = cosf;
        } else if (strcmp(op, "tanf") == 0) {
            fn = tanf;
        } else if (strcmp(op, "logf") == 0) {
            fn = logf;
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
