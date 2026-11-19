#include "g6_eq_apo.h"

#include <math.h>

#define G6_TAU_F32 6.28318548f
#define G6_SHELF_MID 0.707f
#define G6_SHELF_BASE 1.4f
#define G6_LOW_SHELF_WIDTH 0.4f
#define G6_HIGH_SHELF_WIDTH 0.45f

static int valid_design(float sample_rate, float frequency, float ratio)
{
    if (!(sample_rate > 0.0f) || !isfinite(sample_rate)) {
        return 0;
    }
    if (!(frequency > 0.0f) || !isfinite(frequency) ||
        frequency >= sample_rate * 0.5f) {
        return 0;
    }
    if (!(ratio > 0.0f) || !isfinite(ratio)) {
        return 0;
    }
    return 1;
}

static int valid_gain(float gain)
{
    return isfinite(gain) && gain >= 0.0f;
}

static int linear_from_db(float gain_db, float divisor, float *out)
{
    float linear;
    if (!isfinite(gain_db)) {
        return G6_EQ_APO_INVALID_ARGUMENT;
    }
    linear = powf(10.0f, gain_db / divisor);
    if (!isfinite(linear) || linear <= 0.0f) {
        return G6_EQ_APO_GAIN_RANGE;
    }
    *out = linear;
    return G6_EQ_APO_OK;
}

static void clear_coeff(G6EqApoCoefficients *out)
{
    out->c8 = 0.0f;
    out->c12 = 0.0f;
    out->c16 = 0.0f;
    out->c20 = 0.0f;
    out->c24 = 0.0f;
}

static void bypass_coeff(G6EqApoCoefficients *out)
{
    out->c8 = 0.0f;
    out->c12 = 0.0f;
    out->c16 = 1.0f;
    out->c20 = 0.0f;
    out->c24 = 0.0f;
}

int g6_eq_apo_peak_linear(float gain, float sample_rate, float frequency,
                          float ratio, G6EqApoCoefficients *out)
{
    float a, w, w0, k, t, c, half_low, half_high, edge;

    if (out == NULL || !valid_design(sample_rate, frequency, ratio) ||
        !valid_gain(gain)) {
        return G6_EQ_APO_INVALID_ARGUMENT;
    }
    if (gain == 0.0f) {
        clear_coeff(out);
        return G6_EQ_APO_OK;
    }
    if (gain == 1.0f) {
        bypass_coeff(out);
        return G6_EQ_APO_OK;
    }

    a = sqrtf(gain);
    w = frequency / sample_rate;
    w0 = w * G6_TAU_F32;
    k = tanf((w0 / ratio) * 0.5f) / a;
    t = (k - 1.0f) / (k + 1.0f);
    c = cosf(w0);
    half_low = 0.5f * (1.0f - a);
    half_high = 0.5f * (1.0f + a);
    edge = (1.0f - t) * c;

    out->c8 = edge;
    out->c12 = t;
    out->c16 = half_high - half_low * t;
    out->c20 = -edge;
    out->c24 = half_low - half_high * t;

    if (!isfinite(out->c8) || !isfinite(out->c12) || !isfinite(out->c16) ||
        !isfinite(out->c20) || !isfinite(out->c24)) {
        return G6_EQ_APO_GAIN_RANGE;
    }
    return G6_EQ_APO_OK;
}

int g6_eq_apo_low_shelf_linear(float gain, float sample_rate, float frequency,
                               float ratio, G6EqApoCoefficients *out)
{
    float a, w0, warped, s, c, denominator, root, sroot;
    float a_minus_1, a_plus_1, scaled, d;

    if (out == NULL || !valid_design(sample_rate, frequency, ratio) ||
        !valid_gain(gain)) {
        return G6_EQ_APO_INVALID_ARGUMENT;
    }
    if (gain == 0.0f) {
        clear_coeff(out);
        return G6_EQ_APO_OK;
    }
    if (gain == 1.0f) {
        bypass_coeff(out);
        return G6_EQ_APO_OK;
    }

    a = sqrtf(gain);
    w0 = (frequency / sample_rate) * G6_TAU_F32;
    warped = w0 / ratio;
    warped *= 2.0f;
    warped /= 3.0f;
    warped += w0;
    s = sinf(warped);
    c = cosf(warped);

    denominator = 1.0f / ratio;
    denominator -= G6_SHELF_MID;
    denominator *= G6_LOW_SHELF_WIDTH;
    denominator = G6_SHELF_BASE - denominator;

    root = (gain + 1.0f) / denominator;
    root -= gain;
    root += a * 2.0f;
    root -= 1.0f;
    if (!(root >= 0.0f) || !isfinite(root)) {
        return G6_EQ_APO_DEGENERATE;
    }
    sroot = sqrtf(root);

    a_minus_1 = a - 1.0f;
    a_plus_1 = a + 1.0f;
    scaled = sroot * s;
    d = a_minus_1 * c + a_plus_1 + scaled;
    if (!isfinite(d) || d == 0.0f) {
        return G6_EQ_APO_DEGENERATE;
    }

    out->c8 = 2.0f * (a_plus_1 * c + a_minus_1) / d;
    out->c12 = -((a_minus_1 * c + a_plus_1) - scaled) / d;
    out->c16 = a * (scaled + a_plus_1 - a_minus_1 * c) / d;
    out->c20 = 2.0f * a * (a_minus_1 - a_plus_1 * c) / d;
    out->c24 = a * (a_plus_1 - a_minus_1 * c - scaled) / d;

    if (!isfinite(out->c8) || !isfinite(out->c12) || !isfinite(out->c16) ||
        !isfinite(out->c20) || !isfinite(out->c24)) {
        return G6_EQ_APO_GAIN_RANGE;
    }
    return G6_EQ_APO_OK;
}

int g6_eq_apo_high_shelf_linear(float gain, float sample_rate, float frequency,
                                float ratio, G6EqApoCoefficients *out)
{
    float a, w0, warped, s, c, denominator, root, sroot;
    float a_minus_1, a_plus_1, scaled, d;

    if (out == NULL || !valid_design(sample_rate, frequency, ratio) ||
        !valid_gain(gain)) {
        return G6_EQ_APO_INVALID_ARGUMENT;
    }
    if (gain == 0.0f) {
        clear_coeff(out);
        return G6_EQ_APO_OK;
    }
    if (gain == 1.0f) {
        bypass_coeff(out);
        return G6_EQ_APO_OK;
    }

    a = sqrtf(gain);
    w0 = (frequency / sample_rate) * G6_TAU_F32;
    warped = w0 - (w0 / ratio) * G6_HIGH_SHELF_WIDTH;
    s = sinf(warped);
    c = cosf(warped);

    denominator = 1.0f / ratio;
    denominator -= G6_SHELF_MID;
    denominator *= 2.0f;
    denominator = G6_SHELF_BASE - denominator;

    root = (gain + 1.0f) / denominator;
    root -= gain;
    root += a * 2.0f;
    root -= 1.0f;
    if (!(root >= 0.0f) || !isfinite(root)) {
        return G6_EQ_APO_DEGENERATE;
    }
    sroot = sqrtf(root);

    a_minus_1 = a - 1.0f;
    a_plus_1 = a + 1.0f;
    scaled = sroot * s;
    d = a_plus_1 - a_minus_1 * c + scaled;
    if (!isfinite(d) || d == 0.0f) {
        return G6_EQ_APO_DEGENERATE;
    }

    out->c8 = -2.0f * (a_minus_1 - a_plus_1 * c) / d;
    out->c12 = -((a_plus_1 - a_minus_1 * c) - scaled) / d;
    out->c16 = a * (scaled + a_plus_1 + a_minus_1 * c) / d;
    out->c20 = -2.0f * a * (a_plus_1 * c + a_minus_1) / d;
    out->c24 = a * (a_plus_1 + a_minus_1 * c - scaled) / d;

    if (!isfinite(out->c8) || !isfinite(out->c12) || !isfinite(out->c16) ||
        !isfinite(out->c20) || !isfinite(out->c24)) {
        return G6_EQ_APO_GAIN_RANGE;
    }
    return G6_EQ_APO_OK;
}

int g6_eq_apo_peak_db(float gain_db, float sample_rate, float frequency,
                      float ratio, G6EqApoCoefficients *out)
{
    float gain;
    int status = linear_from_db(gain_db, 10.0f, &gain);
    if (status != G6_EQ_APO_OK) {
        return status;
    }
    return g6_eq_apo_peak_linear(gain, sample_rate, frequency, ratio, out);
}

int g6_eq_apo_low_shelf_db(float gain_db, float sample_rate, float frequency,
                           G6EqApoCoefficients *out)
{
    float gain;
    int status = linear_from_db(gain_db, 20.0f, &gain);
    if (status != G6_EQ_APO_OK) {
        return status;
    }
    return g6_eq_apo_low_shelf_linear(gain, sample_rate, frequency, 1.0f, out);
}

int g6_eq_apo_high_shelf_db(float gain_db, float sample_rate, float frequency,
                            G6EqApoCoefficients *out)
{
    float gain;
    int status = linear_from_db(gain_db, 20.0f, &gain);
    if (status != G6_EQ_APO_OK) {
        return status;
    }
    return g6_eq_apo_high_shelf_linear(gain, sample_rate, frequency, 1.0f, out);
}

void g6_eq_apo_reset(G6EqApoState *state)
{
    if (state != NULL) {
        state->w1 = 0.0f;
        state->w2 = 0.0f;
    }
}

float g6_eq_apo_process(const G6EqApoCoefficients *coeff, G6EqApoState *state,
                        float x)
{
    float w = x + coeff->c8 * state->w1 + coeff->c12 * state->w2;
    float y = coeff->c16 * w + coeff->c20 * state->w1 + coeff->c24 * state->w2;
    state->w2 = state->w1;
    state->w1 = w;
    return y;
}
