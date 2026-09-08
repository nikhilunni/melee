/*
 * Reference-oracle driver for the FObj keyframe interpreter. Built by
 * tests/anim_ref_oracle.rs together with the verbatim copies of fobj.c and
 * spline.c in this directory.
 *
 * Usage: driver <op> <in-file> <out-file>
 *
 *   op        input record                        output record
 *   helmite   6 x u32 (f32 bits: fterm, time,     u32 (f32 bits)
 *             p0, p1, d0, d1)
 *   fobj      one track (see below)               nsteps x STEP_WORDS u32
 *
 * fobj input, all u32 words:
 *   obj_type, frac_value, frac_slope, startframe (f32 bits, the desc
 *   value), req_startframe (f32 bits), nsteps, length, then length bytes of
 *   keyframe stream padded to a word boundary, then nsteps rate words (f32
 *   bits). The track is loaded with HSD_FObjLoadDesc, started with
 *   HSD_FObjReqAnimAll(req_startframe), then HSD_FObjInterpretAnim is called
 *   once per rate. After each call one output record is written:
 *
 *   [0] number of callback invocations during the call
 *   [1] first emitted value (f32 bits, 0 if none)
 *   [2] last emitted value
 *   [3] wrapping sum of all emitted value bits
 *   [4] number of invocations that happened while op_intrp == HSD_A_OP_NONE
 *       (the C passes an uninitialised HSD_ObjData there; the Rust cannot
 *       match it, so the generator must avoid this and the test asserts 0)
 *   [5] flags  [6] op  [7] op_intrp  [8] nb_pack  [9] fterm
 *   [10] ad - ad_head  [11] time  [12] p0  [13] p1  [14] d0  [15] d1
 *
 * All records are host-endian raw bytes.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fobj.h"
#include "spline.h"

#define STEP_WORDS 16

/* ---- stubs for the HSD allocator and debug hooks ---------------------- */

void HSD_ObjAllocInit(HSD_ObjAllocData* data, size_t size, size_t align)
{
    (void) align;
    data->size = size;
}

void* HSD_ObjAlloc(HSD_ObjAllocData* data)
{
    return malloc(data->size);
}

void HSD_ObjFree(HSD_ObjAllocData* data, void* obj)
{
    (void) data;
    free(obj);
}

void anim_ref_assert_fail(const char* file, int line, const char* expr)
{
    fprintf(stderr, "%s:%d: HSD_ASSERT failed: %s\n", file, line, expr);
    exit(3);
}

/* ---- io helpers -------------------------------------------------------- */

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
static uint32_t rd32(const unsigned char* p)
{
    uint32_t u;
    memcpy(&u, p, 4);
    return u;
}
static void wr32(FILE* out, uint32_t u)
{
    fwrite(&u, 4, 1, out);
}

/* ---- fobj op ----------------------------------------------------------- */

typedef struct {
    HSD_FObj* fobj;
    uint32_t count;
    uint32_t first;
    uint32_t last;
    uint32_t sum;
    uint32_t uninit;
} StepRecord;

static void record_update(void* obj, enum_t type, HSD_ObjData* val)
{
    StepRecord* rec = obj;
    uint32_t bits = f32_to_bits(val->fv);
    (void) type;
    if (rec->fobj->op_intrp == HSD_A_OP_NONE) {
        rec->uninit += 1;
    }
    if (rec->count == 0) {
        rec->first = bits;
    }
    rec->last = bits;
    rec->sum += bits;
    rec->count += 1;
}

static void run_fobj(const unsigned char* in, size_t len, FILE* out)
{
    size_t p = 0;
    HSD_FObjInitAllocData();
    while (p + 28 <= len) {
        HSD_FObjDesc desc;
        HSD_FObj* fobj;
        uint32_t nsteps, length, i;
        float req_start;

        memset(&desc, 0, sizeof desc);
        desc.type = (u8) rd32(in + p);
        desc.frac_value = (u8) rd32(in + p + 4);
        desc.frac_slope = (u8) rd32(in + p + 8);
        desc.startframe = bits_to_f32(rd32(in + p + 12));
        req_start = bits_to_f32(rd32(in + p + 16));
        nsteps = rd32(in + p + 20);
        length = rd32(in + p + 24);
        p += 28;
        desc.length = length;
        desc.ad = malloc(length ? length : 1);
        memcpy(desc.ad, in + p, length);
        p += (length + 3) & ~3u;

        fobj = HSD_FObjLoadDesc(&desc);
        HSD_FObjReqAnimAll(fobj, req_start);
        for (i = 0; i < nsteps; ++i) {
            StepRecord rec;
            float rate = bits_to_f32(rd32(in + p));
            p += 4;
            memset(&rec, 0, sizeof rec);
            rec.fobj = fobj;
            HSD_FObjInterpretAnim(fobj, &rec, record_update, rate);
            wr32(out, rec.count);
            wr32(out, rec.first);
            wr32(out, rec.last);
            wr32(out, rec.sum);
            wr32(out, rec.uninit);
            wr32(out, fobj->flags);
            wr32(out, fobj->op);
            wr32(out, fobj->op_intrp);
            wr32(out, fobj->nb_pack);
            wr32(out, fobj->fterm);
            wr32(out, (uint32_t) (fobj->ad - fobj->ad_head));
            wr32(out, f32_to_bits(fobj->time));
            wr32(out, f32_to_bits(fobj->p0));
            wr32(out, f32_to_bits(fobj->p1));
            wr32(out, f32_to_bits(fobj->d0));
            wr32(out, f32_to_bits(fobj->d1));
        }
        HSD_FObjRemoveAll(fobj);
        free(desc.ad);
    }
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

    if (strcmp(op, "helmite") == 0) {
        size_t n = len / 24;
        for (size_t i = 0; i < n; ++i) {
            const unsigned char* r = in + i * 24;
            float v = splGetHelmite(bits_to_f32(rd32(r)), bits_to_f32(rd32(r + 4)),
                                    bits_to_f32(rd32(r + 8)), bits_to_f32(rd32(r + 12)),
                                    bits_to_f32(rd32(r + 16)), bits_to_f32(rd32(r + 20)));
            wr32(out, f32_to_bits(v));
        }
    } else if (strcmp(op, "fobj") == 0) {
        run_fobj(in, len, out);
    } else {
        fprintf(stderr, "unknown op %s\n", op);
        return 2;
    }

    fclose(out);
    free(in);
    return 0;
}
