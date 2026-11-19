#ifndef G6_BASS_H
#define G6_BASS_H

#include "g6_biquad.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Missing-fundamental bass reference.
 *
 * A low-pass extracts the bass band, then 2nd/3rd-order terms generate
 * harmonics above the cutoff. The harmonic path is high-passed so the
 * fundamental itself is not boosted, then softly low-passed. This is an
 * approximation of the "Bass" effect described in the G6 survey, not a
 * bit-exact port of the SB-Axx1 DSP.
 */

#define G6_BASS_MAX_CHANNELS 8
#define G6_BASS_TAPS 10

typedef struct {
    uint32_t sample_rate;
    uint32_t channels;
    int strength;
    float cutoff_hz;
    int mode; /* 0 = reference low-pass/x2/x3; 1 = legacy experimental
                 FIR/cubic on audio, not the APO envelope limiter */
    G6Biquad lowpass[G6_BASS_MAX_CHANNELS];
    G6Biquad highpass[G6_BASS_MAX_CHANNELS];
    G6Biquad smooth[G6_BASS_MAX_CHANNELS];
    G6Biquad dc[G6_BASS_MAX_CHANNELS];
    float fir_history[G6_BASS_MAX_CHANNELS][G6_BASS_TAPS];
    unsigned fir_position[G6_BASS_MAX_CHANNELS];
} G6BassState;

int g6_bass_init(G6BassState *state, uint32_t sample_rate, uint32_t channels);

/* strength 0..100; cutoff_hz is clamped to 40..400 (reference mode). */
void g6_bass_set(G6BassState *state, int strength, float cutoff_hz);

/* 0 = reference, 1 = APO-like 10-tap FIR + cubic shaper. */
void g6_bass_set_mode(G6BassState *state, int mode);

/* In-place interleaved float32. */
int g6_bass_process_interleaved(G6BassState *state, float *samples,
                                size_t frames, size_t channels);

#ifdef __cplusplus
}
#endif

#endif
