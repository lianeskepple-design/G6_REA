#ifndef G6_EQ_APO_H
#define G6_EQ_APO_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Instruction-level port of the G6 APO 10-band EQ coefficient functions.
 *
 * Evidence: KSUSBAPO64.dll 1.2.109.0 (SHA-256
 * f7c649544e8d738df4fb814609f2adc48fbe95091a1ae11ac842910020037156).
 *
 *   peak coefficients       VA 0x180087120
 *   low shelf coefficients  VA 0x180087970
 *   high shelf coefficients VA 0x1800887E0
 *   per-sample kernel       VA 0x1801469E0 (direct form II)
 *
 * The band object stores five float32 slots consumed by the kernel:
 *   +0x08 = c8  = -a1
 *   +0x0c = c12 = -a2
 *   +0x10 = c16 = b0
 *   +0x14 = c20 = b1
 *   +0x18 = c24 = b2
 * and a two-float state at +0x1c / +0x20:
 *   w[n] = x[n] + c8*w[n-1] + c12*w[n-2]
 *   y[n] = c16*w[n] + c20*w[n-1] + c24*w[n-2]
 *
 * All coefficient arithmetic is float32. On MSVC use /fp:strict when
 * comparing against the Rust/reference implementation.
 *
 * Gain convention: the DLL stores a linear value in [B+0x28]. The recovered
 * peak formula reaches sqrt(gain) at its centre and the shelf formulas reach
 * gain at their plateau. These entry points take the UI-visible dB value and
 * use the per-type conversion that makes the labelled dB correct:
 *   peaking: 10^(dB/10) (power);
 *   shelves: 10^(dB/20) (amplitude).
 * Raw-linear entry points are provided for callers that recover the real DLL
 * conversion.
 */

enum {
    G6_EQ_APO_OK = 0,
    G6_EQ_APO_INVALID_ARGUMENT = 1,
    G6_EQ_APO_NYQUIST = 2,
    G6_EQ_APO_GAIN_RANGE = 3,
    G6_EQ_APO_DEGENERATE = 4
};

typedef struct {
    float c8;
    float c12;
    float c16;
    float c20;
    float c24;
} G6EqApoCoefficients;

typedef struct {
    float w1;
    float w2;
} G6EqApoState;

/* UI-visible dB entry points. */
int g6_eq_apo_peak_db(float gain_db, float sample_rate, float frequency,
                      float ratio, G6EqApoCoefficients *out);
int g6_eq_apo_low_shelf_db(float gain_db, float sample_rate, float frequency,
                           G6EqApoCoefficients *out);
int g6_eq_apo_high_shelf_db(float gain_db, float sample_rate, float frequency,
                            G6EqApoCoefficients *out);

/* Raw linear-gain entry points (value stored in [B+0x28]). */
int g6_eq_apo_peak_linear(float gain, float sample_rate, float frequency,
                          float ratio, G6EqApoCoefficients *out);
int g6_eq_apo_low_shelf_linear(float gain, float sample_rate, float frequency,
                               float ratio, G6EqApoCoefficients *out);
int g6_eq_apo_high_shelf_linear(float gain, float sample_rate, float frequency,
                                float ratio, G6EqApoCoefficients *out);

void g6_eq_apo_reset(G6EqApoState *state);
float g6_eq_apo_process(const G6EqApoCoefficients *coeff, G6EqApoState *state,
                        float x);

#ifdef __cplusplus
}
#endif

#endif
