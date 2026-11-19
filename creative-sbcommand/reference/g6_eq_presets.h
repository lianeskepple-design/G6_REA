#ifndef G6_EQ_PRESETS_H
#define G6_EQ_PRESETS_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

enum { G6_EQ_BAND_COUNT = 10 };

typedef struct {
    float preamp_db;
    float gain_db[G6_EQ_BAND_COUNT];
} G6EqSettings;

typedef struct {
    const char *id;
    int order;
    int type;
    int speaker_method;
    G6EqSettings speaker;
    G6EqSettings headphone;
} G6EqPreset;

/* Gain indices correspond to the shared band frequencies in Hz. */
extern const unsigned int g6_eq_band_hz[G6_EQ_BAND_COUNT];
extern const G6EqPreset g6_eq_presets[];
extern const size_t g6_eq_preset_count;

/* Case-sensitive factory ID lookup. Returns NULL if the ID is absent. */
const G6EqPreset *g6_eq_find_preset(const char *id);

#ifdef __cplusplus
}
#endif

#endif
