#include "g6_eq_dsp.h"

#include <float.h>
#include <math.h>
#include <string.h>

static const double tau = 6.283185307179586476925286766559;

static G6EqStatus design_band(G6EqBiquad *filter, size_t index,
                              uint32_t sample_rate, double gain_db)
{
    const double frequency = g6_eq_band_hz[index];
    if (frequency >= sample_rate / 2.0) {
        return G6_EQ_NYQUIST;
    }

    const double a = pow(10.0, gain_db / 40.0);
    if (!isfinite(a) || a <= 0.0) {
        return G6_EQ_GAIN_RANGE;
    }
    const double w = tau * frequency / sample_rate;
    const double c = cos(w);
    double b0, b1, b2, a0, a1, a2;

    if (index != 0 && index + 1 != G6_EQ_BAND_COUNT) {
        const double ratio = g6_eq_band_hz[index + 1] / frequency;
        const double alpha = tan(w / (2.0 * ratio));
        b0 = 1.0 + alpha * a;
        b1 = -2.0 * c;
        b2 = 1.0 - alpha * a;
        a0 = 1.0 + alpha / a;
        a1 = b1;
        a2 = 1.0 - alpha / a;
    } else {
        /* RBJ low/high shelf, S = 1, as in the Rust software EQ. */
        const double alpha = sin(w) / 2.0 * sqrt(2.0);
        const double t = 2.0 * sqrt(a) * alpha;
        const double p = a - 1.0;
        const double m = a + 1.0;
        if (index == 0) {
            b0 = a * (m - p * c + t);
            b1 = 2.0 * a * (p - m * c);
            b2 = a * (m - p * c - t);
            a0 = m + p * c + t;
            a1 = -2.0 * (p + m * c);
            a2 = m + p * c - t;
        } else {
            b0 = a * (m + p * c + t);
            b1 = -2.0 * a * (p + m * c);
            b2 = a * (m + p * c - t);
            a0 = m - p * c + t;
            a1 = 2.0 * (p - m * c);
            a2 = m - p * c - t;
        }
    }

    if (!isfinite(a0) || a0 == 0.0) {
        return G6_EQ_GAIN_RANGE;
    }
    const G6EqBiquad design = { b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0 };
    if (!isfinite(design.b0) || !isfinite(design.b1) ||
        !isfinite(design.b2) || !isfinite(design.a1) || !isfinite(design.a2)) {
        return G6_EQ_GAIN_RANGE;
    }
    *filter = design;
    return G6_EQ_OK;
}

G6EqStatus g6_eq_chain_design(G6EqChain *out, const G6EqSettings *settings,
                              uint32_t sample_rate, double extra_gain_db)
{
    if (out == NULL || settings == NULL || sample_rate == 0) {
        return G6_EQ_INVALID_ARGUMENT;
    }

    const double total_db = settings->preamp_db + extra_gain_db;
    if (!isfinite(total_db)) {
        return G6_EQ_GAIN_RANGE;
    }
    const double gain = pow(10.0, total_db / 20.0);
    if (!isfinite(gain) || gain > FLT_MAX) {
        return G6_EQ_GAIN_RANGE;
    }

    G6EqChain design = { 0 };
    design.input_gain = gain;
    design.sample_rate = sample_rate;
    for (size_t i = 0; i < G6_EQ_BAND_COUNT; ++i) {
        const double band_db = settings->gain_db[i];
        if (!isfinite(band_db)) {
            return G6_EQ_GAIN_RANGE;
        }
        if (band_db == 0.0) {
            continue;
        }
        const G6EqStatus status = design_band(&design.filters[design.filter_count],
                                               i, sample_rate, band_db);
        if (status != G6_EQ_OK) {
            return status;
        }
        ++design.filter_count;
    }
    *out = design;
    return G6_EQ_OK;
}

void g6_eq_reset_channel(G6EqChannelState *state)
{
    memset(state, 0, sizeof(*state));
}

double g6_eq_process_sample(const G6EqChain *chain, G6EqChannelState *state,
                            double input)
{
    double x = input * chain->input_gain;
    for (size_t i = 0; i < chain->filter_count; ++i) {
        const G6EqBiquad *f = &chain->filters[i];
        double *z = state->history[i];
        const double y = f->b0 * x + f->b1 * z[0] + f->b2 * z[1]
                         - f->a1 * z[2] - f->a2 * z[3];
        z[1] = z[0];
        z[0] = x;
        z[3] = z[2];
        z[2] = y;
        x = y;
    }
    return x;
}

G6EqStatus g6_eq_process_interleaved(const G6EqChain *chain,
                                     G6EqChannelState *states,
                                     float *samples, size_t frames,
                                     size_t channels)
{
    if (chain == NULL || chain->sample_rate == 0 ||
        chain->filter_count > G6_EQ_BAND_COUNT || channels == 0 ||
        frames > SIZE_MAX / channels / sizeof(*samples)) {
        return G6_EQ_INVALID_ARGUMENT;
    }
    if (frames != 0 && (states == NULL || samples == NULL)) {
        return G6_EQ_INVALID_ARGUMENT;
    }

    for (size_t frame = 0; frame < frames; ++frame) {
        for (size_t channel = 0; channel < channels; ++channel) {
            float *sample = &samples[frame * channels + channel];
            if (!isfinite(*sample)) {
                return G6_EQ_SAMPLE_RANGE;
            }
            const double value = g6_eq_process_sample(chain, &states[channel], *sample);
            if (!isfinite(value) || fabs(value) > FLT_MAX) {
                return G6_EQ_SAMPLE_RANGE;
            }
            *sample = (float)value;
        }
    }
    return G6_EQ_OK;
}

double g6_eq_response_db(const G6EqChain *chain, double frequency_hz)
{
    if (chain == NULL || chain->sample_rate == 0 ||
        chain->filter_count > G6_EQ_BAND_COUNT || isnan(frequency_hz)) {
        return NAN;
    }
    const double f = fmax(0.0, fmin(frequency_hz, chain->sample_rate / 2.0));
    const double w = tau * f / chain->sample_rate;
    const double c = cos(w);
    const double s = sin(w);
    const double z2r = c * c - s * s;
    const double z2i = -2.0 * c * s;
    double db = 20.0 * log10(chain->input_gain);
    for (size_t i = 0; i < chain->filter_count; ++i) {
        const G6EqBiquad *b = &chain->filters[i];
        const double nr = b->b0 + b->b1 * c + b->b2 * z2r;
        const double ni = b->b1 * -s + b->b2 * z2i;
        const double dr = 1.0 + b->a1 * c + b->a2 * z2r;
        const double di = b->a1 * -s + b->a2 * z2i;
        db += 20.0 * log10(sqrt((nr * nr + ni * ni) / (dr * dr + di * di)));
    }
    return db;
}

static G6EqStatus apo_status(int status)
{
    switch (status) {
    case G6_EQ_APO_OK:
        return G6_EQ_OK;
    case G6_EQ_APO_NYQUIST:
        return G6_EQ_NYQUIST;
    case G6_EQ_APO_INVALID_ARGUMENT:
        return G6_EQ_INVALID_ARGUMENT;
    default:
        return G6_EQ_GAIN_RANGE;
    }
}

G6EqStatus g6_eq_apo_chain_design(G6EqApoChain *out, const G6EqSettings *settings,
                                  uint32_t sample_rate, double extra_gain_db)
{
    if (out == NULL || settings == NULL || sample_rate == 0) {
        return G6_EQ_INVALID_ARGUMENT;
    }

    const double total_db = settings->preamp_db + extra_gain_db;
    if (!isfinite(total_db)) {
        return G6_EQ_GAIN_RANGE;
    }
    const float gain = (float)pow(10.0, total_db / 20.0);
    if (!isfinite(gain) || gain <= 0.0f) {
        return G6_EQ_GAIN_RANGE;
    }

    G6EqApoChain design = { 0 };
    design.input_gain = gain;
    design.sample_rate = sample_rate;
    for (size_t i = 0; i < G6_EQ_BAND_COUNT; ++i) {
        const double band_db = settings->gain_db[i];
        if (!isfinite(band_db)) {
            return G6_EQ_GAIN_RANGE;
        }
        if (band_db == 0.0) {
            continue;
        }
        const float frequency = (float)g6_eq_band_hz[i];
        int status;
        if (i == 0) {
            status = g6_eq_apo_low_shelf_db((float)band_db, (float)sample_rate,
                                            frequency,
                                            &design.filters[design.filter_count]);
        } else if (i + 1 == G6_EQ_BAND_COUNT) {
            status = g6_eq_apo_high_shelf_db((float)band_db, (float)sample_rate,
                                             frequency,
                                             &design.filters[design.filter_count]);
        } else {
            const float ratio =
                (float)(g6_eq_band_hz[i + 1] / g6_eq_band_hz[i]);
            status = g6_eq_apo_peak_db((float)band_db, (float)sample_rate,
                                       frequency, ratio,
                                       &design.filters[design.filter_count]);
        }
        if (status != G6_EQ_APO_OK) {
            return apo_status(status);
        }
        ++design.filter_count;
    }
    *out = design;
    return G6_EQ_OK;
}

void g6_eq_apo_reset_channel_state(G6EqApoChannelState *state)
{
    if (state != NULL) {
        memset(state, 0, sizeof(*state));
    }
}

float g6_eq_apo_process_chain_sample(const G6EqApoChain *chain,
                                     G6EqApoChannelState *state, float input)
{
    float x = input * chain->input_gain;
    for (size_t i = 0; i < chain->filter_count; ++i) {
        x = g6_eq_apo_process(&chain->filters[i], &state->bands[i], x);
    }
    return x;
}

G6EqStatus g6_eq_apo_process_chain_interleaved(const G6EqApoChain *chain,
                                               G6EqApoChannelState *states,
                                               float *samples, size_t frames,
                                               size_t channels)
{
    if (chain == NULL || chain->sample_rate == 0 ||
        chain->filter_count > G6_EQ_BAND_COUNT || channels == 0 ||
        frames > SIZE_MAX / channels / sizeof(*samples)) {
        return G6_EQ_INVALID_ARGUMENT;
    }
    if (frames != 0 && (states == NULL || samples == NULL)) {
        return G6_EQ_INVALID_ARGUMENT;
    }

    for (size_t frame = 0; frame < frames; ++frame) {
        for (size_t channel = 0; channel < channels; ++channel) {
            float *sample = &samples[frame * channels + channel];
            if (!isfinite(*sample)) {
                return G6_EQ_SAMPLE_RANGE;
            }
            const float value =
                g6_eq_apo_process_chain_sample(chain, &states[channel], *sample);
            if (!isfinite(value)) {
                return G6_EQ_SAMPLE_RANGE;
            }
            *sample = value;
        }
    }
    return G6_EQ_OK;
}
