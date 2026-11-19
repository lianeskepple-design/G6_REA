#ifndef G6_EQ_DSP_H
#define G6_EQ_DSP_H

#include "g6_eq_presets.h"
#include "g6_eq_apo.h"

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    G6_EQ_OK = 0,
    G6_EQ_INVALID_ARGUMENT,
    G6_EQ_NYQUIST,
    G6_EQ_GAIN_RANGE,
    G6_EQ_SAMPLE_RANGE
} G6EqStatus;

typedef struct {
    double b0, b1, b2;
    double a1, a2;
} G6EqBiquad;

/* A successfully designed chain may be shared by independent channels. */
typedef struct {
    double input_gain;
    uint32_t sample_rate;
    size_t filter_count;
    G6EqBiquad filters[G6_EQ_BAND_COUNT];
} G6EqChain;

/* One state per channel. Zero-initialize or reset after a chain change. */
typedef struct {
    double history[G6_EQ_BAND_COUNT][4];
} G6EqChannelState;

/* On failure, out is unchanged. extra_gain_db is signed and is not a limiter. */
G6EqStatus g6_eq_chain_design(G6EqChain *out, const G6EqSettings *settings,
                              uint32_t sample_rate, double extra_gain_db);

void g6_eq_reset_channel(G6EqChannelState *state);

/* Requires a designed chain, initialized state, and finite PCM input. */
double g6_eq_process_sample(const G6EqChain *chain, G6EqChannelState *state,
                            double input);

/*
 * In-place float PCM, one initialized state per channel. Values above 1.0
 * remain unclipped. On a sample error, a prefix may already be processed.
 * Reset all channel states before restarting the stream after such an error.
 */
G6EqStatus g6_eq_process_interleaved(const G6EqChain *chain,
                                     G6EqChannelState *states,
                                     float *samples, size_t frames,
                                     size_t channels);

/* Includes input gain. Frequencies clamp to [0, Nyquist]. */
double g6_eq_response_db(const G6EqChain *chain, double frequency_hz);

/*
 * APO-exact chain: float32 coefficients and direct-form II state, matching
 * KSUSBAPO64.dll 0x1801469E0. The filter slots are the recovered
 * [B+0x08]..[B+0x18] values; gain conversion follows the per-type convention
 * documented in g6_eq_apo.h.
 */
typedef struct {
    float input_gain;
    uint32_t sample_rate;
    size_t filter_count;
    G6EqApoCoefficients filters[G6_EQ_BAND_COUNT];
} G6EqApoChain;

typedef struct {
    G6EqApoState bands[G6_EQ_BAND_COUNT];
} G6EqApoChannelState;

/* On failure, out is unchanged. Bands with exactly 0 dB are skipped. */
G6EqStatus g6_eq_apo_chain_design(G6EqApoChain *out, const G6EqSettings *settings,
                                  uint32_t sample_rate, double extra_gain_db);

void g6_eq_apo_reset_channel_state(G6EqApoChannelState *state);

/* Requires a designed chain, initialized state, and finite PCM input. */
float g6_eq_apo_process_chain_sample(const G6EqApoChain *chain,
                                     G6EqApoChannelState *state, float input);

/*
 * In-place float PCM with the APO-exact kernel. Same error and reset rules as
 * g6_eq_process_interleaved. Values above 1.0 remain unclipped.
 */
G6EqStatus g6_eq_apo_process_chain_interleaved(const G6EqApoChain *chain,
                                               G6EqApoChannelState *states,
                                               float *samples, size_t frames,
                                               size_t channels);

#ifdef __cplusplus
}
#endif

#endif
