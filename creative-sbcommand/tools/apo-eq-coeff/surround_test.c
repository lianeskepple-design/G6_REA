/*
 * CMSS3D matrix probe.
 *
 * Build (MSVC developer prompt):
 *   cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I ..\..\reference
 *      surround_test.c ..\..\reference\g6_surround.c /Fe:surround_test.exe
 */

#include "g6_surround.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static uint32_t bits(float value)
{
    uint32_t word;
    memcpy(&word, &value, sizeof(word));
    return word;
}

static void print_targets(unsigned mask, int strength)
{
    G6SurroundState state;
    memset(&state, 0, sizeof(state));
    g6_surround_set(&state, mask, strength);
    printf("mask %03x %d", mask, strength);
    for (int i = 0; i < 5; ++i) {
        printf(" %08x", bits(state.target[i]));
    }
    printf("\n");
}

int main(void)
{
    static const unsigned masks[] = {
        0x3b, 0x33, 0x37, 0x3f, 0x07, 0x0f,
        0x603, 0x607, 0x60b, 0x60f, 0x637, 0x63b, 0x63f
    };

    for (size_t i = 0; i < sizeof(masks) / sizeof(masks[0]); ++i) {
        print_targets(masks[i], 50);
    }

    {
        G6SurroundState state;
        float fl = 1.0f;
        float fr = 1.0f;
        float el = 0.5f;
        float er = 0.5f;
        const float center = 0.2f;
        const float lfe = 0.1f;
        float rear_l = 0.0f;
        float rear_r = 0.0f;

        memset(&state, 0, sizeof(state));
        g6_surround_set(&state, 0x3f, 50);
        /* First block ramps from zero; use a second call for the steady state. */
        g6_surround_process(&state, &fl, &fr, &el, &er, &center, &lfe,
                            &rear_l, &rear_r, 1);
        fl = 1.0f;
        fr = 1.0f;
        el = 0.5f;
        er = 0.5f;
        rear_l = 0.0f;
        rear_r = 0.0f;
        g6_surround_process(&state, &fl, &fr, &el, &er, &center, &lfe,
                            &rear_l, &rear_r, 1);
        printf("steady %08x %08x %08x %08x %08x %08x\n",
               bits(fl), bits(fr), bits(el), bits(er), bits(rear_l), bits(rear_r));
    }

    {
        G6SurroundState state;
        float fl[8];
        float fr[8];
        float el[8];
        float er[8];
        const float center[8] = {0};
        const float lfe[8] = {0};
        float rear_l[8] = {0};
        float rear_r[8] = {0};

        memset(&state, 0, sizeof(state));
        for (int i = 0; i < 8; ++i) {
            fl[i] = 1.0f;
            fr[i] = 1.0f;
            el[i] = 1.0f;
            er[i] = 1.0f;
        }
        g6_surround_set(&state, 0x3f, 50);
        g6_surround_process(&state, fl, fr, el, er, center, lfe, rear_l, rear_r, 8);
        printf("ramp");
        for (int i = 0; i < 8; ++i) {
            printf(" %08x", bits(fl[i]));
        }
        printf("\n");
        printf("ramp_state %d %08x\n", state.transition, bits(state.current[0]));
    }
    return 0;
}
