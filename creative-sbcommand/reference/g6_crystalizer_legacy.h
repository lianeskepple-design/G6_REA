#ifndef G6_CRYSTALIZER_LEGACY_H
#define G6_CRYSTALIZER_LEGACY_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    G6_CRYSTALIZER_LEGACY_PARAMETER_COUNT = 18,
    G6_CRYSTALIZER_LEGACY_HISTORY_COUNT = 40,
    G6_CRYSTALIZER_LEGACY_BLOCK_MAX = 128
};

typedef struct {
    float value[G6_CRYSTALIZER_LEGACY_PARAMETER_COUNT];
} G6CrystalizerLegacyParameters;

typedef struct {
    float reference_envelope;
    float first_band_envelope;
    float first_history[G6_CRYSTALIZER_LEGACY_HISTORY_COUNT];
    float second_history[G6_CRYSTALIZER_LEGACY_HISTORY_COUNT];
    float first_activity;
    float second_activity;
    size_t next_history_slot;
} G6CrystalizerLegacyAnalysis;

/* Interpolate the legacy parameter bank. Valid strength is 0 through 100. */
int g6_crystalizer_legacy_parameters(G6CrystalizerLegacyParameters *out,
                                      unsigned int strength);

/* Construct the mono input for the original analysis filters. */
float g6_crystalizer_legacy_mix(float left, float right);

void g6_crystalizer_legacy_reset(G6CrystalizerLegacyAnalysis *state);

/*
 * Update the recovered analysis state for one block of up to 128 frames.
 * mono contains the mixed signal. first_filtered and second_filtered
 * contain the corresponding outputs of the two upstream analysis filters.
 * Initialize state with reset and parameters with the interpolation function.
 * Returns 1 on success or 0 on invalid input, leaving state unchanged.
 * This function computes detector state; it does not render effect PCM.
 */
int g6_crystalizer_legacy_analyze_block(
    G6CrystalizerLegacyAnalysis *state,
    const G6CrystalizerLegacyParameters *parameters,
    const float *mono, const float *first_filtered, const float *second_filtered,
    size_t frames);

#ifdef __cplusplus
}
#endif

#endif
