#ifndef G6_BIQUAD_H
#define G6_BIQUAD_H

#include <math.h>

/* Small RBJ biquad shared by the Bass / Dialog+ reference stages. */
typedef struct {
    float b0, b1, b2;
    float a1, a2;
    float z1, z2;
} G6Biquad;

static inline void g6_biquad_reset(G6Biquad *b)
{
    b->z1 = 0.0f;
    b->z2 = 0.0f;
}

static inline float g6_biquad_run(G6Biquad *b, float x)
{
    const float y = b->b0 * x + b->z1;
    b->z1 = b->b1 * x - b->a1 * y + b->z2;
    b->z2 = b->b2 * x - b->a2 * y;
    return y;
}

static inline void g6_biquad_lowpass(G6Biquad *b, float fs, float f, float q)
{
    float w0;
    float alpha;
    float cosw;
    float a0;

    if (f < 10.0f) {
        f = 10.0f;
    }
    if (f > fs * 0.45f) {
        f = fs * 0.45f;
    }
    w0 = 6.28318548f * f / fs;
    cosw = cosf(w0);
    alpha = sinf(w0) / (2.0f * q);
    a0 = 1.0f + alpha;
    b->b0 = (1.0f - cosw) * 0.5f / a0;
    b->b1 = (1.0f - cosw) / a0;
    b->b2 = b->b0;
    b->a1 = -2.0f * cosw / a0;
    b->a2 = (1.0f - alpha) / a0;
    g6_biquad_reset(b);
}

static inline void g6_biquad_highpass(G6Biquad *b, float fs, float f, float q)
{
    float w0;
    float alpha;
    float cosw;
    float a0;

    if (f < 10.0f) {
        f = 10.0f;
    }
    if (f > fs * 0.45f) {
        f = fs * 0.45f;
    }
    w0 = 6.28318548f * f / fs;
    cosw = cosf(w0);
    alpha = sinf(w0) / (2.0f * q);
    a0 = 1.0f + alpha;
    b->b0 = (1.0f + cosw) * 0.5f / a0;
    b->b1 = -(1.0f + cosw) / a0;
    b->b2 = b->b0;
    b->a1 = -2.0f * cosw / a0;
    b->a2 = (1.0f - alpha) / a0;
    g6_biquad_reset(b);
}

#endif
