#include "g6_crystalizer_legacy.h"

#include <math.h>

/*
 * KSUSBAPO64.dll 1.2.109.0, VA 0x1801f1e90 through 0x1801f1faf.
 * Each decimal literal round-trips to the original binary32 value.
 */
static const float parameter_banks[4][G6_CRYSTALIZER_LEGACY_PARAMETER_COUNT] = {
    {
        -0.903836012f, -0.950764000f, -0.954675019f, -0.977081001f, -0.988475025f,
         0.499179989f,  0.720260978f,  0.740343988f,  0.861871004f,  0.928560019f,
         0.130436003f,  0.521744013f,  0.110532001f,  0.332439989f,  0.604892015f,
         0.449999988f,  1.19998801f,   1.99987805f
    },
    {
        -0.938269973f, -0.968658984f, -0.971168995f, -0.985481024f, -0.992713988f,
         0.473345011f,  0.704512000f,  0.725588977f,  0.853541017f,  0.924100995f,
         0.152164996f,  0.608721972f,  0.0496179983f, 0.182322994f,  0.463085502f,
         0.449999988f,  3.33323193f,   2.79976797f
    },
    {
        -0.955911994f, -0.977712989f, -0.979505002f, -0.989700019f, -0.994836986f,
         0.388936013f,  0.652549028f,  0.676826000f,  0.825672984f,  0.909062028f,
         0.260872006f,  0.869593978f,  0.0406179987f, 0.0740619972f, 0.399318010f,
         0.5f,          4.13312197f,   3.59996295f
    },
    {
        -0.964829028f, -0.982259989f, -0.983690023f, -0.991811991f, -0.995896995f,
         0.358323991f,  0.633565009f,  0.658981979f,  0.815349996f,  0.903443992f,
         0.304360986f,  1.17392504f,   0.0246210005f, 0.0300810002f, 0.399318010f,
         0.200000003f,  5.19974422f,   5.46647501f
    }
};

enum {
    HISTORY_SCAN_COUNT = 35,
    FIRST_ACTIVITY_RELEASE = 12,
    SECOND_ACTIVITY_RELEASE = 13,
    ACTIVITY_SCALE = 14,
    ACTIVITY_THRESHOLD = 15
};

int g6_crystalizer_legacy_parameters(G6CrystalizerLegacyParameters *out,
                                      unsigned int strength)
{
    if (out == NULL || strength > 100) {
        return 0;
    }
    const float u = (float)strength / 100.0f;
    const float level = u <= 0.5f ? u * 0.8f : (u - 0.5f) * 0.6f * 2.0f + 0.4f;
    size_t bank;
    float weight;
    if (level <= 0.38f) {
        bank = 0;
        weight = level / 0.38f;
    } else if (level <= 0.75f) {
        bank = 1;
        weight = (level - 0.38f) / 0.37f;
    } else {
        bank = 2;
        weight = (level - 0.75f) * 4.0f;
    }
    for (size_t i = 0; i < G6_CRYSTALIZER_LEGACY_PARAMETER_COUNT; ++i) {
        out->value[i] = (1.0f - weight) * parameter_banks[bank][i]
                       + weight * parameter_banks[bank + 1][i];
    }
    return 1;
}

float g6_crystalizer_legacy_mix(float left, float right)
{
    return (left + right) * 0.75f;
}

void g6_crystalizer_legacy_reset(G6CrystalizerLegacyAnalysis *state)
{
    if (state != NULL) {
        const G6CrystalizerLegacyAnalysis empty = {0};
        *state = empty;
    }
}

static float update_activity(float level, float maximum, float previous,
                             float scale, float threshold, float release)
{
    float activity = fminf(0.5f, fmaxf(0.0f, level - 0.8f * maximum - threshold) * scale);
    if (previous > activity) {
        activity = (1.0f - release) * previous + release * activity;
        if (activity < 1.0e-10f) {
            activity = 0.0f;
        }
    }
    return activity;
}

int g6_crystalizer_legacy_analyze_block(
    G6CrystalizerLegacyAnalysis *state,
    const G6CrystalizerLegacyParameters *parameters,
    const float *mono, const float *first_filtered, const float *second_filtered,
    size_t frames)
{
    if (state == NULL || parameters == NULL || mono == NULL ||
        first_filtered == NULL || second_filtered == NULL || frames == 0 ||
        frames > G6_CRYSTALIZER_LEGACY_BLOCK_MAX ||
        state->next_history_slot >= G6_CRYSTALIZER_LEGACY_HISTORY_COUNT) {
        return 0;
    }

    float reference_peak = 0.0f;
    float first_sum = 0.0f;
    float second_sum = 0.0f;
    for (size_t i = 0; i < frames; ++i) {
        if (!isfinite(mono[i]) || !isfinite(first_filtered[i]) ||
            !isfinite(second_filtered[i])) {
            return 0;
        }
        reference_peak = fmaxf(reference_peak, fabsf(mono[i]));
        first_sum += fabsf(first_filtered[i]);
        second_sum += fabsf(second_filtered[i]);
    }
    if (!isfinite(first_sum) || !isfinite(second_sum)) {
        return 0;
    }

    const float old = state->reference_envelope;
    const float envelope = reference_peak > old
        ? 0.99f * old + 0.01f * reference_peak
        : 0.999f * old + 0.001f * reference_peak;
    const float reference = fmaxf(0.03f, envelope);
    const float first_ratio = first_sum / reference;
    const float second_ratio = second_sum / reference;
    if (!isfinite(first_ratio) || !isfinite(second_ratio)) {
        return 0;
    }
    state->reference_envelope = reference;

    const float first = 0.7f * sqrtf(sqrtf(first_ratio));
    const float second = 1.3f * sqrtf(sqrtf(second_ratio));
    const float previous = state->first_band_envelope;
    state->first_band_envelope = first > previous
        ? 0.2f * previous + 0.8f * first
        : 0.9f * previous + 0.1f * first;

    const size_t slot = state->next_history_slot;
    state->first_history[slot] = state->first_band_envelope;
    state->second_history[slot] = second;
    state->next_history_slot = (slot + 1) % G6_CRYSTALIZER_LEGACY_HISTORY_COUNT;

    /* Scan 35 forward from the next write slot, skipping the newest five. */
    size_t scan = state->next_history_slot;
    float first_maximum = 0.0f;
    float second_maximum = 0.0f;
    for (size_t i = 0; i < HISTORY_SCAN_COUNT; ++i) {
        first_maximum = fmaxf(first_maximum, state->first_history[scan]);
        second_maximum = fmaxf(second_maximum, state->second_history[scan]);
        scan = (scan + 1) % G6_CRYSTALIZER_LEGACY_HISTORY_COUNT;
    }

    const float scale = parameters->value[ACTIVITY_SCALE];
    const float threshold = parameters->value[ACTIVITY_THRESHOLD];
    state->first_activity = update_activity(
        state->first_band_envelope, first_maximum, state->first_activity,
        scale, threshold, parameters->value[FIRST_ACTIVITY_RELEASE]);
    state->second_activity = update_activity(
        second, second_maximum, state->second_activity,
        scale, threshold, parameters->value[SECOND_ACTIVITY_RELEASE]);
    return 1;
}
