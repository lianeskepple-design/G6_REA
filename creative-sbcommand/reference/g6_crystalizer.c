#include "g6_crystalizer.h"

#include <math.h>
#include <string.h>

/* Analysis filters, grouped by sample-rate class:
 *   0=22050, 1=44100, 2=48000, 3=96000, 4=192000.
 * Each filter is 3 denominator + 4 numerator coefficients, copied from
 * 0x180195a44 / 0x1801959f0 (first) and 0x180195ae4 / 0x180195a90 (second).
 */
static const float analysis_first_den[5][3] = {
    { 2.70068121f, -2.44379687f, 0.739700139f },
    { 2.85020638f, -2.71109629f, 0.86043334f },
    { 2.86235762f, -2.73373818f, 0.871024668f },
    { 2.93110919f, -2.86448121f, 0.933326185f },
    { 2.9655323f, -2.93163133f, 0.966093123f }
};

static const float analysis_first_num[5][4] = {
    { 0.0282577593f, -0.0265499298f, -0.0265499298f, 0.0282577593f },
    { 0.0150161376f, -0.014787958f, -0.014787958f, 0.0150161376f },
    { 0.0138701461f, -0.0136921862f, -0.0136921862f, 0.0138701461f },
    { 0.00715662446f, -0.00713364128f, -0.00713364128f, 0.00715662446f },
    { 0.00363776833f, -0.00363484677f, -0.00363484677f, 0.00363776833f }
};

static const float analysis_second_den[5][3] = {
    { -2.63391948f, -2.53106618f, -0.881794333f },
    { -1.05723643f, -1.08735812f, -0.70898968f },
    { -0.793858826f, -0.893759787f, -0.687055707f },
    { 0.759954393f, -0.191214889f, -0.502502382f },
    { 1.53758836f, -0.503638148f, -0.238011643f }
};

static const float analysis_second_num[5][4] = {
    { 0.00191905815f, -0.00575717445f, 0.00575717445f, -0.00191905815f },
    { 0.0401415043f, -0.120424509f, 0.120424509f, -0.0401415043f },
    { 0.051605653f, -0.154816955f, 0.154816955f, -0.051605653f },
    { 0.181083351f, -0.543250084f, 0.543250084f, -0.181083351f },
    { 0.350401849f, -1.05120564f, 1.05120564f, -0.350401849f }
};

static unsigned class_from_rate(uint32_t sample_rate)
{
    switch (sample_rate) {
    case 22050:  return 0;
    case 44100:  return 1;
    case 48000:  return 2;
    case 96000:  return 3;
    case 192000: return 4;
    default:     return 2; /* DLL falls back to class 2 for unknown rates. */
    }
}

static void load_filter3(G6CrystalizerFilter3 *filter, const float den[3],
                         const float num[4])
{
    filter->c8 = den[0];
    filter->c12 = den[1];
    filter->c16 = den[2];
    filter->c20 = num[0];
    filter->c24 = num[1];
    filter->c28 = num[2];
    filter->c32 = num[3];
    filter->w1 = 0.0f;
    filter->w2 = 0.0f;
    filter->w3 = 0.0f;
}

static float filter3_run(G6CrystalizerFilter3 *filter, float x)
{
    const float w = x + filter->c8 * filter->w1 + filter->c12 * filter->w2
                    + filter->c16 * filter->w3;
    const float y = filter->c20 * w + filter->c24 * filter->w1
                    + filter->c28 * filter->w2 + filter->c32 * filter->w3;
    filter->w3 = filter->w2;
    filter->w2 = filter->w1;
    filter->w1 = w;
    return y;
}

int g6_crystalizer_init(G6CrystalizerState *state, uint32_t sample_rate)
{
    unsigned index;
    size_t i;

    if (state == NULL || sample_rate == 0) {
        return 0;
    }
    memset(state, 0, sizeof(*state));
    state->sample_rate = sample_rate;
    state->class_index = class_from_rate(sample_rate);
    index = state->class_index;

    load_filter3(&state->analysis[0], analysis_first_den[index],
                 analysis_first_num[index]);
    load_filter3(&state->analysis[1], analysis_second_den[index],
                 analysis_second_num[index]);

    /* DLL startup: 40 history slots per band are filled with 2.0f. */
    for (i = 0; i < G6_CRYSTALIZER_LEGACY_HISTORY_COUNT; ++i) {
        state->detector.first_history[i] = 2.0f;
        state->detector.second_history[i] = 2.0f;
    }
    state->detector.reference_envelope = 0.03f;
    state->detector.first_band_envelope = 0.0f;
    state->detector.first_activity = 0.0f;
    state->detector.second_activity = 0.0f;
    state->detector.next_history_slot = 0;

    state->output_filter.c8 = 0.0f;
    state->output_filter.c12 = 0.0f;
    state->output_filter.c16 = 1.0f;
    state->output_filter.c20 = 0.0f;
    state->output_filter.c24 = 0.0f;
    state->output_filter.w1 = 0.0f;
    state->output_filter.w2 = 0.0f;
    state->window_phase = 0;
    state->window_enabled = 1;
    state->dry_mix_enabled = 1;

    return g6_crystalizer_set_strength(state, 50);
}

int g6_crystalizer_set_strength(G6CrystalizerState *state, unsigned strength)
{
    G6CrystalizerLegacyParameters parameters;
    if (state == NULL || strength > 100) {
        return 0;
    }
    if (!g6_crystalizer_legacy_parameters(&parameters, strength)) {
        return 0;
    }
    state->strength = strength;
    for (size_t i = 0; i < G6_CRYSTALIZER_LEGACY_PARAMETER_COUNT; ++i) {
        state->params.value[i] = parameters.value[i];
    }
    return 1;
}

void g6_crystalizer_compute_filters(G6CrystalizerState *state,
                                    float first_activity,
                                    float second_activity)
{
    const float s = state->params.value[state->class_index];
    const float t = state->params.value[5 + state->class_index];
    const float a = 1.0f + 4.0f * state->params.value[11] * second_activity;
    const float b = 1.0f + 4.0f * state->params.value[10] * first_activity;
    float x;
    float y;

    state->one_pole.c1 = t;
    state->one_pole.c2 = 0.5f * ((a + 1.0f) - t * (1.0f - a));
    state->one_pole.c3 = 0.5f * ((1.0f - a) - t * (a + 1.0f));

    state->biquad.c8 = -2.0f * s;
    state->biquad.c12 = -(s * s);
    x = 0.5f * ((b + 1.0f) - s * (1.0f - b));
    y = 0.5f * (s * (b + 1.0f) - (1.0f - b));
    state->biquad.c16 = x * x;
    state->biquad.c20 = 2.0f * x * y;
    state->biquad.c24 = y * y;
}

void g6_crystalizer_set_output_filter(G6CrystalizerState *state, float c8,
                                      float c12, float c16, float c20,
                                      float c24)
{
    if (state == NULL) {
        return;
    }
    state->output_filter.c8 = c8;
    state->output_filter.c12 = c12;
    state->output_filter.c16 = c16;
    state->output_filter.c20 = c20;
    state->output_filter.c24 = c24;
    for (size_t channel = 0; channel < G6_CRYSTALIZER_MAX_CHANNELS; ++channel) {
        state->output_w1[channel] = 0.0f;
        state->output_w2[channel] = 0.0f;
    }
}

static float hann256(unsigned index)
{
    return 0.5f * (1.0f - cosf(6.28318548f * (float)(index % 256u) / 256.0f));
}

static float ramp128(unsigned index)
{
    const float s = sinf(3.14159265f * (float)(index % 128u) / 128.0f);
    return 1.0f / (1.0f - 0.5f * s * s);
}

float g6_crystalizer_window_gain(unsigned index)
{
    return hann256(index) * ramp128(index);
}

static float one_pole_run(G6CrystalizerOnePole *filter, float *state, float x)
{
    const float w = filter->c1 * *state + x;
    *state = filter->c3 * *state + filter->c2 * w;
    return *state;
}

static float biquad_run(G6CrystalizerBiquad *filter, float *w1, float *w2,
                        float x)
{
    const float w = x + filter->c8 * *w1 + filter->c12 * *w2;
    const float y = filter->c16 * w + filter->c20 * *w1 + filter->c24 * *w2;
    *w2 = *w1;
    *w1 = w;
    return y;
}

int g6_crystalizer_process_interleaved(G6CrystalizerState *state, float *samples,
                                       size_t frames, size_t channels)
{
    float mono[G6_CRYSTALIZER_BLOCK_MAX];
    float first[G6_CRYSTALIZER_BLOCK_MAX];
    float second[G6_CRYSTALIZER_BLOCK_MAX];
    size_t position = 0;

    if (state == NULL || samples == NULL || channels == 0 ||
        frames > SIZE_MAX / channels / sizeof(*samples)) {
        return 0;
    }

    while (position < frames) {
        size_t count = frames - position;
        size_t active_channels = channels;

        if (count > G6_CRYSTALIZER_BLOCK_MAX) {
            count = G6_CRYSTALIZER_BLOCK_MAX;
        }
        if (active_channels > G6_CRYSTALIZER_MAX_CHANNELS) {
            active_channels = G6_CRYSTALIZER_MAX_CHANNELS;
        }

        for (size_t i = 0; i < count; ++i) {
            const float *frame = &samples[(position + i) * channels];
            const float left = frame[0];
            const float right = (channels > 1) ? frame[1] : frame[0];
            const float mixed = (channels > 1) ? (left + right) * 0.75f : left;
            mono[i] = mixed;
            first[i] = filter3_run(&state->analysis[0], mixed);
            second[i] = filter3_run(&state->analysis[1], mixed);
        }

        if (!g6_crystalizer_legacy_analyze_block(
                &state->detector, &state->params,
                mono, first, second, count)) {
            return 0;
        }
        g6_crystalizer_compute_filters(state, state->detector.first_activity,
                                       state->detector.second_activity);

        for (size_t channel = 0; channel < active_channels; ++channel) {
            float scratch[256];
            for (size_t i = 0; i < 128; ++i) {
                scratch[i] = state->prev_input[channel][i];
            }
            for (size_t i = 0; i < 128; ++i) {
                scratch[128 + i] = (i < count)
                    ? samples[(position + i) * channels + channel] : 0.0f;
            }
            for (size_t i = 0; i < 256; ++i) {
                const float window = state->window_enabled ? hann256((unsigned)i) : 1.0f;
                scratch[i] = one_pole_run(&state->one_pole,
                                          &state->one_pole_state[channel],
                                          scratch[i] * window);
            }
            for (size_t i = 0; i < 256; ++i) {
                scratch[i] = biquad_run(&state->biquad,
                                        &state->biquad_w1[channel],
                                        &state->biquad_w2[channel], scratch[i]);
            }
            if (state->window_enabled) {
                for (size_t i = 0; i < 256; ++i) {
                    scratch[i] *= hann256((unsigned)i);
                }
            }
            /*
             * +0x1F8 (FUN_180146190): scratch[0..127] += previous block
             * from the second dry buffer. The weight bank at 0x180199E80
             * is {1,1,1,1}, so this is a plain add.
             */
            if (state->dry_mix_enabled) {
                for (size_t i = 0; i < count; ++i) {
                    scratch[i] += state->ola_b[channel][i];
                }
            }
            for (size_t i = 0; i < 128; ++i) {
                const float dry = (i < count)
                    ? samples[(position + i) * channels + channel] : 0.0f;
                state->prev_input[channel][i] = dry;
                state->ola_b[channel][i] = dry;
            }
            for (size_t i = 0; i < count; ++i) {
                float value = scratch[i];
                if (state->window_enabled) {
                    value *= ramp128((unsigned)i);
                }
                value = biquad_run(&state->output_filter,
                                   &state->output_w1[channel],
                                   &state->output_w2[channel], value);
                samples[(position + i) * channels + channel] = value;
            }
        }

        position += count;
    }
    return 1;
}
