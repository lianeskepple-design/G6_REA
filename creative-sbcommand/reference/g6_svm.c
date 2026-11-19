#include "g6_svm.h"

#include <math.h>
#include <string.h>

typedef struct {
    float rate;
    float k1;
    float k2;
} G6SvmFilterPair;

static const G6SvmFilterPair filter_pairs[] = {
    { 192000.0f, 0.99669999f, -0.99349999f },
    { 176400.0f, 0.99650002f, -0.99290001f },
    { 96000.0f, 0.99349999f, -0.98699999f },
    { 88200.0f, 0.99290001f, -0.98589998f },
    { 48000.0f, 0.98710001f, -0.97420001f },
    { 44100.0f, 0.98600000f, -0.97189999f },
    { 32000.0f, 0.98070002f, -0.96149999f },
    { 24000.0f, 0.97450000f, -0.94900000f },
    { 22050.0f, 0.97229999f, -0.94459999f },
    { 16000.0f, 0.96219999f, -0.92439997f },
    { 12000.0f, 0.95020002f, -0.90039998f },
    { 11025.0f, 0.94599998f, -0.89209998f },
    { 0.0f, 0.92699999f, -0.85409999f }
};

static float time_constant(unsigned sample_rate, float seconds)
{
    return 1.0f - powf(10.0f, -1.0f / ((float)sample_rate * seconds));
}

int g6_svm_init(G6SvmState *state, unsigned sample_rate, unsigned channels)
{
    size_t i;

    if (state == NULL || sample_rate == 0 || channels == 0 ||
        channels > G6_SVM_MAX_CHANNELS) {
        return 0;
    }
    memset(state, 0, sizeof(*state));
    state->sample_rate = sample_rate;
    state->channels = channels;
    for (i = 0; i < sizeof(filter_pairs) / sizeof(filter_pairs[0]); ++i) {
        if ((float)sample_rate >= filter_pairs[i].rate) {
            state->k1 = filter_pairs[i].k1;
            state->k2 = filter_pairs[i].k2;
            break;
        }
    }
    state->alpha = time_constant(sample_rate, 0.04f);
    state->beta = time_constant(sample_rate, 0.01f);
    state->attack = time_constant(sample_rate, 0.0018638f);
    state->release = time_constant(sample_rate, 8.0f);
    state->ramp_step = 1.0f / (0.08f * (float)sample_rate);
    state->lookahead = (unsigned)(0.05f * (float)sample_rate);
    if (state->lookahead > 30) {
        state->lookahead = 30;
    }
    state->threshold = -6.2f;
    state->gain = 1.0f;
    state->mix = 1.0f;
    return 1;
}

int g6_svm_set_level(G6SvmState *state, float level_db)
{
    if (state == NULL || !isfinite(level_db)) {
        return 0;
    }
    state->threshold = (level_db - 12.0f) / 10.0f;
    return 1;
}

void g6_svm_set_mix(G6SvmState *state, float mix)
{
    if (state == NULL) {
        return;
    }
    state->mix = mix < 0.0f ? 0.0f : (mix > 1.0f ? 1.0f : mix);
}

int g6_svm_process(G6SvmState *state, float *const *buffers, size_t frames)
{
    size_t n;

    if (state == NULL || buffers == NULL || frames == 0) {
        return 0;
    }
    for (n = 0; n < frames; ++n) {
        float detector = 0.0f;
        unsigned channel;

        for (channel = 0; channel < state->channels; ++channel) {
            G6SvmChannel *ch = &state->channel[channel];
            float *buffer = buffers[channel];
            float x;
            float filtered;

            if (buffer == NULL) {
                continue;
            }
            x = buffer[n];
            ch->ring[ch->write] = x;
            ch->write = (ch->write + 1u) % G6_SVM_RING;

            filtered = (x - ch->prev_x) * state->k1 - state->k2 * ch->prev_filtered;
            ch->prev_x = x;
            ch->prev_filtered = filtered;
            ch->energy = state->alpha * filtered * filtered
                         + (1.0f - state->alpha) * ch->energy;
            if (ch->energy > detector) {
                detector = ch->energy;
            }
        }

        {
            float target = 1.0f;
            if (detector > 0.0f) {
                const float level = log10f(detector);
                float over = level - state->threshold;
                if (over < 0.0f) {
                    over = 0.0f;
                }
                target = powf(10.0f, -0.3f * over);
            }
            {
                const float coefficient = (target < state->gain)
                                              ? state->attack
                                              : state->release;
                state->gain += (target - state->gain) * coefficient;
            }
        }

        for (channel = 0; channel < state->channels; ++channel) {
            G6SvmChannel *ch = &state->channel[channel];
            float *buffer = buffers[channel];
            unsigned read;
            float delayed;

            if (buffer == NULL) {
                continue;
            }
            read = (ch->write + G6_SVM_RING - state->lookahead) % G6_SVM_RING;
            delayed = ch->ring[read];
            buffer[n] = delayed * (1.0f + state->mix * (state->gain - 1.0f));
        }
    }
    return 1;
}
