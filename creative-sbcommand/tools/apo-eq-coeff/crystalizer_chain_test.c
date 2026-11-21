/*
 * Crystalizer render-chain probe.
 *
 * Prints the analysis filter coefficients, their impulse responses, the
 * activity-derived one-pole/biquad coefficients, and a short stereo block
 * result as hex float32. Compare with crystalizer_chain_ref.py.
 *
 * Build (MSVC developer prompt):
 *   cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I ..\..\reference
 *      crystalizer_chain_test.c ..\..\reference\g6_crystalizer.c
 *      ..\..\reference\g6_crystalizer_legacy.c /Fe:crystalizer_chain_test.exe
 */

#include "g6_crystalizer.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define TEST_FRAMES 8192

static uint32_t bits(float value)
{
    uint32_t word;
    memcpy(&word, &value, sizeof(word));
    return word;
}

static float filter3_impulse(const G6CrystalizerFilter3 *filter, int index)
{
    G6CrystalizerFilter3 local = *filter;
    float y = 0.0f;
    for (int n = 0; n <= index; ++n) {
        const float x = (n == 0) ? 1.0f : 0.0f;
        const float w = x + local.c8 * local.w1 + local.c12 * local.w2
                        + local.c16 * local.w3;
        y = local.c20 * w + local.c24 * local.w1 + local.c28 * local.w2
            + local.c32 * local.w3;
        local.w3 = local.w2;
        local.w2 = local.w1;
        local.w1 = w;
    }
    return y;
}

int main(void)
{
    G6CrystalizerState state;
    float samples[TEST_FRAMES * 2];

    if (!g6_crystalizer_init(&state, 48000)) {
        return 1;
    }
    printf("class %u\n", state.class_index);
    for (int band = 0; band < 2; ++band) {
        const G6CrystalizerFilter3 *f = &state.analysis[band];
        printf("analysis%d %08x %08x %08x %08x %08x %08x %08x\n", band,
               bits(f->c8), bits(f->c12), bits(f->c16), bits(f->c20),
               bits(f->c24), bits(f->c28), bits(f->c32));
    }
    for (int band = 0; band < 2; ++band) {
        const G6CrystalizerFilter3 *f = &state.analysis[band];
        printf("impulse%d", band);
        for (int n = 0; n < 16; ++n) {
            printf(" %08x", bits(filter3_impulse(f, n)));
        }
        printf("\n");
    }

    if (!g6_crystalizer_set_strength(&state, 50)) {
        return 2;
    }
    g6_crystalizer_compute_filters(&state, 0.1f, 0.2f);
    printf("onepole %08x %08x %08x\n", bits(state.one_pole.c1),
           bits(state.one_pole.c2), bits(state.one_pole.c3));
    printf("biquad %08x %08x %08x %08x %08x\n", bits(state.biquad.c8),
           bits(state.biquad.c12), bits(state.biquad.c16),
           bits(state.biquad.c20), bits(state.biquad.c24));

    for (size_t n = 0; n < TEST_FRAMES; ++n) {
        const float value = 0.25f * sinf(6.28318548f * 1000.0f
                                         * (float)n / 48000.0f);
        samples[n * 2] = value;
        samples[n * 2 + 1] = value;
    }
    if (!g6_crystalizer_process_interleaved(&state, samples, TEST_FRAMES, 2)) {
        return 3;
    }
    printf("window");
    for (int n = 0; n < 8; ++n) {
        printf(" %08x", bits(g6_crystalizer_window_gain((unsigned)n)));
    }
    printf("\n");
    printf("block");
    for (int n = 0; n < 8; ++n) {
        printf(" %08x %08x", bits(samples[n * 2]), bits(samples[n * 2 + 1]));
    }
    printf("\n");
    printf("steady");
    for (int n = 256; n < 264; ++n) {
        printf(" %08x %08x", bits(samples[n * 2]), bits(samples[n * 2 + 1]));
    }
    printf("\n");
    {
        double sum = 0.0;
        for (int n = 256; n < 512; ++n) {
            sum += (double)samples[n * 2] * samples[n * 2]
                   + (double)samples[n * 2 + 1] * samples[n * 2 + 1];
        }
        printf("steady_rms %08x\n", bits((float)sqrt(sum / (2.0 * 256.0))));
    }
    {
        int finite = 1;
        for (size_t n = 0; n < TEST_FRAMES * 2; ++n) {
            if (!isfinite(samples[n])) {
                finite = 0;
            }
        }
        printf("finite %d\n", finite);
    }
    {
        G6CrystalizerState activity_state;
        float activity_pcm[128 * 2];

        if (!g6_crystalizer_init(&activity_state, 48000)) {
            return 5;
        }
        for (int block = 0; block < 48; ++block) {
            for (int i = 0; i < 128; ++i) {
                const float value = 0.05f * sinf(6.28318548f * 1000.0f
                                                 * (float)(block * 128 + i) / 48000.0f);
                activity_pcm[i * 2] = value;
                activity_pcm[i * 2 + 1] = value;
            }
            if (!g6_crystalizer_process_interleaved(&activity_state, activity_pcm,
                                                    128, 2)) {
                return 5;
            }
        }
        for (int i = 0; i < 128; ++i) {
            const float value = 0.6f * sinf(6.28318548f * 1000.0f
                                             * (float)(6144 + i) / 48000.0f);
            activity_pcm[i * 2] = value;
            activity_pcm[i * 2 + 1] = value;
        }
        if (!g6_crystalizer_process_interleaved(&activity_state, activity_pcm, 128, 2)) {
            return 5;
        }
        {
            float max1 = 0.0f;
            float max2 = 0.0f;
            for (size_t i = 0; i < G6_CRYSTALIZER_LEGACY_HISTORY_COUNT; ++i) {
                if (activity_state.detector.first_history[i] > max1) {
                    max1 = activity_state.detector.first_history[i];
                }
                if (activity_state.detector.second_history[i] > max2) {
                    max2 = activity_state.detector.second_history[i];
                }
            }
            printf("activity_debug %08x %08x %08x %08x\n",
                   bits(activity_state.detector.first_band_envelope),
                   bits(activity_state.detector.reference_envelope),
                   bits(max1), bits(max2));
        }
        printf("activity %08x %08x\n",
               bits(activity_state.detector.first_activity),
               bits(activity_state.detector.second_activity));
    }

    {
        G6CrystalizerState quiet;
        G6CrystalizerState loud;
        float quiet_pcm[TEST_FRAMES * 2];
        float loud_pcm[TEST_FRAMES * 2];
        float maximum = 0.0f;

        for (size_t n = 0; n < TEST_FRAMES * 2; ++n) {
            const size_t frame = n / 2;
            const float value = 0.25f * sinf(6.28318548f * 1000.0f
                                             * (float)frame / 48000.0f);
            quiet_pcm[n] = value;
            loud_pcm[n] = quiet_pcm[n];
        }
        if (!g6_crystalizer_init(&quiet, 48000) ||
            !g6_crystalizer_init(&loud, 48000) ||
            !g6_crystalizer_set_strength(&quiet, 0) ||
            !g6_crystalizer_set_strength(&loud, 100) ||
            !g6_crystalizer_process_interleaved(&quiet, quiet_pcm, TEST_FRAMES, 2) ||
            !g6_crystalizer_process_interleaved(&loud, loud_pcm, TEST_FRAMES, 2)) {
            return 4;
        }
        for (size_t n = 0; n < TEST_FRAMES * 2; ++n) {
            const float delta = fabsf(quiet_pcm[n] - loud_pcm[n]);
            if (delta > maximum) {
                maximum = delta;
            }
        }
        printf("strength_delta %08x\n", bits(maximum));
    }

    /*
     * KSUSBAPO64.dll has no extra output biquad: 0x18008e920 ends with the ramp mix at
     * vtable +0x180, and displacement 0xf18 never appears in .text. This marker keeps
     * the claim checkable instead of leaving an unused stage in the chain.
     */
    printf("output_stage none\n");
    {
        G6CrystalizerState mixed;
        G6CrystalizerState plain;
        float mixed_pcm[256 * 2];
        float plain_pcm[256 * 2];
        int delay_ok = 1;
        int drymix_ok = 1;
        float drymix_max = 0.0f;

        for (size_t n = 0; n < 256 * 2; ++n) {
            mixed_pcm[n] = 0.0f;
            plain_pcm[n] = 0.0f;
        }
        mixed_pcm[0] = 0.5f;
        mixed_pcm[1] = 0.5f;
        plain_pcm[0] = 0.5f;
        plain_pcm[1] = 0.5f;
        if (!g6_crystalizer_init(&mixed, 48000) ||
            !g6_crystalizer_init(&plain, 48000)) {
            return 7;
        }
        plain.dry_mix_enabled = 0;
        if (!g6_crystalizer_process_interleaved(&mixed, mixed_pcm, 256, 2) ||
            !g6_crystalizer_process_interleaved(&plain, plain_pcm, 256, 2)) {
            return 7;
        }
        for (int n = 0; n < 8; ++n) {
            if (mixed_pcm[n * 2] != 0.0f || mixed_pcm[n * 2 + 1] != 0.0f) {
                delay_ok = 0;
            }
        }
        for (int n = 0; n < 128; ++n) {
            const float s = sinf(3.14159265f * (float)n / 128.0f);
            const float ramp = 1.0f / (1.0f - 0.5f * s * s);
            const float expected = (n == 0) ? 0.5f * ramp : 0.0f;
            const float delta = fabsf((mixed_pcm[(128 + n) * 2]
                                       - plain_pcm[(128 + n) * 2]) - expected);
            if (delta > drymix_max) {
                drymix_max = delta;
            }
            if (delta > 1.0e-6f) {
                drymix_ok = 0;
            }
        }
        printf("delay_ok %d\n", delay_ok);
        printf("drymix_ok %d\n", drymix_ok);
        printf("drymix_delta %08x\n", bits(drymix_max));
    }
    return 0;
}
