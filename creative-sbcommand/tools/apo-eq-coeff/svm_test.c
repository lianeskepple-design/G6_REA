/*
 * Smart Volume AGC probe.
 *
 * Build (MSVC developer prompt):
 *   cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I ..\..\reference
 *      svm_test.c ..\..\reference\g6_svm.c /Fe:svm_test.exe
 */

#include "g6_svm.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define FRAMES 1000

static uint32_t bits(float value)
{
    uint32_t word;
    memcpy(&word, &value, sizeof(word));
    return word;
}

static double rms(const float *buffer, size_t count)
{
    double sum = 0.0;
    for (size_t i = 0; i < count; ++i) {
        sum += (double)buffer[i] * buffer[i];
    }
    return sqrt(sum / (double)count);
}

static void fill_sine(float *buffer, float amplitude)
{
    for (size_t i = 0; i < FRAMES; ++i) {
        buffer[i] = amplitude * sinf(6.28318548f * 1000.0f * (float)i / 48000.0f);
    }
}

int main(void)
{
    G6SvmState state;
    float left[FRAMES];
    float right[FRAMES];
    float *buffers[2] = { left, right };

    if (!g6_svm_init(&state, 48000, 2)) {
        return 1;
    }
    if (!g6_svm_set_level(&state, -20.0f)) {
        return 1;
    }
    printf("constants %08x %08x %08x %08x %08x %08x %08x %u\n",
           bits(state.k1), bits(state.k2), bits(state.alpha), bits(state.beta),
           bits(state.attack), bits(state.release), bits(state.ramp_step),
           state.lookahead);

    fill_sine(left, 0.01f);
    fill_sine(right, 0.01f);
    if (!g6_svm_process(&state, buffers, FRAMES)) {
        return 2;
    }
    printf("quiet_gain %08x\n", bits(state.gain));

    fill_sine(left, 0.5f);
    fill_sine(right, 0.5f);
    {
        const double input_rms = rms(left, FRAMES);
        if (!g6_svm_process(&state, buffers, FRAMES)) {
            return 3;
        }
        printf("loud input_rms %08x output_rms %08x gain %08x\n",
               bits((float)input_rms), bits((float)rms(left, FRAMES)),
               bits(state.gain));
    }

    for (int block = 0; block < 200; ++block) {
        fill_sine(left, 0.01f);
        fill_sine(right, 0.01f);
        if (!g6_svm_process(&state, buffers, FRAMES)) {
            return 4;
        }
    }
    printf("release_gain %08x\n", bits(state.gain));
    return 0;
}
