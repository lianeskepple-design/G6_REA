#include "g6_bass.h"

#include <math.h>
#include <string.h>

/*
 * 2026-11-21 APO boundary (KSUSBAPO64.dll 1.2.109.0):
 * - CBassBoost is a 12 x 0x28 state array at CBassManager+0x60. It has no
 *   per-sample code and delegates to the global DSP object (slot +0x288).
 * - The APO "bass" path is redirect + limiter: knee -4.5 dB, slope 0.5,
 *   attack = 1 - exp(-2432 / (rate * 250 / 8 or 16)).
 * - Mode 0 below stays a PDF-direction reference, not an APO port; it is not
 *   bit-exact against the APO.
 */

/*
 * Legacy experimental tap set, read from KSUSBAPO64.dll 0x1801F7A28.
 * 2026-11-19 correction: in the APO this FIR smooths the limiter's gain
 * envelope, not the audio. Mode 1 below applies it to audio and is kept only
 * for historical comparison; it is not a bit-accurate APO implementation.
 */
static const float apo_fir[G6_BASS_TAPS] = {
    0.0f, 0.0f, 0.05f, 0.1f, 0.15f, 0.2f, 0.2f, 0.15f, 0.1f, 0.05f
};

static void reset_channel(G6BassState *state, uint32_t channel)
{
    g6_biquad_reset(&state->lowpass[channel]);
    g6_biquad_reset(&state->highpass[channel]);
    g6_biquad_reset(&state->smooth[channel]);
    g6_biquad_reset(&state->dc[channel]);
    memset(state->fir_history[channel], 0,
           sizeof(state->fir_history[channel]));
    state->fir_position[channel] = 0;
}

int g6_bass_init(G6BassState *state, uint32_t sample_rate, uint32_t channels)
{
    uint32_t channel;

    if (state == NULL || sample_rate == 0 || channels == 0 ||
        channels > G6_BASS_MAX_CHANNELS) {
        return 0;
    }
    memset(state, 0, sizeof(*state));
    state->sample_rate = sample_rate;
    state->channels = channels;
    state->mode = 0;
    for (channel = 0; channel < channels; ++channel) {
        g6_biquad_highpass(&state->dc[channel], (float)sample_rate,
                           20.0f, 0.70710678f);
    }
    g6_bass_set(state, 0, 120.0f);
    return 1;
}

void g6_bass_set(G6BassState *state, int strength, float cutoff_hz)
{
    uint32_t channel;
    float fs;

    if (state == NULL) {
        return;
    }
    if (strength < 0) {
        strength = 0;
    }
    if (strength > 100) {
        strength = 100;
    }
    if (cutoff_hz < 40.0f) {
        cutoff_hz = 40.0f;
    }
    if (cutoff_hz > 400.0f) {
        cutoff_hz = 400.0f;
    }
    if (state->strength == strength && state->cutoff_hz == cutoff_hz) {
        return;
    }
    state->strength = strength;
    state->cutoff_hz = cutoff_hz;
    fs = (float)state->sample_rate;
    for (channel = 0; channel < state->channels; ++channel) {
        g6_biquad_lowpass(&state->lowpass[channel], fs, cutoff_hz, 0.70710678f);
        g6_biquad_highpass(&state->highpass[channel], fs,
                           cutoff_hz * 1.5f, 0.70710678f);
        g6_biquad_lowpass(&state->smooth[channel], fs, 2000.0f, 0.70710678f);
    }
}

void g6_bass_set_mode(G6BassState *state, int mode)
{
    uint32_t channel;

    if (state == NULL) {
        return;
    }
    mode = (mode != 0) ? 1 : 0;
    if (state->mode == mode) {
        return;
    }
    state->mode = mode;
    for (channel = 0; channel < state->channels; ++channel) {
        reset_channel(state, channel);
    }
}

int g6_bass_process_interleaved(G6BassState *state, float *samples,
                                size_t frames, size_t channels)
{
    size_t frame;
    uint32_t channel;

    if (state == NULL || samples == NULL || channels == 0 ||
        channels > state->channels || state->channels == 0) {
        return 0;
    }
    if (state->strength == 0) {
        return 1;
    }
    for (frame = 0; frame < frames; ++frame) {
        for (channel = 0; channel < channels; ++channel) {
            const size_t index = frame * channels + channel;
            const float x = samples[index];

            if (state->mode == 0) {
                const float low = g6_biquad_run(&state->lowpass[channel], x);
                const float h2 = low * low;
                const float h3 = low * h2;
                float harmonic = h2 * 0.5f + h3 * 0.25f;

                harmonic = g6_biquad_run(&state->highpass[channel], harmonic);
                harmonic = g6_biquad_run(&state->smooth[channel], harmonic);
                samples[index] = x + ((float)state->strength / 100.0f)
                                         * harmonic;
            } else {
                unsigned position = state->fir_position[channel];
                float fir = apo_fir[0] * x;
                float phase;
                float frac;
                float shaped;
                unsigned k;

                for (k = 1; k < G6_BASS_TAPS; ++k) {
                    fir += apo_fir[k]
                           * state->fir_history[channel]
                                 [(position + G6_BASS_TAPS - k) % G6_BASS_TAPS];
                }
                state->fir_history[channel][position] = x;
                state->fir_position[channel] =
                    (position + 1) % G6_BASS_TAPS;

                phase = fir * 0.166096404f * 8.0f;
                frac = phase - roundf(phase);
                shaped = (((frac * 0.0397112630f + 0.2327848822f) * frac
                           + 0.6930736303f) * frac + 1.0f) - 1.0e-6f;
                shaped = g6_biquad_run(&state->dc[channel], shaped - 1.0f);
                samples[index] = x + ((float)state->strength / 100.0f)
                                         * 0.17782794f * shaped;
            }
        }
    }
    return 1;
}
