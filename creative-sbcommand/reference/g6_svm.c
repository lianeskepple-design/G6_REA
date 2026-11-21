#include "g6_svm.h"

#include <math.h>
#include <string.h>

/* (k1, k2) detector pairs by sample rate, table selected in 0x180092a50. */
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

/* ctor constants: threshold 0xc0c66666, ratio 0x3ecccccd, makeup exponent 1.1. */
#define G6_SVM_THRESHOLD (-6.2f)
#define G6_SVM_RATIO 0.4f
#define G6_SVM_MAKEUP_EXPONENT 1.1f
#define G6_SVM_DELAY_SECONDS 0.02f
#define G6_SVM_MIN_DELAY_SAMPLES 30.0f
#define G6_SVM_MAX_DELAY_SECONDS 0.05f
#define G6_SVM_RAMP_SECONDS 0.08f
#define G6_SVM_ENERGY_SECONDS 0.04f
#define G6_SVM_MAKEUP_SECONDS 0.01f
#define G6_SVM_ATTACK_SECONDS (0.004f * 0.46593979f)
#define G6_SVM_RELEASE_SECONDS 8.0f

/* 1 - 10^(-1/(fs*seconds)), the shape used by 0x180092da0. */
static float time_constant(unsigned sample_rate, float seconds)
{
    return 1.0f - powf(10.0f, -1.0f / ((float)sample_rate * seconds));
}

static unsigned delay_index(unsigned write, float delay)
{
    const unsigned taps = (unsigned)(delay + 0.5f);
    if (write < taps) {
        return write - taps + G6_SVM_RING;
    }
    return write - taps;
}

/*
 * 0x1800935c0: clamp the requested delay and hand it to the tap that the crossfade
 * is currently fading away from (crossfade <= 0.5 uses tap A, otherwise tap B).
 */
static void apply_delay(G6SvmState *state)
{
    float delay;

    if (!state->enabled) {
        delay = G6_SVM_MIN_DELAY_SAMPLES;
    } else {
        delay = state->delay_seconds * (float)state->sample_rate;
        if (delay < G6_SVM_MIN_DELAY_SAMPLES) {
            delay = G6_SVM_MIN_DELAY_SAMPLES;
        }
        {
            const float maximum = G6_SVM_MAX_DELAY_SECONDS * (float)state->sample_rate;
            if (delay > maximum) {
                delay = maximum;
            }
        }
    }

    if (state->crossfade <= 0.5f) {
        state->delay_a = delay;
        state->target_up = 1;
        state->target_down = 0;
    } else {
        state->delay_b = delay;
        state->target_up = 0;
        state->target_down = 1;
    }
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
    state->alpha_energy = time_constant(sample_rate, G6_SVM_ENERGY_SECONDS);
    state->alpha_makeup = time_constant(sample_rate, G6_SVM_MAKEUP_SECONDS);
    state->ramp_step = 1.0f / ((float)sample_rate * G6_SVM_RAMP_SECONDS);
    state->attack = time_constant(sample_rate, G6_SVM_ATTACK_SECONDS);
    state->release = time_constant(sample_rate, G6_SVM_RELEASE_SECONDS);
    state->threshold = G6_SVM_THRESHOLD;
    state->ratio = G6_SVM_RATIO;

    state->enabled = 1;
    state->wet = 1.0f;
    state->makeup = powf(10.0f, G6_SVM_MAKEUP_EXPONENT);
    state->gain1 = 1.0f;
    state->gain2 = 1.0f;
    state->mix_gain = 1.0f;
    state->crossfade = 1.0f;
    state->delay_seconds = G6_SVM_DELAY_SECONDS;
    state->user_mix = 1.0f;
    /* 0x180092bc0 seeds both taps with 0.05*fs, then the ctor applies 0.02 s. */
    state->delay_a = G6_SVM_MAX_DELAY_SECONDS * (float)sample_rate;
    state->delay_b = state->delay_a;
    apply_delay(state);
    return 1;
}

int g6_svm_set_enabled(G6SvmState *state, int enabled)
{
    if (state == NULL) {
        return 0;
    }
    enabled = enabled != 0;
    if (enabled == state->enabled) {
        return 1;
    }
    state->enabled = enabled;
    if (enabled) {
        state->wet = 1.0f;
        state->makeup = powf(10.0f, G6_SVM_MAKEUP_EXPONENT);
    } else {
        state->wet = 0.0f;
        state->makeup = 1.0f;
    }
    apply_delay(state);
    return 1;
}

int g6_svm_set_delay_seconds(G6SvmState *state, float seconds)
{
    if (state == NULL || !isfinite(seconds)) {
        return 0;
    }
    if (seconds < 0.0f) {
        seconds = 0.0f;
    }
    state->delay_seconds = seconds;
    apply_delay(state);
    return 1;
}

int g6_svm_set_attack_seconds(G6SvmState *state, float seconds)
{
    if (state == NULL || !isfinite(seconds) || seconds <= 0.0f) {
        return 0;
    }
    state->attack = time_constant(state->sample_rate, seconds);
    return 1;
}

int g6_svm_set_release_seconds(G6SvmState *state, float seconds)
{
    if (state == NULL || !isfinite(seconds) || seconds <= 0.0f) {
        return 0;
    }
    state->release = time_constant(state->sample_rate, seconds);
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

int g6_svm_set_threshold_dbfs(G6SvmState *state, float threshold_dbfs)
{
    if (state == NULL || !isfinite(threshold_dbfs)) {
        return 0;
    }
    /* dBFS of energy -> log10(energy): 10*log10(x) = dBFS. */
    state->threshold = threshold_dbfs / 10.0f;
    return 1;
}

void g6_svm_set_mix(G6SvmState *state, float mix)
{
    if (state == NULL) {
        return;
    }
    state->user_mix = mix < 0.0f ? 0.0f : (mix > 1.0f ? 1.0f : mix);
}

void g6_svm_settle(G6SvmState *state)
{
    float delay;

    if (state == NULL) {
        return;
    }
    delay = state->enabled
                ? state->delay_seconds * (float)state->sample_rate
                : G6_SVM_MIN_DELAY_SAMPLES;
    if (delay < G6_SVM_MIN_DELAY_SAMPLES) {
        delay = G6_SVM_MIN_DELAY_SAMPLES;
    }
    {
        const float maximum = G6_SVM_MAX_DELAY_SECONDS * (float)state->sample_rate;
        if (delay > maximum) {
            delay = maximum;
        }
    }
    state->delay_a = delay;
    state->delay_b = delay;
    state->crossfade = 1.0f;
    state->target_up = 0;
    state->target_down = 0;
}

int g6_svm_active(const G6SvmState *state)
{
    if (state == NULL) {
        return 0;
    }
    /* A pending tap crossfade still has to run even after a bypass request. */
    return (state->target_up || state->target_down) ? 1 : 0;
}

int g6_svm_process(G6SvmState *state, float *const *buffers, size_t frames)
{
    size_t n;
    float values[G6_SVM_MAX_CHANNELS];

    if (state == NULL || buffers == NULL || frames == 0) {
        return 0;
    }
    for (n = 0; n < frames; ++n) {
        float detector = 0.0f;
        float applied;
        unsigned channel;

        for (channel = 0; channel < state->channels; ++channel) {
            G6SvmChannel *ch = &state->channel[channel];
            float *buffer = buffers[channel];
            float x;
            float filtered;

            if (buffer == NULL) {
                values[channel] = 0.0f;
                continue;
            }
            x = buffer[n];
            values[channel] = x;
            ch->ring[ch->write] = x;
            ch->write = (ch->write + 1u) % G6_SVM_RING;

            filtered = (x - ch->prev_x) * state->k1 - state->k2 * ch->prev_filtered;
            ch->prev_x = x;
            ch->prev_filtered = filtered;
            ch->energy = state->alpha_energy * filtered * filtered
                         + (1.0f - state->alpha_energy) * ch->energy;
            if (ch->energy > detector) {
                detector = ch->energy;
            }
        }

        {
            float target = 1.0f;
            if (detector > 0.0f) {
                const float level = log10f(detector);
                if (level >= state->threshold) {
                    target = powf(10.0f, (level - state->threshold) * state->ratio
                                        + state->threshold - level);
                }
            }

            {
                /* 0x180092f30: falling gain uses attack, rising gain uses release. */
                const float previous = state->gain1;
                const float coefficient = (previous <= target) ? state->release
                                                               : state->attack;
                state->gain1 = (1.0f - coefficient) * previous + coefficient * target;
                /*
                 * +0x4be8 follows the previous gain1 with the attack coefficient, and
                 * +0x4bec smooths the wet/makeup blend at 10 ms; the output uses the
                 * value computed on the previous frame.
                 */
                {
                    const float blend = ((state->wet * state->gain2 + 1.0f) - state->wet)
                                        * state->makeup;
                    applied = state->mix_gain;
                    state->gain2 = (1.0f - state->attack) * state->gain2
                                   + state->attack * previous;
                    state->mix_gain = (blend - state->mix_gain) * state->alpha_makeup
                                      + state->mix_gain;
                }
            }
        }

        for (channel = 0; channel < state->channels; ++channel) {
            G6SvmChannel *ch = &state->channel[channel];
            float *buffer = buffers[channel];
            float processed;
            float dry;

            if (buffer == NULL) {
                continue;
            }
            dry = values[channel];
            processed = ((1.0f - state->crossfade)
                             * ch->ring[delay_index(ch->write, state->delay_b)]
                         + state->crossfade
                               * ch->ring[delay_index(ch->write, state->delay_a)])
                        * applied;
            buffer[n] = dry + state->user_mix * (processed - dry);
        }

        /* Crossfade advance, 0x180092f30 tail. */
        if (state->target_down) {
            const float next = state->crossfade - state->ramp_step;
            state->crossfade = next;
            if (next < 0.0f) {
                state->crossfade = 0.0f;
                state->target_down = 0;
            }
        } else if (state->target_up) {
            const float next = state->crossfade + state->ramp_step;
            state->crossfade = next;
            if (next > 1.0f) {
                state->crossfade = 1.0f;
                state->target_up = 0;
            }
        }
    }
    return 1;
}
