#ifndef G6_SVM_H
#define G6_SVM_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Legacy Gaming Smart Volume (AGC) reference, reconstructed from
 * KSUSBAPO64.dll 1.2.109.0:
 *
 *   init                 0x180092810 (sample rate, 2 x 0x2590 state objects)
 *   detector filter     0x180092a50 (k1/k2 by sample rate)
 *   parameter update    0x180092da0 (time constants, look-ahead)
 *   sample loop         0x180092f30 (filter, energy, max, log gain, delay taps)
 *   mode/threshold      0x1800935c0 / 0x1800936c0 / 0x180093710
 *
 * Recovered constants:
 *   energy smoothing 40 ms, gain smoothing 10 ms, ramp 80 ms,
 *   attack 1 - 10^(-1/(0.0018638*fs)), release 1 - 10^(-1/(8*fs)),
 *   look-ahead min(0.05*fs, 30) samples, default threshold log10 = -6.2
 *   (level_db = -50 through 0x180093710's (level_db-12)/10 mapping).
 *
 * The log-domain gain law below uses the recovered 0.3 slope; the final
 * two-tap crossfade and per-mode curve refinements are documented as open.
 */

#define G6_SVM_MAX_CHANNELS 8
#define G6_SVM_RING 2400

typedef struct {
    float prev_x;
    float prev_filtered;
    float energy;
    float ring[G6_SVM_RING];
    unsigned write;
} G6SvmChannel;

typedef struct {
    unsigned sample_rate;
    unsigned channels;
    unsigned mask;
    unsigned lookahead;
    float k1;
    float k2;
    float alpha;
    float beta;
    float attack;
    float release;
    float ramp_step;
    float threshold;
    float level;
    float gain;
    float mix;
    G6SvmChannel channel[G6_SVM_MAX_CHANNELS];
} G6SvmState;

int g6_svm_init(G6SvmState *state, unsigned sample_rate, unsigned channels);

/* level_db is mapped as threshold = (level_db - 12) / 10, as 0x180093710. */
int g6_svm_set_level(G6SvmState *state, float level_db);

/* mix = 1.0 applies the compressor; 0.0 bypasses it. */
void g6_svm_set_mix(G6SvmState *state, float mix);

/* Non-interleaved channel buffers; returns 1 on success. */
int g6_svm_process(G6SvmState *state, float *const *buffers, size_t frames);

#ifdef __cplusplus
}
#endif

#endif
