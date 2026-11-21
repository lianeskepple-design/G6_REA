#ifndef G6_CHAIN_CAPI_H
#define G6_CHAIN_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Opaque offline/realtime chain wrapper for the reference DSP modules:
 * Crystalizer -> Surround (6-channel only) -> Smart Volume.
 *
 * The EQ is applied in Rust before this chain; the wrapper deliberately does
 * not expose the C struct layouts. All processing is float32 interleaved and
 * is split into 128-frame blocks internally so the Surround transition ramp
 * matches the DLL's block behaviour.
 */

typedef struct G6Chain G6Chain;

G6Chain *g6_chain_create(uint32_t sample_rate, uint32_t channels);
void g6_chain_destroy(G6Chain *chain);

/* enabled=0 bypasses the stage; strength is 0..100. */
int g6_chain_set_crystalizer(G6Chain *chain, int enabled, uint32_t strength);

/* Missing-fundamental Bass reference; cutoff is 40..400 Hz. */
int g6_chain_set_bass(G6Chain *chain, int enabled, int strength,
                      float cutoff_hz);

/* 0 = reference low-pass/x2/x3, 1 = APO-like 10-tap FIR + cubic. */
int g6_chain_set_bass_mode(G6Chain *chain, int mode);

/* Dialog+ reference: 2-4 kHz band + light Wiener-style gate. */
int g6_chain_set_dialog(G6Chain *chain, int enabled, int strength);

/* enabled=0 bypasses; requires 6 channels when enabled. */
int g6_chain_set_surround(G6Chain *chain, int enabled, uint32_t mask,
                          int strength);

/* enabled=0 bypasses; mix is clamped to 0..1. */
int g6_chain_set_svm(G6Chain *chain, int enabled, float level_db, float mix);

/* Smart Volume look-ahead in seconds; the DLL clamps it to [30, 0.05*fs] samples. */
int g6_chain_set_svm_delay(G6Chain *chain, float seconds);

/* In-place interleaved float32. Returns 1 on success, 0 on failure. */
int g6_chain_process_interleaved(G6Chain *chain, float *samples, size_t frames);

#ifdef __cplusplus
}
#endif

#endif
