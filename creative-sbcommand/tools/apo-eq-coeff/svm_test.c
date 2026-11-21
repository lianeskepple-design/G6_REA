/*
 * Smart Volume (CPlugAGCCompressor) probe.
 *
 * Build (MSVC developer prompt):
 *   cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I ..\..\reference
 *      svm_test.c ..\..\reference\g6_svm.c /Fe:svm_test.exe
 *
 * The probe drives one sample at a time so that the internal gain states can be
 * printed alongside the output, and the Python reference re-runs the same script.
 */

#include "g6_svm.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define BLOCKS 6
#define BLOCK_FRAMES 1024
#define TRACE_STRIDE 8
#define PI_F 3.14159265f

static uint32_t bits(float value)
{
    uint32_t word;
    memcpy(&word, &value, sizeof(word));
    return word;
}

static float sample_at(float amplitude, unsigned index)
{
    return amplitude * sinf(2.0f * PI_F * 1000.0f * (float)index / 48000.0f);
}

static void print_constants(const G6SvmState *state, const char *tag)
{
    printf("%s %08x %08x %08x %08x %08x %08x %08x %08x %08x %08x %08x "
           "%08x %08x %08x\n",
           tag,
           bits(state->k1), bits(state->k2), bits(state->alpha_energy),
           bits(state->alpha_makeup), bits(state->ramp_step), bits(state->attack),
           bits(state->release), bits(state->threshold), bits(state->ratio),
           bits(state->wet), bits(state->makeup), bits(state->delay_a),
           bits(state->delay_b), bits(state->crossfade));
}

int main(void)
{
    G6SvmState state;
    float left[1];
    float right[1];
    float *buffers[2] = { left, right };
    int block;
    unsigned index = 0;
    unsigned sample;

    if (!g6_svm_init(&state, 48000, 2)) {
        return 1;
    }
    /* -62 dBFS of energy is the DLL constructor default: threshold = -6.2 in log10. */
    if (!g6_svm_set_threshold_dbfs(&state, -62.0f)) {
        return 1;
    }
    printf("enabled %d channels %u\n", state.enabled, state.channels);
    print_constants(&state, "constants");

    for (block = 0; block < BLOCKS; ++block) {
        const float amplitude = (block < 2) ? 0.01f : ((block < 4) ? 0.5f : 0.01f);
        for (sample = 0; sample < BLOCK_FRAMES; ++sample) {
            const float input = sample_at(amplitude, index);
            left[0] = input;
            right[0] = input;
            if (!g6_svm_process(&state, buffers, 1)) {
                return 2;
            }
            if (index % TRACE_STRIDE == 0) {
                printf("frame %u %08x %08x %08x %08x %08x %08x %08x\n", index,
                       bits(state.gain1), bits(state.gain2), bits(state.mix_gain),
                       bits(state.crossfade), bits(left[0]),
                       bits(state.channel[0].energy), bits(input));
            }
            index += 1;
        }
    }
    printf("tail %08x %08x %08x %08x\n", bits(state.gain1), bits(state.gain2),
           bits(state.mix_gain), bits(state.crossfade));

    /* Bypass: 0x180093420 sets wet = 0, makeup = 1 and delays the taps to 30. */
    if (!g6_svm_set_enabled(&state, 0)) {
        return 3;
    }
    print_constants(&state, "bypass");
    printf("active %d\n", g6_svm_active(&state));
    return 0;
}
