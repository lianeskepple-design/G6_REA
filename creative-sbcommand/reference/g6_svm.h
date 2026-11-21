#ifndef G6_SVM_H
#define G6_SVM_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Smart Volume (CPlugAGCCompressor), decoded from KSUSBAPO64.dll 1.2.109.0.
 *
 * Anchors (all in KSUSBAPO64.dll):
 *   ctor                     0x180092810  sets the constants below and the defaults
 *   reset                    0x180092bc0  clears the delay rings, gains = 1.0
 *   detector coefficients    0x180092a50  13-entry (k1,k2) table by sample rate
 *   parameter update         0x180092da0  sample-rate dependent time constants
 *   sample loop              0x180092f30  detector -> gain -> two-tap delayed output
 *   enable / bypass          0x180093420  wet and makeup crossfade
 *   delay change             0x1800935c0  clamp + ping-pong crossfade target
 *   attack / release setters 0x180093520 / 0x1800936c0
 *
 * Constants read straight from the image (see tools/apo-eq-coeff/pe_read.py):
 *   pow/log base      10.0        (0x18018870c)
 *   threshold         -6.2        (ctor stores 0xc0c66666 at +0x4b98) in log10(energy)
 *   gain-law ratio    0.4         (0x18009289f, 0x3ecccccd) -> 0.6 gain slope
 *   energy smoothing  0.04 s      (0x18019683c)
 *   makeup smoothing  0.01 s      (0x18018bfc0, +0x4ba4)
 *   attack            0.004 s * 0.46593979 = 1.86376 ms (0x180196848)
 *   release           8 s         (ctor 0x41000000 at +0x4bf4)
 *   crossfade step    1/(0.08*fs) (0x180196844)
 *   delay clamp       30 .. 0.05*fs samples (0x180196854 / 0x180196840)
 *   delay default     0.02 s      (ctor 0x3ca3d70a at +0x4bf0) -> 960 samples @ 48 kHz
 *   makeup gain       10^1.1      (ctor: pow(10, 0x18019684c = 1.1)) = 12.589
 *   delay ring        2400 floats per channel (0x960, stride 0x2590)
 *
 * The DLL never touches +0x4b98 or +0x4b8c outside the constructor, so the gain law
 * has no UI level parameter: Smart Volume is on/off, and the only exposed knobs are
 * enable (wet/makeup crossfade), delay and the attack/release time constants.
 *
 * Differences from the shipped implementation that this file replaces:
 *   - look-ahead is 20 ms by default, not 30 samples;
 *   - the output is a two-tap crossfade between delays +0x4b78 / +0x4b7c;
 *   - three cascaded smoothers (gain, gain with attack, makeup/wet at 10 ms);
 *   - a +22 dB (10^1.1) makeup gain when enabled.
 *
 * The log10/pow calls go through libm, while the DLL uses its own vectorised
 * approximations (0x1800dc0b0 / 0x1800d96d0), so runtime gain values match to a
 * tolerance rather than bit for bit. The fixed coefficients are compared exactly.
 */

#define G6_SVM_MAX_CHANNELS 8
#define G6_SVM_RING 2400

typedef struct {
    float prev_x;
    float prev_filtered;
    float energy;
    unsigned write;
    float ring[G6_SVM_RING];
} G6SvmChannel;

typedef struct {
    unsigned sample_rate;
    unsigned channels;

    /* detector + time constants, from 0x180092a50 / 0x180092da0 */
    float k1;
    float k2;
    float alpha_energy; /* +0x4ba0 */
    float alpha_makeup; /* +0x4ba4 */
    float ramp_step;    /* +0x4bc8 */
    float attack;       /* +0x4b84 */
    float release;      /* +0x4b88 */
    float threshold;    /* +0x4b98, log10 of energy */
    float ratio;        /* +0x4b8c */
    float wet;          /* +0x4b9c: 1.0 enabled, 0.0 bypassed */
    float makeup;       /* +0x4b90: 10^1.1 enabled, 1.0 bypassed */
    float gain1;        /* +0x4be4 */
    float gain2;        /* +0x4be8 */
    float mix_gain;     /* +0x4bec, applied one frame late */
    float delay_seconds;/* +0x4bf0 */
    float delay_a;      /* +0x4b78 */
    float delay_b;      /* +0x4b7c */
    float crossfade;    /* +0x4bc4, 1.0 = tap A, 0.0 = tap B */
    int target_up;      /* +0x4bb8 */
    int target_down;    /* +0x4bb9 */
    int enabled;        /* +0x4b80 */

    /*
     * Not present in the DLL: final dry/wet blend so the CLI can bypass without the
     * 30-sample bypass delay. 1.0 = processed, 0.0 = untouched input.
     */
    float user_mix;

    G6SvmChannel channel[G6_SVM_MAX_CHANNELS];
} G6SvmState;

int g6_svm_init(G6SvmState *state, unsigned sample_rate, unsigned channels);

/* Emulates 0x180093420: wet/makeup crossfade plus the delay reset. */
int g6_svm_set_enabled(G6SvmState *state, int enabled);

/* Emulates 0x1800935c0: seconds clamped to [30 samples, 0.05*fs] and crossfaded. */
int g6_svm_set_delay_seconds(G6SvmState *state, float seconds);

/* Emulates 0x180093520 / 0x1800936c0: attack and release in seconds. */
int g6_svm_set_attack_seconds(G6SvmState *state, float seconds);
int g6_svm_set_release_seconds(G6SvmState *state, float seconds);

/*
 * LEGACY, not derived from the DLL: the old header mapped level_db onto the
 * threshold as (level_db - 12) / 10. Kept so existing CLI flags keep working;
 * new code should use g6_svm_set_threshold_dbfs.
 */
int g6_svm_set_level(G6SvmState *state, float level_db);

/* threshold in dBFS of the energy detector: -62 dBFS reproduces the -6.2 default. */
int g6_svm_set_threshold_dbfs(G6SvmState *state, float threshold_dbfs);

/* 1.0 applies the compressor, 0.0 returns the untouched input. Extension. */
void g6_svm_set_mix(G6SvmState *state, float mix);

/*
 * Put a freshly initialised state into steady state: both taps at the current delay
 * and no pending crossfade. `0x180092810` starts a 50 ms -> 20 ms fade on power-up;
 * a client that is not emulating power-on does not need that transient, and without
 * settling the chain would keep a 2400-sample tap alive and look "active" forever.
 */
void g6_svm_settle(G6SvmState *state);

/* 1 while the compressor still has a wet/makeup crossfade to finish. */
int g6_svm_active(const G6SvmState *state);

/*
 * Non-interleaved channel buffers, processed in place. The DLL keeps separate input
 * (+0x18) and output (+0x3c) channel counts; both are equal here.
 */
int g6_svm_process(G6SvmState *state, float *const *buffers, size_t frames);

#ifdef __cplusplus
}
#endif

#endif
