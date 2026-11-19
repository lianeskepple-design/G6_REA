#ifndef G6_DIALOG_H
#define G6_DIALOG_H

#include "g6_biquad.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Dialog+ reference: 2-4 kHz band extraction plus a light Wiener-style
 * gate. The band envelope is compared with a slow noise floor; only the
 * voice band is added back, scaled by strength. This is an approximation
 * of the G6 survey's "2-4 kHz boost + simple noise suppression", not a
 * bit-exact port of the SB-Axx1 DSP.
 */

#define G6_DIALOG_MAX_CHANNELS 8

typedef struct {
    float band_env;
    float noise_env;
    float attack;
    float release;
    G6Biquad highpass;
    G6Biquad lowpass;
} G6DialogChannel;

typedef struct {
    uint32_t sample_rate;
    uint32_t channels;
    int strength;
    G6DialogChannel channel[G6_DIALOG_MAX_CHANNELS];
} G6DialogState;

int g6_dialog_init(G6DialogState *state, uint32_t sample_rate,
                   uint32_t channels);

/* strength 0..100. */
void g6_dialog_set(G6DialogState *state, int strength);

/* In-place interleaved float32. */
int g6_dialog_process_interleaved(G6DialogState *state, float *samples,
                                  size_t frames, size_t channels);

#ifdef __cplusplus
}
#endif

#endif
