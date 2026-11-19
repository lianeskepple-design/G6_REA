#include "g6_dialog.h"

#include <math.h>
#include <string.h>

int g6_dialog_init(G6DialogState *state, uint32_t sample_rate,
                   uint32_t channels)
{
    uint32_t channel;
    float fs;

    if (state == NULL || sample_rate == 0 || channels == 0 ||
        channels > G6_DIALOG_MAX_CHANNELS) {
        return 0;
    }
    memset(state, 0, sizeof(*state));
    state->sample_rate = sample_rate;
    state->channels = channels;
    fs = (float)sample_rate;
    for (channel = 0; channel < channels; ++channel) {
        G6DialogChannel *ch = &state->channel[channel];
        g6_biquad_highpass(&ch->highpass, fs, 2000.0f, 0.70710678f);
        g6_biquad_lowpass(&ch->lowpass, fs, 4000.0f, 0.70710678f);
        ch->attack = 1.0f - expf(-1.0f / (fs * 0.005f));
        ch->release = 1.0f - expf(-1.0f / (fs * 0.100f));
        ch->band_env = 0.0f;
        ch->noise_env = 1.0e-6f;
    }
    g6_dialog_set(state, 0);
    return 1;
}

void g6_dialog_set(G6DialogState *state, int strength)
{
    if (state == NULL) {
        return;
    }
    if (strength < 0) {
        strength = 0;
    }
    if (strength > 100) {
        strength = 100;
    }
    state->strength = strength;
}

int g6_dialog_process_interleaved(G6DialogState *state, float *samples,
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
            G6DialogChannel *ch = &state->channel[channel];
            const size_t index = frame * channels + channel;
            const float x = samples[index];
            const float band = g6_biquad_run(
                &ch->lowpass, g6_biquad_run(&ch->highpass, x));
            const float level = fabsf(band);
            float env;
            float snr;
            float gain;

            if (level > ch->band_env) {
                env = ch->attack * level + (1.0f - ch->attack) * ch->band_env;
            } else {
                env = ch->release * level + (1.0f - ch->release) * ch->band_env;
            }
            ch->band_env = env;
            if (env < ch->noise_env) {
                ch->noise_env = env;
            } else {
                ch->noise_env = ch->noise_env * 0.9999f + env * 0.0001f;
            }
            if (ch->noise_env < 1.0e-6f) {
                ch->noise_env = 1.0e-6f;
            }
            snr = env / ch->noise_env;
            gain = snr / (snr + 1.0f);
            samples[index] = x + ((float)state->strength / 100.0f)
                                     * 2.0f * gain * band;
        }
    }
    return 1;
}
