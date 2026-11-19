#ifndef G6_CRYSTALIZER_H
#define G6_CRYSTALIZER_H

#include "g6_crystalizer_legacy.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Functional render chain for the legacy Gaming Crystalizer, reconstructed
 * from KSUSBAPO64.dll 1.2.109.0:
 *
 *   mono mix        (L+R)*0.75                    VA 0x18008e030
 *   analysis 1      3rd-order low-pass            tables 0x180195a44/0x1801959f0
 *   analysis 2      3rd-order high-pass           tables 0x180195ae4/0x180195a90
 *   detector        history/activity              VA 0x18008e030 (+0x3e4..0x60d)
 *   coefficients    one-pole + squared one-pole   VA 0x18008e611..0x18008e737
 *   per-channel     3-coeff one-pole -> biquad    VAs 0x1801468c0 / 0x1801469e0
 *   overlap         scratch = [prev dry, current dry] (256), Hann x2,
 *                   +0x1F8 adds the previous +0x600 dry block
 *                   (bank 0x180199E80 = {1,1,1,1}), first 128 x ramp
 *                   0x180195FD0, then the +0xF18 biquad (0x18008EC00).
 *
 * Mono analysis input is (L+R)*0.75 (DAT_18018B7C4). Strength acts through
 * the interpolated 18-value parameter bank, as in the DLL. The coefficient
 * source for +0xF18 is still unknown; the default is a passthrough and
 * callers can install measured values.
 */

#define G6_CRYSTALIZER_BLOCK_MAX 128
#define G6_CRYSTALIZER_MAX_CHANNELS 3

typedef struct {
    float c8, c12, c16;
    float c20, c24, c28, c32;
    float w1, w2, w3;
} G6CrystalizerFilter3;

typedef struct {
    float c1, c2, c3;
    float state;
} G6CrystalizerOnePole;

typedef struct {
    float c8, c12, c16, c20, c24;
    float w1, w2;
} G6CrystalizerBiquad;

typedef struct {
    uint32_t sample_rate;
    unsigned class_index;
    unsigned strength;
    G6CrystalizerLegacyParameters params;
    G6CrystalizerFilter3 analysis[2];
    G6CrystalizerLegacyAnalysis detector;
    G6CrystalizerOnePole one_pole;
    G6CrystalizerBiquad biquad;
    G6CrystalizerBiquad output_filter;
    float one_pole_state[G6_CRYSTALIZER_MAX_CHANNELS];
    float biquad_w1[G6_CRYSTALIZER_MAX_CHANNELS];
    float biquad_w2[G6_CRYSTALIZER_MAX_CHANNELS];
    float output_w1[G6_CRYSTALIZER_MAX_CHANNELS];
    float output_w2[G6_CRYSTALIZER_MAX_CHANNELS];
    float prev_input[G6_CRYSTALIZER_MAX_CHANNELS][128];
    float ola_b[G6_CRYSTALIZER_MAX_CHANNELS][128];
    unsigned window_phase;
    int window_enabled;
    /*
     * +0x1F8 dry overlap: adds the previous block from ola_b into the
     * windowed filter output before the ramp. The DLL weight bank is
     * {1,1,1,1}; disable only for differential tests.
     */
    int dry_mix_enabled;
} G6CrystalizerState;

/* Selects the sample-rate class and loads the two analysis filters. */
int g6_crystalizer_init(G6CrystalizerState *state, uint32_t sample_rate);

/* Interpolates the 18-value parameter bank (0..100, as the DLL does). */
int g6_crystalizer_set_strength(G6CrystalizerState *state, unsigned strength);

/*
 * Computes the one-pole and biquad coefficients from the current detector
 * activities. Exposed for tests; the block processor calls it internally.
 */
void g6_crystalizer_compute_filters(G6CrystalizerState *state,
                                    float first_activity,
                                    float second_activity);

/*
 * Fixed output biquad at object +0xF18 (0x18008EC00). The DLL's coefficient
 * source was not found in .text; the default is a passthrough and callers can
 * install measured values. Reset after changing it.
 */
void g6_crystalizer_set_output_filter(G6CrystalizerState *state, float c8,
                                      float c12, float c16, float c20,
                                      float c24);

/*
 * Output window: 256-sample Hann (0x180195BD0) times the 128-sample gain ramp
 * 1/(1 - 0.5*sin^2(pi*n/128)) (0x180195FD0). Exposed for tests.
 */
float g6_crystalizer_window_gain(unsigned index);

/*
 * In-place interleaved float PCM. Channels beyond the first three are copied
 * unchanged (the DLL processes at most three). Returns 1 on success, 0 on
 * invalid input or non-finite detector input.
 */
int g6_crystalizer_process_interleaved(G6CrystalizerState *state, float *samples,
                                       size_t frames, size_t channels);

#ifdef __cplusplus
}
#endif

#endif
